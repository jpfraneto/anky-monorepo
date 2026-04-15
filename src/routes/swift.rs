/// /swift/v1/* — Mobile API for the Anky iOS app
///
/// Auth: Bearer token in `Authorization: Bearer <session_token>` header.
/// Session tokens are the same as web sessions (auth_sessions table).
/// Mobile auth via Privy SDK: POST /swift/v1/auth/privy → returns session_token as JSON.
/// Seed-phrase identity auth: POST /swift/v2/auth/challenge + POST /swift/v2/auth/verify.
/// Supports both Solana Ed25519 (base58 pubkey) and legacy EVM secp256k1 (0x address).
use crate::db::queries;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use rand::RngCore;
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1, SecretKey,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sha3::{Digest, Keccak256};
use sqlx::Row as SqlxRow;
use std::collections::HashMap;

// ===== Auth helpers =====

/// Extract user_id from `Authorization: Bearer <token>` header.
pub async fn bearer_auth(state: &AppState, headers: &HeaderMap) -> Result<String, AppError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("missing Authorization: Bearer header".into()))?;

    let db = crate::db::conn(&state.db)?;
    let (user_id, _) = queries::get_auth_session(&db, token)?
        .ok_or_else(|| AppError::Unauthorized("invalid or expired session token".into()))?;

    Ok(user_id)
}

async fn bearer_wallet_auth(state: &AppState, headers: &HeaderMap) -> Result<String, AppError> {
    let user_id = bearer_auth(state, headers).await?;
    let db = crate::db::conn(&state.db)?;
    let wallet_address = queries::get_user_wallet(&db, &user_id)?
        .ok_or_else(|| AppError::Unauthorized("seed identity required".into()))?;
    normalize_seed_wallet_address(&wallet_address)
}

/// Returns true if the wallet address looks like a Solana base58 pubkey (not 0x-prefixed).
fn is_solana_address(wallet_address: &str) -> bool {
    let trimmed = wallet_address.trim();
    !trimmed.starts_with("0x") && !trimmed.starts_with("0X") && trimmed.len() >= 32
}

/// Normalize a wallet address. Accepts both Solana (base58) and EVM (0x-prefixed).
/// Solana addresses are returned as-is (base58). EVM addresses are lowercased.
fn normalize_seed_wallet_address(wallet_address: &str) -> Result<String, AppError> {
    let trimmed = wallet_address.trim();

    if is_solana_address(trimmed) {
        return crate::services::wallet::normalize_solana_address(trimmed);
    }

    // Legacy EVM path
    let without_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);

    if without_prefix.len() != 40 || !without_prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::BadRequest(
            "wallet_address must be a solana base58 pubkey or 0x-prefixed EVM address".into(),
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
    if is_solana_address(wallet_address) {
        format!(
            "anky.app sign in\n\naddress: {}\nchallenge: {}\nnonce: {}\n\nsign this only inside the anky app.",
            wallet_address, challenge_id, nonce
        )
    } else {
        format!(
            "anky.app base identity sign in\n\naddress: {}\nchallenge id: {}\nnonce: {}\nchain id: 8453\n\nsign this only inside the anky app.",
            wallet_address, challenge_id, nonce
        )
    }
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

/// Verify an Ed25519 signature (Solana path).
/// Signature: base58-encoded 64 bytes. Pubkey: base58-encoded 32 bytes.
/// The message is signed directly (raw bytes, no wrapping).
fn verify_solana_signature(
    wallet_address: &str,
    message: &str,
    signature_b58: &str,
) -> Result<(), AppError> {
    crate::services::wallet::verify_solana_signature(wallet_address, message, signature_b58)
}

/// Verify a signature from a seed-derived keypair.
/// Dispatches to Ed25519 (Solana) or ECDSA (EVM) based on wallet address format.
fn verify_seed_auth_signature(
    wallet_address: &str,
    message: &str,
    signature: &str,
) -> Result<(), AppError> {
    let normalized_wallet = normalize_seed_wallet_address(wallet_address)?;

    if is_solana_address(&normalized_wallet) {
        return verify_solana_signature(&normalized_wallet, message, signature);
    }

    // Legacy EVM path
    let sig_bytes = parse_evm_signature(signature)?;
    let recovery_id = parse_recovery_id(sig_bytes[64])?;
    let recoverable_signature = RecoverableSignature::from_compact(&sig_bytes[..64], recovery_id)
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
        let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
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
    let db = crate::db::conn(&state.db)?;
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
    pub is_premium: bool,
    pub preferred_language: String,
}

/// GET /swift/v1/me
pub async fn get_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = crate::db::conn(&state.db)?;

    let username = queries::get_user_username(&db, &user_id).ok().flatten();
    let email = queries::get_user_email(&db, &user_id).ok().flatten();
    let wallet = queries::get_user_wallet(&db, &user_id).ok().flatten();

    // Try to get X profile info via the x_users table
    let (display_name, profile_image_url) = {
        let mut stmt = db.prepare(
            "SELECT display_name, profile_image_url FROM x_users WHERE user_id = ?1 LIMIT 1",
        );
        if let Ok(ref mut s) = stmt {
            let mut rows = s.query_map(crate::params![user_id], |row| {
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
    let is_premium = queries::is_user_premium(&db, &user_id).unwrap_or(false);

    // If no profile image from X, use latest anky image
    let effective_profile_image = profile_image_url.or_else(|| {
        let mut stmt = db.prepare(
            "SELECT COALESCE(image_webp, image_path) FROM ankys WHERE user_id = ?1 AND status = 'complete' AND image_path IS NOT NULL ORDER BY created_at DESC LIMIT 1"
        ).ok()?;
        let path: Option<String> = stmt.query_row(crate::params![user_id], |row| row.get(0)).ok()?;
        path.map(|p| if p.starts_with("http") { p } else { format!("https://anky.app/data/images/{}", p) })
    });

    Ok(Json(MeResponse {
        user_id,
        username,
        display_name,
        profile_image_url: effective_profile_image,
        email,
        wallet_address: wallet,
        total_writings,
        total_ankys,
        is_premium,
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
    /// The processed anky reflection (from ankys table), distinct from session-level `response`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_reflection: Option<String>,
    pub created_at: String,
}

/// GET /swift/v1/writings
pub async fn list_writings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WritingItem>>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = crate::db::conn(&state.db)?;
    let writings = queries::get_user_writings_with_ankys(&db, &user_id)?;
    let items = writings
        .into_iter()
        .map(|w| {
            // Prefer anky reflection over session-level response for backwards compat
            let response = w.response.or_else(|| w.anky_reflection.clone());
            WritingItem {
                id: w.id,
                content: w.content,
                duration_seconds: w.duration_seconds,
                word_count: w.word_count,
                is_anky: w.is_anky,
                response,
                anky_id: w.anky_id,
                anky_title: w.anky_title,
                anky_image_path: w.anky_image_path.map(|p| {
                    if p.starts_with("http") {
                        p
                    } else {
                        format!("https://anky.app/data/images/{}", p)
                    }
                }),
                anky_reflection: w.anky_reflection,
                created_at: w.created_at,
            }
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
    /// Anky's personalized response (null if still generating — poll status)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_response: Option<String>,
    /// Suggested prompt for next session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_prompt: Option<String>,
    /// Mood: reflective, celebratory, gentle, curious, deep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
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
        let db = crate::db::conn(&state.db)?;
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
            let db = crate::db::conn(&state.db)?;
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
            anky_response: None,
            next_prompt: None,
            mood: None,
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
            anky_response: None,
            next_prompt: None,
            mood: None,
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
                anky_response: None,
                next_prompt: None,
                mood: None,
            }));
        }

        // Privy users: persist + spawn feedback + guidance in background
        {
            let db = crate::db::conn(&state.db)?;
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
            let feedback =
                crate::services::claude::call_haiku(&fb_state.config.anthropic_api_key, &prompt)
                    .await
                    .unwrap_or_else(|_| "keep flowing — every word counts.".into());
            let Some(db) = crate::db::get_conn_logged(&fb_state.db) else {
                return;
            };
            let _ = db.execute(
                "UPDATE writing_sessions SET response = ?1 WHERE id = ?2",
                crate::params![&feedback, &fb_session],
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
            anky_response: None,
            next_prompt: None,
            mood: None,
        }));
    }

    // === ANKY: the real thing ===
    // Persist the writing session + create the anky record — then return immediately

    {
        let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
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

    // Log writing session on-chain via spl-memo
    {
        let log_state = state.clone();
        let log_aid = anky_id.clone();
        let log_sid = session_id.clone();
        let log_uid = user_id.clone();
        let log_text = req.text.clone();
        let log_duration = req.duration as i64;
        let log_words = word_count;
        tokio::spawn(async move {
            use sha2::{Digest, Sha256};
            let session_hash = format!("{:x}", Sha256::digest(log_text.as_bytes()));
            if let Err(e) = crate::pipeline::image_gen::log_session_onchain(
                &log_state,
                &log_aid,
                &log_sid,
                &log_uid,
                &session_hash,
                log_duration,
                log_words,
            )
            .await
            {
                tracing::warn!(
                    "session on-chain log failed for {}: {}",
                    &log_aid[..8.min(log_aid.len())],
                    e
                );
            }
        });
    }

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
        let db = crate::db::conn(&state.db)?;
        queries::is_user_pro(&db, &user_id).unwrap_or(false)
    };
    crate::services::redis_queue::enqueue_job(
        &state.config.redis_url,
        &crate::state::GpuJob::AnkyImage {
            anky_id: anky_id.clone(),
            session_id: session_id.clone(),
            user_id: user_id.clone(),
            writing: req.text.clone(),
        },
        is_pro,
    )
    .await?;

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

    // anky_response is generated by the post-writing pipeline (event-driven).
    // For ankys: triggered inside queue_post_writing_cuentacuentos lifecycle.
    // Client polls /writing/{sessionId}/status to get it when ready.

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
        anky_response: None, // generated async — poll status endpoint
        next_prompt: None,
        mood: None,
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
    /// Anky's personalized response to the writing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_response: Option<String>,
    /// Next writing prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_prompt: Option<String>,
    /// Mood: reflective, celebratory, gentle, curious, deep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
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

async fn get_local_first_writing_status(
    state: &AppState,
    user_id: &str,
    session_hash: &str,
) -> Result<Option<WritingStatusResponse>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
            ws.duration_seconds,
            ws.word_count,
            ws.anky_response,
            ws.anky_next_prompt,
            ws.anky_mood,
            a.id,
            a.title,
            a.reflection,
            a.image_path,
            COALESCE(a.image_status, 'pending'),
            COALESCE(a.solana_status, 'pending'),
            a.done_at
        FROM ankys a
        JOIN writing_sessions ws ON ws.id = a.writing_session_id
        WHERE a.user_id = $1
          AND a.session_hash = $2
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(session_hash)
    .fetch_optional(&state.db)
    .await?;

    Ok(row.map(|row| {
        let image_status: String = row.get(9);
        let solana_status: String = row.get(10);
        let done_at: Option<String> = row.get(11);
        let status = if done_at.is_some()
            || (image_status == "complete" && matches!(solana_status.as_str(), "complete" | "skipped"))
        {
            "complete".to_string()
        } else {
            "generating".to_string()
        };

        WritingStatusResponse {
            session_id: session_hash.to_string(),
            is_anky: true,
            duration_seconds: row.get(0),
            word_count: row.get(1),
            anky: Some(AnkyStatusInfo {
                id: row.get(5),
                status,
                image_url: row.get(8),
                title: row.get(6),
                reflection: row.get(7),
            }),
            cuentacuentos: None,
            anky_response: row.get(2),
            next_prompt: row.get(3),
            mood: row.get(4),
        }
    }))
}

/// GET /swift/v2/writing/{sessionId}/status
pub async fn get_writing_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<WritingStatusResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    if let Some(status) = get_local_first_writing_status(&state, &user_id, &session_id).await? {
        return Ok(Json(status));
    }

    let db = crate::db::conn(&state.db)?;

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

    // Get anky_response from writing_sessions.anky_response column
    let (anky_response, next_prompt_text, mood) = db
        .query_row(
            "SELECT anky_response, anky_next_prompt, anky_mood FROM writing_sessions WHERE id = ?1",
            crate::params![&session_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0).unwrap_or(None),
                    row.get::<_, Option<String>>(1).unwrap_or(None),
                    row.get::<_, Option<String>>(2).unwrap_or(None),
                ))
            },
        )
        .unwrap_or((None, None, None));

    Ok(Json(WritingStatusResponse {
        session_id,
        is_anky: writing.is_anky,
        duration_seconds: writing.duration_seconds,
        word_count: writing.word_count,
        anky: anky_info,
        cuentacuentos: cuentacuentos_info,
        anky_response,
        next_prompt: next_prompt_text,
        mood,
    }))
}

