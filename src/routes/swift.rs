/// /swift/v1/* — Mobile API for the Anky iOS app
///
/// Auth: Bearer token in `Authorization: Bearer <session_token>` header.
/// Session tokens are the same as web sessions (auth_sessions table).
/// Mobile auth via Privy SDK: POST /swift/v1/auth/privy → returns session_token as JSON.
/// Seed-phrase identity auth on Base/EVM: POST /swift/v2/auth/challenge + POST /swift/v2/auth/verify.
use crate::db::queries;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use rand::RngCore;
use rusqlite;
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::collections::HashMap;

// ===== Auth helpers =====

/// Extract user_id from `Authorization: Bearer <token>` header.
async fn bearer_auth(state: &AppState, headers: &HeaderMap) -> Result<String, AppError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("missing Authorization: Bearer header".into()))?;

    let db = state.db.lock().await;
    let (user_id, _) = queries::get_auth_session(&db, token)?
        .ok_or_else(|| AppError::Unauthorized("invalid or expired session token".into()))?;

    Ok(user_id)
}

async fn bearer_wallet_auth(state: &AppState, headers: &HeaderMap) -> Result<String, AppError> {
    let user_id = bearer_auth(state, headers).await?;
    let db = state.db.lock().await;
    let wallet_address = queries::get_user_wallet(&db, &user_id)?
        .ok_or_else(|| AppError::Unauthorized("seed identity required".into()))?;
    normalize_seed_wallet_address(&wallet_address)
}

fn normalize_seed_wallet_address(wallet_address: &str) -> Result<String, AppError> {
    let trimmed = wallet_address.trim();
    let without_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);

    if without_prefix.len() != 40 || !without_prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::BadRequest(
            "wallet_address must be a 0x-prefixed EVM address".into(),
        ));
    }

    Ok(format!("0x{}", without_prefix.to_lowercase()))
}

fn evm_address_from_public_key(public_key: &PublicKey) -> String {
    let uncompressed = public_key.serialize_uncompressed();
    let digest = Keccak256::digest(&uncompressed[1..]);
    format!("0x{}", hex::encode(&digest[12..]))
}

fn build_seed_auth_message(wallet_address: &str, challenge_id: &str, nonce: &str) -> String {
    format!(
        "anky.app base identity sign in\n\naddress: {}\nchallenge id: {}\nnonce: {}\nchain id: 8453\n\nsign this only inside the anky app.",
        wallet_address, challenge_id, nonce
    )
}

fn ethereum_personal_sign_hash(message: &str) -> [u8; 32] {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.as_bytes().len());
    let mut hasher = Keccak256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(message.as_bytes());
    let digest = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    hash
}

fn parse_evm_signature(signature_hex: &str) -> Result<[u8; 65], AppError> {
    let trimmed = signature_hex.trim();
    let without_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let raw = hex::decode(without_prefix)
        .map_err(|_| AppError::BadRequest("invalid signature encoding".into()))?;
    raw.try_into()
        .map_err(|_| AppError::BadRequest("invalid signature length".into()))
}

fn parse_recovery_id(recovery_byte: u8) -> Result<RecoveryId, AppError> {
    let normalized = match recovery_byte {
        0 | 1 => recovery_byte as i32,
        27 | 28 => (recovery_byte - 27) as i32,
        _ => return Err(AppError::BadRequest("invalid signature recovery id".into())),
    };
    RecoveryId::from_i32(normalized)
        .map_err(|_| AppError::BadRequest("invalid signature recovery id".into()))
}

fn verify_seed_auth_signature(
    wallet_address: &str,
    message: &str,
    signature_hex: &str,
) -> Result<(), AppError> {
    let normalized_wallet = normalize_seed_wallet_address(wallet_address)?;
    let signature = parse_evm_signature(signature_hex)?;
    let recovery_id = parse_recovery_id(signature[64])?;
    let recoverable_signature = RecoverableSignature::from_compact(&signature[..64], recovery_id)
        .map_err(|_| AppError::BadRequest("invalid signature".into()))?;
    let digest = ethereum_personal_sign_hash(message);
    let message = Message::from_digest_slice(&digest)
        .map_err(|_| AppError::BadRequest("invalid signature".into()))?;
    let public_key = Secp256k1::new()
        .recover_ecdsa(&message, &recoverable_signature)
        .map_err(|_| AppError::Unauthorized("signature verification failed".into()))?;
    let recovered_address = evm_address_from_public_key(&public_key);

    if recovered_address != normalized_wallet {
        return Err(AppError::Unauthorized(
            "signature does not match wallet".into(),
        ));
    }

    Ok(())
}

// ===== Auth =====

#[derive(Deserialize)]
pub struct PrivyAuthRequest {
    pub auth_token: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub ok: bool,
    pub session_token: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
}

#[derive(Deserialize)]
pub struct SeedAuthChallengeRequest {
    pub wallet_address: String,
}

#[derive(Serialize)]
pub struct SeedAuthChallengeResponse {
    pub ok: bool,
    pub challenge_id: String,
    pub message: String,
    pub expires_at: String,
}

#[derive(Deserialize)]
pub struct SeedAuthVerifyRequest {
    pub wallet_address: String,
    pub challenge_id: String,
    pub signature: String,
}

/// POST /swift/v2/auth/challenge
/// Create a one-time sign-in challenge for a locally derived Base/EVM seed identity.
/// The iOS app should sign the returned `message` using EIP-191 / personal_sign semantics.
pub async fn auth_seed_challenge(
    State(state): State<AppState>,
    Json(req): Json<SeedAuthChallengeRequest>,
) -> Result<Json<SeedAuthChallengeResponse>, AppError> {
    let wallet_address = normalize_seed_wallet_address(&req.wallet_address)?;

    let challenge_id = uuid::Uuid::new_v4().to_string();
    let mut nonce_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = hex::encode(nonce_bytes);
    let message = build_seed_auth_message(&wallet_address, &challenge_id, &nonce);
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(10))
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    {
        let db = state.db.lock().await;
        queries::create_auth_challenge(&db, &challenge_id, &wallet_address, &message, &expires_at)?;
    }

    Ok(Json(SeedAuthChallengeResponse {
        ok: true,
        challenge_id,
        message,
        expires_at,
    }))
}

