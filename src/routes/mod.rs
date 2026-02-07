pub mod api;
pub mod collection;
pub mod credits;
pub mod extension_api;
pub mod health;
pub mod notification;
pub mod pages;
pub mod payment;
pub mod poiesis;
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

pub fn build_router(state: AppState) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin([
            "https://anky.app".parse::<HeaderValue>().unwrap(),
            "https://www.anky.app".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, "x-api-key".parse().unwrap()])
        .allow_credentials(false);

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
        .route("/generate", axum::routing::get(pages::generate))
        .route("/sleeping", axum::routing::get(pages::sleeping))
        // Writing
        .route("/write", axum::routing::post(writing::process_writing))
        .route("/writings", axum::routing::get(writing::get_writings))
        // Collection
        .route("/collection/create", axum::routing::post(collection::create_collection))
        .route("/collection/{id}", axum::routing::get(collection::get_collection))
        // Poiesis
        .route("/poiesis", axum::routing::get(poiesis::poiesis_page))
        .route("/poiesis/stream", axum::routing::get(poiesis::poiesis_stream))
        // Payment
        .route("/payment/verify", axum::routing::post(payment::verify_payment))
        // Notifications
        .route("/notify/signup", axum::routing::post(notification::signup))
        // API
        .route("/api/ankys", axum::routing::get(api::list_ankys))
        .route("/api/generate", axum::routing::post(api::generate_anky))
        // Agent registration (no auth required)
        .route("/api/v1/register", axum::routing::post(extension_api::register))
        // Credits
        .route("/credits", axum::routing::get(credits::credits_page))
        .route("/credits/create-key", axum::routing::post(credits::create_key))
        .route("/credits/verify-payment", axum::routing::post(credits::verify_credit_payment))
        .route("/credits/usage", axum::routing::get(credits::usage_stats))
        // Skills (for agents)
        .route("/skills", axum::routing::get(skills))
        // Health
        .route("/health", axum::routing::get(health::health_check))
        // Extension API (authed)
        .merge(extension_routes)
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/data/images", ServeDir::new("data/images"))
        // Middleware layers (applied bottom-up)
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(64 * 1024)) // 64KB body limit
        .layer(axum::middleware::from_fn(
            middleware::security_headers::security_headers,
        ))
        .layer(axum::middleware::from_fn(
            middleware::honeypot::honeypot_and_attack_detection,
        ))
        .with_state(state)
}
