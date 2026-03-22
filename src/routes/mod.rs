pub mod api;
pub mod auth;
pub mod collection;
pub mod dashboard;
pub mod evolve;
pub mod extension_api;
pub mod generations;
pub mod health;
pub mod interview;
pub mod live;
pub mod notification;
pub mod pages;
pub mod payment;
pub mod payment_helper;
pub mod poiesis;
pub mod prompt;
pub mod session;
pub mod settings;
pub mod simulations;
pub mod social_context;
pub mod swift;
pub mod training;
pub mod voices;
pub mod webhook_farcaster;
pub mod webhook_x;
pub mod writing;

use crate::middleware;
use crate::state::AppState;
use axum::http::{header, HeaderValue, Method};
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

async fn farcaster_manifest() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        include_str!("../../static/farcaster.json"),
    )
}

async fn agent_manifest() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        include_str!("../../static/agent.json"),
    )
}

async fn service_worker() -> ([(axum::http::HeaderName, &'static str); 2], &'static str) {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            ("Service-Worker-Allowed".parse().unwrap(), "/"),
        ],
        include_str!("../../static/sw.js"),
    )
}

async fn prompt_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../PROMPT.md"),
    )
}

async fn skills() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        include_str!("../../skills.md"),
    )
}

async fn skills_redirect() -> axum::response::Redirect {
    axum::response::Redirect::permanent("/skills")
}

async fn anky_skill_bundle() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        "Anky installable skill bundle\n\n\
Bundle URL: https://anky.app/agent-skills/anky\n\
Manifest: https://anky.app/agent-skills/anky/manifest.json\n\
Entrypoint: https://anky.app/agent-skills/anky/SKILL.md\n\
\n\
Supporting files:\n\
- https://anky.app/agent-skills/anky/references/api.md\n\
- https://anky.app/agent-skills/anky/references/automation.md\n\
- https://anky.app/agent-skills/anky/references/quality.md\n\
- https://anky.app/agent-skills/anky/scripts/anky_session.py\n\
- https://anky.app/agent-skills/anky/agents/openai.yaml\n\
\n\
Session replay endpoint:\n\
- https://anky.app/api/v1/session/{session_id}/events (requires X-API-Key)\n\
- https://anky.app/api/v1/session/{session_id}/result (requires X-API-Key)\n\
\n\
Canonical practice doc: https://anky.app/skills\n",
    )
}

