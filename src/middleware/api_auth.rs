use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;

use crate::db::queries;
use crate::state::AppState;

pub async fn require_api_key(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let api_key = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let api_key = match api_key {
        Some(k) if k.starts_with("anky_") && k.len() == 37 => k,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "missing or invalid API key",
                    "hint": "set X-API-Key header with your anky_ key. get one at https://anky.app/credits"
                })),
            )
                .into_response();
        }
    };

    // Validate key exists and has balance
    let db = state.db.lock().await;
    match queries::get_api_key(&db, &api_key) {
        Ok(Some(key_record)) if key_record.is_active => {
            if key_record.balance_usd <= 0.0 {
                return (
                    StatusCode::PAYMENT_REQUIRED,
                    Json(json!({
                        "error": "insufficient balance",
                        "balance": key_record.balance_usd,
                        "hint": "add credits at https://anky.app/credits"
                    })),
                )
                    .into_response();
            }
            // Store key info in request extensions for downstream handlers
            req.extensions_mut().insert(ApiKeyInfo {
                key: api_key,
                balance_usd: key_record.balance_usd,
            });
            drop(db);
            next.run(req).await
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid API key" })),
        )
            .into_response(),
    }
}

#[derive(Clone, Debug)]
pub struct ApiKeyInfo {
    pub key: String,
    pub balance_usd: f64,
}
