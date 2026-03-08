/// AI-generated personalized meditation and breathwork sessions.
///
/// Premium users: Claude Haiku, generated immediately in the background.
/// Free users: Ollama (local qwen), processed by the queue worker in main.rs.
use crate::db::queries;
use crate::state::AppState;
use anyhow::Result;

// ===== Mood detection =====

/// Pick a breathwork style based on the emotional tone of the writing.
/// Returns one of: wim_hof, box, 4_7_8, pranayama, energizing, calming
pub fn detect_breathwork_style(writing: &str) -> &'static str {
    let text = writing.to_lowercase();

    let grief_loss = ["grief", "loss", "died", "death", "miss", "gone", "funeral", "mourn"];
    let anxiety = ["anxious", "anxiety", "panic", "worry", "scared", "fear", "overwhelm", "stress", "dread"];
    let anger = ["angry", "rage", "furious", "hate", "resentment", "frustrated", "mad"];
    let low_energy = ["tired", "exhausted", "drained", "empty", "numb", "flat", "foggy", "unmotivated"];
    let high_energy = ["excited", "electric", "alive", "energy", "fire", "passion", "burst", "surge"];
    let spiritual = ["soul", "spirit", "god", "universe", "presence", "consciousness", "divine", "sacred", "awakening", "meditation", "yoga"];

    let score = |words: &[&str]| words.iter().filter(|&&w| text.contains(w)).count();

    let grief_score = score(&grief_loss);
    let anxiety_score = score(&anxiety);
    let anger_score = score(&anger);
    let low_score = score(&low_energy);
    let high_score = score(&high_energy);
    let spiritual_score = score(&spiritual);

    let max = [grief_score, anxiety_score, anger_score, low_score, high_score, spiritual_score]
        .iter()
        .copied()
        .max()
        .unwrap_or(0);

    if max == 0 {
        return "box"; // neutral default
    }

    if grief_score == max { "calming" }
    else if anxiety_score == max { "4_7_8" }
    else if anger_score == max { "wim_hof" } // channel it outward
    else if low_score == max { "energizing" }
    else if high_score == max { "wim_hof" }
    else { "pranayama" } // spiritual
}

// ===== Prompts =====

fn meditation_system_prompt() -> &'static str {
    "You are Anky — a spiritual teacher, mirror of the unconscious, and compassionate guide. \
     You have just read someone's raw unfiltered writing. You respond by creating a personalized \
     guided meditation that meets them exactly where they are. \
     Output ONLY valid JSON — no markdown, no code fences, no explanation."
}

fn breathwork_system_prompt() -> &'static str {
    "You are Anky — a breathwork facilitator and spiritual guide. \
     You create structured breathwork sessions that match the emotional and energetic state \
     of the practitioner. Output ONLY valid JSON — no markdown, no code fences, no explanation."
}

fn meditation_prompt(writing: &str, memory_context: Option<&str>) -> String {
    let memory_section = memory_context
        .map(|m| format!("\n\nWhat you know about this person across time:\n{}", m))
        .unwrap_or_default();

    format!(
        r#"Someone just wrote this in a stream-of-consciousness practice:

---
{}
---
{}

Generate a 10-minute personalized guided meditation (600 seconds) as JSON.
The meditation should respond directly to what was expressed — the themes, tensions, emotions, images.
Not generic. Not about breathing techniques. A genuine spiritual response.

JSON structure:
{{
  "title": "A poetic title that reflects what was written",
  "description": "1-2 sentences on what this sit is for",
  "duration_seconds": 600,
  "background_beat_bpm": 40,
  "phases": [
    {{
      "name": "Phase name",
      "phase_type": "narration|breathing|hold|rest|body_scan|visualization",
      "duration_seconds": 60,
      "narration": "What Anky says. First person from Anky's voice. Warm, direct, non-spiritual-bypassing. References specific things from the writing.",
      "inhale_seconds": null,
      "exhale_seconds": null,
      "hold_seconds": null,
      "reps": null
    }}
  ]
}}

Rules:
- Phases sum to ~600 seconds
- Reference specific words, images, or feelings from the writing
- Don't solve their problems — hold space for them
- Include at least one breathing phase (even if brief)
- body_scan: guide attention through the body
- visualization: create an image or scene to sit in
- narration: speaking only
- rest: silence / integration (10-30s)
- The tone is warm, present, slightly mystical — never clinical
- 8-10 phases minimum"#,
        writing.chars().take(2000).collect::<String>(),
        memory_section
    )
}