// ===== Retry Reflection =====

/// POST /swift/v2/writing/{sessionId}/retry-reflection
/// Re-triggers the anky response + reflection generation for a session that failed.
/// Returns immediately; the phone polls /status to pick up the result.
pub async fn retry_reflection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    // Verify the session exists and belongs to this user
    let (writing_text, is_anky, anky_id) = {
        let db = crate::db::conn(&state.db)?;
        // Check ownership
        let owner: String = db
            .query_row(
                "SELECT user_id FROM writing_sessions WHERE id = ?1",
                crate::params![&session_id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::NotFound("writing session not found".into()))?;
        if owner != user_id {
            return Err(AppError::Forbidden("not your session".into()));
        }
        let ws = queries::get_writing_session(&db, &session_id)
            .ok()
            .flatten()
            .ok_or_else(|| AppError::NotFound("writing session not found".into()))?;
        let anky_id: Option<String> = db
            .query_row(
                "SELECT id FROM ankys WHERE writing_session_id = ?1 LIMIT 1",
                crate::params![&session_id],
                |row| row.get(0),
            )
            .ok();
        (ws.content, ws.is_anky, anky_id)
    };

    // Check if there's already a good anky_response
    {
        let db = crate::db::conn(&state.db)?;
        let existing: Option<String> = db
            .query_row(
                "SELECT anky_response FROM writing_sessions WHERE id = ?1",
                crate::params![&session_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        if let Some(ref resp) = existing {
            if !resp.is_empty() {
                return Ok(Json(serde_json::json!({
                    "ok": true,
                    "status": "already_exists",
                    "message": "reflection already exists"
                })));
            }
        }
    }

    // Spawn anky response generation in background
    let bg_state = state.clone();
    let bg_session = session_id.clone();
    let bg_user = user_id.clone();
    let bg_writing = writing_text.clone();
    tokio::spawn(async move {
        crate::pipeline::guidance_gen::generate_anky_response(
            &bg_state,
            &bg_user,
            &bg_session,
            &bg_writing,
        )
        .await;
    });

    // If there's an anky record, also retry the title+reflection (the deeper one)
    if let Some(aid) = anky_id {
        let bg_state2 = state.clone();
        let bg_writing2 = writing_text;
        let bg_aid = aid;
        tokio::spawn(async move {
            let (tx, _rx) = tokio::sync::mpsc::channel::<String>(64);
            match crate::services::claude::stream_title_and_reflection_best(
                &bg_state2.config,
                &bg_writing2,
                tx,
                None,
            )
            .await
            {
                Ok((full_text, _input_tokens, _output_tokens, model, provider)) => {
                    let (title, reflection) =
                        crate::services::claude::parse_title_reflection(&full_text);
                    if let Some(db) = crate::db::get_conn_logged(&bg_state2.db) {
                        let _ = queries::update_anky_title_reflection(
                            &db,
                            &bg_aid,
                            &title,
                            &reflection,
                        );
                    }
                    tracing::info!(
                        "Retry reflection saved for {} via {}/{}",
                        &bg_aid[..8.min(bg_aid.len())],
                        provider,
                        model
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Retry reflection failed for {}: {}",
                        &bg_aid[..8.min(bg_aid.len())],
                        e
                    );
                }
            }
        });
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "status": "retrying",
        "message": "reflection generation restarted — poll status for result"
    })))
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
        let db = crate::db::conn(&state.db)?;
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
    let db = crate::db::conn(&state.db)?;
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
    let db = crate::db::conn(&state.db)?;
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

    let db = crate::db::conn(&state.db)?;
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
    let db = crate::db::conn(&state.db)?;
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
    let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;

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
    let db = crate::db::conn(&state.db)?;
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
    let db = crate::db::conn(&state.db)?;
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
    fn verifies_evm_seed_auth_signatures() {
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

    #[test]
    fn verifies_solana_ed25519_signatures() {
        use ed25519_dalek::{Signer, SigningKey};

        let mut rng = rand::thread_rng();
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let wallet_address = bs58::encode(verifying_key.as_bytes()).into_string();

        let message = build_seed_auth_message(&wallet_address, "challenge-1", "nonce-1");
        let signature = signing_key.sign(message.as_bytes());
        let signature_b58 = bs58::encode(signature.to_bytes()).into_string();

        let verified = verify_seed_auth_signature(&wallet_address, &message, &signature_b58);
        assert!(verified.is_ok(), "ed25519 signature should verify");
    }

    #[test]
    fn rejects_wrong_solana_signature() {
        use ed25519_dalek::{Signer, SigningKey};

        let mut rng = rand::thread_rng();
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let wallet_address = bs58::encode(verifying_key.as_bytes()).into_string();

        // Sign with a DIFFERENT key
        let mut other_seed = [0u8; 32];
        rng.fill_bytes(&mut other_seed);
        let other_key = SigningKey::from_bytes(&other_seed);

        let message = build_seed_auth_message(&wallet_address, "challenge-1", "nonce-1");
        let bad_sig = other_key.sign(message.as_bytes());
        let bad_sig_b58 = bs58::encode(bad_sig.to_bytes()).into_string();

        let result = verify_seed_auth_signature(&wallet_address, &message, &bad_sig_b58);
        assert!(result.is_err(), "wrong key should fail verification");
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
        let db = crate::db::conn(&state.db)?;
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

/// GET /swift/v2/prompt/{id} — fetch a prompt by ID (for deep links, no auth required)
pub async fn get_prompt_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = crate::db::conn(&state.db)?;
    let prompt = queries::get_prompt_by_id(&db, &id)?
        .ok_or_else(|| AppError::NotFound("prompt not found".into()))?;
    Ok(Json(serde_json::json!({
        "id": prompt.id,
        "text": prompt.prompt_text,
    })))
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
        let db = crate::db::conn(&state.db)?;
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

// ===== Chat Prompt =====

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatPromptResponse {
    pub ok: bool,
    pub text: String,
    pub message_id: String,
}

/// GET /swift/v2/chat/prompt
/// Returns Anky's opening message for a new writing session.
/// This is pre-computed by the post-writing pipeline — no on-demand LLM call.
/// First-ever user: generic. Returning user: reads from next_prompts table.
pub async fn get_chat_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ChatPromptResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let message_id = uuid::Uuid::new_v4().to_string();

    let db = crate::db::conn(&state.db)?;

    // Check if this is a first-time user (no writing sessions)
    let session_count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM writing_sessions WHERE user_id = ?1",
            crate::params![&user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if session_count == 0 {
        return Ok(Json(ChatPromptResponse {
            ok: true,
            text: "tell me who you are.".to_string(),
            message_id,
        }));
    }

    // Return the pre-computed next prompt from the pipeline
    let text = db
        .query_row(
            "SELECT prompt FROM next_prompts WHERE user_id = ?1 ORDER BY created_at DESC LIMIT 1",
            crate::params![&user_id],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "what are you carrying today?".to_string());

    Ok(Json(ChatPromptResponse {
        ok: true,
        text,
        message_id,
    }))
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
        let db = crate::db::conn(&state.db)?;
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

// ===== You: Ankys List =====

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YouAnkyItem {
    pub id: String,
    pub title: Option<String>,
    pub image_url: Option<String>,
    pub image_png_url: Option<String>,
    pub reflection: Option<String>,
    pub created_at: String,
}

/// GET /swift/v2/you/ankys
/// Returns the user's completed ankys for the profile grid.
pub async fn get_you_ankys(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<YouAnkyItem>>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let ankys = {
        let db = crate::db::conn(&state.db)?;
        queries::get_user_ankys(&db, &user_id)?
    };

    let items: Vec<YouAnkyItem> = ankys
        .into_iter()
        .map(|a| {
            let make_url = |p: &str| -> String {
                if p.starts_with("http") {
                    p.to_string()
                } else {
                    format!("/static/{}", p.trim_start_matches("static/"))
                }
            };
            let image_url = a
                .image_webp
                .as_deref()
                .map(make_url)
                .or_else(|| a.image_path.as_deref().map(make_url));
            let image_png_url = a.image_path.as_deref().map(make_url);
            YouAnkyItem {
                id: a.id,
                title: a.title,
                image_url,
                image_png_url,
                reflection: a.reflection,
                created_at: a.created_at,
            }
        })
        .collect();

    Ok(Json(items))
}

// ===== You: Items =====

/// GET /swift/v2/you/items
/// Returns the user's current 8 kingdom items — the living interpretation from Honcho context.
/// If the user has a minted mirror, returns those frozen items.
/// Otherwise, derives fresh items from their latest Honcho context.
pub async fn get_you_items(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    // Check if user has items from a minted mirror
    let existing = {
        let db = crate::db::conn(&state.db)?;
        queries::get_user_mirror_items(&db, &user_id)
            .map_err(|e| AppError::Internal(format!("DB: {}", e)))?
    };

    if let Some((mirror_id, items_json)) = existing {
        if let Some(items) = crate::routes::api::AnkyItems::from_json(&items_json) {
            return Ok(Json(serde_json::json!({
                "mirror_id": mirror_id,
                "items": items.items,
                "source": "mirror",
            })));
        }
    }

    // No mirror items — try to derive from Honcho context + latest writing
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

    if honcho_context.is_none() {
        return Ok(Json(serde_json::json!({
            "items": null,
            "source": "none",
            "message": "write your first anky to discover your items",
        })));
    }

    // Get latest writing for context
    let latest_writing = {
        let db = crate::db::conn(&state.db)?;
        let mut stmt = db.prepare(
            "SELECT content FROM writing_sessions WHERE user_id = ?1 AND is_anky = 1 ORDER BY created_at DESC LIMIT 1"
        ).map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
        let mut rows = stmt
            .query_map(crate::params![user_id], |row| row.get::<_, String>(0))
            .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
        rows.next().and_then(|r| r.ok())
    };

    let writing_text = latest_writing.unwrap_or_default();
    if writing_text.is_empty() {
        return Ok(Json(serde_json::json!({
            "items": null,
            "source": "none",
            "message": "write your first anky to discover your items",
        })));
    }

    let items_system = crate::routes::api::items_system_prompt();
    let items_user = crate::routes::api::items_user_prompt_from_writing(
        &writing_text,
        honcho_context.as_deref(),
    );
    match crate::routes::api::derive_items(
        &state.config.anthropic_api_key,
        &items_system,
        &items_user,
    )
    .await
    {
        Ok(items) => Ok(Json(serde_json::json!({
            "items": items.items,
            "source": "derived",
        }))),
        Err(e) => {
            tracing::warn!("Failed to derive items for user {}: {}", &user_id, e);
            Ok(Json(serde_json::json!({
                "items": null,
                "source": "error",
                "message": "could not derive items right now",
            })))
        }
    }
}

/// POST /swift/v2/mirror/mint — iOS app mirror mint (delegates to the Solana mint handler).
pub async fn swift_mirror_mint(
    state: State<AppState>,
    headers: HeaderMap,
    body: Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    mint_raw_mirror(state, headers, body).await
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
        let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
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
    pub preferred_model: String,
}

/// GET /swift/v2/settings
pub async fn get_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MobileSettingsResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = crate::db::conn(&state.db)?;
    let s = queries::get_user_settings(&db, &user_id)?;
    Ok(Json(MobileSettingsResponse {
        font_family: s.font_family,
        font_size: s.font_size,
        theme: s.theme,
        idle_timeout: s.idle_timeout,
        keyboard_layout: s.keyboard_layout,
        preferred_language: s.preferred_language,
        preferred_model: s.preferred_model,
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
    pub preferred_model: Option<String>,
}

/// PATCH /swift/v2/settings
pub async fn patch_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PatchSettingsRequest>,
) -> Result<Json<MobileSettingsResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = crate::db::conn(&state.db)?;
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
    let preferred_model = req.preferred_model.unwrap_or(existing.preferred_model);

    queries::upsert_user_settings(
        &db,
        &user_id,
        &font_family,
        font_size,
        &theme,
        idle_timeout,
        &keyboard_layout,
        &preferred_language,
        &preferred_model,
    )?;

    Ok(Json(MobileSettingsResponse {
        font_family,
        font_size,
        theme,
        idle_timeout,
        keyboard_layout,
        preferred_language,
        preferred_model,
    }))
}

// ===== Minting (ERC1155 birthSoul on Base) =====

const ANKY_CONTRACT: &str = "0x19a36545CC4707870ad53CaADd23B7A70642F304";
const BASE_CHAIN_ID: u64 = 8453;

/// ABI-encode a uint256 (left-padded to 32 bytes).
fn abi_encode_u256(val: &[u8]) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let start = 32usize.saturating_sub(val.len());
    buf[start..].copy_from_slice(val);
    buf
}

/// ABI-encode an address (left-padded to 32 bytes).
fn abi_encode_address(addr: &str) -> [u8; 32] {
    let without_prefix = addr.strip_prefix("0x").unwrap_or(addr);
    let bytes = hex::decode(without_prefix).unwrap_or_default();
    abi_encode_u256(&bytes)
}

/// u64 to big-endian bytes (trimmed).
fn u64_to_be_bytes_trimmed(v: u64) -> Vec<u8> {
    let bytes = v.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes[start..].to_vec()
}

/// Compute EIP-712 domain separator for Anky contract.
fn anky_domain_separator() -> [u8; 32] {
    let type_hash = Keccak256::digest(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = Keccak256::digest(b"Anky");
    let version_hash = Keccak256::digest(b"1");

    let mut encoded = Vec::with_capacity(5 * 32);
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&name_hash);
    encoded.extend_from_slice(&version_hash);
    encoded.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(BASE_CHAIN_ID)));

    let contract_addr = ANKY_CONTRACT.strip_prefix("0x").unwrap();
    let contract_bytes = hex::decode(contract_addr).unwrap();
    encoded.extend_from_slice(&abi_encode_u256(&contract_bytes));

    let digest = Keccak256::digest(&encoded);
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

/// Compute EIP-712 struct hash for BirthPayload.
fn birth_payload_struct_hash(
    writer: &str,
    session_cid: &str,
    metadata_uri: &str,
    nonce: u64,
    deadline: u64,
) -> [u8; 32] {
    let type_hash = Keccak256::digest(
        b"BirthPayload(address writer,string sessionCID,string metadataURI,uint256 nonce,uint256 deadline)",
    );
    let session_cid_hash = Keccak256::digest(session_cid.as_bytes());
    let metadata_uri_hash = Keccak256::digest(metadata_uri.as_bytes());

    let mut encoded = Vec::with_capacity(6 * 32);
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&abi_encode_address(writer));
    encoded.extend_from_slice(&session_cid_hash);
    encoded.extend_from_slice(&metadata_uri_hash);
    encoded.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(nonce)));
    encoded.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(deadline)));

    let digest = Keccak256::digest(&encoded);
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