/// Core seed verify logic, usable from both mobile (JSON response) and web (cookie response).
pub async fn auth_seed_verify_inner(
    state: &AppState,
    req: &SeedAuthVerifyRequest,
) -> Result<AuthResponse, AppError> {
    let wallet_address = normalize_seed_wallet_address(&req.wallet_address)?;

    let (user_id, username) = {
        let db = state.db.lock().await;
        let challenge = queries::get_active_auth_challenge(&db, &req.challenge_id)?
            .ok_or_else(|| AppError::Unauthorized("invalid or expired challenge".into()))?;

        if challenge.id != req.challenge_id {
            return Err(AppError::Unauthorized("invalid challenge".into()));
        }
        if challenge.wallet_address != wallet_address {
            return Err(AppError::Unauthorized(
                "challenge does not match wallet".into(),
            ));
        }

        verify_seed_auth_signature(&wallet_address, &challenge.challenge_text, &req.signature)?;

        let user_id = if let Some(existing) = queries::get_user_by_wallet(&db, &wallet_address)? {
            existing
        } else {
            let uid = uuid::Uuid::new_v4().to_string();
            queries::create_user_with_wallet(&db, &uid, &wallet_address)?;
            uid
        };
        let username = queries::get_user_username(&db, &user_id).ok().flatten();

        if !queries::consume_auth_challenge(&db, &req.challenge_id)? {
            return Err(AppError::Unauthorized("challenge already used".into()));
        }

        (user_id, username)
    };

    let session_token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(90))
        .unwrap()
        .to_rfc3339();
    {
        let db = state.db.lock().await;
        queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;
    }

    Ok(AuthResponse {
        ok: true,
        session_token,
        user_id,
        username,
        email: None,
        wallet_address: Some(wallet_address),
    })
}

/// POST /swift/v2/auth/verify
/// Verify an EVM signature from the seed-derived keypair and return a normal Anky session token.
pub async fn auth_seed_verify(
    State(state): State<AppState>,
    Json(req): Json<SeedAuthVerifyRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let result = auth_seed_verify_inner(&state, &req).await?;

    state.emit_log(
        "INFO",
        "swift_auth",
        &format!(
            "Seed identity login: {} ({})",
            &result.user_id[..8],
            result.wallet_address.as_deref().unwrap_or("?")
        ),
    );

    Ok(Json(result))
}

/// POST /swift/v1/auth/privy
/// Verify a Privy auth token and return a session token for subsequent mobile requests.
/// The returned `session_token` must be stored securely (iOS Keychain) and sent as
/// `Authorization: Bearer <session_token>` on every request.
pub async fn auth_privy(
    State(state): State<AppState>,
    Json(req): Json<PrivyAuthRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let app_id = &state.config.privy_app_id;
    let app_secret = &state.config.privy_app_secret;
    let verification_key = &state.config.privy_verification_key;

    if app_id.is_empty() || app_secret.is_empty() {
        return Err(AppError::Internal("Privy not configured".into()));
    }

    // Verify Privy JWT
    let privy_did = if !verification_key.is_empty() {
        use jsonwebtoken::{Algorithm, DecodingKey, Validation};
        #[derive(serde::Deserialize)]
        struct Claims {
            sub: String,
        }
        let key = DecodingKey::from_ec_pem(verification_key.as_bytes())
            .map_err(|e| AppError::Internal(format!("Invalid Privy key: {}", e)))?;
        let mut val = Validation::new(Algorithm::ES256);
        val.set_issuer(&["privy.io"]);
        val.set_audience(&[app_id.as_str()]);
        jsonwebtoken::decode::<Claims>(&req.auth_token, &key, &val)
            .map_err(|_| AppError::BadRequest("invalid privy token".into()))?
            .claims
            .sub
    } else {
        let client = reqwest::Client::new();
        let resp = client
            .post("https://auth.privy.io/api/v1/sessions/verify")
            .header("privy-app-id", app_id.as_str())
            .basic_auth(app_id, Some(app_secret))
            .json(&serde_json::json!({ "auth_token": req.auth_token }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Privy verify failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(AppError::BadRequest("invalid privy token".into()));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Privy parse failed: {}", e)))?;
        body.get("user")
            .and_then(|u| u.get("id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Internal("no user id in Privy response".into()))?
    };

    // Look up or create user
    let (user_id, email, wallet, username) = {
        let db = state.db.lock().await;
        if let Some(uid) = queries::get_user_by_privy_did(&db, &privy_did)? {
            let email = queries::get_user_email(&db, &uid).ok().flatten();
            let wallet = queries::get_user_wallet(&db, &uid).ok().flatten();
            let username = queries::get_user_username(&db, &uid).ok().flatten();
            (uid, email, wallet, username)
        } else {
            // New user — fetch details from Privy API
            let client = reqwest::Client::new();
            let encoded = urlencoding::encode(&privy_did);
            let resp = client
                .get(format!("https://auth.privy.io/api/v1/users/{}", encoded))
                .header("privy-app-id", app_id.as_str())
                .basic_auth(app_id, Some(app_secret))
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("Privy user fetch: {}", e)))?;

            let user_data: serde_json::Value = if resp.status().is_success() {
                resp.json().await.unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

            let linked = user_data
                .get("linked_accounts")
                .and_then(|la| la.as_array())
                .cloned()
                .unwrap_or_default();

            let email: Option<String> = linked.iter().find_map(|a| {
                if a.get("type")?.as_str()? == "email" {
                    a.get("address")?.as_str().map(|s| s.to_string())
                } else {
                    None
                }
            });
            let wallet: Option<String> = linked.iter().find_map(|a| {
                if a.get("type")?.as_str()? == "wallet" {
                    a.get("address")?.as_str().map(|s| s.to_string())
                } else {
                    None
                }
            });

            let uid = uuid::Uuid::new_v4().to_string();
            queries::ensure_user(&db, &uid)?;
            queries::set_privy_did(&db, &uid, &privy_did)?;
            if let Some(ref e) = email {
                let _ = queries::set_email(&db, &uid, e);
            }
            if let Some(ref w) = wallet {
                let _ = queries::set_wallet_address(&db, &uid, w);
            }

            state.emit_log(
                "INFO",
                "swift_auth",
                &format!(
                    "New mobile user: {} (privy: {})",
                    &uid[..8],
                    &privy_did[..12]
                ),
            );
            (uid, email, wallet, None)
        }
    };

    // Create session token
    let session_token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(90))
        .unwrap()
        .to_rfc3339();
    {
        let db = state.db.lock().await;
        queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;
    }

    state.emit_log(
        "INFO",
        "swift_auth",
        &format!(
            "Mobile login: {} ({})",
            &user_id[..8],
            username.as_deref().unwrap_or("no username")
        ),
    );

    Ok(Json(AuthResponse {
        ok: true,
        session_token,
        user_id,
        username,
        email,
        wallet_address: wallet,
    }))
}

/// DELETE /swift/v1/auth/session — invalidate the current bearer token
pub async fn auth_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::BadRequest("missing Authorization header".into()))?;
    let db = state.db.lock().await;
    queries::delete_auth_session(&db, token)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ===== Me =====

#[derive(Serialize)]
pub struct MeResponse {
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    pub total_writings: i32,
    pub total_ankys: i32,
    pub preferred_language: String,
}

/// GET /swift/v1/me
pub async fn get_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = state.db.lock().await;

    let username = queries::get_user_username(&db, &user_id).ok().flatten();
    let email = queries::get_user_email(&db, &user_id).ok().flatten();
    let wallet = queries::get_user_wallet(&db, &user_id).ok().flatten();

    // Try to get X profile info via the x_users table
    let (display_name, profile_image_url) = {
        let mut stmt = db.prepare(
            "SELECT display_name, profile_image_url FROM x_users WHERE user_id = ?1 LIMIT 1",
        );
        if let Ok(ref mut s) = stmt {
            let mut rows = s.query_map(rusqlite::params![user_id], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            });
            if let Ok(ref mut r) = rows {
                r.next().and_then(|v| v.ok()).unwrap_or((None, None))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    };

    let writings = queries::get_user_writings(&db, &user_id).unwrap_or_default();
    let total_writings = writings.len() as i32;
    let total_ankys = writings.iter().filter(|w| w.is_anky).count() as i32;
    let settings = queries::get_user_settings(&db, &user_id)?;

    Ok(Json(MeResponse {
        user_id,
        username,
        display_name,
        profile_image_url,
        email,
        wallet_address: wallet,
        total_writings,
        total_ankys,
        preferred_language: settings.preferred_language,
    }))
}

// ===== Writings =====

#[derive(Serialize)]
pub struct WritingItem {
    pub id: String,
    pub content: String,
    pub duration_seconds: f64,
    pub word_count: i32,
    pub is_anky: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_image_path: Option<String>,
    pub created_at: String,
}

/// GET /swift/v1/writings
pub async fn list_writings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WritingItem>>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = state.db.lock().await;
    let writings = queries::get_user_writings_with_ankys(&db, &user_id)?;
    let items = writings
        .into_iter()
        .map(|w| WritingItem {
            id: w.id,
            content: w.content,
            duration_seconds: w.duration_seconds,
            word_count: w.word_count,
            is_anky: w.is_anky,
            response: w.response,
            anky_id: w.anky_id,
            anky_title: w.anky_title,
            anky_image_path: w.anky_image_path,
            created_at: w.created_at,
        })
        .collect();
    Ok(Json(items))
}

