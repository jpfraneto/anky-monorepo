mod ankyverse;
mod config;
mod create_videos;
mod db;
mod error;
mod memory;
mod middleware;
mod models;
mod pipeline;
mod routes;
mod services;
mod sse;
mod state;
mod storage;
mod training;

use crate::config::Config;
use crate::state::{AppState, GpuStatus};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

/// GPU job worker: drains pro channel before free channel, runs one job at a time.
async fn gpu_job_worker(
    state: AppState,
    mut pro_rx: tokio::sync::mpsc::UnboundedReceiver<state::GpuJob>,
    mut free_rx: tokio::sync::mpsc::UnboundedReceiver<state::GpuJob>,
) {
    loop {
        // Try pro channel first, fall back to free channel
        let job = tokio::select! {
            biased; // always try pro first
            Some(job) = pro_rx.recv() => job,
            Some(job) = free_rx.recv() => job,
            else => break, // both channels closed
        };

        match job {
            state::GpuJob::AnkyImage {
                anky_id,
                session_id,
                user_id,
                writing,
            } => {
                if let Err(e) = pipeline::image_gen::generate_anky_from_writing(
                    &state,
                    &anky_id,
                    &session_id,
                    &user_id,
                    &writing,
                )
                .await
                {
                    state.emit_log(
                        "ERROR",
                        "gpu_queue",
                        &format!(
                            "Anky gen failed for {}: {}",
                            &anky_id[..8.min(anky_id.len())],
                            e
                        ),
                    );
                    let db = state.db.lock().await;
                    let _ = db::queries::mark_anky_failed(&db, &anky_id);
                }
            }
            state::GpuJob::CuentacuentosImages { cuentacuentos_id } => {
                if let Err(e) =
                    pipeline::image_gen::generate_cuentacuentos_images(&cuentacuentos_id, &state)
                        .await
                {
                    state.emit_log(
                        "ERROR",
                        "gpu_queue",
                        &format!(
                            "Cuentacuentos image gen failed for {}: {}",
                            &cuentacuentos_id[..8.min(cuentacuentos_id.len())],
                            e
                        ),
                    );
                }
            }
            state::GpuJob::CuentacuentosAudio { cuentacuentos_id } => {
                if let Err(e) =
                    pipeline::guidance_gen::generate_cuentacuentos_audio(&state, &cuentacuentos_id)
                        .await
                {
                    state.emit_log(
                        "ERROR",
                        "gpu_queue",
                        &format!(
                            "Cuentacuentos TTS failed for {}: {}",
                            &cuentacuentos_id[..8.min(cuentacuentos_id.len())],
                            e
                        ),
                    );
                }
            }
        }
    }
}

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
    tracing::info!(
        "Templates loaded: {:?}",
        tera.get_template_names().collect::<Vec<_>>()
    );

    // SSE broadcast channel
    let (log_tx, _) = broadcast::channel::<sse::logger::LogEntry>(1000);

    // Webhook event log channel
    let (webhook_log_tx, _) = broadcast::channel::<String>(200);

    // Live streaming state
    let (live_status_tx, _) = broadcast::channel::<state::LiveStatusEvent>(100);
    let (live_text_tx, _) = broadcast::channel::<state::LiveTextEvent>(100);

    // Frame buffer for Rust-rendered livestream frames
    let frame_buffer = services::stream::new_frame_buffer();

    // GPU job priority queue (pro channel drained before free channel)
    let (gpu_queue, gpu_pro_rx, gpu_free_rx) = state::GpuJobQueue::new();

    // Build state
    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
        tera: Arc::new(tera),
        config: Arc::new(config),
        gpu_status: Arc::new(RwLock::new(GpuStatus::Idle)),
        log_tx,
        live_state: Arc::new(RwLock::new(state::LiveState::default())),
        live_status_tx,
        live_text_tx,
        frame_buffer,
        write_limiter: state::RateLimiter::new(5, std::time::Duration::from_secs(600)),
        waiting_room: Arc::new(RwLock::new(VecDeque::new())),
        image_limiter: state::RateLimiter::new(1, std::time::Duration::from_secs(300)),
        webhook_log_tx,
        memory_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        sessions: routes::session::new_session_map(),
        slot_tracker: routes::simulations::SlotTracker::new(),
        log_history: Arc::new(Mutex::new(VecDeque::new())),
        gpu_queue,
    };

    // Start GPU job worker — drains pro channel first, then free channel
    {
        let worker_state = state.clone();
        tokio::spawn(async move {
            gpu_job_worker(worker_state, gpu_pro_rx, gpu_free_rx).await;
        });
        tracing::info!("GPU priority job worker spawned");
    }

    // Start chunked session reaper (kills sessions after 8s silence, finalizes ankys)
    routes::session::spawn_session_reaper(state.sessions.clone(), state.clone());

    // Init health tracking
    routes::health::init_start_time();

    // Start training scheduler
    let sched_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = training::schedule::start_scheduler(sched_state).await {
            tracing::error!("Failed to start training scheduler: {}", e);
        }
    });

    // Start push notification scheduler
    let push_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = services::push_scheduler::start_scheduler(push_state).await {
            tracing::error!("Failed to start push notification scheduler: {}", e);
        }
    });

    // Livestream disabled — too slow, not worth it
    tracing::info!("Livestream disabled");

    state.emit_log(
        "INFO",
        "server",
        &format!("Anky server starting on port {}", port),
    );

    if let Err(e) = storage::files::backfill_writings_to_files(&state).await {
        tracing::error!("Writing archive backfill failed: {}", e);
        state.emit_log(
            "ERROR",
            "writing_archive",
            &format!("Startup backfill failed: {}", e),
        );
    }

    // Honcho historical backfill — send all existing writings so Honcho builds user models
    if services::honcho::is_configured(&state.config) {
        let honcho_state = state.clone();
        tokio::spawn(async move {
            // Wait 30s after boot so the server is ready before hammering the API
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            services::honcho::backfill_all_writings(&honcho_state).await;
        });
    }

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
                retry_state.emit_log(
                    "INFO",
                    "retry",
                    &format!("Auto-retrying {} failed ankys", failed.len()),
                );
                for (anky_id, session_id, writing) in failed {
                    let s = retry_state.clone();
                    let aid = anky_id.clone();
                    let sid = session_id.clone();
                    let text = writing.clone();
                    tokio::spawn(async move {
                        if let Err(e) = pipeline::image_gen::generate_anky_from_writing(
                            &s,
                            &aid,
                            &sid,
                            "auto-retry",
                            &text,
                        )
                        .await
                        {
                            s.emit_log(
                                "ERROR",
                                "retry",
                                &format!("Auto-retry failed for {}: {}", &aid[..8], e),
                            );
                            let db = s.db.lock().await;
                            let _ = db::queries::mark_anky_failed(&db, &aid);
                        }
                    });
                    // Small delay between retries
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }

            // Also retry failed prompts
            let failed_prompts = {
                let db = retry_state.db.lock().await;
                db::queries::get_failed_prompts(&db).unwrap_or_default()
            };
            if !failed_prompts.is_empty() {
                retry_state.emit_log(
                    "INFO",
                    "retry",
                    &format!("Auto-retrying {} failed prompts", failed_prompts.len()),
                );
                for (prompt_id, prompt_text) in failed_prompts {
                    let s = retry_state.clone();
                    let pid = prompt_id.clone();
                    let pt = prompt_text.clone();
                    tokio::spawn(async move {
                        match pipeline::prompt_gen::generate_prompt_image(&s, &pid, &pt).await {
                            Ok(image_path) => {
                                let db = s.db.lock().await;
                                let _ = db::queries::update_prompt_image(&db, &pid, &image_path);
                                s.emit_log(
                                    "INFO",
                                    "retry",
                                    &format!("Prompt {} retry succeeded", &pid[..8]),
                                );
                            }
                            Err(e) => {
                                s.emit_log(
                                    "ERROR",
                                    "retry",
                                    &format!("Prompt {} retry failed: {}", &pid[..8], e),
                                );
                                let db = s.db.lock().await;
                                let _ = db::queries::update_prompt_status(&db, &pid, "failed");
                            }
                        }
                    });
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    // Retry failed cuentacuentos phase images every 5 minutes
    let story_retry_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            let cuentacuentos_ids = {
                let db = story_retry_state.db.lock().await;
                db::queries::get_retryable_failed_cuentacuentos_ids(&db).unwrap_or_default()
            };

            if !cuentacuentos_ids.is_empty() {
                story_retry_state.emit_log(
                    "INFO",
                    "retry",
                    &format!(
                        "Auto-retrying story images for {} cuentacuentos",
                        cuentacuentos_ids.len()
                    ),
                );
            }

            for cuentacuentos_id in cuentacuentos_ids {
                {
                    let db = story_retry_state.db.lock().await;
                    let _ =
                        db::queries::requeue_retryable_cuentacuentos_images(&db, &cuentacuentos_id);
                }

                if let Err(e) = pipeline::image_gen::generate_cuentacuentos_images(
                    &cuentacuentos_id,
                    &story_retry_state,
                )
                .await
                {
                    story_retry_state.emit_log(
                        "ERROR",
                        "retry",
                        &format!(
                            "Story image retry failed for {}: {}",
                            &cuentacuentos_id[..8.min(cuentacuentos_id.len())],
                            e
                        ),
                    );
                }

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    });

    // Live session watchdog — disabled (livestream killed)

    // Checkpoint recovery watchdog — recover orphaned writing sessions every 5 minutes
    let recovery_state = state.clone();
    tokio::spawn(async move {
        // Wait 60s before first check
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            let recovered = {
                let db = recovery_state.db.lock().await;
                match db::queries::recover_orphaned_checkpoints(&db) {
                    Ok(n) => n,
                    Err(e) => {
                        tracing::warn!("Checkpoint recovery error: {}", e);
                        0
                    }
                }
            };
            if recovered > 0 {
                tracing::info!(
                    "Recovered {} orphaned writing sessions from checkpoints",
                    recovered
                );
                recovery_state.emit_log(
                    "INFO",
                    "recovery",
                    &format!(
                        "Recovered {} orphaned writing sessions from checkpoints",
                        recovered
                    ),
                );
            }
        }
    });
    tracing::info!("Checkpoint recovery watchdog spawned (every 5 minutes)");

    // Farcaster (Neynar) webhook — ensure subscription exists on startup
    let fc_state = state.clone();
    tokio::spawn(async move {
        let cfg = &fc_state.config;
        if cfg.neynar_api_key.is_empty() || cfg.farcaster_bot_fid == 0 {
            tracing::info!("Neynar not configured, skipping Farcaster webhook setup");
            return;
        }
        // Small delay to let server start first
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        match services::neynar::ensure_webhook(
            &cfg.neynar_api_key,
            "https://anky.app/webhooks/farcaster",
            cfg.farcaster_bot_fid,
        )
        .await
        {
            Ok(id) => {
                tracing::info!("Farcaster webhook ready: {}", id);
                fc_state.emit_log(
                    "INFO",
                    "farcaster",
                    &format!("Webhook subscription active: {}", id),
                );
            }
            Err(e) => {
                tracing::error!("Failed to set up Farcaster webhook: {}", e);
                fc_state.emit_log(
                    "ERROR",
                    "farcaster",
                    &format!("Webhook setup failed: {}", e),
                );
            }
        }
    });

    // Farcaster notification backfill — catches mentions/replies if webhook
    // delivery is delayed or dropped.
    let fc_backfill_state = state.clone();
    tokio::spawn(async move {
        let cfg = &fc_backfill_state.config;
        if cfg.neynar_api_key.is_empty() || cfg.farcaster_bot_fid == 0 {
            return;
        }

        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        loop {
            match routes::webhook_farcaster::backfill_recent_interactions(
                fc_backfill_state.clone(),
                25,
            )
            .await
            {
                Ok(queued) if queued > 0 => {
                    tracing::info!("Farcaster backfill queued {} missed interactions", queued);
                    fc_backfill_state.emit_log(
                        "INFO",
                        "farcaster",
                        &format!("Backfill queued {} missed interactions", queued),
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Farcaster backfill failed: {}", e);
                    fc_backfill_state.emit_log(
                        "WARN",
                        "farcaster",
                        &format!("Backfill failed: {}", e),
                    );
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        }
    });

    // X v2 Filtered Stream — real-time mention detection, reconnects automatically
    let stream_state = state.clone();
    tokio::spawn(async move {
        let bearer = stream_state.config.twitter_bot_bearer_token.clone();
        if bearer.is_empty() {
            tracing::warn!("X bearer token not configured, skipping filtered stream");
            return;
        }

        // Ensure the @ankydotapp mention filter rule exists
        match services::x_bot::ensure_mention_rule(&bearer).await {
            Ok(_) => tracing::info!("X stream rule ready"),
            Err(e) => {
                tracing::error!("Failed to set up stream rule: {}", e);
                stream_state.emit_log("ERROR", "x_stream", &format!("Rule setup failed: {}", e));
                return;
            }
        }

        // Connect and reconnect with exponential backoff
        let mut backoff = 5u64;
        loop {
            let start = std::time::Instant::now();
            match services::x_bot::run_filtered_stream(&bearer, &stream_state).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Filtered stream error: {}", e);
                    stream_state.emit_log("ERROR", "x_stream", &format!("Disconnected: {}", e));
                }
            }
            // Reset backoff if connection was stable for >60s
            if start.elapsed().as_secs() > 60 {
                backoff = 5;
            }
            tracing::info!("Filtered stream reconnecting in {}s", backoff);
            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
            backoff = (backoff * 2).min(300);
        }
    });

    // System summary worker — generates a 30-minute activity digest
    let summary_state = state.clone();
    tokio::spawn(async move {
        // Wait 30 minutes before first summary
        tokio::time::sleep(std::time::Duration::from_secs(1800)).await;
        loop {
            routes::dashboard::generate_system_summary(&summary_state).await;
            tokio::time::sleep(std::time::Duration::from_secs(1800)).await;
        }
    });
    tracing::info!("System summary worker spawned (every 30m)");

    // Build router
    let app = routes::build_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Listening on 0.0.0.0:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
