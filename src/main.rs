mod config;
mod db;
mod error;
mod middleware;
mod models;
mod pipeline;
mod routes;
mod services;
mod sse;
mod state;
mod training;

use crate::config::Config;
use crate::state::{AppState, GpuStatus};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "anky=info,tower_http=info".into()),
        )
        .init();

    // Load config
    let config = Config::from_env()?;
    let port = config.port;

    tracing::info!("Starting Anky server on port {}", port);

    // Open database
    std::fs::create_dir_all("data")?;
    let conn = db::open_db("data/anky.db")?;
    tracing::info!("Database initialized at data/anky.db");

    // Load templates
    let tera = tera::Tera::new("templates/**/*.html")?;
    tracing::info!("Templates loaded: {:?}", tera.get_template_names().collect::<Vec<_>>());

    // SSE broadcast channel
    let (log_tx, _) = broadcast::channel::<sse::logger::LogEntry>(1000);

    // Build state
    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
        tera: Arc::new(tera),
        config: Arc::new(config),
        gpu_status: Arc::new(RwLock::new(GpuStatus::Idle)),
        log_tx,
    };

    // Init health tracking
    routes::health::init_start_time();

    // Start training scheduler
    let sched_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = training::schedule::start_scheduler(sched_state).await {
            tracing::error!("Failed to start training scheduler: {}", e);
        }
    });

    state.emit_log("INFO", "server", &format!("Anky server starting on port {}", port));

    // Retry failed ankys every 5 minutes
    let retry_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            let failed = {
                let db = retry_state.db.lock().await;
                db::queries::get_failed_ankys(&db).unwrap_or_default()
            };
            if !failed.is_empty() {
                retry_state.emit_log("INFO", "retry", &format!("Auto-retrying {} failed ankys", failed.len()));
                for (anky_id, session_id, writing) in failed {
                    let s = retry_state.clone();
                    let aid = anky_id.clone();
                    let sid = session_id.clone();
                    let text = writing.clone();
                    tokio::spawn(async move {
                        if let Err(e) = pipeline::image_gen::generate_anky_from_writing(
                            &s, &aid, &sid, "auto-retry", &text,
                        ).await {
                            s.emit_log("ERROR", "retry", &format!("Auto-retry failed for {}: {}", &aid[..8], e));
                            let db = s.db.lock().await;
                            let _ = db::queries::mark_anky_failed(&db, &aid);
                        }
                    });
                    // Small delay between retries
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    // Build router
    let app = routes::build_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Listening on 0.0.0.0:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