fn breathwork_prompt(writing: &str, style: &str) -> String {
    let style_desc = match style {
        "wim_hof" => "Wim Hof Method: 3 rounds of 30 power breaths with retention holds. Activates the sympathetic nervous system, builds inner heat and resilience.",
        "box" => "Box Breathing: 4 counts inhale, 4 hold, 4 exhale, 4 hold. Regulates and centers the nervous system.",
        "4_7_8" => "4-7-8 Breathing: 4 counts inhale, 7 hold, 8 exhale. Powerful parasympathetic activation.",
        "pranayama" => "Yoga Pranayama: Nadi Shodhana and Kapalabhati. Balances prana, clears energy channels.",
        "energizing" => "Energizing: Bhastrika (bellows breath) and power breathing. Awakens and activates.",
        "calming" => "Extended exhale: inhale 4, exhale 8. Deep parasympathetic response. Soothes and grounds.",
        _ => "Box Breathing",
    };

    format!(
        r#"Someone just wrote this in a stream-of-consciousness practice:

---
{}
---

Their emotional state calls for: {} — {}

Generate an 8-minute personalized breathwork session (480 seconds) as JSON.
The narration should acknowledge their state and guide them through the practice.

JSON structure:
{{
  "title": "Evocative title",
  "description": "1-2 sentences",
  "style": "{}",
  "duration_seconds": 480,
  "background_beat_bpm": 60,
  "phases": [
    {{
      "name": "Phase name",
      "phase_type": "narration|breathing|hold|rest",
      "duration_seconds": 30,
      "narration": "Anky's guidance. References their emotional state without quoting their writing directly.",
      "inhale_seconds": null,
      "exhale_seconds": null,
      "hold_seconds": null,
      "reps": null
    }}
  ]
}}

Rules:
- Phases sum to ~480 seconds
- Opening narration acknowledges where they are emotionally (30-45s)
- Main practice follows the {} technique precisely
- Closing integration (30-45s)
- 6-8 phases minimum
- background_beat_bpm: match the energy (40-80)"#,
        writing.chars().take(1500).collect::<String>(),
        style,
        style_desc,
        style,
        style_desc
    )
}

fn generic_meditation_prompt() -> &'static str {
    r#"Generate a 10-minute general guided meditation (600 seconds) as JSON.
This is for someone who hasn't written yet today — meet them in the unknown.

JSON structure:
{
  "title": "A poetic title",
  "description": "1-2 sentences",
  "duration_seconds": 600,
  "background_beat_bpm": 40,
  "phases": [
    {
      "name": "Phase name",
      "phase_type": "narration|breathing|hold|rest|body_scan|visualization",
      "duration_seconds": 60,
      "narration": "Anky's guidance.",
      "inhale_seconds": null,
      "exhale_seconds": null,
      "hold_seconds": null,
      "reps": null
    }
  ]
}

Rules:
- Phases sum to ~600 seconds
- Open with settling and arriving
- Include breath awareness, body scan, and a visualization
- Close with integration
- 8-10 phases minimum
- Tone: warm, present, slightly mystical"#
}

fn generic_breathwork_prompt(style: &str) -> String {
    format!(
        r#"Generate an 8-minute {} breathwork session (480 seconds) for general practice as JSON.

JSON structure:
{{
  "title": "Evocative title",
  "description": "1-2 sentences",
  "style": "{}",
  "duration_seconds": 480,
  "background_beat_bpm": 60,
  "phases": [
    {{
      "name": "Phase name",
      "phase_type": "narration|breathing|hold|rest",
      "duration_seconds": 30,
      "narration": "Anky's guidance.",
      "inhale_seconds": null,
      "exhale_seconds": null,
      "hold_seconds": null,
      "reps": null
    }}
  ]
}}

Phases sum to ~480 seconds. 6-8 phases. Warm, grounding tone."#,
        style, style
    )
}

// ===== JSON parsing =====

fn parse_script(raw: &str) -> serde_json::Value {
    let clean = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(clean).unwrap_or(serde_json::json!({ "error": "parse failed" }))
}

// ===== Generation =====