#[derive(Deserialize)]
pub struct MobileWriteRequest {
    pub text: String,
    pub duration: f64,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub keystroke_deltas: Option<Vec<f64>>,
    #[serde(default)]
    pub is_checkpoint: bool,
}

/// What the backend will do with this writing session.
/// The frontend uses this to evolve the UI immediately — no waiting.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WritingOutcome {
    /// Mid-session save — keep writing
    Checkpoint,
    /// Finished but too short to be an anky
    ShortSession,
    /// This is an anky — the real thing
    Anky,
}

/// What pipelines were spawned and will eventually land.
/// The frontend can start animating toward these immediately.
#[derive(Serialize)]
pub struct SpawnedPipelines {
    /// Anky image is being generated (poll /status for progress)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_id: Option<String>,
    /// Ollama feedback is being generated (poll /status)
    pub feedback: bool,
    /// Cuentacuentos story is being generated
    pub cuentacuentos: bool,
}

#[derive(Serialize)]
pub struct MobileWriteResponse {
    pub ok: bool,
    pub session_id: String,
    pub outcome: WritingOutcome,
    pub word_count: i32,
    pub duration_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_score: Option<f64>,
    /// Whether data was persisted server-side
    pub persisted: bool,
    /// What's coming — what the frontend should start evolving toward
    pub spawned: SpawnedPipelines,
    /// Wallet address if seed user (tells frontend this is a v2 identity)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    /// Status endpoint to poll for downstream progress
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// POST /swift/v1/write and /swift/v2/write — unified mobile writing handler.
///
/// Design: persist the raw data as fast as possible, return immediately with
/// what the frontend needs to start evolving the UI, then spawn all processing
/// in the background. Nothing blocks. The frontend polls /status to watch
/// the downstream artifacts materialize.
pub async fn submit_writing_unified(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<MobileWriteRequest>,
) -> Result<Json<MobileWriteResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let word_count = req.text.split_whitespace().count() as i32;
    let session_id = req
        .session_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let flow_score = req
        .keystroke_deltas
        .as_ref()
        .map(|d| queries::calculate_flow_score(d, req.duration, word_count));
    let keystroke_json = req
        .keystroke_deltas
        .as_ref()
        .map(|d| serde_json::to_string(d).unwrap_or_default());

    let is_anky = req.duration >= 480.0 && word_count >= 300;

    // Resolve wallet once — this determines seed vs privy behavior
    let wallet_address = {
        let db = state.db.lock().await;
        queries::get_user_wallet(&db, &user_id).ok().flatten()
    };
    let is_seed_user = wallet_address.is_some();

    let nothing_spawned = SpawnedPipelines {
        anky_id: None,
        feedback: false,
        cuentacuentos: false,
    };

