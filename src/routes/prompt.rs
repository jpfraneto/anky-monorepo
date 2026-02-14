use crate::db::queries;
use crate::error::AppError;
use crate::middleware::api_auth::ApiKeyInfo;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde_json::json;

/// GET /prompt/create — form to write a prompt + pay
pub async fn create_prompt_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    ctx.insert("treasury_address", &state.config.treasury_address);
    let html = state.tera.render("prompt_create.html", &ctx)?;
    Ok(Html(html))
}

#[derive(serde::Deserialize)]
pub struct CreatePromptRequest {
    pub prompt_text: String,
}

/// POST /api/v1/prompt — create prompt (with payment)
pub async fn create_prompt_api(
    State(state): State<AppState>,
    headers: HeaderMap,
    api_key_info: Option<axum::Extension<ApiKeyInfo>>,
    Json(req): Json<CreatePromptRequest>,
) -> Result<Response, AppError> {
    let prompt_text = req.prompt_text.trim().to_string();
    if prompt_text.is_empty() || prompt_text.len() > 500 {
        return Err(AppError::BadRequest("prompt must be 1-500 characters".into()));
    }

    // Validate payment
    let payment = crate::routes::payment_helper::validate_payment(&state, &headers, &api_key_info)
        .await
        .map_err(|e| match e {
            crate::routes::payment_helper::PaymentError::Required => {
                AppError::BadRequest("payment required — send USDC or use API key".into())
            }
            crate::routes::payment_helper::PaymentError::VerificationFailed(r) => {
                AppError::BadRequest(format!("payment verification failed: {}", r))
            }
            crate::routes::payment_helper::PaymentError::ConfigError(r) => {
                AppError::Internal(r)
            }
            crate::routes::payment_helper::PaymentError::DbError(e) => {
                AppError::Internal(format!("db error: {}", e))
            }
        })?;

    let prompt_id = uuid::Uuid::new_v4().to_string();
    let user_id = payment.api_key.as_deref().unwrap_or("anon");

    {
        let db = state.db.lock().await;
        queries::ensure_user(&db, user_id)?;
        queries::insert_prompt(
            &db,
            &prompt_id,
            user_id,
            &prompt_text,
            payment.tx_hash.as_deref(),
        )?;
        queries::update_prompt_status(&db, &prompt_id, "generating")?;
    }

    state.emit_log("INFO", "prompt", &format!("Creating prompt {}: {}", &prompt_id[..8], &prompt_text[..prompt_text.len().min(60)]));

    // Spawn image generation in background
    let s = state.clone();
    let pid = prompt_id.clone();
    let pt = prompt_text.clone();
    tokio::spawn(async move {
        match crate::pipeline::prompt_gen::generate_prompt_image(&s, &pid, &pt).await {
            Ok(image_path) => {
                let db = s.db.lock().await;
                let _ = queries::update_prompt_image(&db, &pid, &image_path);
                s.emit_log("INFO", "prompt", &format!("Prompt {} image complete", &pid[..8]));
            }
            Err(e) => {
                tracing::error!("Prompt image generation failed for {}: {}", &pid[..8], e);
                s.emit_log("ERROR", "prompt", &format!("Prompt {} image failed: {}", &pid[..8], e));
                let db = s.db.lock().await;
                let _ = queries::update_prompt_status(&db, &pid, "failed");
            }
        }
    });

    let url = format!("https://anky.app/prompt/{}", prompt_id);
    Ok(Json(json!({
        "prompt_id": prompt_id,
        "status": "generating",
        "url": url,
        "payment_method": payment.method,
    }))
    .into_response())
}

/// GET /api/v1/prompt/{id} — poll prompt status/details
pub async fn get_prompt_api(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (prompt, sessions_count, creator_username) = {
        let db = state.db.lock().await;
        let prompt = queries::get_prompt_by_id(&db, &id)?;
        match &prompt {
            Some(p) => {
                let count = queries::get_prompt_session_count(&db, &p.id)?;
                let username = queries::get_display_username(&db, &p.creator_user_id)?;
                (prompt, count, username)
            }
            None => return Err(AppError::NotFound("prompt not found".into())),
        }
    };

    let p = prompt.unwrap();
    let image_url = p.image_path.as_ref().map(|path| format!("https://anky.app/data/images/{}", path));
    Ok(Json(json!({
        "id": p.id,
        "prompt_text": p.prompt_text,
        "status": p.status,
        "image_url": image_url,
        "creator_username": creator_username,
        "sessions_count": sessions_count,
        "url": format!("https://anky.app/prompt/{}", p.id),
        "created_at": p.created_at,
    })))
}