pub async fn generate_meditation_premium(
    state: &AppState,
    job_id: &str,
    writing: Option<&str>,
    memory_context: Option<&str>,
) {
    let prompt = match writing {
        Some(w) => meditation_prompt(w, memory_context),
        None => generic_meditation_prompt().to_string(),
    };

    let result = crate::services::claude::call_claude_public(
        &state.config.anthropic_api_key,
        "claude-haiku-4-5-20251001",
        meditation_system_prompt(),
        &prompt,
        4000,
    )
    .await;

    match result {
        Ok(r) => {
            let script = parse_script(&r.text);
            let json = serde_json::to_string(&script).unwrap_or_default();
            let db = state.db.lock().await;
            let _ = queries::set_meditation_script(&db, job_id, &json, "ready");
        }
        Err(e) => {
            tracing::error!("Meditation generation failed for {}: {}", &job_id[..8], e);
            let db = state.db.lock().await;
            let _ = queries::set_meditation_status(&db, job_id, "failed");
        }
    }
}

pub async fn generate_breathwork_premium(
    state: &AppState,
    job_id: &str,
    style: &str,
    writing: Option<&str>,
) {
    let prompt = match writing {
        Some(w) => breathwork_prompt(w, style),
        None => generic_breathwork_prompt(style),
    };

    let result = crate::services::claude::call_claude_public(
        &state.config.anthropic_api_key,
        "claude-haiku-4-5-20251001",
        breathwork_system_prompt(),
        &prompt,
        3000,
    )
    .await;

    match result {
        Ok(r) => {
            let script = parse_script(&r.text);
            let json = serde_json::to_string(&script).unwrap_or_default();
            let db = state.db.lock().await;
            let _ = queries::set_breathwork_script(&db, job_id, &json, "ready");
        }
        Err(e) => {
            tracing::error!("Breathwork generation failed for {}: {}", &job_id[..8], e);
            let db = state.db.lock().await;
            let _ = queries::set_breathwork_status(&db, job_id, "failed");
        }
    }
}

pub async fn generate_meditation_free(
    state: &AppState,
    job_id: &str,
    writing: Option<&str>,
) {
    let prompt = match writing {
        Some(w) => meditation_prompt(w, None),
        None => generic_meditation_prompt().to_string(),
    };

    // Wrap in instruction for Ollama to output JSON
    let full_prompt = format!(
        "Output only valid JSON, no markdown.\n\n{}",
        prompt
    );

    let result = crate::services::ollama::call_ollama(
        &state.config.ollama_base_url,
        &state.config.ollama_model,
        &full_prompt,
    )
    .await;

    match result {
        Ok(r) => {
            let script = parse_script(&r);
            let json = serde_json::to_string(&script).unwrap_or_default();
            let db = state.db.lock().await;
            let _ = queries::set_meditation_script(&db, job_id, &json, "ready");
        }
        Err(e) => {
            tracing::error!("Free meditation generation failed for {}: {}", &job_id[..8], e);
            let db = state.db.lock().await;
            let _ = queries::set_meditation_status(&db, job_id, "failed");
        }
    }
}

pub async fn generate_breathwork_free(
    state: &AppState,
    job_id: &str,
    style: &str,
    writing: Option<&str>,
) {
    let prompt = match writing {
        Some(w) => breathwork_prompt(w, style),
        None => generic_breathwork_prompt(style),
    };

    let full_prompt = format!("Output only valid JSON, no markdown.\n\n{}", prompt);

    let result = crate::services::ollama::call_ollama(
        &state.config.ollama_base_url,
        &state.config.ollama_model,
        &full_prompt,
    )
    .await;

    match result {
        Ok(r) => {
            let script = parse_script(&r);
            let json = serde_json::to_string(&script).unwrap_or_default();
            let db = state.db.lock().await;
            let _ = queries::set_breathwork_script(&db, job_id, &json, "ready");
        }
        Err(e) => {
            tracing::error!("Free breathwork generation failed for {}: {}", &job_id[..8], e);
            let db = state.db.lock().await;
            let _ = queries::set_breathwork_status(&db, job_id, "failed");
        }
    }
}

