use crate::db::{self, queries};
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use axum_extra::extract::Host;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::Deserialize;
use serde_json::json;

/// POST /api/auth/qr — create a QR auth challenge
pub async fn create_challenge(
    State(state): State<AppState>,
    Host(host): Host,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339();

    let db = db::conn(&state.db)?;
    db.execute(
        "INSERT INTO qr_auth_challenges (id, token, expires_at) VALUES (?1, ?2, ?3)",
        crate::params![id, token, expires_at],
    )?;

    let app_scheme_url = format!("anky://seal?challenge={}", token);
    let bridge_url = format!(
        "{}/seal?challenge={}",
        request_origin(&headers, &host),
        urlencoding::encode(&token)
    );
    let qr_svg = render_qr_svg(&bridge_url)?;

    Ok(Json(json!({
        "id": id,
        "token": token,
        "deeplink": bridge_url,
        "bridge_url": bridge_url,
        "app_scheme_url": app_scheme_url,
        "install_url": state.config.ios_app_url,
        "qr_svg": qr_svg,
        "expires_in": 300,
    })))
}

fn request_origin(headers: &HeaderMap, host: &str) -> String {
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .unwrap_or_else(|| {
            if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
                "http".to_string()
            } else {
                "https".to_string()
            }
        });
    format!("{proto}://{host}")
}

pub(crate) fn render_qr_svg(contents: &str) -> Result<String, AppError> {
    let code = QrCode::new(contents.as_bytes())
        .map_err(|e| AppError::Internal(format!("qr encode: {e}")))?;

    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(180, 180)
        .dark_color(svg::Color("#04040d"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

/// GET /api/auth/qr/{id} — poll challenge status (browser polls this)
pub async fn poll_challenge(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::conn(&state.db)?;

    let result = db.query_row(
        "SELECT sealed, session_token, solana_address FROM qr_auth_challenges WHERE id = ?1",
        crate::params![id],
        |row| {
            Ok((
                row.get::<_, bool>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        },
    );

    match result {
        Ok((sealed, session_token, solana_address)) => Ok(Json(json!({
            "sealed": sealed,
            "session_token": session_token,
            "solana_address": solana_address,
        }))),
        Err(_) => Err(AppError::NotFound("challenge not found".into())),
    }
}

#[derive(Deserialize)]
pub struct SealByIdRequest {
    pub signature: String,
    pub solana_address: String,
}

/// POST /api/auth/qr/{id}/seal — seal by challenge ID (if caller knows it)
pub async fn seal_challenge(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SealByIdRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    do_seal(&state, "id", &id, &body.signature, &body.solana_address)
}

#[derive(Deserialize)]
pub struct SealByTokenRequest {
    pub token: String,
    pub signature: String,
    pub solana_address: String,
}

/// POST /api/auth/qr/seal — seal by challenge token (iOS app uses this)
/// The deep link only provides the token, not the ID.
pub async fn seal_by_token(
    State(state): State<AppState>,
    Json(body): Json<SealByTokenRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    do_seal(
        &state,
        "token",
        &body.token,
        &body.signature,
        &body.solana_address,
    )
}

fn do_seal(
    state: &AppState,
    lookup_col: &str,
    lookup_val: &str,
    signature: &str,
    solana_address: &str,
) -> Result<Json<serde_json::Value>, AppError> {
    let solana_address = crate::services::wallet::normalize_solana_address(solana_address)?;

    let db = db::conn(&state.db)?;

    // Lookup by id or token
    let query = if lookup_col == "token" {
        "SELECT id, token, sealed, expires_at FROM qr_auth_challenges WHERE token = ?1"
    } else {
        "SELECT id, token, sealed, expires_at FROM qr_auth_challenges WHERE id = ?1"
    };

    let (challenge_id, challenge_token, sealed, expires_at) = db
        .query_row(query, crate::params![lookup_val.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| AppError::NotFound("challenge not found".into()))?;

    if sealed {
        return Err(AppError::BadRequest("challenge already sealed".into()));
    }

    let now = chrono::Utc::now().to_rfc3339();
    if now > expires_at {
        return Err(AppError::BadRequest("challenge expired".into()));
    }

    crate::services::wallet::verify_solana_signature(&solana_address, &challenge_token, signature)?;

    // Ensure user exists
    queries::create_user_with_wallet(&db, &solana_address, &solana_address)?;

    // Create session
    let session_token = uuid::Uuid::new_v4().to_string();
    let session_expires = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();
    queries::create_auth_session(&db, &session_token, &solana_address, None, &session_expires)?;

    // Mark challenge as sealed
    db.execute(
        "UPDATE qr_auth_challenges SET sealed = true, session_token = ?1, solana_address = ?2 WHERE id = ?3",
        crate::params![session_token, solana_address, challenge_id],
    )?;

    Ok(Json(json!({
        "ok": true,
        "solana_address": solana_address,
    })))
}
