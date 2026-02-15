use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Json;

use crate::db::queries;
use crate::error::AppError;
use crate::middleware::api_auth::ApiKeyInfo;
use crate::middleware::x402;
use crate::models::{BalanceResponse, RegisterRequest, RegisterResponse, TransformRequest, TransformResponse, TransformSummary};
use crate::pipeline::cost;
use crate::services::claude;
use crate::state::AppState;

/// POST /api/v1/register — create a new agent with an API key and 4 free sessions
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::BadRequest("name must be 1-100 characters".into()));
    }

    // Generate API key (same format as credits)
    let api_key = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let hex_chars: Vec<u8> = (0..16).map(|_| rng.gen::<u8>()).collect();
        format!("anky_{}", hex::encode(hex_chars))
    };

    let agent_id = uuid::Uuid::new_v4().to_string();

    let db = state.db.lock().await;

    // Create the API key in api_keys table (free tier tracked via agents table)
    queries::create_api_key(&db, &api_key, Some(name))?;

    // Create the agent record with 4 free sessions
    queries::insert_agent(
        &db,
        &agent_id,
        name,
        req.description.as_deref(),
        req.model.as_deref(),
        &api_key,
    )?;

    Ok(Json(RegisterResponse {
        agent_id,
        api_key,
        free_sessions_remaining: 4,
        message: "save your API key. it is only shown once.".into(),
    }))
}

pub async fn transform(
    State(state): State<AppState>,
    headers: HeaderMap,
    api_key_info: Option<axum::Extension<ApiKeyInfo>>,
    Json(req): Json<TransformRequest>,
) -> Result<Json<TransformResponse>, AppError> {
    if req.writing.trim().is_empty() {
        return Err(AppError::BadRequest("writing cannot be empty".into()));
    }

    if req.writing.len() > 50_000 {
        return Err(AppError::BadRequest("writing too long (max 50000 chars)".into()));
    }

    // Payment: check for x402/wallet payment header
    let payment_method;
    if let Some(sig) = headers
        .get("payment-signature")
        .or_else(|| headers.get("x-payment"))
        .and_then(|v| v.to_str().ok())
    {
        let sig = sig.trim();
        if sig.starts_with("0x") && sig.len() == 66 && sig[2..].chars().all(|c| c.is_ascii_hexdigit()) {
            state.emit_log("INFO", "payment", &format!("Transform wallet payment: {}", sig));
            payment_method = "wallet".to_string();
        } else {
            let facilitator = &state.config.x402_facilitator_url;
            if facilitator.is_empty() {
                return Err(AppError::Internal("x402 facilitator not configured".into()));
            }
            match x402::verify_x402_payment(facilitator, sig, "https://anky.app/api/v1/transform").await {
                Ok(_) => {
                    payment_method = "x402".to_string();
                }
                Err(reason) => {
                    return Err(AppError::PaymentRequired(format!("payment verification failed: {}", reason)));
                }
            }
        }
    } else {
        return Err(AppError::PaymentRequired(
            "payment required. send USDC tx hash in payment-signature header".into(),
        ));
    }

    // Call Claude to transform the writing
    let result = claude::transform_writing(
        &state.config.anthropic_api_key,
        &req.writing,
        req.prompt.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal(format!("transformation failed: {}", e)))?;

    // Calculate cost for tracking
    let cost_usd = cost::calculate_transform_cost(result.input_tokens, result.output_tokens);

    // Record transformation
    let db = state.db.lock().await;
    let transform_id = uuid::Uuid::new_v4().to_string();
    let api_key_str = api_key_info.as_ref().map(|ext| ext.key.clone()).unwrap_or_default();
    queries::insert_transformation(
        &db,
        &transform_id,
        &api_key_str,
        &req.writing,
        req.prompt.as_deref(),
        &result.text,
        result.input_tokens,
        result.output_tokens,
        cost_usd,
    )?;

    // Also record in cost_records for global tracking
    queries::insert_cost_record(
        &db,
        "claude",
        "claude-sonnet-4-20250514",
        result.input_tokens,
        result.output_tokens,
        cost_usd,
        Some(&transform_id),
    )?;

    Ok(Json(TransformResponse {
        transformed: result.text,
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
        cost_usd,
        payment_method,
    }))
}

pub async fn balance(
    State(state): State<AppState>,
    api_key_info: Option<axum::Extension<ApiKeyInfo>>,
) -> Result<Json<BalanceResponse>, AppError> {
    let key_info = api_key_info
        .ok_or_else(|| AppError::Unauthorized("API key required. set X-API-Key header".into()))?;
    let db = state.db.lock().await;
    let key_record = queries::get_api_key(&db, &key_info.key)?
        .ok_or_else(|| AppError::NotFound("API key not found".into()))?;

    let recent = queries::get_recent_transformations(&db, &key_info.key, 10)?;
    let transforms = recent
        .into_iter()
        .map(|t| TransformSummary {
            id: t.id,
            cost_usd: t.cost_usd,
            created_at: t.created_at,
        })
        .collect();

    Ok(Json(BalanceResponse {
        total_spent_usd: key_record.total_spent_usd,
        total_transforms: key_record.total_transforms,
        recent_transforms: transforms,
    }))
}