/// Queue a meditation + breathwork job after a writing session.
/// Called from the write handler — returns immediately, generation is async.
pub async fn queue_post_writing_guidance(
    state: &AppState,
    user_id: &str,
    writing_session_id: &str,
    writing_text: &str,
) {
    let is_premium = {
        let db = state.db.lock().await;
        queries::is_user_premium(&db, user_id).unwrap_or(false)
    };
    let tier = if is_premium { "premium" } else { "free" };

    let style = detect_breathwork_style(writing_text);

    // Create DB records
    let med_id = uuid::Uuid::new_v4().to_string();
    let bw_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().await;
        let _ = queries::create_personalized_meditation(
            &db, &med_id, user_id, Some(writing_session_id), tier,
        );
        let _ = queries::create_personalized_breathwork(
            &db, &bw_id, user_id, Some(writing_session_id), style, tier,
        );
    }

    let writing_owned = writing_text.to_string();
    let med_id_owned = med_id.clone();
    let bw_id_owned = bw_id.clone();
    let style_owned = style.to_string();
    let state_clone = state.clone();

    if is_premium {
        // Generate immediately in background with Claude
        tokio::spawn(async move {
            let s = state_clone.clone();
            let w = writing_owned.clone();
            let mid = med_id_owned.clone();
            tokio::spawn(async move {
                generate_meditation_premium(&s, &mid, Some(&w), None).await;
            });

            let s2 = state_clone.clone();
            tokio::spawn(async move {
                generate_breathwork_premium(&s2, &bw_id_owned, &style_owned, Some(&writing_owned)).await;
            });
        });
    } else {
        // Records are already inserted as 'pending' — the queue worker will pick them up
        tracing::info!(
            user = %user_id,
            "Free tier: meditation + breathwork queued for background generation"
        );
    }
}

/// Queue a generic daily meditation + breathwork for a user with no writing yet.
pub async fn queue_daily_guidance(state: &AppState, user_id: &str) {
    let is_premium = {
        let db = state.db.lock().await;
        queries::is_user_premium(&db, user_id).unwrap_or(false)
    };
    let tier = if is_premium { "premium" } else { "free" };

    let styles = ["box", "calming", "pranayama", "4_7_8", "energizing", "wim_hof"];
    use chrono::Datelike;
    let day_of_year = chrono::Utc::now().ordinal() as usize;
    let style = styles[day_of_year % styles.len()];

    let med_id = uuid::Uuid::new_v4().to_string();
    let bw_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().await;
        let _ = queries::create_personalized_meditation(&db, &med_id, user_id, None, tier);
        let _ = queries::create_personalized_breathwork(&db, &bw_id, user_id, None, style, tier);
    }

    if is_premium {
        let state_clone = state.clone();
        let mid = med_id.clone();
        tokio::spawn(async move {
            generate_meditation_premium(&state_clone, &mid, None, None).await;
        });
        let state_clone2 = state.clone();
        tokio::spawn(async move {
            generate_breathwork_premium(&state_clone2, &bw_id, style, None).await;
        });
    }
    // Free: queue worker picks it up
}

/// Background queue worker — processes one pending free-tier job at a time.
/// Run on a loop in main.rs every 60 seconds.
pub async fn process_free_queue(state: &AppState) -> Result<bool> {
    // Try meditation first
    let med_job = {
        let db = state.db.lock().await;
        queries::get_pending_free_meditation(&db)?
    };

    if let Some((id, user_id, writing_session_id)) = med_job {
        tracing::info!("Queue: processing free meditation {}", &id[..8]);
        {
            let db = state.db.lock().await;
            queries::set_meditation_status(&db, &id, "generating")?;
        }
        let writing = if let Some(ref sid) = writing_session_id {
            let db = state.db.lock().await;
            queries::get_writing_content(&db, sid).ok().flatten()
        } else {
            None
        };
        generate_meditation_free(state, &id, writing.as_deref()).await;
        state.emit_log("INFO", "queue", &format!("Free meditation ready for {}", &user_id[..8]));
        return Ok(true);
    }

    // Then breathwork
    let bw_job = {
        let db = state.db.lock().await;
        queries::get_pending_free_breathwork(&db)?
    };

    if let Some((id, user_id, writing_session_id, style)) = bw_job {
        tracing::info!("Queue: processing free breathwork {}", &id[..8]);
        {
            let db = state.db.lock().await;
            queries::set_breathwork_status(&db, &id, "generating")?;
        }
        let writing = if let Some(ref sid) = writing_session_id {
            let db = state.db.lock().await;
            queries::get_writing_content(&db, sid).ok().flatten()
        } else {
            None
        };
        generate_breathwork_free(state, &id, &style, writing.as_deref()).await;
        state.emit_log("INFO", "queue", &format!("Free breathwork ready for {}", &user_id[..8]));
        return Ok(true);
    }

    Ok(false) // nothing to process
}
