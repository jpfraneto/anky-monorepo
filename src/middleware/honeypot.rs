use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;

const HONEYPOT_PATHS: &[&str] = &[
    "/.env",
    "/.git",
    "/.git/config",
    "/.git/HEAD",
    "/wp-admin",
    "/wp-login.php",
    "/wp-content",
    "/phpmyadmin",
    "/admin",
    "/administrator",
    "/xmlrpc.php",
    "/.htaccess",
    "/.htpasswd",
    "/config.php",
    "/backup",
    "/.aws",
    "/.ssh",
    "/server-status",
];

const ATTACK_PATTERNS: &[&str] = &[
    "SELECT ", "UNION ", "DROP ", "INSERT ", "DELETE ",
    "../", "..\\",
    "<script", "javascript:", "onerror=",
    "/etc/passwd", "/etc/shadow",
    "cmd.exe", "powershell",
];

const PHILOSOPHICAL_RESPONSES: &[&str] = &[
    "the vulnerability you seek is within yourself. have you tried sitting with that discomfort for 8 minutes?",
    "every port scan is really a search for meaning. what are you truly looking for?",
    "the shell you want to pop is the one you've built around your own consciousness.",
    "you won't find secrets here. only the ones you're hiding from yourself.",
    "this path leads nowhere external. the real exploit is self-awareness.",
    "what if instead of breaking in, you tried breaking through — your own creative blocks?",
    "the real injection is the stories you tell yourself about who you are.",
    "traversing paths won't help. try traversing the landscape of your own mind for 8 minutes.",
];

fn get_philosophical_response(seed: usize) -> &'static str {
    PHILOSOPHICAL_RESPONSES[seed % PHILOSOPHICAL_RESPONSES.len()]
}

pub async fn honeypot_and_attack_detection(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_lowercase();
    let query = req.uri().query().unwrap_or("").to_lowercase();

    // Check honeypot paths
    for honeypot in HONEYPOT_PATHS {
        if path == *honeypot || path.starts_with(&format!("{}/", honeypot)) {
            tracing::warn!(path = %req.uri().path(), "honeypot hit");
            let seed = path.len() + query.len();
            return Json(json!({
                "message": get_philosophical_response(seed),
                "suggestion": "try writing at https://anky.app instead",
                "duration": "8 minutes"
            }))
            .into_response();
        }
    }

    // Check for attack patterns in path and query
    let combined = format!("{} {}", path, query);
    for pattern in ATTACK_PATTERNS {
        if combined.contains(&pattern.to_lowercase()) {
            tracing::warn!(
                path = %req.uri().path(),
                query = %req.uri().query().unwrap_or(""),
                pattern = pattern,
                "attack pattern detected"
            );
            let seed = combined.len();
            return Json(json!({
                "message": get_philosophical_response(seed),
                "suggestion": "try writing at https://anky.app instead",
                "duration": "8 minutes"
            }))
            .into_response();
        }
    }

    next.run(req).await
}
