use crate::error::AppError;
use crate::models::{WriteRequest, WriteResponse};
use crate::state::AppState;
use axum::extract::State;
use axum::response::Html;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar};

fn get_or_create_user_id(jar: &CookieJar) -> (String, Option<Cookie<'static>>) {
    if let Some(cookie) = jar.get("anky_user_id") {
        (cookie.value().to_string(), None)
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        let cookie = Cookie::build(("anky_user_id", id.clone()))
            .max_age(time::Duration::days(365))
            .http_only(true)
            .same_site(tower_cookies::cookie::SameSite::Lax)
            .path("/")
            .build();
        (id, Some(cookie))
    }
}

pub async fn process_writing(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<WriteRequest>,
) -> Result<(CookieJar, Json<WriteResponse>), AppError> {
    let (user_id, new_cookie) = get_or_create_user_id(&jar);
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
                error: Some("write more. stream-of-consciousness means letting words flow — at least a few sentences.".into()),
            }),
        ));
    }

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

    // Call Ollama for feedback
    let model = if is_anky {
        state.config.ollama_model.as_str()
    } else {
        "llama3.1:latest"
    };

    let prompt = if is_anky {
        crate::services::ollama::deep_reflection_prompt(&req.text)
    } else {
        crate::services::ollama::quick_feedback_prompt(&req.text, req.duration)
    };

    let response = match crate::services::ollama::call_ollama(
        &state.config.ollama_base_url,
        model,
        &prompt,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Ollama error: {}", e);
            state.emit_log("ERROR", "ollama", &format!("Ollama error: {}", e));
            "the consciousness stream encountered turbulence. try again?".into()
        }
    };

    // Save to DB
    {
        let db = state.db.lock().await;
        crate::db::queries::ensure_user(&db, &user_id)?;
        crate::db::queries::insert_writing_session(
            &db,
            &session_id,
            &user_id,
            &req.text,
            req.duration,
            word_count,
            is_anky,
            Some(&response),
        )?;
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
                None, None, None, None, None, None, None,
                "pending",
            )?;
        }

        anky_id = Some(aid.clone());

        let state_clone = state.clone();
        let text = req.text.clone();
        let sid = session_id.clone();
        let uid = user_id.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::pipeline::image_gen::generate_anky_from_writing(
                &state_clone, &aid, &sid, &uid, &text,
            )
            .await
            {
                tracing::error!("Anky generation failed: {}", e);
                state_clone.emit_log("ERROR", "image_gen", &format!("Generation failed for {}: {}. will retry later.", &aid[..8], e));
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
            error: None,
        }),
    ))
}

pub async fn get_writings(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let (user_id, _) = get_or_create_user_id(&jar);

    let writings = {
        let db = state.db.lock().await;
        crate::db::queries::get_user_writings(&db, &user_id)?
    };

    let mut ctx = tera::Context::new();
    ctx.insert("writings", &serde_json::to_value(
        writings.iter().map(|w| {
            serde_json::json!({
                "id": w.id,
                "content": w.content,
                "duration_seconds": w.duration_seconds,
                "word_count": w.word_count,
                "is_anky": w.is_anky,
                "response": w.response,
                "created_at": w.created_at,
                "duration_display": format!("{}m {}s", (w.duration_seconds / 60.0) as u32, (w.duration_seconds % 60.0) as u32),
            })
        }).collect::<Vec<_>>()
    ).unwrap_or_default());

    let html = state.tera.render("writings.html", &ctx)?;
    Ok(Html(html))
}