/// GET /prompt/{id} — writing page for prompt (with OG tags)
pub async fn prompt_page(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<Html<String>, AppError> {
    let (prompt, creator_username, settings) = {
        let db = state.db.lock().await;
        let prompt = queries::get_prompt_by_id(&db, &id)?;
        let prompt = prompt.ok_or_else(|| AppError::NotFound("prompt not found".into()))?;
        let creator_username = queries::get_display_username(&db, &prompt.creator_user_id)?;

        // Load user settings if logged in
        let settings = if let Some(token) = jar.get("anky_session") {
            if let Ok(Some((user_id, _))) = queries::get_auth_session(&db, token.value()) {
                Some(queries::get_user_settings(&db, &user_id)?)
            } else {
                None
            }
        } else {
            None
        };

        (prompt, creator_username, settings)
    };

    let image_url = prompt
        .image_path
        .as_ref()
        .map(|p| format!("https://anky.app/data/images/{}", p))
        .unwrap_or_else(|| "https://anky.app/static/references/anky-1.png".into());

    let mut ctx = tera::Context::new();
    ctx.insert("id", &prompt.id);
    ctx.insert("prompt_text", &prompt.prompt_text);
    ctx.insert("image_url", &image_url);
    ctx.insert("status", &prompt.status);
    ctx.insert("creator_username", &creator_username);

    // User settings
    if let Some(s) = &settings {
        ctx.insert("user_font_family", &s.font_family);
        ctx.insert("user_font_size", &s.font_size);
        ctx.insert("user_theme", &s.theme);
        ctx.insert("user_idle_timeout", &s.idle_timeout);
    } else {
        ctx.insert("user_font_family", &"monospace");
        ctx.insert("user_font_size", &18);
        ctx.insert("user_theme", &"dark");
        ctx.insert("user_idle_timeout", &8);
    }

    let html = state.tera.render("prompt.html", &ctx)?;
    Ok(Html(html))
}

#[derive(serde::Deserialize)]
pub struct SubmitWritingRequest {
    pub content: String,
    pub keystroke_deltas: String,
    pub page_opened_at: String,
    pub first_keystroke_at: Option<String>,
    pub duration_seconds: f64,
}

/// POST /api/v1/prompt/{id}/write — submit writing session for a prompt
pub async fn submit_prompt_writing(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
    Json(req): Json<SubmitWritingRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Verify prompt exists
    let prompt = {
        let db = state.db.lock().await;
        queries::get_prompt_by_id(&db, &id)?
    };
    let prompt = prompt.ok_or_else(|| AppError::NotFound("prompt not found".into()))?;

    let user_id = jar
        .get("anky_user_id")
        .map(|c| c.value().to_string())
        .or_else(|| jar.get("anky_session").map(|c| c.value().to_string()));

    let session_id = uuid::Uuid::new_v4().to_string();
    let word_count = req.content.split_whitespace().count() as i32;

    {
        let db = state.db.lock().await;
        queries::insert_prompt_session(
            &db,
            &session_id,
            &prompt.id,
            user_id.as_deref(),
            &req.content,
            &req.keystroke_deltas,
            &req.page_opened_at,
            req.first_keystroke_at.as_deref(),
            req.duration_seconds,
            word_count,
        )?;
    }

    state.emit_log(
        "INFO",
        "prompt",
        &format!(
            "Writing submitted for prompt {}: {} words, {:.0}s",
            &prompt.id[..8],
            word_count,
            req.duration_seconds,
        ),
    );

    Ok(Json(json!({
        "session_id": session_id,
        "word_count": word_count,
        "saved": true,
    })))
}

// ===== Agent-friendly prompt API =====

#[derive(serde::Deserialize)]
pub struct ListPromptsQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

/// GET /api/v1/prompts — paginated list of completed prompts
pub async fn list_prompts_api(
    State(state): State<AppState>,
    Query(query): Query<ListPromptsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let (prompts, total) = {
        let db = state.db.lock().await;
        queries::get_prompts_paginated(&db, page, limit)?
    };

    let items: Vec<serde_json::Value> = prompts
        .iter()
        .map(|p| {
            let image_url = p.image_path.as_ref().map(|path| format!("https://anky.app/data/images/{}", path));
            json!({
                "id": p.id,
                "prompt_text": p.prompt_text,
                "image_url": image_url,
                "creator_username": p.creator_username,
                "sessions_count": p.sessions_count,
                "url": format!("https://anky.app/prompt/{}", p.id),
                "created_at": p.created_at,
            })
        })
        .collect();

    Ok(Json(json!({
        "prompts": items,
        "page": page,
        "limit": limit,
        "total": total,
        "total_pages": (total as f64 / limit as f64).ceil() as i32,
    })))
}

/// GET /api/v1/prompts/random — random completed prompt
pub async fn random_prompt_api(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let prompt = {
        let db = state.db.lock().await;
        queries::get_random_prompt(&db)?
    };

    match prompt {
        Some(p) => {
            let image_url = p.image_path.as_ref().map(|path| format!("https://anky.app/data/images/{}", path));
            Ok(Json(json!({
                "id": p.id,
                "prompt_text": p.prompt_text,
                "image_url": image_url,
                "creator_username": p.creator_username,
                "sessions_count": p.sessions_count,
                "url": format!("https://anky.app/prompt/{}", p.id),
                "created_at": p.created_at,
            })))
        }
        None => Err(AppError::NotFound("no prompts found".into())),
    }
}