/// Compute the final EIP-712 digest.
fn eip712_digest(domain_separator: &[u8; 32], struct_hash: &[u8; 32]) -> [u8; 32] {
    let mut data = Vec::with_capacity(2 + 32 + 32);
    data.push(0x19);
    data.push(0x01);
    data.extend_from_slice(domain_separator);
    data.extend_from_slice(struct_hash);
    let digest = Keccak256::digest(&data);
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

/// Sign digest with secp256k1, return 65-byte signature (r || s || v).
fn sign_digest(digest: &[u8; 32], private_key_hex: &str) -> Result<[u8; 65], AppError> {
    let key_bytes = hex::decode(
        private_key_hex
            .strip_prefix("0x")
            .unwrap_or(private_key_hex),
    )
    .map_err(|_| AppError::Internal("invalid ANKY_WALLET_PRIVATE_KEY".into()))?;
    let secret_key = SecretKey::from_slice(&key_bytes)
        .map_err(|_| AppError::Internal("invalid ANKY_WALLET_PRIVATE_KEY".into()))?;
    let message = Message::from_digest_slice(digest)
        .map_err(|_| AppError::Internal("invalid digest".into()))?;
    let secp = Secp256k1::new();
    let sig = secp.sign_ecdsa_recoverable(&message, &secret_key);
    let (recovery_id, compact) = sig.serialize_compact();
    let mut result = [0u8; 65];
    result[..64].copy_from_slice(&compact);
    result[64] = recovery_id.to_i32() as u8 + 27;
    Ok(result)
}

/// Make a JSON-RPC call to the Base RPC.
async fn rpc_call(
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("RPC request failed: {}", e)))?;
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("RPC response parse failed: {}", e)))?;
    if let Some(error) = json.get("error") {
        return Err(AppError::Internal(format!("RPC error: {}", error)));
    }
    Ok(json["result"].clone())
}

