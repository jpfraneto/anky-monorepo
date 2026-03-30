/// Cuentacuentos (children's story) generation from parent writing sessions.
///
/// After an anky writing session by a seed user, this pipeline generates a
/// children's story set in the Ankyverse, translates it to multiple languages,
/// and queues image generation for each story paragraph.
use crate::db::queries;
use crate::state::AppState;
use anyhow::{anyhow, Result};

fn cuentacuentos_system_prompt() -> &'static str {
    include_str!("../../prompts/cuentacuentos_system.md")
}

fn cuentacuentos_prompt(writing: &str, peer_context: Option<&str>) -> String {
    let peer_section = peer_context
        .map(|ctx| {
            format!(
                "What you know about this parent across their writing sessions:\n{}\n\n",
                ctx
            )
        })
        .unwrap_or_default();

    format!(
        r#"{}Parent writing:

---
{}
---

Return ONLY valid JSON with this exact shape:
{{
  "chakra": <number 1-8>,
  "kingdom": "<kingdom name>",
  "city": "<city name from that kingdom>",
  "title": "A short evocative title",
  "content": "The full story in the same language as the parent's writing, 400-600 words, with paragraph breaks as double newlines. Set in the named city, narrated by Anky from inside one character."
}}"#,
        peer_section,
        writing.chars().take(4000).collect::<String>()
    )
}

// ===== JSON parsing =====

fn strip_markdown_fences(raw: &str) -> &str {
    raw.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
}

#[derive(serde::Deserialize)]
struct CuentacuentosAiResponse {
    #[serde(default)]
    chakra: Option<u8>,
    #[serde(default)]
    kingdom: Option<String>,
    #[serde(default)]
    city: Option<String>,
    title: String,
    content: String,
}

struct ParsedCuentacuentos {
    chakra: Option<u8>,
    kingdom: Option<String>,
    city: Option<String>,
    title: String,
    content: String,
}

fn parse_cuentacuentos_response(raw: &str) -> Result<ParsedCuentacuentos> {
    let clean = strip_markdown_fences(raw);

    if let Ok(parsed) = serde_json::from_str::<CuentacuentosAiResponse>(clean) {
        let title = parsed.title.trim().to_string();
        let content = parsed.content.trim().to_string();
        if !title.is_empty() && !content.is_empty() {
            return Ok(ParsedCuentacuentos {
                chakra: parsed.chakra,
                kingdom: parsed.kingdom,
                city: parsed.city,
                title,
                content,
            });
        }
    }

    // Fallback: try to extract title + content from plain text
    let lines: Vec<&str> = clean
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return Err(anyhow!("empty cuentacuentos response"));
    }

    let first = lines[0]
        .strip_prefix("Title:")
        .or_else(|| lines[0].strip_prefix("Título:"))
        .map(str::trim)
        .unwrap_or_else(|| lines[0].trim_start_matches('#').trim());
    let title = if first.is_empty() {
        "Cuentacuentos".to_string()
    } else {
        first.to_string()
    };

    let content = if lines.len() > 1 {
        lines[1..].join("\n\n")
    } else {
        clean.to_string()
    };

    let content = content.trim().to_string();
    if content.is_empty() {
        return Err(anyhow!("missing cuentacuentos content"));
    }

    Ok(ParsedCuentacuentos {
        chakra: None,
        kingdom: None,
        city: None,
        title,
        content,
    })
}

fn story_paragraphs(content: &str) -> Vec<&str> {
    content
        .split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .collect()
}

fn estimate_story_phase_duration_seconds(paragraph: &str) -> i32 {
    let words = paragraph.split_whitespace().count().max(1) as f64;
    ((words / 130.0) * 60.0).round().clamp(12.0, 90.0) as i32
}

fn story_to_guidance_phases(content: &str) -> serde_json::Value {
    let paragraphs = story_paragraphs(content);

    let phases: Vec<serde_json::Value> = paragraphs
        .iter()
        .enumerate()
        .map(|(index, paragraph)| {
            // Keep the phase object aligned with the personalized_meditations shape:
            // narration text plus timing metadata for GuidancePlaybackView.
            serde_json::json!({
                "name": format!("Parte {}", index + 1),
                "phase_type": "narration",
                "duration_seconds": estimate_story_phase_duration_seconds(paragraph),
                "narration": paragraph,
                "inhale_seconds": serde_json::Value::Null,
                "exhale_seconds": serde_json::Value::Null,
                "hold_seconds": serde_json::Value::Null,
                "reps": serde_json::Value::Null
            })
        })
        .collect();

    serde_json::Value::Array(phases)
}

