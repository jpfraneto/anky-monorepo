use crate::error::AppError;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

const CANONICAL_PROCESSOR_INPUT_PREFIX: &str = "anky:canonical:processor-input";
pub const CANONICAL_PROCESSOR_INPUT_TTL_SECS: u64 = 60 * 60 * 24;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProcessorInput {
    pub session_hash: String,
    pub writing_text: String,
    pub duration_seconds: i64,
    pub word_count: i32,
    pub started_at: String,
    pub kingdom: String,
}

fn processor_input_key(session_hash: &str) -> String {
    format!(
        "{}:{}",
        CANONICAL_PROCESSOR_INPUT_PREFIX,
        session_hash.trim()
    )
}

async fn redis_conn(redis_url: &str) -> Result<redis::aio::MultiplexedConnection, AppError> {
    let client = redis::Client::open(redis_url)
        .map_err(|e| AppError::Internal(format!("Redis client error: {}", e)))?;
    client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(format!("Redis connect error: {}", e)))
}

/// Stores the verified canonical processor input in Redis with a short TTL.
/// This is explicit processor scratch state, not archive storage.
pub async fn store_canonical_processor_input(
    redis_url: &str,
    input: &CanonicalProcessorInput,
) -> Result<(), AppError> {
    let mut conn = redis_conn(redis_url).await?;
    let payload = serde_json::to_string(input)
        .map_err(|e| AppError::Internal(format!("Processor input serialize error: {}", e)))?;
    conn.set_ex::<_, _, ()>(
        processor_input_key(&input.session_hash),
        payload,
        CANONICAL_PROCESSOR_INPUT_TTL_SECS,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Redis processor input store error: {}", e)))?;
    Ok(())
}

pub async fn load_canonical_processor_input(
    redis_url: &str,
    session_hash: &str,
) -> Result<Option<CanonicalProcessorInput>, AppError> {
    let mut conn = redis_conn(redis_url).await?;
    let payload: Option<String> = conn
        .get(processor_input_key(session_hash))
        .await
        .map_err(|e| AppError::Internal(format!("Redis processor input load error: {}", e)))?;

    payload
        .map(|payload| {
            serde_json::from_str::<CanonicalProcessorInput>(&payload).map_err(|e| {
                AppError::Internal(format!("Processor input deserialize error: {}", e))
            })
        })
        .transpose()
}
