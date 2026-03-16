use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};

/// Middleware that intercepts requests to pitch.anky.app and serves the pitch deck PDF.
pub async fn pitch_subdomain(req: Request, next: Next) -> Response {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

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