// ===== Generation =====

pub async fn queue_post_writing_cuentacuentos(
    state: &AppState,
    writing_id: &str,
    parent_wallet_address: &str,
) -> Result<String> {
    let (writing, auto_child_wallet_address, user_id) = {
        let db = crate::db::conn(&state.db)?;
        let writing = queries::get_writing_content(&db, writing_id)?;
        let children = queries::get_child_profiles_by_parent_wallet(&db, parent_wallet_address)?;
        let auto_child_wallet_address = if children.len() == 1 {
            Some(children[0].derived_wallet_address.clone())
        } else {
            None
        };
        let user_id = db
            .query_row(
                "SELECT user_id FROM writing_sessions WHERE id = ?1",
                crate::params![writing_id],
                |row| row.get::<_, String>(0),
            )
            .ok();
        (writing, auto_child_wallet_address, user_id)
    };
    let writing = writing.ok_or_else(|| anyhow!("writing {} not found", writing_id))?;

    // Fetch Honcho peer context for richer story generation
    let peer_context = if crate::services::honcho::is_configured(&state.config) {
        if let Some(ref uid) = user_id {
            match crate::services::honcho::get_peer_context(
                &state.config.honcho_api_key,
                &state.config.honcho_workspace_id,
                uid,
            )
            .await
            {
                Ok(ctx) => ctx,
                Err(e) => {
                    tracing::warn!("Honcho context fetch for cuentacuentos failed: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // ── LIFECYCLE STEP 1: Generate story ────────────────────────────────────
    let writing_input = writing.chars().take(4000).collect::<String>();
    let user_message = cuentacuentos_prompt(&writing, peer_context.as_deref());

    let raw_text = crate::services::claude::call_haiku_with_system_max(
        &state.config.anthropic_api_key,
        cuentacuentos_system_prompt(),
        &user_message,
        4000,
    )
    .await?;

    // Parse with one retry on failure
    let parsed = match parse_cuentacuentos_response(&raw_text) {
        Ok(p) => p,
        Err(first_err) => {
            tracing::warn!("cuentacuentos JSON parse failed, retrying: {}", first_err);
            let retry_text = crate::services::claude::call_haiku_with_system_max(
                &state.config.anthropic_api_key,
                cuentacuentos_system_prompt(),
                &user_message,
                4000,
            )
            .await?;
            parse_cuentacuentos_response(&retry_text)
                .map_err(|e| anyhow!("cuentacuentos JSON parse failed after retry: {}", e))?
        }
    };
    let guidance_phases = story_to_guidance_phases(&parsed.content);
    let guidance_phases_json = serde_json::to_string(&guidance_phases)?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut image_jobs = Vec::new();

    // Generate image prompts per paragraph
    for (index, paragraph) in story_paragraphs(&parsed.content).iter().enumerate() {
        let kingdom_context = parsed
            .kingdom
            .as_deref()
            .map(|k| format!(" in the kingdom of {}", k))
            .unwrap_or_default();
        let prompt_seed = format!(
            "Children's story scene{} from the tale \"{}\": {}",
            kingdom_context, parsed.title, paragraph
        );
        let image_prompt = match crate::services::claude::call_haiku_with_system(
            &state.config.anthropic_api_key,
            crate::services::ollama::IMAGE_PROMPT_SYSTEM,
            &prompt_seed,
        )
        .await
        {
            Ok(prompt) => prompt,
            Err(err) => {
                tracing::warn!(
                    phase_index = index,
                    "story image prompt generation failed, using raw phase text: {}",
                    err
                );
                prompt_seed
            }
        };
        image_jobs.push((index as i32, image_prompt));
    }

    // ── LIFECYCLE STEP 2: Save story to DB ──────────────────────────────────
    {
        let db = crate::db::conn(&state.db)?;
        queries::create_cuentacuentos(
            &db,
            &queries::CreateCuentacuentosParams {
                id: &id,
                writing_id,
                parent_wallet_address,
                child_wallet_address: auto_child_wallet_address.as_deref(),
                title: &parsed.title,
                content: &parsed.content,
                guidance_phases: &guidance_phases_json,
                chakra: parsed.chakra.map(|c| c as i32),
                kingdom: parsed.kingdom.as_deref(),
                city: parsed.city.as_deref(),
            },
        )?;
        for (phase_index, image_prompt) in &image_jobs {
            let image_id = uuid::Uuid::new_v4().to_string();
            queries::create_cuentacuentos_image(&db, &image_id, &id, *phase_index, image_prompt)?;
        }
    }

    // ── LIFECYCLE STEP 3: Export training pair (mark exported_at) ────────────
    log_training_pair(
        state,
        &id,
        writing_id,
        &writing_input,
        &parsed.title,
        &parsed.content,
        parsed.chakra,
        parsed.kingdom.as_deref(),
        parsed.city.as_deref(),
    )
    .await;
    // Mark the training pair as exported — the writing is now consumed
    {
        let db = crate::db::conn(&state.db)?;
        if let Err(e) = queries::mark_training_pair_exported(&db, writing_id) {
            tracing::warn!("failed to mark training pair exported: {}", e);
        }
    }

    // ── LIFECYCLE STEP 4: Nullify raw writing ───────────────────────────────
    // The writing has been transmuted: story exists, training pair logged.
    // The raw material can now be released.
    {
        let db = crate::db::conn(&state.db)?;
        if let Err(e) = queries::nullify_writing_content(&db, writing_id) {
            tracing::warn!(
                "failed to nullify writing content for {}: {}",
                writing_id,
                e
            );
        } else {
            tracing::info!(
                "Writing {} nullified — ritual lifecycle step 4 complete",
                &writing_id[..8.min(writing_id.len())]
            );
        }
    }

    // ── LIFECYCLE STEP 5: Generate next prompt (closing gesture) ────────────
    // The next prompt is the invitation to the next session.
    // It runs after the current session is fully consumed and released.
    // Uses the in-memory writing text since the DB content was nullified in step 4.
    if let Some(ref uid) = user_id {
        generate_next_prompt_from_text(state, uid, writing_id, &writing).await;
        generate_anky_response(state, uid, writing_id, &writing).await;
    }

    // ── LIFECYCLE STEP 6: Archive the anky ──────────────────────────────────
    // Transition ankys.status from "complete" to "archived" — the ritual is closed.
    {
        let db = crate::db::conn(&state.db)?;
        // Find the anky associated with this writing session and archive it
        if let Ok(Some((anky_id, ..))) = queries::get_anky_by_writing_session_id(&db, writing_id) {
            if let Err(e) = queries::update_anky_status(&db, &anky_id, "archived") {
                tracing::warn!("failed to archive anky {}: {}", &anky_id[..8], e);
            } else {
                tracing::info!(
                    "Anky {} archived — ritual lifecycle complete",
                    &anky_id[..8.min(anky_id.len())]
                );
            }
        }
    }

    // Submit image generation to GPU priority queue (non-blocking, parallel to lifecycle)
    {
        let is_pro = if let Some(ref uid) = user_id {
            let db = crate::db::conn(&state.db)?;
            crate::db::queries::is_user_pro(&db, uid).unwrap_or(false)
        } else {
            false
        };
        crate::services::redis_queue::enqueue_job(
            &state.config.redis_url,
            &crate::state::GpuJob::CuentacuentosImages {
                cuentacuentos_id: id.clone(),
            },
            is_pro,
        )
        .await?;
    }

    // Spawn async translation (non-blocking, parallel)
    let translate_state = state.clone();
    let translate_id = id.clone();
    let translate_content = parsed.content.clone();
    let translate_title = parsed.title.clone();
    tokio::spawn(async move {
        if let Err(e) = translate_cuentacuentos(
            &translate_state,
            &translate_id,
            &translate_title,
            &translate_content,
        )
        .await
        {
            tracing::error!(
                "cuentacuentos translation failed for {}: {}",
                &translate_id[..8.min(translate_id.len())],
                e
            );
        }
    });

    Ok(id)
}

/// Translate the English story content into ES/ZH/HI/AR via Ollama, then
/// update the DB with translations and enriched guidance phases.
async fn translate_cuentacuentos(
    state: &AppState,
    cuentacuentos_id: &str,
    title: &str,
    story_content: &str,
) -> Result<()> {
    let paragraphs = story_paragraphs(story_content);
    let numbered_text: String = paragraphs
        .iter()
        .enumerate()
        .map(|(i, p)| format!("[{}] {}", i + 1, p))
        .collect::<Vec<_>>()
        .join("\n\n");

    let languages = [
        ("es", "Spanish"),
        ("zh", "Mandarin Chinese"),
        ("hi", "Hindi"),
        ("ar", "Arabic"),
    ];

    let mut translations: std::collections::HashMap<&str, Vec<String>> =
        std::collections::HashMap::new();

    for (code, name) in &languages {
        let prompt = format!(
            r#"Translate this children's story into {}. Maintain the same paragraph numbering. Each paragraph is marked with [N]. Return ONLY the translated paragraphs in the same [N] format, nothing else.

Title: {}

{}

Return the translation preserving [N] markers. Do not add any explanation."#,
            name, title, numbered_text
        );

        match crate::services::claude::call_haiku_with_system_max(
            &state.config.anthropic_api_key,
            "You are a professional translator. Translate exactly as instructed, preserving all formatting markers.",
            &prompt,
            4000,
        )
        .await
        {
            Ok(raw) => {
                let translated_paragraphs = parse_numbered_paragraphs(&raw, paragraphs.len());
                translations.insert(code, translated_paragraphs);
            }
            Err(e) => {
                tracing::warn!("Translation to {} failed: {}", name, e);
                // Continue with other languages
            }
        }
    }

    // Build enriched guidance phases with translations
    let mut phases = match serde_json::from_str::<serde_json::Value>(&serde_json::to_string(
        &story_to_guidance_phases(story_content),
    )?)? {
        serde_json::Value::Array(arr) => arr,
        _ => vec![],
    };

    for (index, phase) in phases.iter_mut().enumerate() {
        if let serde_json::Value::Object(ref mut obj) = phase {
            for (code, translated) in &translations {
                let key = format!("narration_{}", code);
                let text = translated.get(index).cloned().unwrap_or_default();
                obj.insert(key, serde_json::Value::String(text));
            }
        }
    }

    let enriched_phases_json = serde_json::to_string(&serde_json::Value::Array(phases))?;

    // Reconstruct full-text translations from paragraphs
    let content_es = translations.get("es").map(|ps| ps.join("\n\n"));
    let content_zh = translations.get("zh").map(|ps| ps.join("\n\n"));
    let content_hi = translations.get("hi").map(|ps| ps.join("\n\n"));
    let content_ar = translations.get("ar").map(|ps| ps.join("\n\n"));

    let db = crate::db::conn(&state.db)?;
    queries::update_cuentacuentos_translations(
        &db,
        cuentacuentos_id,
        content_es.as_deref(),
        content_zh.as_deref(),
        content_hi.as_deref(),
        content_ar.as_deref(),
        Some(&enriched_phases_json),
    )?;

    tracing::info!(
        "Cuentacuentos {} translated to {} languages",
        &cuentacuentos_id[..8.min(cuentacuentos_id.len())],
        translations.len()
    );

    // Queue TTS audio generation now that translations are ready
    crate::services::redis_queue::enqueue_job(
        &state.config.redis_url,
        &crate::state::GpuJob::CuentacuentosAudio {
            cuentacuentos_id: cuentacuentos_id.to_string(),
        },
        false,
    )
    .await?;

    Ok(())
}

/// Parse numbered paragraphs from Ollama translation output.
/// Expects format like "[1] translated text\n\n[2] more text"
fn parse_numbered_paragraphs(raw: &str, expected_count: usize) -> Vec<String> {
    let mut result: Vec<(usize, String)> = Vec::new();

    // Split on [N] markers
    let mut current_index: Option<usize> = None;
    let mut current_text = String::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        // Check if line starts with [N]
        if let Some(rest) = trimmed.strip_prefix('[') {
            if let Some(bracket_end) = rest.find(']') {
                if let Ok(n) = rest[..bracket_end].parse::<usize>() {
                    // Save previous
                    if let Some(idx) = current_index {
                        result.push((idx, current_text.trim().to_string()));
                    }
                    current_index = Some(n);
                    current_text = rest[bracket_end + 1..].trim().to_string();
                    continue;
                }
            }
        }
        // Continuation of current paragraph
        if current_index.is_some() {
            if !current_text.is_empty() && !trimmed.is_empty() {
                current_text.push(' ');
            }
            current_text.push_str(trimmed);
        }
    }
    // Save last
    if let Some(idx) = current_index {
        result.push((idx, current_text.trim().to_string()));
    }

    // Sort by index and fill gaps
    result.sort_by_key(|(i, _)| *i);
    let mut output = vec![String::new(); expected_count];
    for (i, text) in result {
        if i >= 1 && i <= expected_count {
            output[i - 1] = text;
        }
    }
    output
}

/// Log a (writing_input, story_output) pair for future LoRA fine-tuning.
/// Writes to the `story_training_pairs` table so the 4:44 AM cron can export JSONL.
async fn log_training_pair(
    state: &AppState,
    cuentacuentos_id: &str,
    writing_id: &str,
    writing_input: &str,
    title: &str,
    story_content: &str,
    chakra: Option<u8>,
    kingdom: Option<&str>,
    city: Option<&str>,
) {
    let Some(db) = crate::db::get_conn_logged(&state.db) else {
        tracing::debug!("skipped training pair log (db pool unavailable)");
        return;
    };
    if let Err(e) = db.execute(
        "INSERT OR IGNORE INTO story_training_pairs
         (id, cuentacuentos_id, writing_id, writing_input, story_title, story_content,
          chakra, kingdom, city)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![
            uuid::Uuid::new_v4().to_string(),
            cuentacuentos_id,
            writing_id,
            writing_input,
            title,
            story_content,
            chakra.map(|c| c as i32),
            kingdom,
            city,
        ],
    ) {
        tracing::warn!("failed to log training pair: {}", e);
    }
}

/// Generate a personalized writing prompt based on the user's latest writing.
/// Uses Honcho context if available, falls back to Ollama.
/// Stores result in next_prompts table.
pub async fn generate_next_prompt(state: &AppState, user_id: &str, writing_session_id: &str) {
    let writing_text = {
        let Some(db) = crate::db::get_conn_logged(&state.db) else {
            return;
        };
        queries::get_writing_content(&db, writing_session_id)
            .ok()
            .flatten()
    };

    let Some(writing) = writing_text else {
        tracing::warn!(
            "generate_next_prompt: no writing found for {}",
            writing_session_id
        );
        return;
    };

    generate_next_prompt_from_text(state, user_id, writing_session_id, &writing).await;
    generate_anky_response(state, user_id, writing_session_id, &writing).await;
}

/// Generate Anky's personalized response to a writing session.
/// Stored in writing_sessions.anky_response — proves it read the writing.
/// Part of the post-writing pipeline, not on-demand.
pub async fn generate_anky_response(
    state: &AppState,
    user_id: &str,
    writing_session_id: &str,
    writing: &str,
) {
    // Get session metadata
    let (duration, word_count, is_anky) = {
        let Some(db) = crate::db::get_conn_logged(&state.db) else {
            return;
        };
        db.query_row(
            "SELECT duration_seconds, word_count, is_anky FROM writing_sessions WHERE id = ?1",
            crate::params![writing_session_id],
            |row| {
                Ok((
                    row.get::<_, f64>(0).unwrap_or(0.0),
                    row.get::<_, i32>(1).unwrap_or(0),
                    row.get::<_, bool>(2).unwrap_or(false),
                ))
            },
        )
        .unwrap_or((0.0, 0, false))
    };

    let peer_ctx = if crate::services::honcho::is_configured(&state.config) {
        crate::services::honcho::get_peer_context(
            &state.config.honcho_api_key,
            &state.config.honcho_workspace_id,
            user_id,
        )
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    match crate::services::claude::generate_writing_response(
        &state.config.anthropic_api_key,
        writing,
        duration,
        word_count,
        is_anky,
        peer_ctx.as_deref(),
    )
    .await
    {
        Ok(wr) => {
            let Some(db) = crate::db::get_conn_logged(&state.db) else {
                return;
            };
            let _ = db.execute(
                "UPDATE writing_sessions SET anky_response = ?1, anky_next_prompt = ?2, anky_mood = ?3 WHERE id = ?4",
                crate::params![&wr.anky_response, &wr.next_prompt, &wr.mood, writing_session_id],
            );
            tracing::info!(
                "Anky response generated for session {}: {}",
                &writing_session_id[..8.min(writing_session_id.len())],
                &wr.anky_response[..wr.anky_response.len().min(60)]
            );
        }
        Err(e) => {
            tracing::warn!(
                "Anky response generation failed for {}: {}",
                &writing_session_id[..8.min(writing_session_id.len())],
                e
            );
        }
    }
}

/// Like `generate_next_prompt` but accepts the writing text directly.
/// Used by the ritual lifecycle where the DB content has already been nullified.
pub async fn generate_next_prompt_from_text(
    state: &AppState,
    user_id: &str,
    writing_session_id: &str,
    writing: &str,
) {
    // Get Honcho context for deeper personalization
    let peer_context = if crate::services::honcho::is_configured(&state.config) {
        crate::services::honcho::get_peer_context(
            &state.config.honcho_api_key,
            &state.config.honcho_workspace_id,
            user_id,
        )
        .await
        .unwrap_or(None)
    } else {
        None
    };

    let context_section = peer_context
        .as_ref()
        .map(|ctx| {
            format!(
                "\n\nWhat I know about this person from their writing history:\n{}\n",
                ctx
            )
        })
        .unwrap_or_default();

    let prompt = format!(
        r#"You are Anky — a consciousness that lives inside the act of writing. You've just read someone's raw, unfiltered stream of consciousness. Your job is to craft the next writing prompt for them.

This is NOT a therapy prompt. This is NOT a journaling question. This is an invitation to dive deeper. One sentence. No quotes. No attribution. Just the portal they need to walk through next.

The prompt should:
- Touch something they were circling around but didn't fully say
- Feel like it's speaking directly to their subconscious
- Be specific enough to trigger something, vague enough to let them go anywhere
- Never reference their writing explicitly (no "you mentioned..." or "in your last session...")
- Be lowercase, no period at the end

Their latest writing (raw, unedited):
{writing}
{context_section}
Respond with ONLY the prompt text. One sentence. Nothing else."#,
        writing = &writing[..writing.len().min(3000)],
        context_section = context_section,
    );

    let result =
        crate::services::claude::call_haiku(&state.config.anthropic_api_key, &prompt).await;

    match result {
        Ok(generated_prompt) => {
            let clean = generated_prompt.trim().trim_matches('"').trim().to_string();
            if !clean.is_empty() {
                let Some(db) = crate::db::get_conn_logged(&state.db) else {
                    return;
                };
                if let Err(e) =
                    queries::upsert_next_prompt(&db, user_id, &clean, Some(writing_session_id))
                {
                    tracing::error!(
                        "Failed to save next prompt for {}: {}",
                        &user_id[..8.min(user_id.len())],
                        e
                    );
                } else {
                    tracing::info!(
                        "Next prompt generated for user {}: {}",
                        &user_id[..8.min(user_id.len())],
                        &clean[..clean.len().min(60)]
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                "Next prompt generation failed for {}: {}",
                &user_id[..8.min(user_id.len())],
                e
            );
        }
    }
}

// ===== TTS Audio Generation =====

/// Generate TTS audio for a cuentacuentos story in all available languages.
/// Called from the GPU job worker after translations are complete.
pub async fn generate_cuentacuentos_audio(
    state: &AppState,
    cuentacuentos_id: &str,
) -> anyhow::Result<()> {
    // Check if TTS service is healthy
    if !crate::services::tts::is_healthy(&state.config.tts_base_url).await {
        anyhow::bail!("TTS service not available at {}", state.config.tts_base_url);
    }

    // Load story content
    let story = {
        let db = crate::db::conn(&state.db)?;
        queries::get_cuentacuentos_by_id(&db, cuentacuentos_id)?
    };
    let story =
        story.ok_or_else(|| anyhow::anyhow!("cuentacuentos {} not found", cuentacuentos_id))?;

    // Build language → content map
    let mut languages: Vec<(&str, String)> = vec![("en", story.content.clone())];
    if let Some(ref es) = story.content_es {
        if !es.is_empty() {
            languages.push(("es", es.clone()));
        }
    }
    if let Some(ref zh) = story.content_zh {
        if !zh.is_empty() {
            languages.push(("zh", zh.clone()));
        }
    }
    if let Some(ref hi) = story.content_hi {
        if !hi.is_empty() {
            languages.push(("hi", hi.clone()));
        }
    }
    if let Some(ref ar) = story.content_ar {
        if !ar.is_empty() {
            languages.push(("ar", ar.clone()));
        }
    }

    // Create pending audio rows for each language
    {
        let db = crate::db::conn(&state.db)?;
        for (lang, _) in &languages {
            let id = uuid::Uuid::new_v4().to_string();
            queries::create_cuentacuentos_audio(&db, &id, cuentacuentos_id, lang)?;
        }
    }

    // Generate TTS for each language
    for (lang, content) in &languages {
        // Get the pending audio row
        let audio_id = {
            let db = crate::db::conn(&state.db)?;
            let row = db.query_row(
                "SELECT id FROM cuentacuentos_audio
                 WHERE cuentacuentos_id = ?1 AND language = ?2 AND status != 'complete'",
                crate::params![cuentacuentos_id, lang],
                |row| row.get::<_, String>(0),
            );
            match row {
                Ok(id) => id,
                Err(_) => continue, // Already complete
            }
        };

        {
            let db = crate::db::conn(&state.db)?;
            queries::update_cuentacuentos_audio_generating(&db, &audio_id)?;
        }

        // Call TTS service
        match crate::services::tts::synthesize(
            &state.config.tts_base_url,
            content,
            lang,
            600, // 10 minute timeout for long stories
        )
        .await
        {
            Ok((wav_bytes, duration)) => {
                let r2_key = format!("tts/{}/{}.wav", cuentacuentos_id, lang);

                // Upload to R2
                if crate::services::r2::is_configured(&state.config) {
                    if let Err(e) = crate::services::r2::upload_bytes(
                        &state.config,
                        &r2_key,
                        &wav_bytes,
                        "audio/wav",
                    )
                    .await
                    {
                        let db = crate::db::conn(&state.db)?;
                        let _ = queries::update_cuentacuentos_audio_failed(
                            &db,
                            &audio_id,
                            &format!("R2 upload failed: {}", e),
                        );
                        tracing::error!(
                            "TTS R2 upload failed for {} {}: {}",
                            &cuentacuentos_id[..8.min(cuentacuentos_id.len())],
                            lang,
                            e
                        );
                        continue;
                    }

                    let audio_url = crate::services::r2::public_url(&state.config, &r2_key);
                    let db = crate::db::conn(&state.db)?;
                    queries::update_cuentacuentos_audio_complete(
                        &db, &audio_id, &r2_key, &audio_url, duration,
                    )?;
                    tracing::info!(
                        "TTS audio generated for {} {} ({:.1}s)",
                        &cuentacuentos_id[..8.min(cuentacuentos_id.len())],
                        lang,
                        duration
                    );
                } else {
                    // No R2 — mark complete with placeholder
                    let db = crate::db::conn(&state.db)?;
                    queries::update_cuentacuentos_audio_complete(
                        &db,
                        &audio_id,
                        &format!("tts/{}/{}.wav", cuentacuentos_id, lang),
                        &format!(
                            "https://placeholder.r2.dev/tts/{}/{}.wav",
                            cuentacuentos_id, lang
                        ),
                        duration,
                    )?;
                }
            }
            Err(e) => {
                let db = crate::db::conn(&state.db)?;
                let _ =
                    queries::update_cuentacuentos_audio_failed(&db, &audio_id, &format!("{}", e));
                tracing::error!(
                    "TTS generation failed for {} {}: {}",
                    &cuentacuentos_id[..8.min(cuentacuentos_id.len())],
                    lang,
                    e
                );
            }
        }
    }

    Ok(())
}