    // === CHECKPOINT: mid-session save, return fast ===
    if req.is_checkpoint {
        {
            let db = state.db.lock().await;
            queries::ensure_user(&db, &user_id)?;
            queries::upsert_active_writing_session(
                &db,
                &session_id,
                &user_id,
                &req.text,
                req.duration,
                word_count,
                "active",
                false,
                None,
            )?;
            queries::insert_checkpoint(
                &db,
                &session_id,
                &req.text,
                req.duration,
                word_count,
                None,
            )?;
        }

        tracing::info!(
            user = %user_id,
            duration = req.duration,
            words = word_count,
            "Mobile checkpoint saved"
        );

        // Send checkpoint to Honcho (fire-and-forget)
        if crate::services::honcho::is_configured(&state.config) {
            let h_key = state.config.honcho_api_key.clone();
            let h_ws = state.config.honcho_workspace_id.clone();
            let h_sid = session_id.clone();
            let h_uid = user_id.clone();
            let h_text = req.text.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::services::honcho::send_writing(&h_key, &h_ws, &h_sid, &h_uid, &h_text)
                        .await
                {
                    tracing::warn!("Honcho checkpoint send failed: {}", e);
                }
            });
        }

        return Ok(Json(MobileWriteResponse {
            ok: true,
            session_id,
            outcome: WritingOutcome::Checkpoint,
            word_count,
            duration_seconds: req.duration,
            flow_score,
            persisted: true,
            spawned: nothing_spawned,
            wallet_address: None,
            status_url: None,
            error: None,
        }));
    }

    // === FINAL SUBMISSION: too few words to do anything with ===
    if word_count < 10 {
        return Ok(Json(MobileWriteResponse {
            ok: false,
            session_id: String::new(),
            outcome: WritingOutcome::ShortSession,
            word_count,
            duration_seconds: req.duration,
            flow_score: None,
            persisted: false,
            spawned: nothing_spawned,
            wallet_address: None,
            status_url: None,
            error: Some("write more — at least a few sentences to stream".into()),
        }));
    }

    // === SHORT SESSION (not an anky) ===
    if !is_anky {
        // Seed users: short sessions are local-only, not persisted
        if is_seed_user {
            tracing::info!(
                user = %user_id,
                duration = req.duration,
                words = word_count,
                "Mobile v2 short writing — local only"
            );
            return Ok(Json(MobileWriteResponse {
                ok: true,
                session_id,
                outcome: WritingOutcome::ShortSession,
                word_count,
                duration_seconds: req.duration,
                flow_score,
                persisted: false,
                spawned: nothing_spawned,
                wallet_address: wallet_address.clone(),
                status_url: None,
                error: None,
            }));
        }

        // Privy users: persist + spawn feedback + guidance in background
        {
            let db = state.db.lock().await;
            queries::ensure_user(&db, &user_id)?;
            queries::upsert_completed_writing_session_with_flow(
                &db,
                &session_id,
                &user_id,
                &req.text,
                req.duration,
                word_count,
                false,
                None,
                keystroke_json.as_deref(),
                flow_score,
                None,
            )?;
            if let Some(fs) = flow_score {
                let _ = queries::update_user_flow_stats(&db, &user_id, fs, false);
            }
        }

        tracing::info!(
            user = %user_id,
            duration = req.duration,
            words = word_count,
            "Mobile short writing persisted — spawning feedback + guidance"
        );

        // Spawn Ollama feedback in background (no blocking!)
        let fb_state = state.clone();
        let fb_session = session_id.clone();
        let fb_text = req.text.clone();
        let fb_duration = req.duration;
        tokio::spawn(async move {
            let prompt = crate::services::ollama::quick_feedback_prompt(&fb_text, fb_duration);
            let feedback = crate::services::ollama::call_ollama(
                &fb_state.config.ollama_base_url,
                &fb_state.config.ollama_model,
                &prompt,
            )
            .await
            .unwrap_or_else(|_| "keep flowing — every word counts.".into());
            let db = fb_state.db.lock().await;
            let _ = db.execute(
                "UPDATE writing_sessions SET response = ?1 WHERE id = ?2",
                rusqlite::params![&feedback, &fb_session],
            );
        });

        // Send to Honcho (fire-and-forget)
        if crate::services::honcho::is_configured(&state.config) {
            let h_key = state.config.honcho_api_key.clone();
            let h_ws = state.config.honcho_workspace_id.clone();
            let h_sid = session_id.clone();
            let h_uid = user_id.clone();
            let h_text = req.text.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::services::honcho::send_writing(&h_key, &h_ws, &h_sid, &h_uid, &h_text)
                        .await
                {
                    tracing::warn!("Honcho short-session send failed: {}", e);
                }
            });
        }

        // Generate next writing prompt (fire-and-forget)
        {
            let prompt_state = state.clone();
            let prompt_user = user_id.clone();
            let prompt_session = session_id.clone();
            tokio::spawn(async move {
                crate::pipeline::guidance_gen::generate_next_prompt(
                    &prompt_state,
                    &prompt_user,
                    &prompt_session,
                )
                .await;
            });
        }

        let status_url = format!("/swift/v2/writing/{}/status", session_id);
        return Ok(Json(MobileWriteResponse {
            ok: true,
            session_id,
            outcome: WritingOutcome::ShortSession,
            word_count,
            duration_seconds: req.duration,
            flow_score,
            persisted: true,
            spawned: SpawnedPipelines {
                anky_id: None,
                feedback: true,
                cuentacuentos: false,
            },
            wallet_address: None,
            status_url: Some(status_url),
            error: None,
        }));
    }

    // === ANKY: the real thing ===
    // Persist the writing session + create the anky record — then return immediately

    {
        let db = state.db.lock().await;
        queries::ensure_user(&db, &user_id)?;
        if let Some(existing) = queries::get_writing_session_state(&db, &session_id)? {
            if existing.user_id != user_id {
                let is_placeholder = existing.user_id == "system"
                    || existing.user_id == "recovered-unknown"
                    || existing.user_id.starts_with("recovered-");
                if !is_placeholder {
                    return Err(AppError::Unauthorized(
                        "that writing session belongs to another user".into(),
                    ));
                }
            }
        }

        queries::upsert_completed_writing_session_with_flow(
            &db,
            &session_id,
            &user_id,
            &req.text,
            req.duration,
            word_count,
            true,
            None,
            keystroke_json.as_deref(),
            flow_score,
            None,
        )?;
        if let Some(fs) = flow_score {
            let _ = queries::update_user_flow_stats(&db, &user_id, fs, true);
        }
    }

    // Create the anky DB record BEFORE spawning pipelines (critical bug fix)
    let anky_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().await;
        queries::insert_anky(
            &db,
            &anky_id,
            &session_id,
            &user_id,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            "generating",
            "mobile",
            None,
        )?;
    }

    tracing::info!(
        user = %user_id,
        duration = req.duration,
        words = word_count,
        anky_id = %anky_id,
        is_seed = is_seed_user,
        "Mobile anky persisted — spawning all pipelines"
    );

    // === Everything below is fire-and-forget — the response is already built ===

    // Archive to file if seed user
    if let Some(ref wa) = wallet_address {
        let wa_clone = wa.clone();
        let text_clone = req.text.clone();
        let sid_clone = session_id.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            let archive_timestamp = chrono::Utc::now().timestamp();
            if let Err(e) = crate::storage::files::save_writing_to_file(
                &wa_clone,
                archive_timestamp,
                &text_clone,
            ) {
                state_clone.emit_log(
                    "ERROR",
                    "writing_archive",
                    &format!(
                        "Failed to archive writing {} for {}: {}",
                        &sid_clone[..8.min(sid_clone.len())],
                        wa_clone,
                        e
                    ),
                );
            }
        });
    }

    // Submit anky image generation to priority queue
    let is_pro = {
        let db = state.db.lock().await;
        queries::is_user_pro(&db, &user_id).unwrap_or(false)
    };
    let priority = if is_pro { 1 } else { 0 };
    state.gpu_queue.submit(
        crate::state::GpuJob::AnkyImage {
            anky_id: anky_id.clone(),
            session_id: session_id.clone(),
            user_id: user_id.clone(),
            writing: req.text.clone(),
        },
        priority,
    );

    // Spawn cuentacuentos if seed user
    let has_cuentacuentos = wallet_address.is_some();
    if let Some(ref wa) = wallet_address {
        let cuentacuentos_state = state.clone();
        let cuentacuentos_session = session_id.clone();
        let cuentacuentos_parent_wallet = wa.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::pipeline::guidance_gen::queue_post_writing_cuentacuentos(
                &cuentacuentos_state,
                &cuentacuentos_session,
                &cuentacuentos_parent_wallet,
            )
            .await
            {
                cuentacuentos_state.emit_log(
                    "ERROR",
                    "swift_write",
                    &format!(
                        "Cuentacuentos gen failed for {}: {}",
                        &cuentacuentos_session[..8.min(cuentacuentos_session.len())],
                        e
                    ),
                );
            }
        });
    }

    // Send anky writing to Honcho (fire-and-forget)
    if crate::services::honcho::is_configured(&state.config) {
        let h_key = state.config.honcho_api_key.clone();
        let h_ws = state.config.honcho_workspace_id.clone();
        let h_sid = session_id.clone();
        let h_uid = user_id.clone();
        let h_text = req.text.clone();
        tokio::spawn(async move {
            if let Err(e) =
                crate::services::honcho::send_writing(&h_key, &h_ws, &h_sid, &h_uid, &h_text).await
            {
                tracing::warn!("Honcho anky send failed: {}", e);
            }
        });
    }

    // Next prompt generation is now handled sequentially inside
    // queue_post_writing_cuentacuentos as the final lifecycle step.
    // For non-seed users (no cuentacuentos), the prompt is still generated
    // in the short-session path above.

    let status_url = format!("/swift/v2/writing/{}/status", session_id);
    Ok(Json(MobileWriteResponse {
        ok: true,
        session_id,
        outcome: WritingOutcome::Anky,
        word_count,
        duration_seconds: req.duration,
        flow_score,
        persisted: true,
        spawned: SpawnedPipelines {
            anky_id: Some(anky_id),
            feedback: false,
            cuentacuentos: has_cuentacuentos,
        },
        wallet_address,
        status_url: Some(status_url),
        error: None,
    }))
}

