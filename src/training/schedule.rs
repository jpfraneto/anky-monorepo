use crate::state::AppState;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Start the daily training scheduler (3 AM).
pub async fn start_scheduler(state: AppState) -> anyhow::Result<()> {
    let sched = JobScheduler::new().await?;

    let state_clone = state.clone();
    // Run at 3:00 AM every day
    sched
        .add(Job::new_async("0 0 3 * * *", move |_uuid, _l| {
            let state = state_clone.clone();
            Box::pin(async move {
                tracing::info!("Daily training trigger fired");
                state.emit_log("INFO", "scheduler", "Daily training cycle triggered (3 AM)");
                if let Err(e) = crate::training::orchestrator::run_training_cycle(&state).await {
                    tracing::error!("Training cycle error: {}", e);
                    state.emit_log("ERROR", "scheduler", &format!("Training cycle error: {}", e));
                }
            })
        })?)
        .await?;

    sched.start().await?;
    tracing::info!("Training scheduler started (daily at 3 AM)");

    Ok(())
}
