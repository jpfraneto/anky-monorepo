use axum::extract::State;
use axum::response::Json;

use crate::db::queries;
use crate::error::AppError;
use crate::middleware::api_auth::ApiKeyInfo;
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

    // Create the API key in api_keys table (starts with 0 balance, free tier tracked via agents table)
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
    axum::extract::Extension(key_info): axum::extract::Extension<ApiKeyInfo>,
    Json(req): Json<TransformRequest>,
) -> Result<Json<TransformResponse>, AppError> {
    if req.writing.trim().is_empty() {
        return Err(AppError::BadRequest("writing cannot be empty".into()));
    }

    if req.writing.len() > 50_000 {
        return Err(AppError::BadRequest("writing too long (max 50000 chars)".into()));
    }

    // Call Claude to transform the writing
    let result = claude::transform_writing(
        &state.config.anthropic_api_key,
        &req.writing,
        req.prompt.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal(format!("transformation failed: {}", e)))?;

    // Calculate cost with markup
    let cost_usd = cost::calculate_transform_cost(result.input_tokens, result.output_tokens);

    // Deduct balance and record transformation
    let db = state.db.lock().await;
    let transform_id = uuid::Uuid::new_v4().to_string();
    queries::deduct_balance(&db, &key_info.key, cost_usd)?;
    queries::insert_transformation(
        &db,
        &transform_id,
        &key_info.key,
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

    // Get updated balance
    let updated_key = queries::get_api_key(&db, &key_info.key)?
        .ok_or_else(|| AppError::Internal("key vanished".into()))?;

    Ok(Json(TransformResponse {
        transformed: result.text,
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
        cost_usd,
        balance_remaining: updated_key.balance_usd,
    }))
}

pub async fn balance(
    State(state): State<AppState>,
    axum::extract::Extension(key_info): axum::extract::Extension<ApiKeyInfo>,
) -> Result<Json<BalanceResponse>, AppError> {
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
        balance_usd: key_record.balance_usd,
        total_spent_usd: key_record.total_spent_usd,
        total_transforms: key_record.total_transforms,
        recent_transforms: transforms,
    }))
}
