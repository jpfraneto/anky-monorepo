use crate::error::AppError;
use crate::models::HealthResponse;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub fn init_start_time() {
    START_TIME.get_or_init(std::time::Instant::now);
}

pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, AppError> {
    let gpu_status = state.gpu_status.read().await;
    let total_cost = {
        let db = state.db.lock().await;
        crate::db::queries::get_total_cost(&db).unwrap_or(0.0)
    };

    let uptime = START_TIME
        .get()
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);

    Ok(Json(HealthResponse {
        status: "ok".into(),
        gpu_status: gpu_status.to_string(),
        total_cost_usd: total_cost,
        uptime_seconds: uptime,
    }))
}
