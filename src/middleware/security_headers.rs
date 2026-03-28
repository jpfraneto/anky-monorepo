use axum::extract::Request;
use axum::http::{header, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

pub async fn security_headers(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();

    // Cache control: static assets get cached, SSE streams left alone, everything else must revalidate
    if path.starts_with("/static/") {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=3600, stale-while-revalidate=86400"),
        );
    } else if path.starts_with("/api/stream-") || path.starts_with("/api/live-") {
        // SSE endpoints: no-transform prevents Cloudflare from buffering
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-transform"),
        );
        headers.insert(
            header::HeaderName::from_static("x-accel-buffering"),
            HeaderValue::from_static("no"),
        );
    } else {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        );
        headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    }

    // Allow Farcaster clients to iframe the app (MiniApp runs in iframe)
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("ALLOW-FROM https://farcaster.xyz"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline' https://esm.sh https://static.cloudflareinsights.com https://cdn.privy.io; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob: https:; connect-src 'self' https://auth.privy.io https://esm.sh https://*.privy.io wss://*.privy.io https://fonts.googleapis.com https://fonts.gstatic.com https://static.cloudflareinsights.com; frame-src https://auth.privy.io https://*.privy.io; worker-src 'self' blob:; frame-ancestors https://farcaster.xyz https://*.farcaster.xyz https://warpcast.com https://*.warpcast.com",
        ),
    );

    resp
}
