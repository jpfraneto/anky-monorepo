use crate::error::AppError;
use crate::state::GpuJob;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

const PRO_QUEUE: &str = "anky:jobs:pro";
const FREE_QUEUE: &str = "anky:jobs:free";
const PROCESSING_SET: &str = "anky:jobs:processing";
const FAILED_SET: &str = "anky:jobs:failed";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueuedGpuJob {
    pub id: String,
    pub job: GpuJob,
    pub is_pro: bool,
    pub retry_count: u32,
    pub created_at: i64,
}

async fn redis_conn(redis_url: &str) -> Result<redis::aio::MultiplexedConnection, AppError> {
    let client = redis::Client::open(redis_url)
        .map_err(|e| AppError::Internal(format!("Redis client error: {}", e)))?;
    client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(format!("Redis connect error: {}", e)))
}

fn processing_key(job_id: &str) -> String {
    format!("{}:{}", PROCESSING_SET, job_id)
}

fn failed_key(job_id: &str) -> String {
    format!("{}:{}", FAILED_SET, job_id)
}

async fn enqueue_payload(
    conn: &mut redis::aio::MultiplexedConnection,
    payload: &str,
    is_pro: bool,
) -> Result<(), AppError> {
    let queue = if is_pro { PRO_QUEUE } else { FREE_QUEUE };
    conn.rpush::<_, _, ()>(queue, payload)
        .await
        .map_err(|e| AppError::Internal(format!("Redis push error: {}", e)))?;
    Ok(())
}

pub async fn enqueue_job(redis_url: &str, job: &GpuJob, is_pro: bool) -> Result<String, AppError> {
    let mut conn = redis_conn(redis_url).await?;
    let queued = QueuedGpuJob {
        id: uuid::Uuid::new_v4().to_string(),
        job: job.clone(),
        is_pro,
        retry_count: 0,
        created_at: chrono::Utc::now().timestamp(),
    };
    let payload = serde_json::to_string(&queued)
        .map_err(|e| AppError::Internal(format!("Job serialize error: {}", e)))?;
    enqueue_payload(&mut conn, &payload, queued.is_pro).await?;
    Ok(queued.id)
}

pub async fn dequeue_job(redis_url: &str) -> Result<Option<QueuedGpuJob>, AppError> {
    let mut conn = redis_conn(redis_url).await?;
    let payload: Option<String> = conn.lpop(PRO_QUEUE, None).await.unwrap_or(None);
    let payload = if let Some(payload) = payload {
        Some(payload)
    } else {
        conn.lpop(FREE_QUEUE, None).await.unwrap_or(None)
    };

    match payload {
        None => Ok(None),
        Some(payload) => {
            let job: QueuedGpuJob = serde_json::from_str(&payload)
                .map_err(|e| AppError::Internal(format!("Job deserialize error: {}", e)))?;
            conn.set_ex::<_, _, ()>(processing_key(&job.id), payload, 3600)
                .await
                .map_err(|e| AppError::Internal(format!("Redis processing error: {}", e)))?;
            Ok(Some(job))
        }
    }
}

pub async fn complete_job(redis_url: &str, job_id: &str) -> Result<(), AppError> {
    let mut conn = redis_conn(redis_url).await?;
    conn.del::<_, ()>(processing_key(job_id))
        .await
        .map_err(|e| AppError::Internal(format!("Redis complete error: {}", e)))?;
    Ok(())
}

pub async fn fail_job(redis_url: &str, job: &QueuedGpuJob) -> Result<(), AppError> {
    let mut conn = redis_conn(redis_url).await?;
    let mut failed = job.clone();
    failed.retry_count += 1;
    let payload = serde_json::to_string(&failed)
        .map_err(|e| AppError::Internal(format!("Job serialize error: {}", e)))?;
    conn.del::<_, ()>(processing_key(&job.id))
        .await
        .map_err(|e| AppError::Internal(format!("Redis fail cleanup error: {}", e)))?;
    conn.set_ex::<_, _, ()>(failed_key(&job.id), payload, 86_400)
        .await
        .map_err(|e| AppError::Internal(format!("Redis fail store error: {}", e)))?;
    Ok(())
}

/// On startup: re-queue any jobs that were processing when the server crashed.
pub async fn recover_processing_jobs(redis_url: &str) -> Result<u32, AppError> {
    let mut conn = redis_conn(redis_url).await?;

    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("{}:*", PROCESSING_SET))
        .query_async(&mut conn)
        .await
        .unwrap_or_default();

    let count = keys.len() as u32;
    for key in keys {
        let payload: Option<String> = conn.get(&key).await.unwrap_or(None);
        if let Some(payload) = payload {
            if let Ok(mut job) = serde_json::from_str::<QueuedGpuJob>(&payload) {
                job.retry_count += 1;
                if job.retry_count <= 5 {
                    if let Ok(requeued_payload) = serde_json::to_string(&job) {
                        let _ = enqueue_payload(&mut conn, &requeued_payload, job.is_pro).await;
                    }
                }
            }
            conn.del::<_, ()>(&key).await.ok();
        }
    }

    Ok(count)
}
