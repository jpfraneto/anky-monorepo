use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};

/// Middleware that intercepts requests to pitch.anky.app and serves the pitch deck PDF,
/// and routes ankycoin.com to its own landing page.
pub async fn pitch_subdomain(req: Request, next: Next) -> Response {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // ankycoin.com — serve standalone landing page + farcaster miniapp
    if host == "ankycoin.com" || host == "www.ankycoin.com" {
        let path = req.uri().path();

        // Farcaster manifest
        if path == "/.well-known/farcaster.json" {
            let json = include_str!("../../static/ankycoin-farcaster.json");
            return (
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                json,
            )
                .into_response();
        }

        // Serve static assets, API routes, and special endpoints normally
        if path.starts_with("/static/")
            || path.starts_with("/data/")
            || path.starts_with("/favicon")
            || path.starts_with("/api/")
            || path == "/image.png"
            || path == "/splash.png"
        {
            return next.run(req).await;
        }

        // Everything else gets the landing page
        let html = include_str!("../../templates/ankycoin_landing.html");
        return axum::response::Html(html).into_response();
    }

    if host.starts_with("pitch.anky.app") {
        // For the PDF route itself, let it through
        if req.uri().path() == "/pitch-deck.pdf" {
            return next.run(req).await;
        }
        // Everything else on pitch.anky.app redirects to the PDF
        return Redirect::temporary("/pitch-deck.pdf").into_response();
    }

    next.run(req).await
}