// ===== Writing Status =====

#[derive(Serialize)]
pub struct WritingStatusResponse {
    pub session_id: String,
    pub is_anky: bool,
    pub duration_seconds: f64,
    pub word_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky: Option<AnkyStatusInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cuentacuentos: Option<CuentacuentosStatusInfo>,
}

#[derive(Serialize)]
pub struct AnkyStatusInfo {
    pub id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflection: Option<String>,
}

#[derive(Serialize)]
pub struct CuentacuentosStatusInfo {
    pub id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chakra: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kingdom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub translations_done: Vec<String>,
    pub images_total: i32,
    pub images_done: i32,
}

/// GET /swift/v2/writing/{sessionId}/status
pub async fn get_writing_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<WritingStatusResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = state.db.lock().await;

    // Get the writing session
    let writing = queries::get_writing_session(&db, &session_id)
        .ok()
        .flatten()
        .ok_or_else(|| AppError::NotFound("writing session not found".into()))?;

    // Get anky info
    let anky = queries::get_anky_by_writing_session_id(&db, &session_id)?;
    let anky_info = anky.map(
        |(id, status, image_path, title, reflection)| AnkyStatusInfo {
            id,
            status,
            image_url: image_path,
            title,
            reflection,
        },
    );

    // Get cuentacuentos info
    let cuentacuentos_info =
        if let Some(story) = queries::get_cuentacuentos_by_writing_id(&db, &session_id)? {
            let images = queries::get_cuentacuentos_images(&db, &story.id)?;
            let images_total = images.len() as i32;
            let images_done = images.iter().filter(|i| i.image_url.is_some()).count() as i32;
            let mut translations_done = Vec::new();
            if story.content_es.is_some() {
                translations_done.push("es".to_string());
            }
            if story.content_zh.is_some() {
                translations_done.push("zh".to_string());
            }
            if story.content_hi.is_some() {
                translations_done.push("hi".to_string());
            }
            if story.content_ar.is_some() {
                translations_done.push("ar".to_string());
            }

            let status = if images_done == images_total && images_total > 0 {
                "ready".to_string()
            } else {
                "generating".to_string()
            };

            Some(CuentacuentosStatusInfo {
                id: story.id,
                status,
                chakra: story.chakra,
                kingdom: story.kingdom,
                city: story.city,
                title: Some(story.title),
                translations_done,
                images_total,
                images_done,
            })
        } else {
            None
        };

    Ok(Json(WritingStatusResponse {
        session_id,
        is_anky: writing.is_anky,
        duration_seconds: writing.duration_seconds,
        word_count: writing.word_count,
        anky: anky_info,
        cuentacuentos: cuentacuentos_info,
    }))
}