/// Read on-chain nonce for a writer address from the contract's nonces(address) mapping.
async fn read_contract_nonce(rpc_url: &str, writer_address: &str) -> Result<u64, AppError> {
    // nonces(address) selector = 0x7ecebe00
    let addr_padded = hex::encode(abi_encode_address(writer_address));
    let calldata = format!("0x7ecebe00{}", addr_padded);

    let result = rpc_call(
        rpc_url,
        "eth_call",
        serde_json::json!([
            {"to": ANKY_CONTRACT, "data": calldata},
            "latest"
        ]),
    )
    .await?;

    let hex_str = result
        .as_str()
        .ok_or_else(|| AppError::Internal("nonce call returned non-string".into()))?;
    let hex_trimmed = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    u64::from_str_radix(hex_trimmed, 16)
        .map_err(|_| AppError::Internal("failed to parse nonce".into()))
}

/// Encode birthSoul(string,string,uint256,bytes) calldata for gas estimation.
fn encode_birth_soul_calldata(
    session_cid: &str,
    metadata_uri: &str,
    deadline: u64,
    signature: &[u8; 65],
) -> Vec<u8> {
    // Function selector: keccak256("birthSoul(string,string,uint256,bytes)")
    let selector = &Keccak256::digest(b"birthSoul(string,string,uint256,bytes)")[..4];

    // ABI encoding: 4 dynamic params → offsets then data
    // offset for sessionCID (param 0, dynamic) = 4 * 32 = 128
    // offset for metadataURI (param 1, dynamic) = 128 + 32 + ceil32(sessionCID.len())
    // deadline (param 2, static) = value
    // offset for signature (param 3, dynamic)

    let session_cid_bytes = session_cid.as_bytes();
    let metadata_uri_bytes = metadata_uri.as_bytes();
    let sig_bytes: &[u8] = signature;

    fn pad32(len: usize) -> usize {
        (len + 31) / 32 * 32
    }

    let session_cid_padded = pad32(session_cid_bytes.len());
    let metadata_uri_padded = pad32(metadata_uri_bytes.len());
    let sig_padded = pad32(sig_bytes.len());

    // Offsets (from start of params area, which is right after selector)
    let offset_session_cid: u64 = 4 * 32; // 4 params * 32 bytes each
    let offset_metadata_uri: u64 = offset_session_cid + 32 + session_cid_padded as u64;
    let offset_signature: u64 = offset_metadata_uri + 32 + metadata_uri_padded as u64;

    let mut data = Vec::new();
    data.extend_from_slice(selector);

    // Param 0: offset to sessionCID
    data.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(
        offset_session_cid,
    )));
    // Param 1: offset to metadataURI
    data.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(
        offset_metadata_uri,
    )));
    // Param 2: deadline (uint256)
    data.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(deadline)));
    // Param 3: offset to signature
    data.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(offset_signature)));

    // sessionCID: length + padded data
    data.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(
        session_cid_bytes.len() as u64,
    )));
    data.extend_from_slice(session_cid_bytes);
    data.resize(data.len() + session_cid_padded - session_cid_bytes.len(), 0);

    // metadataURI: length + padded data
    data.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(
        metadata_uri_bytes.len() as u64,
    )));
    data.extend_from_slice(metadata_uri_bytes);
    data.resize(
        data.len() + metadata_uri_padded - metadata_uri_bytes.len(),
        0,
    );

    // signature: length + padded data
    data.extend_from_slice(&abi_encode_u256(&u64_to_be_bytes_trimmed(
        sig_bytes.len() as u64
    )));
    data.extend_from_slice(sig_bytes);
    data.resize(data.len() + sig_padded - sig_bytes.len(), 0);

    data
}