async fn anky_skill_bundle_manifest() -> ([(axum::http::HeaderName, &'static str); 1], &'static str)
{
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        include_str!("../../agent-skills/anky/manifest.json"),
    )
}

async fn anky_skill_bundle_entry_redirect() -> axum::response::Redirect {
    axum::response::Redirect::permanent("/agent-skills/anky/SKILL.md")
}

pub fn build_router(state: AppState) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin([
            "https://anky.app".parse::<HeaderValue>().unwrap(),
            "https://www.anky.app".parse::<HeaderValue>().unwrap(),
            "https://pitch.anky.app".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            "x-api-key".parse().unwrap(),
            "payment-signature".parse().unwrap(),
            "x-payment".parse().unwrap(),
            "x-wallet".parse().unwrap(),
        ])
        .expose_headers([
            "payment-required".parse::<header::HeaderName>().unwrap(),
            "payment-response".parse::<header::HeaderName>().unwrap(),
        ])
        .allow_credentials(false);

    // Mobile CORS — allow any origin for the /swift/* routes (native apps + testing)
    let mobile_cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(false);

    // Paid generate routes (optional API key — payment handled in handler)
    let generate_routes = Router::new()
        .route(
            "/api/v1/generate",
            axum::routing::post(api::generate_anky_paid),
        )
        .route(
            "/api/v1/prompt",
            axum::routing::post(prompt::create_prompt_api),
        )
        .route(
            "/api/v1/prompt/create",
            axum::routing::post(prompt::create_prompt_api),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::api_auth::optional_api_key,
        ));

    // Studio upload route (needs large body limit for video)
    let studio_routes = Router::new()
        .route(
            "/api/v1/studio/upload",
            axum::routing::post(api::upload_studio_video),
        )
        .layer(axum::extract::DefaultBodyLimit::max(512 * 1024 * 1024)); // 512MB

    // Extension API routes (optional API key — payment handled in handler)
    let extension_routes = Router::new()
        .route(
            "/api/v1/transform",
            axum::routing::post(extension_api::transform),
        )
        .route(
            "/api/v1/balance",
            axum::routing::get(extension_api::balance),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::api_auth::optional_api_key,
        ));

    // Swift / Mobile API routes under /swift/v1/
    let swift_routes = Router::new()
        // Auth
        .route(
            "/swift/v1/auth/privy",
            axum::routing::post(swift::auth_privy),
        )
        .route(
            "/swift/v2/auth/challenge",
            axum::routing::post(swift::auth_seed_challenge),
        )
        .route(
            "/swift/v2/auth/verify",
            axum::routing::post(swift::auth_seed_verify),
        )
        .route(
            "/swift/v1/auth/session",
            axum::routing::delete(swift::auth_logout),
        )
        .route(
            "/swift/v2/auth/session",
            axum::routing::delete(swift::auth_logout),
        )
        // Me
        .route("/swift/v1/me", axum::routing::get(swift::get_me))
        .route("/swift/v2/me", axum::routing::get(swift::get_me))
        // Writings
        .route(
            "/swift/v1/writings",
            axum::routing::get(swift::list_writings),
        )
        .route(
            "/swift/v2/writings",
            axum::routing::get(swift::list_writings),
        )
        .route(
            "/swift/v1/write",
            axum::routing::post(swift::submit_writing_unified),
        )
        .route(
            "/swift/v2/write",
            axum::routing::post(swift::submit_writing_unified),
        )
        .route(
            "/swift/v2/writing/{sessionId}/status",
            axum::routing::get(swift::get_writing_status),
        )
        // Children
        .route(
            "/swift/v2/children",
            axum::routing::get(swift::list_children).post(swift::create_child_profile),
        )
        .route(
            "/swift/v2/children/{childId}",
            axum::routing::get(swift::get_child_profile),
        )
        // Cuentacuentos
        .route(
            "/swift/v2/cuentacuentos/ready",
            axum::routing::get(swift::cuentacuentos_ready),
        )
        .route(
            "/swift/v2/cuentacuentos/history",
            axum::routing::get(swift::cuentacuentos_history),
        )
        .route(
            "/swift/v2/cuentacuentos/{id}/complete",
            axum::routing::post(swift::complete_cuentacuentos),
        )
        .route(
            "/swift/v2/cuentacuentos/{id}/assign",
            axum::routing::post(swift::assign_cuentacuentos),
        )
        // Next Prompt
        .route(
            "/swift/v2/next-prompt",
            axum::routing::get(swift::get_next_prompt),
        )
        // You (profile)
        .route("/swift/v2/you", axum::routing::get(swift::get_you))
        // Device Token (legacy path)
        .route(
            "/swift/v2/device-token",
            axum::routing::post(swift::register_device),
        )
        // Devices (new spec path)
        .route(
            "/swift/v2/devices",
            axum::routing::post(swift::register_device).delete(swift::delete_device),
        )
        // Settings
        .route(
            "/swift/v2/settings",
            axum::routing::get(swift::get_settings).patch(swift::patch_settings),
        )
        // Admin
        .route(
            "/swift/v1/admin/premium",
            axum::routing::post(swift::set_premium),
        )
        .layer(mobile_cors);

    Router::new()
        // Pages
        .route("/", axum::routing::get(pages::home))
        .route("/stories", axum::routing::get(pages::stories_page))
        .route("/you", axum::routing::get(pages::you_page))
        .route("/gallery", axum::routing::get(pages::gallery))
        .route(
            "/gallery/dataset-round-two",
            axum::routing::get(pages::dataset_round_two),
        )
        .route(
            "/gallery/dataset-round-two/og-image",
            axum::routing::get(pages::dataset_og_image),
        )
        .route(
            "/gallery/dataset-round-two/eliminate",
            axum::routing::post(pages::dataset_eliminate),
        )
        .route("/video-gallery", axum::routing::get(pages::videos_gallery))
        .route("/feed", axum::routing::get(pages::feed_page))
        .route("/help", axum::routing::get(pages::help))
        .route("/mobile", axum::routing::get(pages::mobile))
        .route("/dca", axum::routing::get(pages::dca_dashboard))
        .route("/dca-bot-code", axum::routing::get(pages::dca_bot_code))
        .route("/login", axum::routing::get(pages::login_page))
        .route("/ankycoin", axum::routing::get(pages::ankycoin_page))
        .route("/leaderboard", axum::routing::get(pages::leaderboard))
        .route("/pitch", axum::routing::get(pages::pitch))
        .route("/generate", axum::routing::get(pages::generate_page))
        .route(
            "/create-videos",
            axum::routing::get(pages::create_videos_page),
        )
        .route(
            "/generate/video",
            axum::routing::get(pages::video_dashboard),
        )
        .route(
            "/video/pipeline",
            axum::routing::get(pages::video_pipeline_page),
        )
        .route(
            "/video-dashboard",
            axum::routing::get(pages::media_dashboard),
        )
        .route("/sleeping", axum::routing::get(pages::sleeping))
        .route("/feedback", axum::routing::get(pages::feedback))
        .route("/changelog", axum::routing::get(pages::changelog))
        // Simulations — 8-slot inference dashboard
        .route(
            "/simulations",
            axum::routing::get(simulations::simulations_page),
        )
        .route(
            "/api/simulations/slots",
            axum::routing::get(simulations::slots_status),
        )
        .route(
            "/api/simulations/slots/stream",
            axum::routing::get(simulations::slots_stream),
        )
        .route(
            "/api/simulations/slots/demo",
            axum::routing::post(simulations::slots_demo),
        )
        .route("/llm", axum::routing::get(pages::llm))
        .route("/pitch-deck", axum::routing::get(pages::pitch_deck))
        .route("/pitch-deck.pdf", axum::routing::get(pages::pitch_deck_pdf))
        .route(
            "/api/v1/llm/training-status",
            axum::routing::post(api::llm_training_status),
        )
        .route("/anky/{id}", axum::routing::get(pages::anky_detail))
        // Public story deep link page (no auth)
        .route(
            "/story/{story_id}",
            axum::routing::get(voices::story_deep_link_page),
        )
        // Prompt pages
        .route(
            "/prompt/create",
            axum::routing::get(prompt::create_prompt_page),
        )
        .route("/prompt/{id}", axum::routing::get(prompt::prompt_page))
        // Prompt API
        .route(
            "/api/v1/prompt/{id}",
            axum::routing::get(prompt::get_prompt_api),
        )
        .route(
            "/api/v1/prompt/{id}/write",
            axum::routing::post(prompt::submit_prompt_writing),
        )
        .route(
            "/api/v1/prompts",
            axum::routing::get(prompt::list_prompts_api),
        )
        .route(
            "/api/v1/prompts/random",
            axum::routing::get(prompt::random_prompt_api),
        )
        // Settings
        .route("/settings", axum::routing::get(settings::settings_page))
        .route(
            "/api/settings",
            axum::routing::post(settings::save_settings),
        )
        .route(
            "/api/claim-username",
            axum::routing::post(settings::claim_username),
        )
        // Auth
        .route("/auth/x/login", axum::routing::get(auth::login))
        .route("/auth/x/callback", axum::routing::get(auth::callback))
        .route("/auth/x/logout", axum::routing::get(auth::logout))
        // Privy auth
        .route(
            "/auth/privy/verify",
            axum::routing::post(auth::privy_verify),
        )
        .route(
            "/auth/privy/logout",
            axum::routing::post(auth::privy_logout),
        )
        // Seed identity auth (web)
        .route("/auth/seed/verify", axum::routing::post(auth::seed_verify))
        .route("/auth/seed/logout", axum::routing::post(auth::seed_logout))
        // Farcaster MiniApp auth
        .route(
            "/auth/farcaster/verify",
            axum::routing::post(auth::farcaster_verify),
        )
        // Writing
        .route("/write", axum::routing::post(writing::process_writing))
        .route("/writings", axum::routing::get(writing::get_writings))
        // Collection
        .route(
            "/collection/create",
            axum::routing::post(collection::create_collection),
        )
        .route(
            "/collection/{id}",
            axum::routing::get(collection::get_collection),
        )
        // Payment
        .route(
            "/payment/verify",
            axum::routing::post(payment::verify_payment),
        )
        // Notifications
        .route("/notify/signup", axum::routing::post(notification::signup))
        // API
        .route("/api/ankys", axum::routing::get(api::list_ankys))
        .route("/api/v1/ankys", axum::routing::get(api::list_ankys))
        .route("/api/generate", axum::routing::post(api::generate_anky))
        .route("/api/v1/anky/{id}", axum::routing::get(api::get_anky))
        .route(
            "/api/stream-reflection/{id}",
            axum::routing::get(api::stream_reflection),
        )
        .route(
            "/api/anky-card/{id}",
            axum::routing::get(api::anky_reflection_card_image),
        )
        .route("/api/checkpoint", axum::routing::post(api::save_checkpoint))
        .route(
            "/api/session/paused",
            axum::routing::get(api::get_paused_writing_session),
        )
        .route(
            "/api/session/pause",
            axum::routing::post(api::pause_writing_session),
        )
        .route(
            "/api/session/resume",
            axum::routing::post(api::resume_writing_session),
        )
        .route(
            "/api/session/discard",
            axum::routing::post(api::discard_paused_writing_session),
        )
        .route(
            "/api/prefetch-memory",
            axum::routing::post(api::prefetch_memory),
        )
        .route("/api/cost-estimate", axum::routing::get(api::cost_estimate))
        .route("/api/treasury", axum::routing::get(api::treasury_address))
        .route("/api/feedback", axum::routing::post(api::submit_feedback))
        .route(
            "/api/v1/feedback",
            axum::routing::post(api::submit_feedback),
        )
        .route("/api/chat", axum::routing::post(api::chat_with_anky))
        .route("/api/chat-quick", axum::routing::post(api::chat_quick))
        .route(
            "/api/suggest-replies",
            axum::routing::post(api::suggest_replies),
        )
        .route("/api/retry-failed", axum::routing::post(api::retry_failed))
        .route(
            "/api/v1/generate/video-frame",
            axum::routing::post(api::generate_video_frame),
        )
        .route(
            "/api/v1/generate/video",
            axum::routing::post(api::generate_video),
        )
        .route(
            "/api/v1/create-videos/{id}",
            axum::routing::get(api::get_create_video_card),
        )
        .route(
            "/api/v1/create-videos/image",
            axum::routing::post(api::generate_create_video_image),
        )
        .route(
            "/api/v1/create-videos/video",
            axum::routing::post(api::generate_create_video_clip),
        )
        .route(
            "/api/v1/video/{id}",
            axum::routing::get(api::get_video_project),
        )
        .route(
            "/api/v1/video/{id}/resume",
            axum::routing::post(api::resume_video_project),
        )
        .route(
            "/api/v1/video/pipeline/config",
            axum::routing::get(api::get_video_pipeline_config),
        )
        .route(
            "/api/v1/video/pipeline/config",
            axum::routing::post(api::save_video_pipeline_config),
        )
        .route("/api/v1/purge-cache", axum::routing::post(api::purge_cache))
        .route("/og/video", axum::routing::get(api::og_video_image))
        .route("/og/dca", axum::routing::get(api::og_dca_image))
        .route("/api/v1/feed", axum::routing::get(api::get_feed))
        .route(
            "/api/v1/anky/{id}/like",
            axum::routing::post(api::toggle_like),
        )
        .route("/api/v1/story/test", axum::routing::post(api::story_test))
        // Admin
        .route(
            "/admin/story-tester",
            axum::routing::get(api::admin_story_tester),
        )
        .route(
            "/api/v1/check-prompt",
            axum::routing::post(api::check_prompt),
        )
        // Farcaster OG embed image
        .route("/api/v1/og-embed", axum::routing::get(api::og_embed_image))
        // Public stories feed (no auth required)
        .route(
            "/api/v1/stories",
            axum::routing::get(swift::list_all_stories),
        )
        .route("/api/v1/stories/{id}", axum::routing::get(swift::get_story))
        // Anky Voices — story recordings
        .route(
            "/api/v1/stories/{story_id}/recordings",
            axum::routing::get(voices::list_recordings).post(voices::create_recording),
        )
        .route(
            "/api/v1/stories/{story_id}/voice",
            axum::routing::get(voices::get_voice),
        )
        .route(
            "/api/v1/stories/{story_id}/recordings/{recording_id}/complete",
            axum::routing::post(voices::complete_listen),
        )
        // Agent registration (no auth required)
        .route(
            "/api/v1/register",
            axum::routing::post(extension_api::register),
        )
        // Chunked writing sessions (agent stream-of-consciousness)
        .route(
            "/api/v1/session/start",
            axum::routing::post(session::start_session),
        )
        .route(
            "/api/v1/session/chunk",
            axum::routing::post(session::send_chunk),
        )
        .route(
            "/api/v1/session/{id}/events",
            axum::routing::get(session::session_events),
        )
        .route(
            "/api/v1/session/{id}/result",
            axum::routing::get(session::session_result),
        )
        .route(
            "/api/v1/session/{id}",
            axum::routing::get(session::session_status),
        )
        // Skills (for agents)
        .route("/PROMPT.md", axum::routing::get(prompt_md))
        .route("/skills", axum::routing::get(skills))
        .route("/skill", axum::routing::get(skills_redirect))
        .route("/skill.md", axum::routing::get(skills_redirect))
        .route("/skills.md", axum::routing::get(skills_redirect))
        .route("/agent-skills/anky", axum::routing::get(anky_skill_bundle))
        .route("/agent-skills/anky/", axum::routing::get(anky_skill_bundle))
        .route(
            "/agent-skills/anky/skill.md",
            axum::routing::get(anky_skill_bundle_entry_redirect),
        )
        .route(
            "/agent-skills/anky/skills.md",
            axum::routing::get(anky_skill_bundle_entry_redirect),
        )
        .route(
            "/agent-skills/anky/manifest.json",
            axum::routing::get(anky_skill_bundle_manifest),
        )
        // Live streaming — disabled (too slow, not worth it)
        // Routes kept in live.rs but not wired up
        .route("/api/ankys/today", axum::routing::get(live::todays_ankys))
        // Interview
        .route("/interview", axum::routing::get(interview::interview_page))
        .route(
            "/ws/interview",
            axum::routing::get(interview::ws_interview_proxy),
        )
        .route(
            "/api/interview/start",
            axum::routing::post(interview::interview_start),
        )
        .route(
            "/api/interview/message",
            axum::routing::post(interview::interview_message),
        )
        .route(
            "/api/interview/end",
            axum::routing::post(interview::interview_end),
        )
        .route(
            "/api/interview/history/{user_id}",
            axum::routing::get(interview::interview_history),
        )
        .route(
            "/api/interview/user-context/{user_id}",
            axum::routing::get(interview::interview_user_context),
        )
        // Stream overlay
        .route("/stream/overlay", axum::routing::get(pages::stream_overlay))
        // Generations review + live dashboard
        .route(
            "/generations",
            axum::routing::get(generations::list_batches),
        )
        .route(
            "/generations/{id}",
            axum::routing::get(generations::review_batch),
        )
        .route(
            "/generations/{id}/status",
            axum::routing::post(generations::save_status),
        )
        .route(
            "/generations/{id}/dashboard",
            axum::routing::get(generations::generation_dashboard),
        )
        .route(
            "/generations/{id}/progress",
            axum::routing::get(generations::generation_progress),
        )
        .route(
            "/generations/{id}/tinder",
            axum::routing::get(generations::review_images),
        )
        .route(
            "/generations/{id}/review",
            axum::routing::post(generations::save_review),
        )
        // Training curation
        .route("/training", axum::routing::get(training::training_page))
        .route("/trainings", axum::routing::get(training::trainings_list))
        .route(
            "/trainings/general-instructions",
            axum::routing::get(training::general_instructions),
        )
        .route(
            "/trainings/{date}",
            axum::routing::get(training::training_run_detail),
        )
        .route(
            "/api/training/next",
            axum::routing::get(training::next_image),
        )
        .route("/api/training/vote", axum::routing::post(training::vote))
        .route(
            "/api/training/heartbeat",
            axum::routing::post(training::training_heartbeat),
        )
        .route(
            "/api/training/state",
            axum::routing::get(training::training_state),
        )
        .route(
            "/training/live",
            axum::routing::get(training::training_live),
        )
        .route(
            "/training/live/samples/{filename}",
            axum::routing::get(training::training_sample_image),
        )
        // Memory
        .route(
            "/api/memory/backfill",
            axum::routing::post(api::memory_backfill),
        )
        // Evolution dashboard (public)
        .route("/evolve", axum::routing::get(evolve::evolve_dashboard))
        // Dashboard
        .route("/dashboard", axum::routing::get(dashboard::dashboard))
        .route(
            "/dashboard/logs",
            axum::routing::get(dashboard::dashboard_logs),
        )
        .route(
            "/dashboard/summaries",
            axum::routing::get(dashboard::dashboard_summaries),
        )
        // Farcaster MiniApp manifest
        .route(
            "/.well-known/farcaster.json",
            axum::routing::get(farcaster_manifest),
        )
        // Agent manifest (8004 registry / OASF)
        .route("/.well-known/agent", axum::routing::get(agent_manifest))
        // Service Worker (served from root for scope)
        .route("/sw.js", axum::routing::get(service_worker))
        // X Account Activity webhook (CRC + events)
        .route("/webhooks/x", axum::routing::get(webhook_x::webhook_crc))
        .route("/webhooks/x", axum::routing::post(webhook_x::webhook_post))
        // Farcaster (Neynar) webhook
        .route(
            "/webhooks/farcaster",
            axum::routing::post(webhook_farcaster::webhook_post),
        )
        // X Webhook live log viewer
        .route(
            "/webhooks/logs",
            axum::routing::get(webhook_x::webhook_logs_page),
        )
        .route(
            "/webhooks/logs/stream",
            axum::routing::get(webhook_x::webhook_logs_stream),
        )
        // Health
        .route("/health", axum::routing::get(health::health_check))
        // Swift / Mobile API
        .merge(swift_routes)
        // Extension API (authed)
        .merge(extension_routes)
        // Paid generate API (optional auth)
        .merge(generate_routes)
        // Studio upload (large body limit)
        .merge(studio_routes)
        // Static files
        .nest_service("/agent-skills", ServeDir::new("agent-skills"))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service(
            "/data/images",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("data/images")),
        )
        .nest_service(
            "/data/anky-images",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("data/anky-images")),
        )
        .nest_service("/data/writings", ServeDir::new("data/writings"))
        .nest_service("/videos", ServeDir::new("videos"))
        .nest_service("/data/videos", ServeDir::new("data/videos"))
        .nest_service("/gen-images", ServeDir::new("data/generations"))
        .nest_service(
            "/data/training-images",
            ServeDir::new("data/training-images"),
        )
        .nest_service("/data/training-runs", ServeDir::new("data/training_runs"))
        // Middleware layers (applied bottom-up)
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(256 * 1024)) // 256KB body limit
        .layer(axum::middleware::from_fn(
            middleware::security_headers::security_headers,
        ))
        .layer(axum::middleware::from_fn(
            middleware::honeypot::honeypot_and_attack_detection,
        ))
        .layer(axum::middleware::from_fn(
            middleware::subdomain::pitch_subdomain,
        ))
        .with_state(state)
}