// ===== Children =====

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChildProfileRequest {
    pub name: String,
    pub birthdate: String,
    pub derived_wallet_address: String,
    pub emoji_pattern: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildProfileItem {
    pub id: String,
    pub parent_wallet_address: String,
    pub derived_wallet_address: String,
    pub name: String,
    pub birthdate: String,
    pub emoji_pattern: Vec<String>,
    pub created_at: String,
}

fn child_profile_to_item(record: queries::ChildProfileRecord) -> ChildProfileItem {
    ChildProfileItem {
        id: record.id,
        parent_wallet_address: record.parent_wallet_address,
        derived_wallet_address: record.derived_wallet_address,
        name: record.name,
        birthdate: record.birthdate,
        emoji_pattern: serde_json::from_str(&record.emoji_pattern).unwrap_or_default(),
        created_at: record.created_at,
    }
}

/// POST /swift/v2/children
pub async fn create_child_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateChildProfileRequest>,
) -> Result<Json<ChildProfileItem>, AppError> {
    let parent_wallet_address = bearer_wallet_auth(&state, &headers).await?;
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    let birthdate = chrono::NaiveDate::parse_from_str(req.birthdate.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("birthdate must be YYYY-MM-DD".into()))?
        .format("%Y-%m-%d")
        .to_string();
    let derived_wallet_address = normalize_seed_wallet_address(&req.derived_wallet_address)?;

    if req.emoji_pattern.len() != 12 {
        return Err(AppError::BadRequest(
            "emojiPattern must contain exactly 12 emoji strings".into(),
        ));
    }

    let emoji_pattern: Vec<String> = req
        .emoji_pattern
        .iter()
        .map(|emoji| emoji.trim().to_string())
        .collect();
    if emoji_pattern.iter().any(|emoji| emoji.is_empty()) {
        return Err(AppError::BadRequest(
            "emojiPattern must contain exactly 12 non-empty emoji strings".into(),
        ));
    }
    let emoji_pattern_json = serde_json::to_string(&emoji_pattern)
        .map_err(|e| AppError::Internal(format!("failed to encode emoji pattern: {}", e)))?;

    let child_id = uuid::Uuid::new_v4().to_string();
    let record = {
        let db = state.db.lock().await;
        queries::create_child_profile(
            &db,
            &child_id,
            &parent_wallet_address,
            &derived_wallet_address,
            name,
            &birthdate,
            &emoji_pattern_json,
        )?;
        queries::get_child_profile_by_id_and_parent_wallet(&db, &child_id, &parent_wallet_address)?
            .ok_or_else(|| AppError::Internal("failed to load child profile after insert".into()))?
    };

    Ok(Json(child_profile_to_item(record)))
}

/// GET /swift/v2/children
pub async fn list_children(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ChildProfileItem>>, AppError> {
    let parent_wallet_address = bearer_wallet_auth(&state, &headers).await?;
    let db = state.db.lock().await;
    let children = queries::get_child_profiles_by_parent_wallet(&db, &parent_wallet_address)?;
    Ok(Json(
        children
            .into_iter()
            .map(child_profile_to_item)
            .collect::<Vec<_>>(),
    ))
}

/// GET /swift/v2/children/:childId
pub async fn get_child_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(child_id): Path<String>,
) -> Result<Json<ChildProfileItem>, AppError> {
    let parent_wallet_address = bearer_wallet_auth(&state, &headers).await?;
    let db = state.db.lock().await;
    let child =
        queries::get_child_profile_by_id_and_parent_wallet(&db, &child_id, &parent_wallet_address)?
            .ok_or_else(|| AppError::NotFound("child profile not found".into()))?;
    Ok(Json(child_profile_to_item(child)))
}