/// Get the current transaction count (nonce) for an address.
async fn get_tx_count(rpc_url: &str, address: &str) -> Result<u64, AppError> {
    let result = rpc_call(
        rpc_url,
        "eth_getTransactionCount",
        serde_json::json!([address, "latest"]),
    )
    .await?;
    let hex_str = result.as_str().unwrap_or("0x0");
    let hex_trimmed = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    u64::from_str_radix(hex_trimmed, 16)
        .map_err(|_| AppError::Internal("failed to parse tx count".into()))
}

/// RLP-encode a single item (bytes).
fn rlp_encode_bytes(data: &[u8]) -> Vec<u8> {
    if data.len() == 1 && data[0] < 0x80 {
        return data.to_vec();
    }
    if data.is_empty() {
        return vec![0x80];
    }
    if data.len() < 56 {
        let mut out = vec![0x80 + data.len() as u8];
        out.extend_from_slice(data);
        out
    } else {
        let len_bytes = {
            let len = data.len();
            let be = len.to_be_bytes();
            let start = be.iter().position(|&b| b != 0).unwrap_or(be.len() - 1);
            be[start..].to_vec()
        };
        let mut out = vec![0xb7 + len_bytes.len() as u8];
        out.extend_from_slice(&len_bytes);
        out.extend_from_slice(data);
        out
    }
}

/// RLP-encode a list of already-encoded items.
fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = items.iter().flat_map(|i| i.iter().copied()).collect();
    if payload.len() < 56 {
        let mut out = vec![0xc0 + payload.len() as u8];
        out.extend_from_slice(&payload);
        out
    } else {
        let len_bytes = {
            let len = payload.len();
            let be = len.to_be_bytes();
            let start = be.iter().position(|&b| b != 0).unwrap_or(be.len() - 1);
            be[start..].to_vec()
        };
        let mut out = vec![0xf7 + len_bytes.len() as u8];
        out.extend_from_slice(&len_bytes);
        out.extend_from_slice(&payload);
        out
    }
}

/// Trim leading zeros from a big-endian byte slice (but keep at least empty for 0).
fn trim_leading_zeros(data: &[u8]) -> &[u8] {
    let start = data.iter().position(|&b| b != 0).unwrap_or(data.len());
    &data[start..]
}

/// Send an ETH transfer from the Anky wallet to a recipient.
/// Returns the transaction hash.
async fn send_eth_transfer(
    rpc_url: &str,
    private_key_hex: &str,
    to_address: &str,
    value_wei: u64,
) -> Result<String, AppError> {
    let key_bytes = hex::decode(
        private_key_hex
            .strip_prefix("0x")
            .unwrap_or(private_key_hex),
    )
    .map_err(|_| AppError::Internal("invalid private key".into()))?;
    let secret_key = SecretKey::from_slice(&key_bytes)
        .map_err(|_| AppError::Internal("invalid private key".into()))?;
    let secp = Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let from_address = evm_address_from_public_key(&public_key);

    let nonce = get_tx_count(rpc_url, &from_address).await?;

    // Get gas price
    let gas_price_result = rpc_call(rpc_url, "eth_gasPrice", serde_json::json!([])).await?;
    let gas_price_hex = gas_price_result.as_str().unwrap_or("0x0");
    let gas_price = u64::from_str_radix(
        gas_price_hex.strip_prefix("0x").unwrap_or(gas_price_hex),
        16,
    )
    .unwrap_or(1_000_000_000);

    // Use EIP-1559: max_priority_fee = 100000 (0.1 gwei), max_fee = gas_price * 2
    let max_priority_fee: u64 = 100_000; // very low on Base
    let max_fee_per_gas = gas_price
        .saturating_mul(2)
        .max(max_priority_fee + gas_price);
    let gas_limit: u64 = 21_000;

    // Build EIP-1559 transaction (type 2)
    // Fields: [chain_id, nonce, max_priority_fee, max_fee_per_gas, gas_limit, to, value, data, access_list]
    let to_bytes = hex::decode(to_address.strip_prefix("0x").unwrap_or(to_address))
        .map_err(|_| AppError::Internal("invalid to address".into()))?;
    let value_bytes = u64_to_be_bytes_trimmed(value_wei);

    let chain_id_bytes = u64_to_be_bytes_trimmed(BASE_CHAIN_ID);
    let nonce_bytes = u64_to_be_bytes_trimmed(nonce);
    let mpf_bytes = u64_to_be_bytes_trimmed(max_priority_fee);
    let mfpg_bytes = u64_to_be_bytes_trimmed(max_fee_per_gas);
    let gl_bytes = u64_to_be_bytes_trimmed(gas_limit);

    let items = vec![
        rlp_encode_bytes(trim_leading_zeros(&chain_id_bytes)),
        rlp_encode_bytes(trim_leading_zeros(&nonce_bytes)),
        rlp_encode_bytes(trim_leading_zeros(&mpf_bytes)),
        rlp_encode_bytes(trim_leading_zeros(&mfpg_bytes)),
        rlp_encode_bytes(trim_leading_zeros(&gl_bytes)),
        rlp_encode_bytes(&to_bytes),
        rlp_encode_bytes(trim_leading_zeros(&value_bytes)),
        rlp_encode_bytes(&[]), // data (empty for ETH transfer)
        rlp_encode_list(&[]),  // access_list (empty)
    ];

    // Sign: keccak256(0x02 || rlp([chain_id, nonce, ...]))
    let unsigned_rlp = rlp_encode_list(&items);
    let mut to_hash = vec![0x02];
    to_hash.extend_from_slice(&unsigned_rlp);
    let tx_hash = Keccak256::digest(&to_hash);

    let msg = Message::from_digest_slice(&tx_hash)
        .map_err(|_| AppError::Internal("invalid tx digest".into()))?;
    let sig = secp.sign_ecdsa_recoverable(&msg, &secret_key);
    let (recovery_id, compact) = sig.serialize_compact();

    let r = &compact[..32];
    let s = &compact[32..];
    let v = recovery_id.to_i32() as u8;

    // Signed tx: 0x02 || rlp([chain_id, nonce, max_priority_fee, max_fee_per_gas, gas_limit, to, value, data, access_list, v, r, s])
    let mut signed_items = items;
    let v_bytes = [v];
    signed_items.push(rlp_encode_bytes(if v == 0 { &[] } else { &v_bytes }));
    signed_items.push(rlp_encode_bytes(trim_leading_zeros(r)));
    signed_items.push(rlp_encode_bytes(trim_leading_zeros(s)));

    let signed_rlp = rlp_encode_list(&signed_items);
    let mut raw_tx = vec![0x02];
    raw_tx.extend_from_slice(&signed_rlp);

    let raw_tx_hex = format!("0x{}", hex::encode(&raw_tx));

    let result = rpc_call(
        rpc_url,
        "eth_sendRawTransaction",
        serde_json::json!([raw_tx_hex]),
    )
    .await?;

    result
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Internal("eth_sendRawTransaction returned non-string".into()))
}

