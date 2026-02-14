pub mod api;
pub mod auth;
pub mod collection;
pub mod credits;
pub mod dashboard;
pub mod extension_api;
pub mod health;
pub mod live;
pub mod notification;
pub mod pages;
pub mod payment;
pub mod payment_helper;
pub mod poiesis;
pub mod prompt;
pub mod settings;
pub mod writing;

use crate::middleware;
use crate::state::AppState;
use axum::http::{header, HeaderValue, Method};
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;

async fn skills() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        include_str!("../../skills.md"),
    )
}

async fn skills_redirect() -> axum::response::Redirect {
    axum::response::Redirect::permanent("/skills")
}

pub fn build_router(state: AppState) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin([
            "https://anky.app".parse::<HeaderValue>().unwrap(),
            "https://www.anky.app".parse::<HeaderValue>().unwrap(),
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

    // Paid generate route (optional API key — payment handled in handler)
    let generate_routes = Router::new()
        .route("/api/v1/generate", axum::routing::post(api::generate_anky_paid))
        .route("/api/v1/prompt", axum::routing::post(prompt::create_prompt_api))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::api_auth::optional_api_key,
        ));

    // Extension API routes (behind API key auth)
    let extension_routes = Router::new()
        .route("/api/v1/transform", axum::routing::post(extension_api::transform))
        .route("/api/v1/balance", axum::routing::get(extension_api::balance))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::api_auth::require_api_key,
        ));

    Router::new()
        // Pages
        .route("/", axum::routing::get(pages::home))
        .route("/gallery", axum::routing::get(pages::gallery))
        .route("/help", axum::routing::get(pages::help))
        .route("/generate", axum::routing::get(pages::generate_page))
        .route("/sleeping", axum::routing::get(pages::sleeping))
        .route("/feedback", axum::routing::get(pages::feedback))
        .route("/anky/{id}", axum::routing::get(pages::anky_detail))
        // Prompt pages
        .route("/prompt/create", axum::routing::get(prompt::create_prompt_page))
        .route("/prompt/{id}", axum::routing::get(prompt::prompt_page))
        // Prompt API
        .route("/api/v1/prompt/{id}", axum::routing::get(prompt::get_prompt_api))
        .route("/api/v1/prompt/{id}/write", axum::routing::post(prompt::submit_prompt_writing))
        .route("/api/v1/prompts", axum::routing::get(prompt::list_prompts_api))
        .route("/api/v1/prompts/random", axum::routing::get(prompt::random_prompt_api))
        // Settings
        .route("/settings", axum::routing::get(settings::settings_page))
        .route("/api/settings", axum::routing::post(settings::save_settings))
        // Auth
        .route("/auth/x/login", axum::routing::get(auth::login))
        .route("/auth/x/callback", axum::routing::get(auth::callback))
        .route("/auth/x/logout", axum::routing::get(auth::logout))
        // Privy auth
        .route("/auth/privy/verify", axum::routing::post(auth::privy_verify))
        .route("/auth/privy/logout", axum::routing::post(auth::privy_logout))
        // Writing
        .route("/write", axum::routing::post(writing::process_writing))
        .route("/writings", axum::routing::get(writing::get_writings))
        // Collection
        .route("/collection/create", axum::routing::post(collection::create_collection))
        .route("/collection/{id}", axum::routing::get(collection::get_collection))
        // Payment
        .route("/payment/verify", axum::routing::post(payment::verify_payment))
        // Notifications
        .route("/notify/signup", axum::routing::post(notification::signup))
        // API
        .route("/api/ankys", axum::routing::get(api::list_ankys))
        .route("/api/v1/ankys", axum::routing::get(api::list_ankys))
        .route("/api/generate", axum::routing::post(api::generate_anky))
        .route("/api/v1/anky/{id}", axum::routing::get(api::get_anky))
        .route("/api/stream-reflection/{id}", axum::routing::get(api::stream_reflection))
        .route("/api/checkpoint", axum::routing::post(api::save_checkpoint))
        .route("/api/cost-estimate", axum::routing::get(api::cost_estimate))
        .route("/api/treasury", axum::routing::get(api::treasury_address))
        .route("/api/feedback", axum::routing::post(api::submit_feedback))
        .route("/api/chat", axum::routing::post(api::chat_with_anky))
        .route("/api/chat-quick", axum::routing::post(api::chat_quick))
        .route("/api/retry-failed", axum::routing::post(api::retry_failed))
        .route("/api/v1/check-prompt", axum::routing::post(api::check_prompt))
        // Agent registration (no auth required)
        .route("/api/v1/register", axum::routing::post(extension_api::register))
        // Credits
        .route("/credits", axum::routing::get(credits::credits_page))
        .route("/credits/create-key", axum::routing::post(credits::create_key))
        .route("/credits/verify-payment", axum::routing::post(credits::verify_credit_payment))
        .route("/credits/usage", axum::routing::get(credits::usage_stats))
        // Skills (for agents)
        .route("/skills", axum::routing::get(skills))
        .route("/skill", axum::routing::get(skills_redirect))
        .route("/skill.md", axum::routing::get(skills_redirect))
        .route("/skills.md", axum::routing::get(skills_redirect))
        // Live streaming
        .route("/ws/live", axum::routing::get(live::ws_live))
        .route("/api/live-status", axum::routing::get(live::live_status_sse))
        .route("/api/live-check", axum::routing::get(live::live_check))
        // Dashboard
        .route("/dashboard", axum::routing::get(dashboard::dashboard))
        .route("/dashboard/logs", axum::routing::get(dashboard::dashboard_logs))
        // Health
        .route("/health", axum::routing::get(health::health_check))
        // Extension API (authed)
        .merge(extension_routes)
        // Paid generate API (optional auth)
        .merge(generate_routes)
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/data/images", ServeDir::new("data/images"))
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
        .with_state(state)
}
