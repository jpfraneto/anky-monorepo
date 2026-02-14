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

    // Live streaming state
    let (live_status_tx, _) = broadcast::channel::<state::LiveStatusEvent>(100);

    // Build state
    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
        tera: Arc::new(tera),
        config: Arc::new(config),
        gpu_status: Arc::new(RwLock::new(GpuStatus::Idle)),
        log_tx,
        live_state: Arc::new(RwLock::new(state::LiveState::default())),
        live_status_tx,
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

    // Spawn ffmpeg RTMP stream (only if configured)
    if !state.config.pumpfun_rtmp_url.is_empty() && !state.config.pumpfun_stream_key.is_empty() {
        let rtmp_url = state.config.pumpfun_rtmp_url.clone();
        let stream_key = state.config.pumpfun_stream_key.clone();
        let live_state = state.live_state.clone();
        tokio::spawn(async move {
            services::stream::spawn_ffmpeg_loop(rtmp_url, stream_key, live_state).await;
        });
        tracing::info!("Livestream ffmpeg loop spawned");
    } else {
        tracing::info!("Livestream not configured (PUMPFUN_RTMP_URL / PUMPFUN_STREAM_KEY missing)");
    }

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

            // Also retry failed prompts
            let failed_prompts = {
                let db = retry_state.db.lock().await;
                db::queries::get_failed_prompts(&db).unwrap_or_default()
            };
            if !failed_prompts.is_empty() {
                retry_state.emit_log("INFO", "retry", &format!("Auto-retrying {} failed prompts", failed_prompts.len()));
                for (prompt_id, prompt_text) in failed_prompts {
                    let s = retry_state.clone();
                    let pid = prompt_id.clone();
                    let pt = prompt_text.clone();
                    tokio::spawn(async move {
                        match pipeline::prompt_gen::generate_prompt_image(&s, &pid, &pt).await {
                            Ok(image_path) => {
                                let db = s.db.lock().await;
                                let _ = db::queries::update_prompt_image(&db, &pid, &image_path);
                                s.emit_log("INFO", "retry", &format!("Prompt {} retry succeeded", &pid[..8]));
                            }
                            Err(e) => {
                                s.emit_log("ERROR", "retry", &format!("Prompt {} retry failed: {}", &pid[..8], e));
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

    // X Bot mention polling (every 2 minutes, skip if not configured)
    let bot_state = state.clone();
    tokio::spawn(async move {
        // Wait 30s before starting to avoid startup noise
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;

            let cfg = &bot_state.config;
            if cfg.twitter_bot_bearer_token.is_empty() || cfg.twitter_bot_user_id.is_empty() {
                continue; // Bot not configured, skip
            }

            if let Err(e) = run_bot_cycle(&bot_state).await {
                tracing::error!("Bot cycle error: {}", e);
                bot_state.emit_log("ERROR", "x_bot", &format!("Bot cycle error: {}", e));
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

async fn run_bot_cycle(state: &state::AppState) -> anyhow::Result<()> {
    let cfg = &state.config;

    // Get since_id from DB
    let since_id = {
        let db = state.db.lock().await;
        db::queries::get_latest_interaction_tweet_id(&db)?
    };

    let result = services::x_bot::fetch_mentions(
        &cfg.twitter_bot_bearer_token,
        &cfg.twitter_bot_user_id,
        since_id.as_deref(),
    )
    .await?;

    if result.mentions.is_empty() {
        return Ok(());
    }

    state.emit_log(
        "INFO",
        "x_bot",
        &format!("Processing {} new mentions", result.mentions.len()),
    );

    for mention in &result.mentions {
        // Check if already processed
        {
            let db = state.db.lock().await;
            if db::queries::interaction_exists(&db, &mention.id)? {
                continue;
            }
        }

        let author_id = mention.author_id.as_deref().unwrap_or("");
        let username = result
            .users
            .iter()
            .find(|u| u.id == author_id)
            .map(|u| u.username.as_str())
            .unwrap_or("unknown");

        // Rate limit: max 5/day per user
        {
            let db = state.db.lock().await;
            let count = db::queries::count_user_interactions_today(&db, author_id)?;
            if count >= 5 {
                let interaction_id = uuid::Uuid::new_v4().to_string();
                db::queries::insert_x_interaction(
                    &db,
                    &interaction_id,
                    &mention.id,
                    Some(author_id),
                    Some(username),
                    Some(&mention.text),
                    "rate_limited",
                )?;
                continue;
            }
        }

        let interaction_id = uuid::Uuid::new_v4().to_string();
        {
            let db = state.db.lock().await;
            db::queries::insert_x_interaction(
                &db,
                &interaction_id,
                &mention.id,
                Some(author_id),
                Some(username),
                Some(&mention.text),
                "pending",
            )?;
        }

        // Classify with Claude Haiku
        let classification = match services::claude::classify_mention(
            &cfg.anthropic_api_key,
            &mention.text,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                state.emit_log("ERROR", "x_bot", &format!("Classification failed: {}", e));
                let db = state.db.lock().await;
                let _ = db::queries::update_x_interaction_status(
                    &db,
                    &interaction_id,
                    "failed",
                    None,
                    None,
                    None,
                );
                continue;
            }
        };

        if !classification.is_genuine {
            let db = state.db.lock().await;
            let _ = db::queries::update_x_interaction_status(
                &db,
                &interaction_id,
                "classified_spam",
                Some("spam"),
                None,
                None,
            );
            continue;
        }

        let prompt_text = classification.prompt_text.unwrap_or_else(|| mention.text.clone());
        state.emit_log(
            "INFO",
            "x_bot",
            &format!("Genuine mention from @{}: {}", username, &prompt_text[..prompt_text.len().min(60)]),
        );

        // Create prompt
        let prompt_id = uuid::Uuid::new_v4().to_string();
        {
            let db = state.db.lock().await;
            db::queries::ensure_user(&db, author_id)?;
            db::queries::insert_prompt(&db, &prompt_id, author_id, &prompt_text, None)?;
            db::queries::update_prompt_status(&db, &prompt_id, "generating")?;
            db::queries::update_x_interaction_status(
                &db,
                &interaction_id,
                "prompt_created",
                Some("genuine"),
                Some(&prompt_id),
                None,
            )?;
        }

        // Generate prompt image
        match pipeline::prompt_gen::generate_prompt_image(state, &prompt_id, &prompt_text).await {
            Ok(image_path) => {
                let db = state.db.lock().await;
                let _ = db::queries::update_prompt_image(&db, &prompt_id, &image_path);
            }
            Err(e) => {
                state.emit_log("ERROR", "x_bot", &format!("Image gen failed: {}", e));
                let db = state.db.lock().await;
                let _ = db::queries::update_prompt_status(&db, &prompt_id, "failed");
            }
        }

        // Reply with link
        let reply_text = format!(
            "here's your prompt, @{}. write for 8 minutes without stopping.\n\nhttps://anky.app/prompt/{}",
            username, prompt_id
        );

        match services::x_bot::reply_to_tweet(
            &cfg.twitter_bot_api_key,
            &cfg.twitter_bot_api_secret,
            &cfg.twitter_bot_access_token,
            &cfg.twitter_bot_access_secret,
            &mention.id,
            &reply_text,
        )
        .await
        {
            Ok(reply_id) => {
                let db = state.db.lock().await;
                let _ = db::queries::update_x_interaction_status(
                    &db,
                    &interaction_id,
                    "replied",
                    Some("genuine"),
                    Some(&prompt_id),
                    Some(&reply_id),
                );
                state.emit_log("INFO", "x_bot", &format!("Replied to @{}: {}", username, reply_id));
            }
            Err(e) => {
                state.emit_log("ERROR", "x_bot", &format!("Reply failed: {}", e));
                let db = state.db.lock().await;
                let _ = db::queries::update_x_interaction_status(
                    &db,
                    &interaction_id,
                    "failed",
                    Some("genuine"),
                    Some(&prompt_id),
                    None,
                );
            }
        }

        // Small delay between processing mentions
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    Ok(())
}