/// Wait for a transaction to be confirmed (poll receipt).
async fn wait_for_receipt(
    rpc_url: &str,
    tx_hash: &str,
    max_wait_secs: u64,
) -> Result<serde_json::Value, AppError> {
    let start = std::time::Instant::now();
    loop {
        let result = rpc_call(
            rpc_url,
            "eth_getTransactionReceipt",
            serde_json::json!([tx_hash]),
        )
        .await?;
        if !result.is_null() {
            return Ok(result);
        }
        if start.elapsed().as_secs() > max_wait_secs {
            return Err(AppError::Internal(
                "gas funding tx not confirmed in time".into(),
            ));
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

/// Pin a JSON object to IPFS via Pinata and return the IPFS CID.
async fn pinata_pin_json(
    jwt: &str,
    name: &str,
    json_value: &serde_json::Value,
) -> Result<String, AppError> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "pinataContent": json_value,
        "pinataMetadata": { "name": name },
    });
    let resp = client
        .post("https://api.pinata.cloud/pinning/pinJSONToIPFS")
        .header("Authorization", format!("Bearer {}", jwt))
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Pinata JSON pin failed: {}", e)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Pinata JSON pin error {}: {}",
            status, text
        )));
    }
    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Pinata response parse: {}", e)))?;
    result
        .get("IpfsHash")
        .and_then(|h| h.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Internal("Pinata response missing IpfsHash".into()))
}

/// Pin a file (by local path) to IPFS via Pinata and return the IPFS CID.
async fn pinata_pin_file(jwt: &str, name: &str, file_path: &str) -> Result<String, AppError> {
    let file_bytes = tokio::fs::read(file_path)
        .await
        .map_err(|e| AppError::Internal(format!("failed to read image file: {}", e)))?;

    let mime = if file_path.ends_with(".webp") {
        "image/webp"
    } else if file_path.ends_with(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(name.to_string())
        .mime_str(mime)
        .map_err(|e| AppError::Internal(format!("multipart error: {}", e)))?;

    let metadata = serde_json::json!({ "name": name }).to_string();
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("pinataMetadata", metadata);

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.pinata.cloud/pinning/pinFileToIPFS")
        .header("Authorization", format!("Bearer {}", jwt))
        .multipart(form)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Pinata file pin failed: {}", e)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Pinata file pin error {}: {}",
            status, text
        )));
    }
    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Pinata response parse: {}", e)))?;
    result
        .get("IpfsHash")
        .and_then(|h| h.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Internal("Pinata response missing IpfsHash".into()))
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PrepareMintResponse {
    pub session_cid: String,
    pub metadata_uri: String,
    pub deadline: String,
    pub signature: String,
    pub nonce: u64,
    pub contract_address: String,
    pub chain_id: u64,
    pub gas_limit: String,
    pub max_fee_per_gas: String,
    pub max_priority_fee_per_gas: String,
    pub base_rpc_url: String,
}

/// POST /swift/v2/writing/{sessionId}/prepare-mint
pub async fn prepare_mint(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<PrepareMintResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let wallet_address = {
        let db = crate::db::conn(&state.db)?;
        queries::get_user_wallet(&db, &user_id)?
            .ok_or_else(|| AppError::Unauthorized("seed identity required".into()))?
    };
    let wallet_address = normalize_seed_wallet_address(&wallet_address)?;

    let private_key = &state.config.anky_wallet_private_key;
    if private_key.is_empty() {
        return Err(AppError::Internal("minting not configured".into()));
    }

    let pinata_jwt = &state.config.pinata_jwt;
    if pinata_jwt.is_empty() {
        return Err(AppError::Internal("IPFS pinning not configured".into()));
    }

    // Check eligibility
    let (anky_id, writing_content, _db_wallet) = {
        let db = crate::db::conn(&state.db)?;
        queries::check_mint_eligibility(&db, &session_id, &user_id)?.ok_or_else(|| {
            AppError::BadRequest(
                "anky not eligible for minting: must be complete, is_anky, not already minted"
                    .into(),
            )
        })?
    };

    // Get anky details for metadata
    let anky_detail = {
        let db = crate::db::conn(&state.db)?;
        queries::get_anky_by_id(&db, &anky_id)?
            .ok_or_else(|| AppError::Internal("anky record not found".into()))?
    };

    // Get writing session for attributes
    let (duration_seconds, word_count) = {
        let db = crate::db::conn(&state.db)?;
        if let Some(ref ws_id) = anky_detail.writing_session_id {
            let ws = queries::get_writing_session(&db, ws_id)?;
            match ws {
                Some(w) => (w.duration_seconds, w.word_count),
                None => (0.0, 0),
            }
        } else {
            (0.0, 0)
        }
    };

    // Rate limit: max 1 mint per wallet per hour
    {
        let db = crate::db::conn(&state.db)?;
        if queries::check_mint_rate_limit(&db, &wallet_address)? {
            return Err(AppError::RateLimited(3600));
        }
    }

    let rpc_url = &state.config.base_rpc_url;

    // Compute session CID = hex(sha256(writing_content))
    let session_cid = {
        let hash = <Sha256 as sha2::Digest>::digest(writing_content.as_bytes());
        hex::encode(hash)
    };

    // Pin image to IPFS (prefer webp, fall back to original)
    let image_local_path = anky_detail
        .image_webp
        .as_deref()
        .or(anky_detail.image_path.as_deref());

    let image_ipfs_url = if let Some(img_path) = image_local_path {
        // Convert URL-style path to local filesystem path
        let local_path = if img_path.starts_with("http") {
            // External URL — skip pinning, use as-is
            img_path.to_string()
        } else {
            let fs_path = if img_path.starts_with('/') {
                img_path.to_string()
            } else {
                format!("/home/kithkui/anky/{}", img_path)
            };
            let image_name = format!("anky-{}.webp", &anky_id[..8]);
            match pinata_pin_file(pinata_jwt, &image_name, &fs_path).await {
                Ok(cid) => format!("ipfs://{}", cid),
                Err(e) => {
                    tracing::warn!("Failed to pin image to IPFS: {}, using fallback", e);
                    format!("https://anky.app/{}", img_path.trim_start_matches('/'))
                }
            }
        };
        local_path
    } else {
        String::new()
    };

    // Build ERC1155 metadata JSON
    let metadata_json = serde_json::json!({
        "name": anky_detail.title.as_deref().unwrap_or("Anky"),
        "description": anky_detail.reflection.as_deref().unwrap_or("A writing born from 8 minutes of uninterrupted flow."),
        "image": image_ipfs_url,
        "external_url": format!("https://anky.app/anky/{}", anky_id),
        "attributes": [
            { "trait_type": "Duration", "value": format!("{:.0}", duration_seconds) },
            { "trait_type": "Word Count", "value": word_count.to_string() },
        ]
    });

    // Pin metadata JSON to IPFS
    let metadata_name = format!("anky-{}-metadata.json", &anky_id[..8]);
    let metadata_cid = pinata_pin_json(pinata_jwt, &metadata_name, &metadata_json).await?;
    let metadata_uri = format!("ipfs://{}", metadata_cid);

    // Read on-chain nonce
    let nonce = read_contract_nonce(rpc_url, &wallet_address).await?;

    // Deadline = now + 300 seconds
    let deadline = chrono::Utc::now().timestamp() as u64 + 300;

    // Sign EIP-712 digest
    let domain_separator = anky_domain_separator();
    let struct_hash = birth_payload_struct_hash(
        &wallet_address,
        &session_cid,
        &metadata_uri,
        nonce,
        deadline,
    );
    let digest = eip712_digest(&domain_separator, &struct_hash);
    let signature = sign_digest(&digest, private_key)?;
    let signature_hex = format!("0x{}", hex::encode(signature));

    // Estimate gas for the birthSoul call
    let calldata = encode_birth_soul_calldata(&session_cid, &metadata_uri, deadline, &signature);
    let calldata_hex = format!("0x{}", hex::encode(&calldata));

    let gas_estimate_result = rpc_call(
        rpc_url,
        "eth_estimateGas",
        serde_json::json!([{
            "from": wallet_address,
            "to": ANKY_CONTRACT,
            "data": calldata_hex,
        }]),
    )
    .await;

    let gas_limit = match gas_estimate_result {
        Ok(val) => {
            let hex_str = val.as_str().unwrap_or("0x30000");
            u64::from_str_radix(hex_str.strip_prefix("0x").unwrap_or(hex_str), 16)
                .unwrap_or(200_000)
        }
        Err(_) => 200_000, // fallback
    };
    // Add 20% buffer
    let gas_limit = gas_limit.saturating_mul(120) / 100;

    // Get current gas prices
    let gas_price_result = rpc_call(rpc_url, "eth_gasPrice", serde_json::json!([])).await?;
    let gas_price_hex = gas_price_result.as_str().unwrap_or("0x0");
    let gas_price = u64::from_str_radix(
        gas_price_hex.strip_prefix("0x").unwrap_or(gas_price_hex),
        16,
    )
    .unwrap_or(1_000_000_000);

    let max_priority_fee: u64 = 100_000; // ~0.1 gwei, cheap on Base
    let max_fee_per_gas = gas_price
        .saturating_mul(2)
        .max(max_priority_fee + gas_price);

    // Fund the user's wallet: send 2x estimated gas cost in ETH
    let funding_amount = gas_limit.saturating_mul(max_fee_per_gas).saturating_mul(2);

    let funding_tx_hash =
        send_eth_transfer(rpc_url, private_key, &wallet_address, funding_amount).await?;

    // Wait for the funding tx to confirm (max 30 seconds)
    wait_for_receipt(rpc_url, &funding_tx_hash, 30).await?;

    // Update DB
    {
        let db = crate::db::conn(&state.db)?;
        queries::set_anky_gas_funded(&db, &anky_id, &session_cid, &metadata_uri)?;
    }

    state.emit_log(
        "INFO",
        "mint",
        &format!(
            "Gas funded for anky {} (wallet: {}, funding_tx: {})",
            &anky_id[..8],
            &wallet_address,
            &funding_tx_hash
        ),
    );

    Ok(Json(PrepareMintResponse {
        session_cid,
        metadata_uri,
        deadline: deadline.to_string(),
        signature: signature_hex,
        nonce,
        contract_address: ANKY_CONTRACT.to_string(),
        chain_id: BASE_CHAIN_ID,
        gas_limit: gas_limit.to_string(),
        max_fee_per_gas: max_fee_per_gas.to_string(),
        max_priority_fee_per_gas: max_priority_fee.to_string(),
        base_rpc_url: rpc_url.clone(),
    }))
}

