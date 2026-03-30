use crate::db::queries;
use crate::error::AppError;
use crate::models::{InterviewEndRequest, InterviewMessageRequest, InterviewStartRequest};
use crate::routes::auth;
use crate::state::AppState;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::{Html, IntoResponse, Json};
use axum_extra::extract::cookie::CookieJar;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as TungMsg;

pub async fn interview_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();

    // If user is logged in, pass their identity to the template
    if let Some(user) = auth::get_auth_user(&state, &jar).await {
        ctx.insert("user_id", &user.user_id);
        ctx.insert("username", &user.username.unwrap_or_default());
        ctx.insert("pfp_url", &user.profile_image_url.unwrap_or_default());
        ctx.insert("logged_in", &true);
    } else {
        ctx.insert("logged_in", &false);
    }

    let html = state.tera.render("interview.html", &ctx)?;
    Ok(Html(html))
}

/// GET /ws/interview — WebSocket proxy to Python interview engine on port 8890.
pub async fn ws_interview_proxy(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_interview_proxy)
}

fn tung_to_axum(msg: TungMsg) -> Option<Message> {
    match msg {
        TungMsg::Text(t) => Some(Message::Text(t.as_str().into())),
        TungMsg::Binary(b) => Some(Message::Binary(bytes::Bytes::copy_from_slice(&b))),
        TungMsg::Ping(b) => Some(Message::Ping(bytes::Bytes::copy_from_slice(&b))),
        TungMsg::Pong(b) => Some(Message::Pong(bytes::Bytes::copy_from_slice(&b))),
        TungMsg::Close(_) => None,
        _ => None,
    }
}

fn axum_to_tung(msg: Message) -> Option<TungMsg> {
    match msg {
        Message::Text(t) => Some(TungMsg::text(t.to_string())),
        Message::Binary(b) => Some(TungMsg::binary(b.to_vec())),
        Message::Ping(b) => Some(TungMsg::Ping(bytes::Bytes::copy_from_slice(&b))),
        Message::Pong(b) => Some(TungMsg::Pong(bytes::Bytes::copy_from_slice(&b))),
        Message::Close(_) => None,
    }
}

async fn handle_interview_proxy(axum_ws: WebSocket) {
    let url = "ws://127.0.0.1:8890";

    let upstream = match connect_async(url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            tracing::error!("Failed to connect to interview engine: {}", e);
            let mut ws = axum_ws;
            let _ = ws
                .send(Message::Text(
                    r#"{"type":"error","message":"interview engine not running"}"#.into(),
                ))
                .await;
            return;
        }
    };

    let (mut upstream_tx, mut upstream_rx) = upstream.split();
    let (mut axum_tx, mut axum_rx) = axum_ws.split();

    // Browser → Python
    let browser_to_python = tokio::spawn(async move {
        while let Some(Ok(msg)) = axum_rx.next().await {
            if let Some(tung_msg) = axum_to_tung(msg) {
                if upstream_tx.send(tung_msg).await.is_err() {
                    return;
                }
            } else {
                return;
            }
        }
    });

    // Python → Browser
    let python_to_browser = tokio::spawn(async move {
        while let Some(Ok(msg)) = upstream_rx.next().await {
            if let Some(axum_msg) = tung_to_axum(msg) {
                if axum_tx.send(axum_msg).await.is_err() {
                    return;
                }
            } else {
                return;
            }
        }
    });

    tokio::select! {
        _ = browser_to_python => {}
        _ = python_to_browser => {}
    }
}

// --- Internal API endpoints for Python interview engine ---

/// POST /api/interview/start
pub async fn interview_start(
    State(state): State<AppState>,
    Json(req): Json<InterviewStartRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = crate::db::conn(&state.db)?;
    queries::create_interview(
        &db,
        &req.id,
        req.user_id.as_deref(),
        &req.guest_name,
        req.is_anonymous,
    )?;
    Ok(Json(serde_json::json!({"ok": true, "id": req.id})))
}

/// POST /api/interview/message
pub async fn interview_message(
    State(state): State<AppState>,
    Json(req): Json<InterviewMessageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = crate::db::conn(&state.db)?;
    queries::save_interview_message(&db, &req.interview_id, &req.role, &req.content)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

/// POST /api/interview/end
pub async fn interview_end(
    State(state): State<AppState>,
    Json(req): Json<InterviewEndRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = crate::db::conn(&state.db)?;
    queries::end_interview(
        &db,
        &req.interview_id,
        req.summary.as_deref(),
        req.duration_seconds,
        req.message_count,
    )?;
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    5
}

/// GET /api/interview/history/:user_id
pub async fn interview_history(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = crate::db::conn(&state.db)?;
    let history = queries::get_user_interview_history(&db, &user_id, q.limit)?;
    Ok(Json(serde_json::json!(history)))
}

/// GET /api/interview/user-context/:user_id
pub async fn interview_user_context(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = crate::db::conn(&state.db)?;
    match queries::get_user_context_for_interview(&db, &user_id)? {
        Some(ctx) => Ok(Json(serde_json::json!(ctx))),
        None => Ok(Json(serde_json::json!(null))),
    }
}
