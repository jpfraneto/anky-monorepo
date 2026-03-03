use crate::error::AppError;
use crate::models::{WriteRequest, WriteResponse};
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Html;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar};

fn get_or_create_user_id(jar: &CookieJar, token_header: Option<&str>) -> (String, Option<Cookie<'static>>) {
    if let Some(cookie) = jar.get("anky_user_id") {
        return (cookie.value().to_string(), None);
    }
    // Farcaster webview fallback: cookie absent but localStorage token sent as header
    let id = if let Some(t) = token_header {
        let t = t.trim();
        // Accept only UUID-shaped tokens to prevent junk keys
        if t.len() == 36 && t.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            t.to_string()
        } else {
            uuid::Uuid::new_v4().to_string()
        }
    } else {
        uuid::Uuid::new_v4().to_string()
    };
    let cookie = Cookie::build(("anky_user_id", id.clone()))
        .max_age(time::Duration::days(365))
        .http_only(false)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .path("/")
        .build();
    (id, Some(cookie))
}

pub async fn process_writing(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<WriteRequest>,
) -> Result<(CookieJar, Json<WriteResponse>), AppError> {
    // Agent auth: if X-API-Key header is present, validate it.
    let api_key = headers.get("x-api-key").and_then(|v| v.to_str().ok());

    if let Some(key) = api_key {
        // Validate the API key
        let valid = {
            let db = state.db.lock().await;
            crate::db::queries::get_agent_by_key(&db, key)
                .ok()
                .flatten()
                .is_some()
        };
        if !valid {
            return Ok((
                jar,
                Json(WriteResponse {
                    response: String::new(),
                    duration: 0.0,
                    is_anky: false,
                    anky_id: None,
                    estimated_wait_seconds: None,
                    flow_score: None,
                    error: Some(
                        "invalid API key. register at POST /api/v1/register to get one.".into(),
                    ),
                }),
            ));
        }
    }

    // Rate limit by IP (CF-Connecting-IP behind Cloudflare, fall back to cookie user id)
    let rate_key = headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            jar.get("anky_user_id")
                .map(|c| c.value().to_string())
                .unwrap_or_else(|| "anonymous".into())
        });

    if let Err(retry_after) = state.write_limiter.check(&rate_key).await {
        tracing::warn!(key = %rate_key, "Rate limited on /write");
        return Err(AppError::RateLimited(retry_after));
    }

    let token_header = headers.get("x-anky-user-token").and_then(|v| v.to_str().ok());
    let (user_id, new_cookie) = get_or_create_user_id(&jar, token_header);
    let jar = if let Some(c) = new_cookie {
        jar.add(c)
    } else {
        jar
    };

    let word_count = req.text.split_whitespace().count() as i32;

    // Reject submissions with fewer than 10 words — no DB save, no Ollama call
    if word_count < 10 {
        return Ok((
            jar,
            Json(WriteResponse {
                response: String::new(),
                duration: req.duration,
                is_anky: false,
                anky_id: None,
                estimated_wait_seconds: None,
                flow_score: None,
                error: Some("write more. stream-of-consciousness means letting words flow — at least a few sentences.".into()),
            }),
        ));
    }

    // Compute flow score from keystroke deltas
    let flow_score = req
        .keystroke_deltas
        .as_ref()
        .map(|deltas| crate::db::queries::calculate_flow_score(deltas, req.duration, word_count));
    let keystroke_json = req
        .keystroke_deltas
        .as_ref()
        .map(|d| serde_json::to_string(d).unwrap_or_default());

    let mut is_anky = req.duration >= 480.0;

    // Downgrade: 8+ minutes but fewer than 300 words is not a real anky
    if is_anky && word_count < 300 {
        is_anky = false;
    }

    let session_id = uuid::Uuid::new_v4().to_string();

    let mins = (req.duration / 60.0) as u32;
    let secs = (req.duration % 60.0) as u32;
    tracing::info!(
        user = %user_id,
        duration = format!("{}m{}s", mins, secs),
        words = word_count,
        is_anky = is_anky,
        "Processing writing session"
    );

    state.emit_log(
        "INFO",
        "writing",
        &format!(
            "New session: {}m{}s, {} words, anky={}",
            mins, secs, word_count, is_anky
        ),
    );

    // Save to DB FIRST — before Ollama call — so writing is never lost
    {
        let db = state.db.lock().await;
        crate::db::queries::ensure_user(&db, &user_id)?;
        crate::db::queries::insert_writing_session_with_flow(
            &db,
            &session_id,
            &user_id,
            &req.text,
            req.duration,
            word_count,
            is_anky,
            None, // response filled in after Ollama
            keystroke_json.as_deref(),
            flow_score,
        )?;
        // Update leaderboard stats
        if let Some(fs) = flow_score {
            let _ = crate::db::queries::update_user_flow_stats(&db, &user_id, fs, is_anky);
        }
    }

    // Call Ollama for feedback (writing is already saved above)
    // For anky sessions, skip the blocking Ollama call — return immediately so the
    // frontend can start streaming the Claude reflection ASAP.
    let response = if is_anky {
        let state_bg = state.clone();
        let sid_bg = session_id.clone();
        let text_bg = req.text.clone();
        let model_bg = state.config.ollama_model.clone();
        tokio::spawn(async move {
            let prompt = crate::services::ollama::deep_reflection_prompt(&text_bg);
            match crate::services::ollama::call_ollama(&state_bg.config.ollama_base_url, &model_bg, &prompt).await {
                Ok(r) => {
                    let db = state_bg.db.lock().await;
                    let _ = db.execute(
                        "UPDATE writing_sessions SET response = ?1 WHERE id = ?2",
                        rusqlite::params![&r, &sid_bg],
                    );
                }
                Err(e) => {
                    tracing::error!("Ollama error (background): {}", e);
                    state_bg.emit_log("ERROR", "ollama", &format!("Ollama bg error: {}", e));
                }
            }
        });
        "your anky is being born. the reflection is streaming...".into()
    } else {
        let model = "llama3.1:latest";
        let prompt = crate::services::ollama::quick_feedback_prompt(&req.text, req.duration);
        let r = match crate::services::ollama::call_ollama(&state.config.ollama_base_url, model, &prompt).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Ollama error: {}", e);
                state.emit_log("ERROR", "ollama", &format!("Ollama error: {}", e));
                "the consciousness stream encountered turbulence. try again?".into()
            }
        };
        // Update the writing session with Ollama's response
        {
            let db = state.db.lock().await;
            let _ = db.execute(
                "UPDATE writing_sessions SET response = ?1 WHERE id = ?2",
                rusqlite::params![&r, &session_id],
            );
        }
        r
    };

    // Handle inquiry: mark answered and generate next question
    if let Some(ref inquiry_id) = req.inquiry_id {
        if !inquiry_id.is_empty() {
            let db = state.db.lock().await;
            let _ =
                crate::db::queries::mark_inquiry_answered(&db, inquiry_id, &req.text, &session_id);

            // Spawn background task to generate next inquiry via Claude
            let state_bg = state.clone();
            let uid_bg = user_id.clone();
            tokio::spawn(async move {
                if let Err(e) = generate_next_inquiry(&state_bg, &uid_bg).await {
                    tracing::warn!("Failed to generate next inquiry: {}", e);
                }
            });
        }
    }

    let mut anky_id = None;

    // If it's an Anky, kick off background image generation
    if is_anky {
        let aid = uuid::Uuid::new_v4().to_string();
        {
            let db = state.db.lock().await;
            crate::db::queries::insert_anky(
                &db,
                &aid,
                &session_id,
                &user_id,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                "generating",
                "written",
            )?;
        }

        anky_id = Some(aid.clone());

        let state_clone = state.clone();
        let text = req.text.clone();
        let sid = session_id.clone();
        let uid = user_id.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::pipeline::image_gen::generate_anky_from_writing(
                &state_clone,
                &aid,
                &sid,
                &uid,
                &text,
            )
            .await
            {
                tracing::error!("Anky generation failed: {}", e);
                state_clone.emit_log(
                    "ERROR",
                    "image_gen",
                    &format!(
                        "Generation failed for {}: {}. will retry later.",
                        &aid[..8],
                        e
                    ),
                );
                // Mark as failed so retry can pick it up
                let db = state_clone.db.lock().await;
                let _ = crate::db::queries::mark_anky_failed(&db, &aid);
            }
        });
    }

    Ok((
        jar,
        Json(WriteResponse {
            response,
            duration: req.duration,
            is_anky,
            anky_id,
            estimated_wait_seconds: if is_anky { Some(45) } else { None },
            flow_score,
            error: None,
        }),
    ))
}