#[derive(Deserialize)]
pub struct ConfirmMintRequest {
    pub tx_hash: String,
}

#[derive(Serialize)]
pub struct ConfirmMintResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    pub explorer_url: String,
}

/// POST /swift/v2/writing/{sessionId}/confirm-mint
pub async fn confirm_mint(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ConfirmMintRequest>,
) -> Result<Json<ConfirmMintResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let anky_id = {
        let db = crate::db::conn(&state.db)?;
        queries::get_anky_for_mint_confirm(&db, &session_id, &user_id)?.ok_or_else(|| {
            AppError::BadRequest("anky not found or not eligible for mint confirmation".into())
        })?
    };

    let rpc_url = &state.config.base_rpc_url;

    // Get transaction receipt
    let receipt = rpc_call(
        rpc_url,
        "eth_getTransactionReceipt",
        serde_json::json!([req.tx_hash]),
    )
    .await?;

    if receipt.is_null() {
        return Err(AppError::BadRequest(
            "transaction not found or not yet confirmed".into(),
        ));
    }

    // Verify status = 1 (success)
    let status = receipt
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("0x0");
    if status != "0x1" {
        return Err(AppError::BadRequest("transaction failed on-chain".into()));
    }

    // Verify the 'to' address matches the contract
    let to_addr = receipt.get("to").and_then(|t| t.as_str()).unwrap_or("");
    let contract_lower = ANKY_CONTRACT.to_lowercase();
    if to_addr.to_lowercase() != contract_lower {
        return Err(AppError::BadRequest(
            "transaction is not to the Anky contract".into(),
        ));
    }

    // Try to parse token_id from SoulBorn event logs
    // SoulBorn(uint256 indexed tokenId, address indexed writer, string sessionCID)
    // Topic0 = keccak256("SoulBorn(uint256,address,string)")
    let soul_born_topic = hex::encode(Keccak256::digest(b"SoulBorn(uint256,address,string)"));
    let soul_born_topic = format!("0x{}", soul_born_topic);

    let token_id: Option<String> = receipt
        .get("logs")
        .and_then(|logs| logs.as_array())
        .and_then(|logs| {
            logs.iter().find_map(|log| {
                let topics = log.get("topics")?.as_array()?;
                if topics.first()?.as_str()? == soul_born_topic {
                    // tokenId is topic[1] (indexed)
                    let token_id_hex = topics.get(1)?.as_str()?;
                    let trimmed = token_id_hex.strip_prefix("0x").unwrap_or(token_id_hex);
                    let id = u64::from_str_radix(trimmed, 16).ok()?;
                    Some(id.to_string())
                } else {
                    None
                }
            })
        });

    // Update DB
    {
        let db = crate::db::conn(&state.db)?;
        queries::set_anky_minted(&db, &anky_id, &req.tx_hash, token_id.as_deref())?;
    }

    state.emit_log(
        "INFO",
        "mint",
        &format!(
            "Anky {} minted! tx: {}, token_id: {:?}",
            &anky_id[..8],
            &req.tx_hash,
            &token_id
        ),
    );

    Ok(Json(ConfirmMintResponse {
        ok: true,
        token_id,
        explorer_url: format!("https://basescan.org/tx/{}", req.tx_hash),
    }))
}

// ─── Solana Mirror Minting (Sojourn 9, iOS raw path) ────────────────────────

const KINGDOMS: [(&str, &str); 8] = [
    ("Primordia", "Root"),
    ("Emblazion", "Sacral"),
    ("Chryseos", "Solar Plexus"),
    ("Eleutheria", "Heart"),
    ("Voxlumis", "Throat"),
    ("Insightia", "Third Eye"),
    ("Claridium", "Crown"),
    ("Poiesis", "Transcendent"),
];

fn kingdom_from_address(address: &str) -> (i32, &'static str, &'static str) {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(address.as_bytes());
    let val = u64::from_le_bytes(hash[..8].try_into().unwrap());
    let idx = (val % 8) as usize;
    (idx as i32, KINGDOMS[idx].0, KINGDOMS[idx].1)
}

