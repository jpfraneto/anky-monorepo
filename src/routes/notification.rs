use crate::error::AppError;
use crate::models::NotifySignupRequest;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

#[derive(serde::Serialize)]
pub struct SignupResponse {
    pub ok: bool,
}

pub async fn signup(
    State(state): State<AppState>,
    Json(req): Json<NotifySignupRequest>,
) -> Result<Json<SignupResponse>, AppError> {
    let db = state.db.lock().await;
    crate::db::queries::insert_notification_signup(
        &db,
        req.email.as_deref(),
        req.telegram_chat_id.as_deref(),
    )?;

    state.emit_log(
        "INFO",
        "notification",
        &format!(
            "New notification signup: email={}, telegram={}",
            req.email.as_deref().unwrap_or("none"),
            req.telegram_chat_id.as_deref().unwrap_or("none")
        ),
    );

    Ok(Json(SignupResponse { ok: true }))
}