pub async fn get_writings(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let (user_id, _) = get_or_create_user_id(&jar, None);

    let writings = {
        let db = state.db.lock().await;
        crate::db::queries::get_user_writings_with_ankys(&db, &user_id)?
    };

    let now = chrono::Utc::now();

    let mut ctx = tera::Context::new();
    ctx.insert(
        "writings",
        &serde_json::to_value(
            writings
                .iter()
                .map(|w| {
                    let first_line: String = w
                        .content
                        .lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(60)
                        .collect();
                    let duration_display = format!(
                        "{}m {}s",
                        (w.duration_seconds / 60.0) as u32,
                        (w.duration_seconds % 60.0) as u32
                    );
                    let relative_time = if let Ok(dt) =
                        chrono::NaiveDateTime::parse_from_str(&w.created_at, "%Y-%m-%d %H:%M:%S")
                    {
                        let diff = now.naive_utc() - dt;
                        if diff.num_days() > 0 {
                            format!("{}d ago", diff.num_days())
                        } else if diff.num_hours() > 0 {
                            format!("{}h ago", diff.num_hours())
                        } else {
                            format!("{}m ago", diff.num_minutes().max(1))
                        }
                    } else {
                        w.created_at.clone()
                    };
                    // Prefer Claude reflection over Ollama response for ankys
                    let display_response = if w.is_anky {
                        w.anky_reflection.as_ref().or(w.response.as_ref()).cloned()
                    } else {
                        w.response.clone()
                    };
                    serde_json::json!({
                        "id": w.id,
                        "content": w.content,
                        "first_line": first_line,
                        "duration_seconds": w.duration_seconds,
                        "duration_display": duration_display,
                        "word_count": w.word_count,
                        "is_anky": w.is_anky,
                        "response": display_response,
                        "created_at": w.created_at,
                        "relative_time": relative_time,
                        "anky_id": w.anky_id,
                        "anky_title": w.anky_title,
                        "anky_image_path": w.anky_image_path,
                        "conversation_json": w.conversation_json,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );

    let html = state.tera.render("writings.html", &ctx)?;
    Ok(Html(html))
}

/// Generate the next inquiry for a user based on their history.
async fn generate_next_inquiry(state: &AppState, user_id: &str) -> anyhow::Result<()> {
    let api_key = &state.config.anthropic_api_key;
    if api_key.is_empty() {
        return Ok(());
    }

    let (history, lang) = {
        let db = state.db.lock().await;
        let h = crate::db::queries::get_inquiry_history(&db, user_id, 10)?;
        let l = crate::db::queries::get_inquiry_language(&db, user_id)?
            .unwrap_or_else(|| "en".to_string());
        (h, l)
    };

    // Build context from history
    let mut context = String::new();
    for (q, a, _) in history.iter().rev() {
        context.push_str(&format!("Q: {}\n", q));
        if let Some(answer) = a {
            let excerpt: String = answer.chars().take(500).collect();
            context.push_str(&format!("A: {}\n\n", excerpt));
        } else {
            context.push_str("A: [skipped/unanswered]\n\n");
        }
    }

    // Get psychological profile if available
    let profile = {
        let db = state.db.lock().await;
        let mut stmt =
            db.prepare("SELECT psychological_profile FROM user_profiles WHERE user_id = ?1")?;
        let mut rows = stmt.query_map(rusqlite::params![user_id], |row| {
            row.get::<_, Option<String>>(0)
        })?;
        rows.next()
            .and_then(|r| r.ok())
            .flatten()
            .unwrap_or_default()
    };

    let system = format!(
        "You generate self-inquiry questions. Rules:\n\
         - Maximum 12 words. Shorter is better. 5-8 words is ideal.\n\
         - Ask about what the person DOES, not what they THINK. Behavior, not philosophy.\n\
         - Target the gap between who they perform being and who they actually are.\n\
         - No therapy-speak. No \"how does that make you feel.\" No \"tell me about.\"\n\
         - Be specific and concrete. \"What did you last lie about?\" not \"What is truth?\"\n\
         - The question should sting a little. It should be hard to answer honestly.\n\
         - Language: {}. Output ONLY the question. No quotes, no explanation.",
        lang
    );

    let user_msg = if context.is_empty() && profile.is_empty() {
        "Generate a sharp opening question for someone who just arrived. \
         Hit them where they live. No warm-up."
            .to_string()
    } else {
        let mut msg = String::new();
        if !profile.is_empty() {
            msg.push_str(&format!("PSYCHOLOGICAL PROFILE:\n{}\n\n", profile));
        }
        if !context.is_empty() {
            msg.push_str(&format!("CONVERSATION HISTORY:\n{}", context));
        }
        msg.push_str(
            "\nBased on what they revealed, ask the ONE question they're hoping you won't ask.",
        );
        msg
    };

    let result = crate::services::claude::call_claude_public(
        api_key,
        "claude-haiku-4-5-20251001",
        &system,
        &user_msg,
        200,
    )
    .await?;

    let question = result.text.trim().to_string();
    if !question.is_empty() {
        let db = state.db.lock().await;
        crate::db::queries::create_inquiry(&db, user_id, &question, &lang)?;
    }

    Ok(())
}