/// POST /swift/v2/mint-mirror (also aliased as /mirror/mint) — mint a raw mirror cNFT from a writing session.
/// Body: { "recipient": "base58-pubkey", "writing_session_id": "uuid" }
///   Also accepts "solana_address" as an alias for "recipient".
/// First seal = mint. Subsequent seals return existing mint info (not an error).
pub async fn mint_raw_mirror(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let solana_address = body["recipient"]
        .as_str()
        .or_else(|| body["solana_address"].as_str())
        .ok_or_else(|| AppError::BadRequest("recipient (solana base58 pubkey) required".into()))?
        .trim();

    let writing_session_id = body["writing_session_id"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("writing_session_id required".into()))?;

    // Validate base58 Solana pubkey (32 bytes when decoded, 32-44 chars as base58)
    if solana_address.len() < 32 || solana_address.len() > 44 {
        return Err(AppError::BadRequest("invalid solana address length".into()));
    }
    if solana_address.starts_with("0x") {
        return Err(AppError::BadRequest(
            "this looks like an Ethereum address — send a Solana pubkey (base58)".into(),
        ));
    }
    // Verify all chars are valid base58
    if !solana_address
        .chars()
        .all(|c| matches!(c, '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z'))
    {
        return Err(AppError::BadRequest(
            "invalid base58 characters in solana address".into(),
        ));
    }

    if state.config.solana_mint_worker_url.is_empty() {
        return Err(AppError::Internal(
            "solana mint worker not configured".into(),
        ));
    }

    // Validate writing session: must be a real anky (8+ min) owned by this user
    {
        let db = crate::db::conn(&state.db)?;
        let mut stmt = db
            .prepare("SELECT is_anky, user_id FROM writing_sessions WHERE id = ?1")
            .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
        let row = stmt
            .query_row(crate::params![writing_session_id], |row| {
                Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| AppError::NotFound("writing session not found".into()))?;

        if row.0 == 0 {
            return Err(AppError::BadRequest(
                "only real ankys (8+ minutes) can be minted".into(),
            ));
        }
        if row.1 != user_id {
            return Err(AppError::BadRequest(
                "this writing session belongs to another user".into(),
            ));
        }
    }

    // Check if THIS writing session was already minted — idempotent return
    {
        let db = crate::db::conn(&state.db)?;
        let mut stmt = db
            .prepare(
                "SELECT m.solana_mint_tx, m.id, m.kingdom, m.kingdom_name, a.image_path, a.image_webp
                 FROM mirrors m
                 LEFT JOIN ankys a ON a.writing_session_id = m.writing_session_id
                 WHERE m.writing_session_id = ?1 AND m.solana_mint_tx IS NOT NULL
                 LIMIT 1",
            )
            .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
        let existing = stmt
            .query_row(crate::params![writing_session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i32>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .ok();
        if let Some((tx, mid, kingdom, kingdom_name, img_path, img_webp)) = existing {
            let (kid, kname, kchakra) = kingdom_from_address(solana_address);
            let image_url = img_webp.or(img_path).map(|p| {
                if p.starts_with("http") {
                    p
                } else {
                    format!("https://anky.app/data/images/{}", p)
                }
            });
            return Ok(Json(serde_json::json!({
                "success": true,
                "already_minted": true,
                "mirror_id": mid,
                "kingdom": kingdom_name.unwrap_or_else(|| kname.to_string()),
                "kingdom_chakra": kchakra,
                "kingdom_id": kingdom.unwrap_or(kid),
                "tx_signature": tx,
                "image_url": image_url,
                "error": null,
            })));
        }
    }

    // Check if user already has ANY mint — one cNFT per wallet, ever
    {
        let db = crate::db::conn(&state.db)?;
        if let Some((tx, mirror_id, kingdom, kingdom_name)) =
            queries::get_user_existing_mint(&db, &user_id)
                .map_err(|e| AppError::Internal(format!("DB: {}", e)))?
        {
            let (_, _, kchakra) = kingdom_from_address(solana_address);
            return Ok(Json(serde_json::json!({
                "success": true,
                "already_minted": true,
                "mirror_id": mirror_id,
                "kingdom": kingdom_name.unwrap_or_default(),
                "kingdom_chakra": kchakra,
                "kingdom_id": kingdom.unwrap_or(0),
                "tx_signature": tx,
                "image_url": null,
                "error": null,
            })));
        }
    }

    // Check supply cap
    {
        let db = crate::db::conn(&state.db)?;
        let minted = queries::count_minted_mirrors(&db)
            .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
        if minted >= 3456 {
            return Err(AppError::BadRequest(
                "all 3456 mirrors have been claimed".into(),
            ));
        }
    }

    let (kingdom_id, kingdom_name, chakra) = kingdom_from_address(solana_address);
    let mirror_id = uuid::Uuid::new_v4().to_string();

    // Get anky image URL for the response
    let anky_image_url = {
        let db = crate::db::conn(&state.db)?;
        let mut stmt = db
            .prepare("SELECT image_path, image_webp FROM ankys WHERE writing_session_id = ?1 AND status = 'complete' LIMIT 1")
            .ok();
        stmt.and_then(|mut s| {
            s.query_row(crate::params![writing_session_id], |row| {
                let path: Option<String> = row.get(0)?;
                let webp: Option<String> = row.get(1)?;
                Ok(webp.or(path))
            })
            .ok()
        })
        .flatten()
        .map(|p| {
            if p.starts_with("http") {
                p
            } else {
                format!("https://anky.app/data/images/{}", p)
            }
        })
    };

    // Insert raw mirror record linked to this writing session
    {
        let db = crate::db::conn(&state.db)?;
        queries::insert_raw_mirror_for_session(
            &db,
            &mirror_id,
            &user_id,
            solana_address,
            writing_session_id,
        )
        .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
    }

    let metadata_uri = format!("https://anky.app/api/mirror/metadata/{}", mirror_id);
    let name = "Anky Mirror — raw".to_string();

    // Call Cloudflare Worker to mint
    let client = reqwest::Client::new();
    let worker_resp = client
        .post(format!("{}/mint", state.config.solana_mint_worker_url))
        .header(
            "Authorization",
            format!("Bearer {}", state.config.solana_mint_worker_secret),
        )
        .json(&serde_json::json!({
            "mirror_id": mirror_id,
            "recipient": solana_address,
            "name": name,
            "uri": metadata_uri,
            "kingdom": kingdom_id,
            "symbol": "ANKY",
        }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("mint worker request failed: {}", e)))?;

    if !worker_resp.status().is_success() {
        let err_text = worker_resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "mint worker error: {}",
            err_text
        )));
    }

    let mint_result: serde_json::Value = worker_resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("mint worker response parse: {}", e)))?;

    let tx_signature = mint_result["signature"].as_str().unwrap_or("");

    // Update DB with Solana tx details
    {
        let db = crate::db::conn(&state.db)?;
        queries::set_mirror_minted(
            &db,
            &mirror_id,
            tx_signature,
            solana_address,
            mint_result["asset_id"].as_str(),
            kingdom_id,
            kingdom_name,
        )
        .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "already_minted": false,
        "mirror_id": mirror_id,
        "kingdom": kingdom_name,
        "kingdom_chakra": chakra,
        "kingdom_id": kingdom_id,
        "tx_signature": tx_signature,
        "image_url": anky_image_url,
        "error": null,
    })))
}

/// GET /api/v1/anky/{id}/metadata — public ERC1155-compliant metadata
pub async fn anky_metadata(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = crate::db::conn(&state.db)?;
    let anky = queries::get_anky_by_id(&db, &id)?
        .ok_or_else(|| AppError::NotFound("anky not found".into()))?;

    let image_url = anky
        .image_webp
        .as_deref()
        .or(anky.image_path.as_deref())
        .map(|p| {
            if p.starts_with("http") {
                p.to_string()
            } else {
                format!("https://anky.app/{}", p.trim_start_matches('/'))
            }
        })
        .unwrap_or_default();

    // Get writing session for word count, duration, and session hash
    let (duration_seconds, word_count, session_hash) =
        if let Some(ref ws_id) = anky.writing_session_id {
            let ws = queries::get_writing_session(&db, ws_id)?;
            let hash = crate::routes::sealed::get_session_hash_by_session_id(&db, ws_id);
            match ws {
                Some(w) => (w.duration_seconds, w.word_count as f64, hash),
                None => (0.0, 0.0, hash),
            }
        } else {
            (0.0, 0.0, None)
        };

    let kingdom = anky.kingdom_name.as_deref().unwrap_or("Unknown");
    let chakra = anky.kingdom_chakra.as_deref().unwrap_or("Unknown");

    let mut attributes = vec![
        serde_json::json!({ "trait_type": "Sojourn", "value": 9 }),
        serde_json::json!({ "trait_type": "Kingdom", "value": kingdom }),
        serde_json::json!({ "trait_type": "Chakra", "value": chakra }),
        serde_json::json!({ "trait_type": "Duration", "value": format!("{:.0}s", duration_seconds) }),
        serde_json::json!({ "trait_type": "Word Count", "value": format!("{:.0}", word_count) }),
    ];
    if let Some(ref hash) = session_hash {
        attributes.push(serde_json::json!({ "trait_type": "Session Hash", "value": hash }));
    }

    Ok(Json(serde_json::json!({
        "name": anky.title.unwrap_or_else(|| "Anky".to_string()),
        "description": anky.reflection.unwrap_or_else(|| "A writing born from 8 minutes of uninterrupted flow.".to_string()),
        "image": image_url,
        "external_url": format!("https://anky.app/anky/{}", id),
        "attributes": attributes,
    })))
}
