mod ankyverse;
mod config;
mod create_videos;
mod db;
mod error;
pub mod kingdoms;
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

use crate::config::{Config, RunMode};
use crate::error::AppError;
use crate::state::{AppState, GpuStatus};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

async fn process_gpu_job(state: &AppState, job: &state::GpuJob) -> Result<(), AppError> {
    match job {
        state::GpuJob::AnkyImage {
            anky_id,
            session_id,
            user_id,
            writing,
        } => {
            let fid: Option<u64> = {
                let conn = crate::db::conn(&state.db)?;
                crate::db::queries::get_user_farcaster_fid(&conn, user_id)
                    .ok()
                    .flatten()
                    .and_then(|f| f.parse::<u64>().ok())
            };
            let kingdom = if let Some(fid) = fid {
                crate::kingdoms::kingdom_for_fid(fid)
            } else {
                crate::kingdoms::kingdom_for_session(session_id)
            };

            {
                let conn = crate::db::conn(&state.db)?;
                let _ = crate::db::queries::set_anky_kingdom(
                    &conn,
                    anky_id,
                    kingdom.id,
                    kingdom.name,
                    kingdom.chakra,
                );
            }

            state.emit_log(
                "INFO",
                "gpu_queue",
                &format!(
                    "Anky {} assigned to kingdom {} ({})",
                    &anky_id[..8.min(anky_id.len())],
                    kingdom.name,
                    kingdom.chakra
                ),
            );

            pipeline::image_gen::generate_anky_from_writing(
                state, anky_id, session_id, user_id, writing,
            )
            .await?;
        }
        state::GpuJob::CuentacuentosImages { cuentacuentos_id } => {
            pipeline::image_gen::generate_cuentacuentos_images(cuentacuentos_id, state).await?;
        }
        state::GpuJob::CuentacuentosAudio { cuentacuentos_id } => {
            pipeline::guidance_gen::generate_cuentacuentos_audio(state, cuentacuentos_id).await?;
        }
    }

    Ok(())
}