// ===== Cuentacuentos =====

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CuentacuentosQuery {
    #[serde(default)]
    pub child_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignCuentacuentosRequest {
    pub child_wallet_address: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CuentacuentosItem {
    pub id: String,
    pub writing_id: String,
    pub parent_wallet_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_wallet_address: Option<String>,
    pub title: String,
    pub content: String,
    pub guidance_phases: serde_json::Value,
    pub played: bool,
    pub generated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chakra: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kingdom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_es: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_zh: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_ar: Option<String>,
    /// TTS audio URLs keyed by language code: {"en": "https://...", "es": "https://..."}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_urls: Option<HashMap<String, String>>,
}

fn cuentacuentos_to_item(record: queries::CuentacuentosRecord) -> CuentacuentosItem {
    CuentacuentosItem {
        id: record.id,
        writing_id: record.writing_id,
        parent_wallet_address: record.parent_wallet_address,
        child_wallet_address: record.child_wallet_address,
        title: record.title,
        content: record.content,
        guidance_phases: serde_json::from_str(&record.guidance_phases)
            .unwrap_or(serde_json::json!([])),
        played: record.played,
        generated_at: record.generated_at,
        chakra: record.chakra,
        kingdom: record.kingdom,
        city: record.city,
        content_es: record.content_es,
        content_zh: record.content_zh,
        content_hi: record.content_hi,
        content_ar: record.content_ar,
        audio_urls: None,
    }
}

fn cuentacuentos_to_ready_item(
    record: queries::CuentacuentosRecord,
    images: Vec<queries::CuentacuentosImageRecord>,
    audio: Vec<queries::CuentacuentosAudioRecord>,
) -> CuentacuentosItem {
    let audio_urls: HashMap<String, String> = audio
        .into_iter()
        .filter_map(|a| a.audio_url.map(|url| (a.language, url)))
        .collect();
    let image_urls: HashMap<i32, Option<String>> = images
        .into_iter()
        .map(|image| (image.phase_index, image.image_url))
        .collect();

    let guidance_phases = match serde_json::from_str::<serde_json::Value>(&record.guidance_phases)
        .unwrap_or(serde_json::json!([]))
    {
        serde_json::Value::Array(phases) => serde_json::Value::Array(
            phases
                .into_iter()
                .enumerate()
                .map(|(index, phase)| {
                    let mut obj = phase.as_object().cloned().unwrap_or_default();
                    let phase_type = obj
                        .get("phase_type")
                        .and_then(|value| value.as_str())
                        .unwrap_or("narration")
                        .to_string();
                    let content = obj
                        .get("content")
                        .and_then(|value| value.as_str())
                        .or_else(|| obj.get("narration").and_then(|value| value.as_str()))
                        .unwrap_or("")
                        .to_string();

                    obj.insert("phase_type".into(), serde_json::Value::String(phase_type));
                    obj.insert("content".into(), serde_json::Value::String(content.clone()));
                    obj.insert("narration".into(), serde_json::Value::String(content));
                    obj.insert(
                        "image_url".into(),
                        image_urls
                            .get(&(index as i32))
                            .cloned()
                            .flatten()
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                    );

                    serde_json::Value::Object(obj)
                })
                .collect(),
        ),
        _ => serde_json::json!([]),
    };

    CuentacuentosItem {
        id: record.id,
        writing_id: record.writing_id,
        parent_wallet_address: record.parent_wallet_address,
        child_wallet_address: record.child_wallet_address,
        title: record.title,
        content: record.content,
        guidance_phases,
        played: record.played,
        generated_at: record.generated_at,
        chakra: record.chakra,
        kingdom: record.kingdom,
        city: record.city,
        content_es: record.content_es,
        content_zh: record.content_zh,
        content_hi: record.content_hi,
        content_ar: record.content_ar,
        audio_urls: if audio_urls.is_empty() {
            None
        } else {
            Some(audio_urls)
        },
    }
}

async fn resolve_child_wallet_scope(
    state: &AppState,
    parent_wallet_address: &str,
    child_id: Option<&str>,
) -> Result<Option<String>, AppError> {
    let Some(child_id) = child_id else {
        return Ok(None);
    };

    let db = state.db.lock().await;
    let child =
        queries::get_child_profile_by_id_and_parent_wallet(&db, child_id, parent_wallet_address)?
            .ok_or_else(|| AppError::NotFound("child profile not found".into()))?;
    Ok(Some(child.derived_wallet_address))
}

/// GET /swift/v2/cuentacuentos/ready
pub async fn cuentacuentos_ready(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<CuentacuentosQuery>,
) -> Result<Json<Option<CuentacuentosItem>>, AppError> {
    let parent_wallet_address = bearer_wallet_auth(&state, &headers).await?;
    let child_wallet_address =
        resolve_child_wallet_scope(&state, &parent_wallet_address, params.child_id.as_deref())
            .await?;
    let db = state.db.lock().await;
    let story = queries::get_ready_cuentacuentos(
        &db,
        &parent_wallet_address,
        child_wallet_address.as_deref(),
    )?;
    let item = if let Some(story) = story {
        let images = queries::get_cuentacuentos_images(&db, &story.id)?;
        let audio = queries::get_cuentacuentos_audio(&db, &story.id)?;
        Some(cuentacuentos_to_ready_item(story, images, audio))
    } else {
        None
    };
    Ok(Json(item))
}

/// GET /swift/v2/cuentacuentos/history
pub async fn cuentacuentos_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<CuentacuentosQuery>,
) -> Result<Json<Vec<CuentacuentosItem>>, AppError> {
    let parent_wallet_address = bearer_wallet_auth(&state, &headers).await?;
    let child_wallet_address =
        resolve_child_wallet_scope(&state, &parent_wallet_address, params.child_id.as_deref())
            .await?;
    let db = state.db.lock().await;
    let stories = queries::get_cuentacuentos_history(
        &db,
        &parent_wallet_address,
        child_wallet_address.as_deref(),
    )?;
    let mut items = Vec::new();
    for story in stories {
        let images = queries::get_cuentacuentos_images(&db, &story.id)?;
        let audio = queries::get_cuentacuentos_audio(&db, &story.id)?;
        items.push(cuentacuentos_to_ready_item(story, images, audio));
    }
    Ok(Json(items))
}

/// POST /swift/v2/cuentacuentos/:id/complete
pub async fn complete_cuentacuentos(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let parent_wallet_address = bearer_wallet_auth(&state, &headers).await?;
    {
        let db = state.db.lock().await;
        queries::get_cuentacuentos_by_id_and_parent_wallet(&db, &id, &parent_wallet_address)?
            .ok_or_else(|| AppError::NotFound("cuentacuentos not found".into()))?;
        queries::mark_cuentacuentos_played(&db, &id)?;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /swift/v2/cuentacuentos/:id/assign
pub async fn assign_cuentacuentos(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<AssignCuentacuentosRequest>,
) -> Result<Json<CuentacuentosItem>, AppError> {
    let parent_wallet_address = bearer_wallet_auth(&state, &headers).await?;
    let child_wallet_address = normalize_seed_wallet_address(&req.child_wallet_address)?;

    let updated_story = {
        let db = state.db.lock().await;

        queries::get_child_profile_by_derived_wallet_and_parent_wallet(
            &db,
            &child_wallet_address,
            &parent_wallet_address,
        )?
        .ok_or_else(|| AppError::NotFound("child profile not found".into()))?;

        queries::get_cuentacuentos_by_id_and_parent_wallet(&db, &id, &parent_wallet_address)?
            .ok_or_else(|| AppError::NotFound("cuentacuentos not found".into()))?;

        let assigned = queries::assign_cuentacuentos_to_child(
            &db,
            &id,
            &parent_wallet_address,
            &child_wallet_address,
        )?;
        if !assigned {
            return Err(AppError::NotFound("cuentacuentos not found".into()));
        }

        queries::get_cuentacuentos_by_id_and_parent_wallet(&db, &id, &parent_wallet_address)?
            .ok_or_else(|| AppError::Internal("failed to load assigned cuentacuentos".into()))?
    };

    Ok(Json(cuentacuentos_to_item(updated_story)))
}

// ===== Public Stories Feed =====

/// GET /api/v1/stories — public feed of all stories, newest first.
/// No auth required. Returns stories with images decorated.
pub async fn list_all_stories(
    State(state): State<AppState>,
) -> Result<Json<Vec<CuentacuentosItem>>, AppError> {
    let db = state.db.lock().await;
    let stories = queries::get_all_cuentacuentos(&db, 50)?;
    let mut items = Vec::new();
    for story in stories {
        let images = queries::get_cuentacuentos_images(&db, &story.id)?;
        let audio = queries::get_cuentacuentos_audio(&db, &story.id)?;
        items.push(cuentacuentos_to_ready_item(story, images, audio));
    }
    Ok(Json(items))
}

/// GET /api/v1/stories/{id} — public single story with images.
pub async fn get_story(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CuentacuentosItem>, AppError> {
    let db = state.db.lock().await;
    let story = queries::get_cuentacuentos_by_id(&db, &id)?
        .ok_or_else(|| AppError::NotFound("story not found".into()))?;
    let images = queries::get_cuentacuentos_images(&db, &story.id)?;
    let audio = queries::get_cuentacuentos_audio(&db, &story.id)?;
    Ok(Json(cuentacuentos_to_ready_item(story, images, audio)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::{Secp256k1, SecretKey};

    #[test]
    fn verifies_seed_auth_signatures() {
        let secp = Secp256k1::new();
        let mut rng = secp256k1::rand::rngs::OsRng;
        let secret_key = SecretKey::new(&mut rng);
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let wallet_address = evm_address_from_public_key(&public_key);
        let message = build_seed_auth_message(&wallet_address, "challenge-1", "nonce-1");
        let digest = ethereum_personal_sign_hash(&message);
        let message = Message::from_digest_slice(&digest).unwrap();
        let signature = secp.sign_ecdsa_recoverable(&message, &secret_key);
        let (recovery_id, compact) = signature.serialize_compact();
        let mut encoded = [0u8; 65];
        encoded[..64].copy_from_slice(&compact);
        encoded[64] = recovery_id.to_i32() as u8 + 27;
        let signature_hex = format!("0x{}", hex::encode(encoded));

        let verified = verify_seed_auth_signature(
            &wallet_address,
            &build_seed_auth_message(&wallet_address, "challenge-1", "nonce-1"),
            &signature_hex,
        );
        assert!(verified.is_ok());
    }
}

/// POST /swift/v1/admin/premium — toggle premium for a user (simple internal endpoint)
/// Body: { "user_id": "...", "is_premium": true }
pub async fn set_premium(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Must be called with a valid session (any user for now — restrict later)
    let _caller = bearer_auth(&state, &headers).await?;

    let target_user_id = body
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("user_id required".into()))?;
    let is_premium = body
        .get("is_premium")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    {
        let db = state.db.lock().await;
        queries::set_user_premium(&db, target_user_id, is_premium)?;
    }

    state.emit_log(
        "INFO",
        "premium",
        &format!(
            "User {} premium = {}",
            &target_user_id[..8.min(target_user_id.len())],
            is_premium
        ),
    );

    Ok(Json(
        serde_json::json!({ "ok": true, "is_premium": is_premium }),
    ))
}

// ===== Next Prompt =====

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextPromptResponse {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_session: Option<String>,
}

/// GET /swift/v2/next-prompt
/// Returns the precomputed writing prompt for this user.
/// If no personalized prompt exists yet, returns a default one.
pub async fn get_next_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<NextPromptResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let prompt_data = {
        let db = state.db.lock().await;
        queries::get_next_prompt(&db, &user_id)?
    };

    match prompt_data {
        Some((prompt, from_session)) => Ok(Json(NextPromptResponse {
            prompt,
            from_session,
        })),
        None => {
            // No personalized prompt yet — return default
            Ok(Json(NextPromptResponse {
                prompt: "what do you want your children to understand about you that you've never been able to say out loud?".to_string(),
                from_session: None,
            }))
        }
    }
}

// ===== You (Profile) =====

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YouResponse {
    pub total_sessions: i32,
    pub total_ankys: i32,
    pub total_words: i32,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub best_flow_score: f64,
    pub avg_flow_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psychological_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emotional_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_tensions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub growth_edges: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub honcho_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

/// GET /swift/v2/you
/// Returns what anky knows about you — your profile built from all your writing.
/// Combines local profile data with Honcho's accumulated peer context.
pub async fn get_you(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<YouResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let profile = {
        let db = state.db.lock().await;
        queries::get_user_profile_full(&db, &user_id)?
    };

    // Get Honcho context in parallel (non-blocking, best-effort)
    let honcho_context = if crate::services::honcho::is_configured(&state.config) {
        crate::services::honcho::get_peer_context(
            &state.config.honcho_api_key,
            &state.config.honcho_workspace_id,
            &user_id,
        )
        .await
        .unwrap_or(None)
    } else {
        None
    };

    match profile {
        Some(p) => Ok(Json(YouResponse {
            total_sessions: p.total_sessions,
            total_ankys: p.total_anky_sessions,
            total_words: p.total_words_written,
            current_streak: p.current_streak,
            longest_streak: p.longest_streak,
            best_flow_score: p.best_flow_score,
            avg_flow_score: p.avg_flow_score,
            psychological_profile: p.psychological_profile,
            emotional_signature: p.emotional_signature,
            core_tensions: p.core_tensions,
            growth_edges: p.growth_edges,
            honcho_context,
            last_updated: p.last_profile_update,
        })),
        None => Ok(Json(YouResponse {
            total_sessions: 0,
            total_ankys: 0,
            total_words: 0,
            current_streak: 0,
            longest_streak: 0,
            best_flow_score: 0.0,
            avg_flow_score: 0.0,
            psychological_profile: None,
            emotional_signature: None,
            core_tensions: None,
            growth_edges: None,
            honcho_context,
            last_updated: None,
        })),
    }
}

// ===== Device Token =====

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceRequest {
    pub device_token: String,
    #[serde(default = "default_platform")]
    pub platform: String,
}

#[derive(Deserialize)]
pub struct DeleteDeviceRequest {
    pub platform: String,
}

fn default_platform() -> String {
    "ios".into()
}

/// POST /swift/v2/devices
/// Register an APNs device token for push notifications. Upserts on (user_id, platform).
pub async fn register_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    if req.device_token.trim().is_empty() {
        return Err(AppError::BadRequest("deviceToken is required".into()));
    }

    {
        let db = state.db.lock().await;
        queries::upsert_device_token(&db, &user_id, req.device_token.trim(), &req.platform)?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /swift/v2/devices
/// Remove device token for this user+platform (called on logout).
pub async fn delete_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<DeleteDeviceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    {
        let db = state.db.lock().await;
        queries::delete_device_token(&db, &user_id, &req.platform)?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ===== Settings (mobile) =====

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSettingsResponse {
    pub font_family: String,
    pub font_size: i32,
    pub theme: String,
    pub idle_timeout: i32,
    pub keyboard_layout: String,
    pub preferred_language: String,
}

/// GET /swift/v2/settings
pub async fn get_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MobileSettingsResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = state.db.lock().await;
    let s = queries::get_user_settings(&db, &user_id)?;
    Ok(Json(MobileSettingsResponse {
        font_family: s.font_family,
        font_size: s.font_size,
        theme: s.theme,
        idle_timeout: s.idle_timeout,
        keyboard_layout: s.keyboard_layout,
        preferred_language: s.preferred_language,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchSettingsRequest {
    pub font_family: Option<String>,
    pub font_size: Option<i32>,
    pub theme: Option<String>,
    pub idle_timeout: Option<i32>,
    pub keyboard_layout: Option<String>,
    pub preferred_language: Option<String>,
}

/// PATCH /swift/v2/settings
pub async fn patch_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PatchSettingsRequest>,
) -> Result<Json<MobileSettingsResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = state.db.lock().await;
    let existing = queries::get_user_settings(&db, &user_id)?;

    let font_family = req.font_family.unwrap_or(existing.font_family);
    let font_size = req.font_size.unwrap_or(existing.font_size).clamp(14, 28);
    let theme = req.theme.unwrap_or(existing.theme);
    let idle_timeout = req
        .idle_timeout
        .unwrap_or(existing.idle_timeout)
        .clamp(5, 15);
    let keyboard_layout = req.keyboard_layout.unwrap_or(existing.keyboard_layout);
    let preferred_language = req
        .preferred_language
        .unwrap_or(existing.preferred_language);

    queries::upsert_user_settings(
        &db,
        &user_id,
        &font_family,
        font_size,
        &theme,
        idle_timeout,
        &keyboard_layout,
        &preferred_language,
    )?;

    Ok(Json(MobileSettingsResponse {
        font_family,
        font_size,
        theme,
        idle_timeout,
        keyboard_layout,
        preferred_language,
    }))
}
