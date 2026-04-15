use crate::db::{self, queries};
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::Html;
use axum::Json;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;

fn generate_slug() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

#[derive(Deserialize)]
pub struct CreateNowRequest {
    pub prompt: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    pub creator_id: Option<String>,
    pub duration_seconds: Option<i32>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

fn default_mode() -> String {
    "sticker".to_string()
}

/// POST /api/v1/now — create a new now
pub async fn create_now(
    State(state): State<AppState>,
    Json(req): Json<CreateNowRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let prompt = req.prompt.trim();
    if prompt.is_empty() || prompt.len() > 500 {
        return Err(AppError::BadRequest(
            "prompt must be 1-500 characters".into(),
        ));
    }

    let mode = match req.mode.as_str() {
        "live" | "sticker" => &req.mode,
        _ => {
            return Err(AppError::BadRequest(
                "mode must be 'live' or 'sticker'".into(),
            ))
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let slug = generate_slug();
    let duration = req.duration_seconds.unwrap_or(480);

    let db = db::conn(&state.db)?;
    queries::insert_now(
        &db,
        &id,
        &slug,
        prompt,
        req.creator_id.as_deref(),
        mode,
        duration,
        None,
        req.latitude,
        req.longitude,
    )?;

    // Enqueue prompt image generation
    let job = crate::state::GpuJob::NowPromptImage {
        now_id: id.clone(),
        prompt: prompt.to_string(),
    };
    let _ = crate::services::redis_queue::enqueue_job(&state.config.redis_url, &job, false).await;

    // Generate QR code
    let qr_url = format!("https://anky.app/n/{}", slug);
    let qr_svg = super::qr_auth::render_qr_svg(&qr_url)?;

    state.emit_log(
        "INFO",
        "now",
        &format!("Now created: {} (slug={}, mode={})", &id[..8], slug, mode),
    );

    Ok(Json(json!({
        "id": id,
        "slug": slug,
        "prompt": prompt,
        "mode": mode,
        "duration_seconds": duration,
        "latitude": req.latitude,
        "longitude": req.longitude,
        "qr_svg": qr_svg,
        "qr_url": qr_url,
        "prompt_image_status": "queued",
        "created_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    })))
}

/// GET /api/v1/now/{slug} — get now state
pub async fn get_now(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::conn(&state.db)?;
    let now = queries::get_now_by_slug(&db, &slug)?
        .ok_or_else(|| AppError::NotFound("now not found".into()))?;

    // Writing chain
    let sessions = queries::get_now_sessions(&db, &now.id)?;
    let sessions_json: Vec<serde_json::Value> = sessions
        .iter()
        .map(|(sid, seq, created)| {
            json!({
                "writing_session_id": sid,
                "sequence": seq,
                "created_at": created,
            })
        })
        .collect();

    // Active presence (last 30 seconds)
    let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(30))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let presence = queries::get_now_active_presence(&db, &now.id, &cutoff)?;
    let presence_json: Vec<serde_json::Value> = presence
        .iter()
        .map(|(uid, name, joined)| {
            json!({
                "user_id": uid,
                "display_name": name,
                "joined_at": joined,
            })
        })
        .collect();

    // Image URL
    let image_url = now
        .prompt_image_path
        .as_ref()
        .map(|p| format!("/images/{}", p));

    // QR code (generated on the fly)
    let qr_url = format!("https://anky.app/n/{}", now.slug);
    let qr_svg = super::qr_auth::render_qr_svg(&qr_url).ok();

    Ok(Json(json!({
        "id": now.id,
        "slug": now.slug,
        "prompt": now.prompt,
        "prompt_image_url": image_url,
        "prompt_image_status": now.prompt_image_status,
        "mode": now.mode,
        "duration_seconds": now.duration_seconds,
        "starts_at": now.starts_at,
        "started": now.started,
        "latitude": now.latitude,
        "longitude": now.longitude,
        "created_at": now.created_at,
        "qr_url": qr_url,
        "qr_svg": qr_svg,
        "sessions": sessions_json,
        "presence": presence_json,
        "session_count": sessions_json.len(),
        "presence_count": presence_json.len(),
    })))
}

#[derive(Deserialize)]
pub struct JoinRequest {
    pub user_id: String,
    #[serde(default)]
    pub display_name: String,
}

/// POST /api/v1/now/{slug}/join — join presence
pub async fn join_now(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<JoinRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::conn(&state.db)?;
    let now = queries::get_now_by_slug(&db, &slug)?
        .ok_or_else(|| AppError::NotFound("now not found".into()))?;

    queries::upsert_now_presence(&db, &now.id, &req.user_id, &req.display_name)?;

    let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(30))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let presence = queries::get_now_active_presence(&db, &now.id, &cutoff)?;
    let presence_json: Vec<serde_json::Value> = presence
        .iter()
        .map(|(uid, name, joined)| {
            json!({
                "user_id": uid,
                "display_name": name,
                "joined_at": joined,
            })
        })
        .collect();

    Ok(Json(json!({
        "ok": true,
        "presence": presence_json,
        "presence_count": presence_json.len(),
    })))
}

#[derive(Deserialize)]
pub struct StartRequest {
    pub creator_id: String,
}

/// POST /api/v1/now/{slug}/start — creator starts the live session
pub async fn start_now(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<StartRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::conn(&state.db)?;
    let now = queries::get_now_by_slug(&db, &slug)?
        .ok_or_else(|| AppError::NotFound("now not found".into()))?;

    if now.mode != "live" {
        return Err(AppError::BadRequest("only live nows can be started".into()));
    }

    if now.started {
        return Err(AppError::BadRequest("already started".into()));
    }

    if now.creator_id.as_deref() != Some(&req.creator_id) {
        return Err(AppError::BadRequest("only the creator can start".into()));
    }

    // Writing begins after duration_seconds countdown
    let starts_at = (chrono::Utc::now() + chrono::Duration::seconds(now.duration_seconds as i64))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    queries::mark_now_started(&db, &now.id, &starts_at)?;

    state.emit_log(
        "INFO",
        "now",
        &format!("Now {} started, writing at {}", &now.id[..8], starts_at),
    );

    Ok(Json(json!({
        "ok": true,
        "starts_at": starts_at,
        "duration_seconds": now.duration_seconds,
    })))
}

#[derive(Deserialize)]
pub struct HeartbeatRequest {
    pub user_id: String,
}

/// POST /api/v1/now/{slug}/heartbeat — keep presence alive
pub async fn heartbeat_now(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::conn(&state.db)?;
    let now = queries::get_now_by_slug(&db, &slug)?
        .ok_or_else(|| AppError::NotFound("now not found".into()))?;

    queries::heartbeat_now_presence(&db, &now.id, &req.user_id)?;

    Ok(Json(json!({ "ok": true })))
}

/// GET /n/{slug} — now page (HTML)
///
/// Smart routing:
/// - If the now hasn't started yet (live mode) → show countdown + presence + QR
/// - If the now has started or is sticker mode → show writing screen with prompt
pub async fn now_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Html<String>, AppError> {
    let db = db::conn(&state.db)?;
    let now = queries::get_now_by_slug(&db, &slug)?
        .ok_or_else(|| AppError::NotFound("now not found".into()))?;

    let image_url = now
        .prompt_image_path
        .as_ref()
        .map(|p| format!("/images/{}", p))
        .unwrap_or_default();

    let sessions = queries::get_now_sessions(&db, &now.id)?;

    let qr_url = format!("https://anky.app/n/{}", now.slug);
    let qr_svg = super::qr_auth::render_qr_svg(&qr_url).unwrap_or_default();

    let mut ctx = tera::Context::new();
    ctx.insert("now_id", &now.id);
    ctx.insert("slug", &now.slug);
    ctx.insert("prompt", &now.prompt);
    ctx.insert("image_url", &image_url);
    ctx.insert("image_status", &now.prompt_image_status);
    ctx.insert("mode", &now.mode);
    ctx.insert("duration_seconds", &now.duration_seconds);
    ctx.insert("starts_at", &now.starts_at);
    ctx.insert("started", &now.started);
    ctx.insert("session_count", &sessions.len());
    ctx.insert("latitude", &now.latitude);
    ctx.insert("longitude", &now.longitude);
    ctx.insert("qr_svg", &qr_svg);
    ctx.insert("qr_url", &qr_url);

    let html = state.tera.render("now.html", &ctx)?;
    Ok(Html(html))
}
