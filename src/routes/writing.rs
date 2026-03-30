use crate::error::AppError;
use crate::models::{WriteRequest, WriteResponse};
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Html;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar};

fn get_or_create_user_id(
    jar: &CookieJar,
    token_header: Option<&str>,
) -> (String, Option<Cookie<'static>>) {
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

    if api_key.is_some() {
        // Agents can no longer batch-submit writing. They must use the chunked session API,
        // which enforces the same 8-second timeout that humans face on the frontend.
        return Ok((
            jar,
            Json(WriteResponse {
                response: String::new(),
                duration: 0.0,
                is_anky: false,
                anky_id: None,
                wallet_address: None,
                estimated_wait_seconds: None,
                flow_score: None,
                error: Some(
                    "this endpoint no longer accepts agent submissions. \
                     the practice changed: you now write in real time, chunk by chunk, \
                     with the same 8-second timeout humans face. \
                     read the updated instructions at https://anky.app/skills \
                     and use POST /api/v1/session/start to begin."
                        .into(),
                ),
                anky_response: None,
                next_prompt: None,
                mood: None,
                model: None,
                provider: None,
                generation_ms: None,
                tokens_used: None,
            }),
        ));
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

    let token_header = headers
        .get("x-anky-user-token")
        .and_then(|v| v.to_str().ok());
    let (user_id, new_cookie) = get_or_create_user_id(&jar, token_header);
    let jar = if let Some(c) = new_cookie {
        jar.add(c)
    } else {
        jar
    };

    let word_count = req.text.split_whitespace().count() as i32;

    // Look up user's preferred model
    let user_preferred_model = {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::get_user_settings(&db, &user_id)
            .ok()
            .map(|s| s.preferred_model)
    };

    // Very short writings (<10 words) — still give a live response via the light model
    if word_count < 10 {
        let nudge = crate::services::ollama::quick_nudge(
            &state.config,
            &req.text,
            user_preferred_model.as_deref(),
        )
        .await
        .unwrap_or_else(|_| "something stirred in you. come back and let it out.".into());
        return Ok((
            jar,
            Json(WriteResponse {
                response: nudge,
                duration: req.duration,
                is_anky: false,
                anky_id: None,
                wallet_address: None,
                estimated_wait_seconds: None,
                flow_score: None,
                error: None,
                anky_response: None,
                next_prompt: None,
                mood: None,
                model: Some("live-nudge".into()),
                provider: Some("ollama".into()),
                generation_ms: None,
                tokens_used: None,
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

    let session_id = req
        .session_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

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
    let wallet_address = {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::ensure_user(&db, &user_id)?;
        let mut wallet_address = crate::db::queries::get_user_wallet(&db, &user_id)?;
        if wallet_address.is_none() {
            let generated_wallet = crate::services::wallet::generate_custodial_wallet();
            crate::db::queries::set_generated_wallet(
                &db,
                &user_id,
                &generated_wallet.address,
                &generated_wallet.secret_key,
            )?;
            wallet_address = Some(generated_wallet.address);
        }
        let mut was_completed = false;
        if let Some(existing) = crate::db::queries::get_writing_session_state(&db, &session_id)? {
            if existing.user_id != user_id {
                // Allow claiming sessions that were auto-recovered with a guessed user_id
                // (e.g. "system", "recovered-unknown") — the real user is submitting now.
                let is_placeholder = existing.user_id == "system"
                    || existing.user_id == "recovered-unknown"
                    || existing.user_id.starts_with("recovered-");
                if !is_placeholder {
                    return Err(AppError::Unauthorized(
                        "that writing session belongs to another user".into(),
                    ));
                }
            }
            was_completed = existing.status == "completed";
        }

        crate::db::queries::upsert_completed_writing_session_with_flow(
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
            req.session_token.as_deref(),
        )?;
        // Update leaderboard stats
        if !was_completed {
            if let Some(fs) = flow_score {
                let _ = crate::db::queries::update_user_flow_stats(&db, &user_id, fs, is_anky);
            }
        }
        wallet_address
    };

    // For anky sessions, return immediately — the frontend will open an SSE
    // connection to /api/stream-reflection/{anky_id} which handles the Claude
    // streaming call. No background call here to avoid duplicate API usage.
    let (response, resp_model, resp_provider, resp_gen_ms) = if is_anky {
        (
            "your anky is being born. the reflection is streaming...".to_string(),
            Some("claude-sonnet-4-20250514".to_string()),
            Some("claude".to_string()),
            None,
        )
    } else {
        let prompt = crate::services::ollama::quick_feedback_prompt(&req.text, req.duration);
        let gen_start = std::time::Instant::now();
        let r = match crate::services::claude::call_haiku(&state.config.anthropic_api_key, &prompt)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Haiku error: {}", e);
                state.emit_log("ERROR", "haiku", &format!("Haiku error: {}", e));
                // Fall back to light model instead of a hardcoded string
                crate::services::ollama::quick_nudge(
                    &state.config,
                    &req.text,
                    user_preferred_model.as_deref(),
                )
                .await
                .unwrap_or_else(|_| "something stirred in you. come back and let it out.".into())
            }
        };
        let gen_elapsed = gen_start.elapsed().as_millis() as u64;
        // Update the writing session with Ollama's response
        {
            let db = crate::db::conn(&state.db)?;
            let _ = db.execute(
                "UPDATE writing_sessions SET response = ?1 WHERE id = ?2",
                crate::params![&r, &session_id],
            );
        }
        (
            r,
            Some("claude-haiku".to_string()),
            Some("claude".to_string()),
            Some(gen_elapsed),
        )
    };

    // Handle inquiry: mark answered and generate next question
    if let Some(ref inquiry_id) = req.inquiry_id {
        if !inquiry_id.is_empty() {
            let db = crate::db::conn(&state.db)?;
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
            let db = crate::db::conn(&state.db)?;
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
                req.prompt_id.as_deref(),
            )?;
        }

        anky_id = Some(aid.clone());

        // Submit to GPU priority queue
        let is_pro = {
            let db = crate::db::conn(&state.db)?;
            crate::db::queries::is_user_pro(&db, &user_id).unwrap_or(false)
        };
        crate::services::redis_queue::enqueue_job(
            &state.config.redis_url,
            &crate::state::GpuJob::AnkyImage {
                anky_id: aid,
                session_id: session_id.clone(),
                user_id: user_id.clone(),
                writing: req.text.clone(),
            },
            is_pro,
        )
        .await?;
    }

    // anky_response is generated by the post-writing pipeline (event-driven),
    // not on-demand. The client polls /writing/{sessionId}/status to get it.

    // For web writes that aren't ankys, spawn the pipeline response generation
    if !is_anky {
        let resp_state = state.clone();
        let resp_uid = user_id.clone();
        let resp_sid = session_id.clone();
        let resp_text = req.text.clone();
        tokio::spawn(async move {
            crate::pipeline::guidance_gen::generate_anky_response(
                &resp_state,
                &resp_uid,
                &resp_sid,
                &resp_text,
            )
            .await;
        });
    }
    // For ankys, the full ritual lifecycle (which includes generate_anky_response)
    // is already triggered by the image pipeline completion.

    Ok((
        jar,
        Json(WriteResponse {
            response,
            duration: req.duration,
            is_anky,
            anky_id,
            wallet_address,
            estimated_wait_seconds: if is_anky { Some(45) } else { None },
            flow_score,
            error: None,
            anky_response: None, // generated async — client polls or waits
            next_prompt: None,
            mood: None,
            model: resp_model,
            provider: resp_provider,
            generation_ms: resp_gen_ms,
            tokens_used: None,
        }),
    ))
}

/// GET /api/writing/{sessionId}/status — web-accessible writing status (cookie auth)
pub async fn get_writing_status_web(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (user_id, _) = get_or_create_user_id(&jar, None);
    let db = crate::db::conn(&state.db)?;

    let (anky_response, next_prompt, mood) = db
        .query_row(
            "SELECT anky_response, anky_next_prompt, anky_mood FROM writing_sessions WHERE id = ?1 AND user_id = ?2",
            crate::params![&session_id, &user_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0).unwrap_or(None),
                    row.get::<_, Option<String>>(1).unwrap_or(None),
                    row.get::<_, Option<String>>(2).unwrap_or(None),
                ))
            },
        )
        .unwrap_or((None, None, None));

    Ok(Json(serde_json::json!({
        "sessionId": session_id,
        "ankyResponse": anky_response,
        "nextPrompt": next_prompt,
        "mood": mood,
    })))
}

pub async fn get_writings(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let (user_id, _) = get_or_create_user_id(&jar, None);

    let writings = {
        let db = crate::db::conn(&state.db)?;
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
    let (history, lang) = {
        let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
        let mut stmt =
            db.prepare("SELECT psychological_profile FROM user_profiles WHERE user_id = ?1")?;
        let mut rows = stmt.query_map(crate::params![user_id], |row| {
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

    let question = crate::services::claude::call_haiku_with_system(
        &state.config.anthropic_api_key,
        &system,
        &user_msg,
    )
    .await?
    .trim()
    .to_string();

    if !question.is_empty() {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::create_inquiry(&db, user_id, &question, &lang)?;
    }

    Ok(())
}