/// GPU job worker: drains Redis pro jobs before free jobs and runs one job at a time.
async fn gpu_job_worker(state: AppState) {
    loop {
        let dequeued =
            match crate::services::redis_queue::dequeue_job(&state.config.redis_url).await {
                Ok(job) => job,
                Err(e) => {
                    state.emit_log(
                        "ERROR",
                        "gpu_queue",
                        &format!("Redis dequeue failed: {}", e),
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
            };

        let Some(job) = dequeued else {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            continue;
        };

        if let Err(e) = process_gpu_job(&state, &job.job).await {
            state.emit_log(
                "ERROR",
                "gpu_queue",
                &format!("GPU job {} failed: {}", &job.id[..8.min(job.id.len())], e),
            );

            if let state::GpuJob::AnkyImage { anky_id, .. } = &job.job {
                if let Ok(conn) = crate::db::conn(&state.db) {
                    let _ = db::queries::mark_anky_failed(&conn, anky_id);
                }
            }

            if let Err(fail_err) =
                crate::services::redis_queue::fail_job(&state.config.redis_url, &job).await
            {
                state.emit_log(
                    "ERROR",
                    "gpu_queue",
                    &format!(
                        "Redis fail handler failed for {}: {}",
                        &job.id[..8],
                        fail_err
                    ),
                );
            }
            continue;
        }

        if let Err(e) =
            crate::services::redis_queue::complete_job(&state.config.redis_url, &job.id).await
        {
            state.emit_log(
                "ERROR",
                "gpu_queue",
                &format!("Redis completion failed for {}: {}", &job.id[..8], e),
            );
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

    tracing::info!(
        "Starting Anky with mode {:?} on port {}",
        config.run_mode,
        port
    );

    // Open database — log before connecting so we can diagnose hangs
    tracing::info!("Connecting to database at {}", config.database_url);
    let db_pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Database initialized");

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

    // Build state
    let state = AppState {
        db: db_pool,
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
    };

    let start_web = state.config.run_mode != RunMode::Worker;
    let start_worker = state.config.run_mode != RunMode::Web;

    // Recover any Redis jobs that were processing when the worker last crashed.
    if start_worker {
        let recovered =
            crate::services::redis_queue::recover_processing_jobs(&state.config.redis_url)
                .await
                .unwrap_or(0);
        if recovered > 0 {
            tracing::info!("Recovered {} jobs from Redis on startup", recovered);
        }
    }

    // Start GPU job worker — drains Redis pro queue before free queue.
    if start_worker {
        let worker_state = state.clone();
        tokio::spawn(async move {
            gpu_job_worker(worker_state).await;
        });
        tracing::info!("GPU priority job worker spawned");
    }

    if !start_web {
        tracing::info!("Worker mode active; Axum server disabled");
        tokio::signal::ctrl_c().await?;
        return Ok(());
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

    // Run backfill in background so it doesn't delay server startup
    let backfill_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = storage::files::backfill_writings_to_files(&backfill_state).await {
            tracing::error!("Writing archive backfill failed: {}", e);
            backfill_state.emit_log(
                "ERROR",
                "writing_archive",
                &format!("Startup backfill failed: {}", e),
            );
        }
    });

    // Honcho historical backfill — send all existing writings so Honcho builds user models
    if services::honcho::is_configured(&state.config) {
        let honcho_state = state.clone();
        tokio::spawn(async move {
            // Wait 30s after boot so the server is ready before hammering the API
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            services::honcho::backfill_all_writings(&honcho_state).await;
        });
    }

    // Retry failed ankys every 5 minutes (with backoff + max retries)
    const MAX_ANKY_RETRIES: u32 = 5;
    let retry_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            let failed = {
                let Some(db) = crate::db::get_conn_logged(&retry_state.db) else {
                    continue;
                };
                db::queries::get_failed_ankys(&db).unwrap_or_default()
            };
            if !failed.is_empty() {
                retry_state.emit_log(
                    "INFO",
                    "retry",
                    &format!(
                        "Checking {} failed ankys for retry eligibility",
                        failed.len()
                    ),
                );
                for (anky_id, session_id, writing) in failed {
                    // Check retry count and backoff
                    let (retry_count, last_retry_at) = {
                        let Some(db) = crate::db::get_conn_logged(&retry_state.db) else {
                            continue;
                        };
                        db::queries::get_anky_retry_info(&db, &anky_id).unwrap_or((0, None))
                    };

                    if retry_count >= MAX_ANKY_RETRIES {
                        // Abandon this anky — too many retries
                        if let Some(db) = crate::db::get_conn_logged(&retry_state.db) {
                            let _ = db::queries::mark_anky_abandoned(&db, &anky_id);
                        }
                        retry_state.emit_log(
                            "WARN",
                            "retry",
                            &format!(
                                "Anky {} abandoned after {} retries",
                                &anky_id[..8.min(anky_id.len())],
                                retry_count
                            ),
                        );
                        continue;
                    }

                    // Exponential backoff: wait at least 2^N * 5 minutes since last attempt
                    if let Some(ref last_at) = last_retry_at {
                        if let Ok(last_time) =
                            chrono::NaiveDateTime::parse_from_str(last_at, "%Y-%m-%d %H:%M:%S")
                        {
                            let now = chrono::Utc::now().naive_utc();
                            let backoff_minutes = (2u64.pow(retry_count)) * 5;
                            let elapsed_minutes = (now - last_time).num_minutes() as u64;
                            if elapsed_minutes < backoff_minutes {
                                continue; // Not enough time has passed
                            }
                        }
                    }

                    // Increment retry count before attempting
                    {
                        if let Some(db) = crate::db::get_conn_logged(&retry_state.db) {
                            let _ = db::queries::increment_anky_retry(&db, &anky_id);
                        }
                    }

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
                                &format!(
                                    "Auto-retry failed for {}: {}",
                                    &aid[..8.min(aid.len())],
                                    e
                                ),
                            );
                            if let Some(db) = crate::db::get_conn_logged(&s.db) {
                                let _ = db::queries::mark_anky_failed(&db, &aid);
                            }
                        }
                    });
                    // Small delay between retries
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }

            // Also retry failed prompts
            let failed_prompts = {
                let Some(db) = crate::db::get_conn_logged(&retry_state.db) else {
                    continue;
                };
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
                                if let Some(db) = crate::db::get_conn_logged(&s.db) {
                                    let _ =
                                        db::queries::update_prompt_image(&db, &pid, &image_path);
                                }
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
                                if let Some(db) = crate::db::get_conn_logged(&s.db) {
                                    let _ = db::queries::update_prompt_status(&db, &pid, "failed");
                                }
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
                let Some(db) = crate::db::get_conn_logged(&story_retry_state.db) else {
                    continue;
                };
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
                    if let Some(db) = crate::db::get_conn_logged(&story_retry_state.db) {
                        let _ = db::queries::requeue_retryable_cuentacuentos_images(
                            &db,
                            &cuentacuentos_id,
                        );
                    }
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
                let Some(db) = crate::db::get_conn_logged(&recovery_state.db) else {
                    continue;
                };
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

            // Recover ankys that are complete but missing reflection (one per cycle to avoid
            // saturating Ollama/Claude and competing with live user requests)
            let missing = {
                let Some(db) = crate::db::get_conn_logged(&recovery_state.db) else {
                    continue;
                };
                db::queries::get_ankys_missing_reflection(&db).unwrap_or_default()
            };
            for (anky_id, _session_id, user_id, writing) in missing.into_iter().take(1) {
                tracing::info!("Recovering missing reflection for anky {}", &anky_id[..8]);
                recovery_state.emit_log(
                    "INFO",
                    "recovery",
                    &format!("Recovering missing reflection for {}", &anky_id[..8]),
                );

                // Try Claude first, fall back to Ollama
                let api_key = recovery_state.config.anthropic_api_key.clone();
                let reflection_result = if !api_key.is_empty() {
                    // Build memory context (best-effort)
                    let memory_ctx = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        crate::memory::recall::build_memory_context(
                            &recovery_state.db,
                            &recovery_state.config.ollama_base_url,
                            &user_id,
                            &writing,
                        ),
                    )
                    .await
                    .ok()
                    .and_then(|r| r.ok())
                    .map(|ctx| ctx.format_for_prompt())
                    .unwrap_or_default();

                    services::claude::generate_title_and_reflection_with_memory_using_model(
                        &api_key,
                        &recovery_state.config.reflection_model,
                        Some(&recovery_state.config.conversation_model),
                        &writing,
                        &memory_ctx,
                    )
                    .await
                    .map(|(r, _)| r.text)
                } else {
                    Err(anyhow::anyhow!("no API key"))
                };

                let full_text = match reflection_result {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::warn!(
                            "Claude recovery failed for {}, trying Haiku fallback: {}",
                            &anky_id[..8],
                            e
                        );
                        let prompt = services::ollama::deep_reflection_prompt(&writing);
                        match services::claude::call_haiku(
                            &recovery_state.config.anthropic_api_key,
                            &prompt,
                        )
                        .await
                        {
                            Ok(text) => text,
                            Err(e2) => {
                                tracing::warn!(
                                    "Claude+Haiku recovery failed for {}, trying OpenRouter: {}",
                                    &anky_id[..8],
                                    e2
                                );
                                let or_key = &recovery_state.config.openrouter_api_key;
                                if !or_key.is_empty() {
                                    match services::openrouter::call_openrouter(
                                        or_key,
                                        "anthropic/claude-3.5-haiku",
                                        "You are a contemplative writing mirror.",
                                        &prompt,
                                        2048,
                                        45,
                                    )
                                    .await
                                    {
                                        Ok(text) => text,
                                        Err(e3) => {
                                            tracing::error!(
                                                "All providers recovery failed for {}: {}",
                                                &anky_id[..8],
                                                e3
                                            );
                                            continue;
                                        }
                                    }
                                } else {
                                    tracing::error!(
                                        "All providers recovery failed for {} (no OpenRouter key)",
                                        &anky_id[..8]
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                };

                let (title, reflection) = services::claude::parse_title_reflection(&full_text);
                if let Some(db) = crate::db::get_conn_logged(&recovery_state.db) {
                    if let Err(e) = db::queries::update_anky_title_reflection(
                        &db,
                        &anky_id,
                        &title,
                        &reflection,
                    ) {
                        tracing::error!("Recovery DB save failed for {}: {}", &anky_id[..8], e);
                    } else {
                        recovery_state.emit_log(
                            "INFO",
                            "recovery",
                            &format!("Reflection recovered for {}: \"{}\"", &anky_id[..8], title),
                        );
                    }
                }
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
