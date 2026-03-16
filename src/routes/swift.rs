/// /swift/v1/* — Mobile API for the Anky iOS app
///
/// Auth: Bearer token in `Authorization: Bearer <session_token>` header.
/// Session tokens are the same as web sessions (auth_sessions table).
/// Mobile auth via Privy SDK: POST /swift/v1/auth/privy → returns session_token as JSON.
/// Seed-phrase identity auth: POST /swift/v2/auth/challenge + POST /swift/v2/auth/verify.
use crate::db::queries;
use crate::error::AppError;
use crate::services::claude;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::RngCore;
use rusqlite;
use serde::{Deserialize, Serialize};

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

fn normalize_seed_wallet_address(wallet_address: &str) -> String {
    wallet_address.trim().to_string()
}

fn parse_sol_public_key(wallet_address: &str) -> Result<VerifyingKey, AppError> {
    let raw = bs58::decode(wallet_address)
        .into_vec()
        .map_err(|_| AppError::BadRequest("invalid public key".into()))?;
    let key_bytes: [u8; 32] = raw
        .try_into()
        .map_err(|_| AppError::BadRequest("invalid public key length".into()))?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| AppError::BadRequest("invalid public key".into()))
}

fn build_seed_auth_message(wallet_address: &str, challenge_id: &str, nonce: &str) -> String {
    format!(
        "anky.app seed identity sign in\n\npublic key: {}\nchallenge id: {}\nnonce: {}\n\nsign this only inside the anky app.",
        wallet_address, challenge_id, nonce
    )
}

fn verify_seed_auth_signature(
    wallet_address: &str,
    message: &str,
    signature_b58: &str,
) -> Result<(), AppError> {
    let verifying_key = parse_sol_public_key(wallet_address)?;
    let raw_sig = bs58::decode(signature_b58.trim())
        .into_vec()
        .map_err(|_| AppError::BadRequest("invalid signature encoding".into()))?;
    let signature = Signature::from_slice(&raw_sig)
        .map_err(|_| AppError::BadRequest("invalid signature".into()))?;
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| AppError::Unauthorized("signature verification failed".into()))
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
/// Create a one-time sign-in challenge for a locally derived seed-phrase identity.
/// The iOS app should sign the returned `message` with the device's seed-derived keypair.
pub async fn auth_seed_challenge(
    State(state): State<AppState>,
    Json(req): Json<SeedAuthChallengeRequest>,
) -> Result<Json<SeedAuthChallengeResponse>, AppError> {
    let wallet_address = normalize_seed_wallet_address(&req.wallet_address);
    if wallet_address.is_empty() {
        return Err(AppError::BadRequest("wallet_address is required".into()));
    }
    let _ = parse_sol_public_key(&wallet_address)?;

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

/// POST /swift/v2/auth/verify
/// Verify a signature from the seed-derived keypair and return a normal Anky session token.
pub async fn auth_seed_verify(
    State(state): State<AppState>,
    Json(req): Json<SeedAuthVerifyRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let wallet_address = normalize_seed_wallet_address(&req.wallet_address);
    if wallet_address.is_empty() {
        return Err(AppError::BadRequest("wallet_address is required".into()));
    }

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

    state.emit_log(
        "INFO",
        "swift_auth",
        &format!(
            "Seed identity login: {} ({})",
            &user_id[..8],
            wallet_address
        ),
    );

    Ok(Json(AuthResponse {
        ok: true,
        session_token,
        user_id,
        username,
        email: None,
        wallet_address: Some(wallet_address),
    }))
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

    Ok(Json(MeResponse {
        user_id,
        username,
        display_name,
        profile_image_url,
        email,
        wallet_address: wallet,
        total_writings,
        total_ankys,
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
}

#[derive(Serialize)]
pub struct MobileWriteResponse {
    pub ok: bool,
    pub session_id: String,
    pub is_anky: bool,
    pub word_count: i32,
    pub flow_score: Option<f64>,
    pub persisted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// POST /swift/v1/write
pub async fn submit_writing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<MobileWriteRequest>,
) -> Result<Json<MobileWriteResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let word_count = req.text.split_whitespace().count() as i32;
    if word_count < 10 {
        return Ok(Json(MobileWriteResponse {
            ok: false,
            session_id: String::new(),
            is_anky: false,
            word_count,
            flow_score: None,
            persisted: false,
            response: None,
            anky_id: None,
            wallet_address: None,
            error: Some("write more — at least a few sentences to stream".into()),
        }));
    }

    let flow_score = req
        .keystroke_deltas
        .as_ref()
        .map(|d| queries::calculate_flow_score(d, req.duration, word_count));
    let keystroke_json = req
        .keystroke_deltas
        .as_ref()
        .map(|d| serde_json::to_string(d).unwrap_or_default());

    let is_anky = req.duration >= 480.0 && word_count >= 300;

    let session_id = req
        .session_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    tracing::info!(
        user = %user_id,
        duration = req.duration,
        words = word_count,
        is_anky = is_anky,
        "Mobile writing submitted"
    );

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
            is_anky,
            None,
            keystroke_json.as_deref(),
            flow_score,
            None,
        )?;
        if let Some(fs) = flow_score {
            let _ = queries::update_user_flow_stats(&db, &user_id, fs, is_anky);
        }
    }

    let (response, anky_id, wallet_address) = if is_anky {
        // Kick off Anky image generation in the background
        let anky_id = uuid::Uuid::new_v4().to_string();
        let state_bg = state.clone();
        let aid = anky_id.clone();
        let sid = session_id.clone();
        let text = req.text.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::pipeline::image_gen::generate_anky_from_writing(
                &state_bg, &aid, &sid, "mobile", &text,
            )
            .await
            {
                state_bg.emit_log(
                    "ERROR",
                    "swift_write",
                    &format!("Anky gen failed for {}: {}", &aid[..8], e),
                );
            }
        });
        let wallet_address = {
            let db = state.db.lock().await;
            queries::get_user_wallet(&db, &user_id).ok().flatten()
        };
        (None, Some(anky_id), wallet_address)
    } else {
        // Short session — get Ollama feedback synchronously
        let prompt = crate::services::ollama::quick_feedback_prompt(&req.text, req.duration);
        let feedback = crate::services::ollama::call_ollama(
            &state.config.ollama_base_url,
            &state.config.ollama_model,
            &prompt,
        )
        .await
        .unwrap_or_else(|_| "keep flowing — every word counts.".into());
        {
            let db = state.db.lock().await;
            let _ = db.execute(
                "UPDATE writing_sessions SET response = ?1 WHERE id = ?2",
                rusqlite::params![&feedback, &session_id],
            );
        }
        (Some(feedback), None, None)
    };

    // Queue personalized meditation + breathwork generation
    let guidance_state = state.clone();
    let guidance_user = user_id.clone();
    let guidance_session = session_id.clone();
    let guidance_text = req.text.clone();
    tokio::spawn(async move {
        crate::pipeline::guidance_gen::queue_post_writing_guidance(
            &guidance_state,
            &guidance_user,
            &guidance_session,
            &guidance_text,
        )
        .await;
    });

    Ok(Json(MobileWriteResponse {
        ok: true,
        session_id,
        is_anky,
        word_count,
        flow_score,
        persisted: true,
        response,
        anky_id,
        wallet_address,
        error: None,
    }))
}

/// POST /swift/v2/write
/// Seed-identity mobile writing flow.
/// Short sessions are local-only and must not be persisted server-side.
pub async fn submit_writing_v2(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<MobileWriteRequest>,
) -> Result<Json<MobileWriteResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let word_count = req.text.split_whitespace().count() as i32;
    if word_count < 10 {
        return Ok(Json(MobileWriteResponse {
            ok: false,
            session_id: String::new(),
            is_anky: false,
            word_count,
            flow_score: None,
            persisted: false,
            response: None,
            anky_id: None,
            wallet_address: None,
            error: Some("write more — at least a few sentences to stream".into()),
        }));
    }

    let flow_score = req
        .keystroke_deltas
        .as_ref()
        .map(|d| queries::calculate_flow_score(d, req.duration, word_count));
    let keystroke_json = req
        .keystroke_deltas
        .as_ref()
        .map(|d| serde_json::to_string(d).unwrap_or_default());

    let is_anky = req.duration >= 480.0 && word_count >= 300;
    let session_id = req
        .session_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    if !is_anky {
        tracing::info!(
            user = %user_id,
            duration = req.duration,
            words = word_count,
            "Mobile v2 writing stayed local-only"
        );
        return Ok(Json(MobileWriteResponse {
            ok: true,
            session_id,
            is_anky: false,
            word_count,
            flow_score,
            persisted: false,
            response: None,
            anky_id: None,
            wallet_address: None,
            error: None,
        }));
    }

    tracing::info!(
        user = %user_id,
        duration = req.duration,
        words = word_count,
        "Mobile v2 anky submitted"
    );

    let wallet_address = {
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

        let wallet_address = queries::get_user_wallet(&db, &user_id)?
            .ok_or_else(|| AppError::Unauthorized("seed identity required".into()))?;
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
        wallet_address
    };

    let anky_id = uuid::Uuid::new_v4().to_string();
    let state_bg = state.clone();
    let aid = anky_id.clone();
    let sid = session_id.clone();
    let text = req.text.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::pipeline::image_gen::generate_anky_from_writing(
            &state_bg, &aid, &sid, "mobile", &text,
        )
        .await
        {
            state_bg.emit_log(
                "ERROR",
                "swift_write",
                &format!("Anky gen failed for {}: {}", &aid[..8], e),
            );
        }
    });

    let guidance_state = state.clone();
    let guidance_user = user_id.clone();
    let guidance_session = session_id.clone();
    let guidance_text = req.text.clone();
    tokio::spawn(async move {
        crate::pipeline::guidance_gen::queue_post_writing_guidance(
            &guidance_state,
            &guidance_user,
            &guidance_session,
            &guidance_text,
        )
        .await;
    });

    Ok(Json(MobileWriteResponse {
        ok: true,
        session_id,
        is_anky: true,
        word_count,
        flow_score,
        persisted: true,
        response: None,
        anky_id: Some(anky_id),
        wallet_address: Some(wallet_address),
        error: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    #[test]
    fn verifies_seed_auth_signatures() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let wallet_address = bs58::encode(signing_key.verifying_key().to_bytes()).into_string();
        let message = build_seed_auth_message(&wallet_address, "challenge-1", "nonce-1");
        let signature = signing_key.sign(message.as_bytes());
        let signature_b58 = bs58::encode(signature.to_bytes()).into_string();

        let verified = verify_seed_auth_signature(&wallet_address, &message, &signature_b58);
        assert!(verified.is_ok());
    }
}

// ===== Sadhana =====

#[derive(Deserialize)]
pub struct CreateSadhanaRequest {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_frequency")]
    pub frequency: String,
    #[serde(default = "default_duration")]
    pub duration_minutes: i32,
    #[serde(default = "default_target_days")]
    pub target_days: i32,
}

fn default_frequency() -> String {
    "daily".into()
}
fn default_duration() -> i32 {
    10
}
fn default_target_days() -> i32 {
    30
}

#[derive(Serialize)]
pub struct SadhanaItem {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub frequency: String,
    pub duration_minutes: i32,
    pub target_days: i32,
    pub start_date: String,
    pub is_active: bool,
    pub created_at: String,
    pub total_checkins: i32,
    pub completed_checkins: i32,
}

/// GET /swift/v1/sadhana
pub async fn list_sadhana(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<SadhanaItem>>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = state.db.lock().await;
    let commitments = queries::get_user_sadhana_commitments(&db, &user_id)?;
    let mut items = Vec::new();
    for c in commitments {
        let checkins = queries::get_sadhana_checkins(&db, &c.id).unwrap_or_default();
        let total = checkins.len() as i32;
        let completed = checkins.iter().filter(|ch| ch.completed).count() as i32;
        items.push(SadhanaItem {
            id: c.id,
            title: c.title,
            description: c.description,
            frequency: c.frequency,
            duration_minutes: c.duration_minutes,
            target_days: c.target_days,
            start_date: c.start_date,
            is_active: c.is_active,
            created_at: c.created_at,
            total_checkins: total,
            completed_checkins: completed,
        });
    }
    Ok(Json(items))
}

/// POST /swift/v1/sadhana
pub async fn create_sadhana(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateSadhanaRequest>,
) -> Result<Json<SadhanaItem>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    {
        let db = state.db.lock().await;
        queries::ensure_user(&db, &user_id)?;
        queries::create_sadhana_commitment(
            &db,
            &id,
            &user_id,
            req.title.trim(),
            req.description.as_deref(),
            &req.frequency,
            req.duration_minutes,
            req.target_days,
            &today,
        )?;
    }

    Ok(Json(SadhanaItem {
        id,
        title: req.title,
        description: req.description,
        frequency: req.frequency,
        duration_minutes: req.duration_minutes,
        target_days: req.target_days,
        start_date: today,
        is_active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        total_checkins: 0,
        completed_checkins: 0,
    }))
}

#[derive(Serialize)]
pub struct SadhanaDetail {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub frequency: String,
    pub duration_minutes: i32,
    pub target_days: i32,
    pub start_date: String,
    pub is_active: bool,
    pub created_at: String,
    pub checkins: Vec<SadhanaCheckinItem>,
}

#[derive(Serialize)]
pub struct SadhanaCheckinItem {
    pub id: String,
    pub date: String,
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
}

/// GET /swift/v1/sadhana/:id
pub async fn get_sadhana(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SadhanaDetail>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = state.db.lock().await;
    let commitment = queries::get_sadhana_commitment(&db, &id, &user_id)?
        .ok_or_else(|| AppError::NotFound("sadhana commitment not found".into()))?;
    let checkins = queries::get_sadhana_checkins(&db, &id).unwrap_or_default();
    Ok(Json(SadhanaDetail {
        id: commitment.id,
        title: commitment.title,
        description: commitment.description,
        frequency: commitment.frequency,
        duration_minutes: commitment.duration_minutes,
        target_days: commitment.target_days,
        start_date: commitment.start_date,
        is_active: commitment.is_active,
        created_at: commitment.created_at,
        checkins: checkins
            .into_iter()
            .map(|ch| SadhanaCheckinItem {
                id: ch.id,
                date: ch.date,
                completed: ch.completed,
                notes: ch.notes,
                created_at: ch.created_at,
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
pub struct SadhanaCheckinRequest {
    pub completed: bool,
    #[serde(default)]
    pub notes: Option<String>,
    /// Date in YYYY-MM-DD format; defaults to today UTC
    #[serde(default)]
    pub date: Option<String>,
}

/// POST /swift/v1/sadhana/:id/checkin
pub async fn sadhana_checkin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<SadhanaCheckinRequest>,
) -> Result<Json<SadhanaCheckinItem>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let date = req
        .date
        .filter(|d| !d.trim().is_empty())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());

    // Verify the commitment belongs to this user
    {
        let db = state.db.lock().await;
        queries::get_sadhana_commitment(&db, &id, &user_id)?
            .ok_or_else(|| AppError::NotFound("sadhana commitment not found".into()))?;
    }

    let checkin_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    {
        let db = state.db.lock().await;
        queries::upsert_sadhana_checkin(
            &db,
            &checkin_id,
            &id,
            &user_id,
            &date,
            req.completed,
            req.notes.as_deref(),
        )?;
    }

    Ok(Json(SadhanaCheckinItem {
        id: checkin_id,
        date,
        completed: req.completed,
        notes: req.notes,
        created_at,
    }))
}

// ===== Meditation =====

#[derive(Deserialize)]
pub struct StartMeditationMobileRequest {
    pub duration_minutes: i32,
}

#[derive(Serialize)]
pub struct MeditationSessionResponse {
    pub session_id: String,
    pub duration_target: i32,
}

/// POST /swift/v1/meditation/start
pub async fn meditation_start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<StartMeditationMobileRequest>,
) -> Result<Json<MeditationSessionResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let duration_seconds = req.duration_minutes * 60;
    {
        let db = state.db.lock().await;
        queries::ensure_user(&db, &user_id)?;
        queries::insert_meditation_session(&db, &session_id, &user_id, duration_seconds)?;
    }
    Ok(Json(MeditationSessionResponse {
        session_id,
        duration_target: duration_seconds,
    }))
}

#[derive(Deserialize)]
pub struct CompleteMeditationMobileRequest {
    pub session_id: String,
    pub actual_seconds: i32,
    pub completed: bool,
}

#[derive(Serialize)]
pub struct MeditationCompleteResponse {
    pub ok: bool,
    pub total_meditations: i32,
    pub current_streak: i32,
}

/// POST /swift/v1/meditation/complete
pub async fn meditation_complete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CompleteMeditationMobileRequest>,
) -> Result<Json<MeditationCompleteResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    {
        let db = state.db.lock().await;
        let was_completed =
            queries::complete_meditation_session(&db, &req.session_id, req.actual_seconds)?;
        if req.completed && was_completed {
            queries::increment_meditation(&db, &user_id)?;
        }
    }
    let progression = {
        let db = state.db.lock().await;
        queries::get_or_create_progression(&db, &user_id)?
    };
    Ok(Json(MeditationCompleteResponse {
        ok: true,
        total_meditations: progression.total_meditations,
        current_streak: progression.current_streak,
    }))
}

#[derive(Serialize)]
pub struct MeditationHistoryItem {
    pub id: String,
    pub duration_target: i32,
    pub duration_actual: Option<i32>,
    pub completed: bool,
    pub created_at: String,
}

/// GET /swift/v1/meditation/history
pub async fn meditation_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MeditationHistoryItem>>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = state.db.lock().await;
    let sessions = queries::get_user_meditation_history(&db, &user_id)?;
    let items = sessions
        .into_iter()
        .map(
            |(id, target, actual, completed, created_at)| MeditationHistoryItem {
                id,
                duration_target: target,
                duration_actual: actual,
                completed,
                created_at,
            },
        )
        .collect();
    Ok(Json(items))
}

// ===== Breathwork =====

/// A single phase in a breathwork session
#[derive(Serialize, Deserialize)]
pub struct BreathworkPhase {
    pub name: String,
    /// "narration" | "breathing" | "hold" | "rest"
    pub phase_type: String,
    pub duration_seconds: i32,
    pub narration: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inhale_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exhale_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reps: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct BreathworkScript {
    pub id: String,
    pub style: String,
    pub title: String,
    pub description: String,
    pub duration_seconds: i32,
    pub background_beat_bpm: i32,
    pub phases: Vec<BreathworkPhase>,
}

/// POST /swift/v1/breathwork/session?style=wim_hof
/// Returns an existing cached session or generates a new one via Claude.
pub async fn get_breathwork_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<BreathworkScript>, AppError> {
    // Auth optional for fetching (still log user if present)
    let _ = bearer_auth(&state, &headers).await;

    let style = params
        .get("style")
        .cloned()
        .unwrap_or_else(|| "wim_hof".into());

    let valid_styles = [
        "wim_hof",
        "box",
        "4_7_8",
        "pranayama",
        "energizing",
        "calming",
    ];
    if !valid_styles.contains(&style.as_str()) {
        return Err(AppError::BadRequest(
            "style must be one of: wim_hof, box, 4_7_8, pranayama, energizing, calming".into(),
        ));
    }

    // Check cache first
    {
        let db = state.db.lock().await;
        if let Ok(Some(cached)) = queries::get_breathwork_session_by_style(&db, &style) {
            // Regenerate if older than 24 hours for freshness
            let generated = chrono::DateTime::parse_from_rfc3339(&cached.generated_at)
                .ok()
                .map(|t| t.with_timezone(&chrono::Utc));
            let fresh = generated
                .map(|t| chrono::Utc::now() - t < chrono::Duration::hours(24))
                .unwrap_or(false);
            if fresh {
                if let Ok(script) = serde_json::from_str::<BreathworkScript>(&cached.script_json) {
                    return Ok(Json(script));
                }
            }
        }
    }

    // Generate new session via Claude
    let script = generate_breathwork_session(&state, &style).await?;
    let script_json = serde_json::to_string(&script)
        .map_err(|e| AppError::Internal(format!("serialize failed: {}", e)))?;

    {
        let db = state.db.lock().await;
        let _ = queries::insert_breathwork_session(
            &db,
            &script.id,
            &script.style,
            script.duration_seconds,
            &script_json,
        );
    }

    Ok(Json(script))
}

async fn generate_breathwork_session(
    state: &AppState,
    style: &str,
) -> Result<BreathworkScript, AppError> {
    let style_desc = match style {
        "wim_hof" => "Wim Hof Method: 3 rounds of 30 power breaths followed by retention holds and recovery breaths. Cold-exposure philosophy, activating the sympathetic nervous system, building inner heat and resilience.",
        "box" => "Box Breathing: 4 counts inhale, 4 hold, 4 exhale, 4 hold — the Navy SEAL technique for stress regulation and focus. Calm, precise, grounding.",
        "4_7_8" => "4-7-8 Breathing: Dr. Andrew Weil's technique — 4 counts inhale through nose, 7 counts hold, 8 counts exhale through mouth. Powerful for calming the nervous system and inducing sleep.",
        "pranayama" => "Yoga Pranayama: Nadi Shodhana (alternate nostril) and Kapalabhati (skull-shining breath). Ancient yogic breath control for balancing prana and clearing energy channels.",
        "energizing" => "Energizing Breathwork: Bellows breath (Bhastrika) and vigorous power breathing to awaken the body, boost oxygen, and create a natural high without substances.",
        "calming" => "Calming Breathwork: Extended exhale technique — inhale for 4, exhale for 8 — activating the parasympathetic nervous system. Perfect for anxiety, before sleep, or after intense activity.",
        _ => "Wim Hof Method",
    };

    let prompt = format!(
        r#"You are Anky — a spiritual guide, breathwork teacher, and consciousness explorer.
Generate an 8-minute ({style}) breathwork session as a JSON object.

Style: {style_desc}

The session must total ~480 seconds and feel like a complete journey: opening narration, the breathwork practice itself with multiple rounds/phases, and a closing integration.

Respond ONLY with valid JSON matching this exact structure:
{{
  "title": "A poetic, evocative title for this session",
  "description": "1-2 sentences describing what the practitioner will experience",
  "background_beat_bpm": 60,
  "phases": [
    {{
      "name": "Phase name",
      "phase_type": "narration|breathing|hold|rest",
      "duration_seconds": 30,
      "narration": "What Anky says to guide the practitioner. Warm, present, slightly mystical. First person from Anky's voice.",
      "inhale_seconds": null,
      "exhale_seconds": null,
      "hold_seconds": null,
      "reps": null
    }}
  ]
}}

Rules:
- Total duration_seconds across all phases must sum to ~480
- narration type: just speaking, no breathing instruction (30-60s)
- breathing type: set inhale_seconds, exhale_seconds, reps. hold_seconds optional for retention
- hold type: set hold_seconds. narration guides the hold
- rest type: silence/integration (10-30s)
- Narration text should be poetic, grounding, and specific to the style
- For Wim Hof: 3 rounds minimum with retention holds
- For box: multiple rounds with precise counts
- background_beat_bpm: match to the energy (40-80 bpm)
- Include at least 6-8 phases for a complete journey
- All narration_seconds fields must be numbers, not null"#,
        style = style,
        style_desc = style_desc,
    );

    let result = claude::call_claude_public(
        &state.config.anthropic_api_key,
        "claude-haiku-4-5-20251001",
        "You are Anky, a breathwork guide. Output only valid JSON, no markdown code blocks, no extra text.",
        &prompt,
        3000,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Claude breathwork gen failed: {}", e)))?;

    // Strip markdown code fences if present
    let clean = result
        .text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let partial: serde_json::Value = serde_json::from_str(clean).map_err(|e| {
        AppError::Internal(format!(
            "Breathwork JSON parse failed: {}\n{}",
            e,
            &clean[..200.min(clean.len())]
        ))
    })?;

    let phases_raw = partial
        .get("phases")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    let mut phases = Vec::new();
    for p in phases_raw {
        phases.push(BreathworkPhase {
            name: p
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Phase")
                .to_string(),
            phase_type: p
                .get("phase_type")
                .and_then(|v| v.as_str())
                .unwrap_or("narration")
                .to_string(),
            duration_seconds: p
                .get("duration_seconds")
                .and_then(|v| v.as_i64())
                .unwrap_or(30) as i32,
            narration: p
                .get("narration")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            inhale_seconds: p.get("inhale_seconds").and_then(|v| v.as_f64()),
            exhale_seconds: p.get("exhale_seconds").and_then(|v| v.as_f64()),
            hold_seconds: p.get("hold_seconds").and_then(|v| v.as_f64()),
            reps: p.get("reps").and_then(|v| v.as_i64()).map(|v| v as i32),
        });
    }

    Ok(BreathworkScript {
        id: uuid::Uuid::new_v4().to_string(),
        style: style.to_string(),
        title: partial
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Breathwork Session")
            .to_string(),
        description: partial
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        duration_seconds: 480,
        background_beat_bpm: partial
            .get("background_beat_bpm")
            .and_then(|v| v.as_i64())
            .unwrap_or(60) as i32,
        phases,
    })
}

#[derive(Deserialize)]
pub struct CompleteBreathworkRequest {
    pub session_id: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// POST /swift/v1/breathwork/complete
pub async fn breathwork_complete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CompleteBreathworkRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().await;
        queries::ensure_user(&db, &user_id)?;
        queries::log_breathwork_completion(
            &db,
            &id,
            &user_id,
            &req.session_id,
            req.notes.as_deref(),
        )?;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /swift/v1/meditation/ready
/// Returns the latest ready personalized meditation for this user.
/// If none exists (or all are older than 24h and user has no writing),
/// triggers a generic daily one and returns `{ status: "generating" }`.
pub async fn meditation_ready(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    if let Some((_id, script_json)) = {
        let db = state.db.lock().await;
        queries::get_ready_meditation(&db, &user_id)?
    } {
        let script: serde_json::Value =
            serde_json::from_str(&script_json).unwrap_or(serde_json::json!({}));
        return Ok(Json(
            serde_json::json!({ "status": "ready", "session": script }),
        ));
    }

    // Nothing ready — queue a daily generic one if we haven't recently
    let has_recent = {
        let db = state.db.lock().await;
        queries::has_recent_ready_meditation(&db, &user_id).unwrap_or(false)
    };
    if !has_recent {
        let state_bg = state.clone();
        let uid = user_id.clone();
        tokio::spawn(async move {
            crate::pipeline::guidance_gen::queue_daily_guidance(&state_bg, &uid).await;
        });
    }

    Ok(Json(serde_json::json!({ "status": "generating" })))
}

/// GET /swift/v1/breathwork/ready
pub async fn breathwork_ready(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    if let Some((_id, style, script_json)) = {
        let db = state.db.lock().await;
        queries::get_ready_breathwork(&db, &user_id)?
    } {
        let script: serde_json::Value =
            serde_json::from_str(&script_json).unwrap_or(serde_json::json!({}));
        return Ok(Json(
            serde_json::json!({ "status": "ready", "style": style, "session": script }),
        ));
    }

    let has_recent = {
        let db = state.db.lock().await;
        queries::has_recent_ready_breathwork(&db, &user_id).unwrap_or(false)
    };
    if !has_recent {
        let state_bg = state.clone();
        let uid = user_id.clone();
        tokio::spawn(async move {
            crate::pipeline::guidance_gen::queue_daily_guidance(&state_bg, &uid).await;
        });
    }

    Ok(Json(serde_json::json!({ "status": "generating" })))
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

// ===== Facilitators =====

#[derive(Deserialize)]
pub struct FacilitatorApplyRequest {
    pub name: String,
    pub bio: String,
    /// JSON array of strings: ["grief", "trauma", "relationships", "meditation", "breathwork", "psychedelics", "somatic", "shadow work"]
    pub specialties: Vec<String>,
    #[serde(default)]
    pub approach: Option<String>,
    pub session_rate_usd: f64,
    #[serde(default)]
    pub booking_url: Option<String>,
    #[serde(default)]
    pub contact_method: Option<String>,
    #[serde(default)]
    pub profile_image_url: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
}

fn default_languages() -> Vec<String> {
    vec!["en".into()]
}

#[derive(Serialize)]
pub struct FacilitatorItem {
    pub id: String,
    pub name: String,
    pub bio: String,
    pub specialties: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approach: Option<String>,
    pub session_rate_usd: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub booking_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub languages: Vec<String>,
    pub status: String,
    pub avg_rating: f64,
    pub total_reviews: i32,
    pub total_sessions: i32,
}

fn record_to_item(r: queries::FacilitatorRecord) -> FacilitatorItem {
    FacilitatorItem {
        id: r.id,
        name: r.name,
        bio: r.bio,
        specialties: serde_json::from_str(&r.specialties).unwrap_or_default(),
        approach: r.approach,
        session_rate_usd: r.session_rate_usd,
        booking_url: r.booking_url,
        contact_method: r.contact_method,
        profile_image_url: r.profile_image_url,
        location: r.location,
        languages: serde_json::from_str(&r.languages).unwrap_or_default(),
        status: r.status,
        avg_rating: r.avg_rating,
        total_reviews: r.total_reviews,
        total_sessions: r.total_sessions,
    }
}

/// POST /swift/v1/facilitators/apply
pub async fn facilitator_apply(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<FacilitatorApplyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    if req.name.trim().is_empty() || req.bio.trim().is_empty() {
        return Err(AppError::BadRequest("name and bio are required".into()));
    }
    if req.specialties.is_empty() {
        return Err(AppError::BadRequest(
            "at least one specialty is required".into(),
        ));
    }
    if req.session_rate_usd <= 0.0 {
        return Err(AppError::BadRequest(
            "session_rate_usd must be positive".into(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let specialties_json = serde_json::to_string(&req.specialties).unwrap_or("[]".into());
    let languages_json = serde_json::to_string(&req.languages).unwrap_or("[\"en\"]".into());

    {
        let db = state.db.lock().await;
        queries::ensure_user(&db, &user_id)?;
        queries::insert_facilitator(
            &db,
            &id,
            &user_id,
            req.name.trim(),
            req.bio.trim(),
            &specialties_json,
            req.approach.as_deref(),
            req.session_rate_usd,
            req.booking_url.as_deref(),
            req.contact_method.as_deref(),
            req.profile_image_url.as_deref(),
            req.location.as_deref(),
            &languages_json,
        )?;
    }

    state.emit_log(
        "INFO",
        "facilitator",
        &format!("New application: {} ({})", req.name, &user_id[..8]),
    );

    Ok(Json(serde_json::json!({
        "ok": true,
        "id": id,
        "status": "pending",
        "message": "your application is under review. we'll be in touch."
    })))
}

/// GET /swift/v1/facilitators — list all approved facilitators
pub async fn list_facilitators(
    State(state): State<AppState>,
) -> Result<Json<Vec<FacilitatorItem>>, AppError> {
    let db = state.db.lock().await;
    let facilitators = queries::get_approved_facilitators(&db)?;
    Ok(Json(facilitators.into_iter().map(record_to_item).collect()))
}

/// GET /swift/v1/facilitators/:id — get profile + reviews
pub async fn get_facilitator(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.lock().await;
    let fac = queries::get_facilitator(&db, &id)?
        .ok_or_else(|| AppError::NotFound("facilitator not found".into()))?;

    if fac.status != "approved" {
        return Err(AppError::NotFound("facilitator not found".into()));
    }

    let reviews = queries::get_facilitator_reviews(&db, &id)?;
    let review_items: Vec<_> = reviews
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "rating": r.rating,
                "review_text": r.review_text,
                "created_at": r.created_at,
            })
        })
        .collect();

    let item = record_to_item(fac);
    let mut json = serde_json::to_value(&item).unwrap_or_default();
    json.as_object_mut()
        .unwrap()
        .insert("reviews".into(), review_items.into());

    Ok(Json(json))
}

#[derive(Deserialize)]
pub struct ReviewRequest {
    pub rating: i32,
    #[serde(default)]
    pub review_text: Option<String>,
}

/// POST /swift/v1/facilitators/:id/review
pub async fn review_facilitator(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ReviewRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    if req.rating < 1 || req.rating > 5 {
        return Err(AppError::BadRequest("rating must be 1-5".into()));
    }

    let review_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().await;
        // Verify facilitator exists and is approved
        let fac = queries::get_facilitator(&db, &id)?
            .ok_or_else(|| AppError::NotFound("facilitator not found".into()))?;
        if fac.status != "approved" {
            return Err(AppError::NotFound("facilitator not found".into()));
        }
        queries::insert_facilitator_review(
            &db,
            &review_id,
            &id,
            &user_id,
            req.rating,
            req.review_text.as_deref(),
        )?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /swift/v1/facilitators/recommended
/// AI-powered: reads the user's writing profile and recommends facilitators
/// whose specialties match their patterns.
pub async fn recommended_facilitators(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let (profile_summary, facilitators) = {
        let db = state.db.lock().await;
        let profile = queries::get_user_profile_summary(&db, &user_id)?;
        let facs = queries::get_approved_facilitators(&db)?;
        (profile, facs)
    };

    if facilitators.is_empty() {
        return Ok(Json(serde_json::json!({
            "facilitators": [],
            "message": "no facilitators available yet. check back soon."
        })));
    }

    let profile_text = match profile_summary {
        Some(p) if !p.trim().is_empty() => p,
        _ => {
            // No profile yet — return all facilitators without ranking
            let items: Vec<_> = facilitators.into_iter().map(record_to_item).collect();
            return Ok(Json(serde_json::json!({
                "facilitators": items,
                "message": "write more so Anky can learn your patterns and recommend the right facilitator for you."
            })));
        }
    };

    // Build a summary of facilitators for Claude
    let fac_summaries: Vec<String> = facilitators
        .iter()
        .map(|f| {
            format!(
                "ID: {} | Name: {} | Specialties: {} | Approach: {} | Rate: ${}/session",
                f.id,
                f.name,
                f.specialties,
                f.approach.as_deref().unwrap_or("not specified"),
                f.session_rate_usd,
            )
        })
        .collect();

    let prompt = format!(
        r#"Based on this person's psychological profile from their writing practice:

{}

And these available facilitators:

{}

Return a JSON array of the top 3 most relevant facilitators, ranked by fit.
For each, include the facilitator ID and a 1-sentence explanation of WHY they'd be a good match.

JSON format:
[
  {{ "id": "...", "reason": "They specialize in X, which connects to the pattern of Y in your writing." }}
]

Only valid JSON, no markdown."#,
        profile_text,
        fac_summaries.join("\n"),
    );

    let result = claude::call_claude_public(
        &state.config.anthropic_api_key,
        "claude-haiku-4-5-20251001",
        "You are Anky, a spiritual guide matching people with human facilitators. Be warm, specific, and honest. Output only valid JSON.",
        &prompt,
        500,
    )
    .await;

    match result {
        Ok(r) => {
            let clean = r
                .text
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            let ranked: Vec<serde_json::Value> = serde_json::from_str(clean).unwrap_or_default();

            // Enrich with full facilitator data
            let mut recommended = Vec::new();
            for rank in &ranked {
                let fac_id = rank.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let reason = rank.get("reason").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(fac) = facilitators.iter().find(|f| f.id == fac_id) {
                    let mut item =
                        serde_json::to_value(record_to_item(queries::FacilitatorRecord {
                            id: fac.id.clone(),
                            user_id: fac.user_id.clone(),
                            name: fac.name.clone(),
                            bio: fac.bio.clone(),
                            specialties: fac.specialties.clone(),
                            approach: fac.approach.clone(),
                            session_rate_usd: fac.session_rate_usd,
                            booking_url: fac.booking_url.clone(),
                            contact_method: fac.contact_method.clone(),
                            profile_image_url: fac.profile_image_url.clone(),
                            location: fac.location.clone(),
                            languages: fac.languages.clone(),
                            status: fac.status.clone(),
                            avg_rating: fac.avg_rating,
                            total_reviews: fac.total_reviews,
                            total_sessions: fac.total_sessions,
                            created_at: fac.created_at.clone(),
                        }))
                        .unwrap_or_default();
                    item.as_object_mut()
                        .unwrap()
                        .insert("match_reason".into(), reason.into());
                    recommended.push(item);
                }
            }

            Ok(Json(serde_json::json!({ "facilitators": recommended })))
        }
        Err(_) => {
            // Fallback: return all without ranking
            let items: Vec<_> = facilitators.into_iter().map(record_to_item).collect();
            Ok(Json(serde_json::json!({ "facilitators": items })))
        }
    }
}

/// POST /swift/v1/admin/facilitator/approve — approve or suspend
pub async fn admin_facilitator(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _caller = bearer_auth(&state, &headers).await?;

    let facilitator_id = body
        .get("facilitator_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("facilitator_id required".into()))?;
    let action = body
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("approve");

    {
        let db = state.db.lock().await;
        match action {
            "approve" => queries::approve_facilitator(&db, facilitator_id)?,
            "suspend" => queries::suspend_facilitator(&db, facilitator_id)?,
            _ => {
                return Err(AppError::BadRequest(
                    "action must be 'approve' or 'suspend'".into(),
                ))
            }
        }
    }

    state.emit_log(
        "INFO",
        "facilitator",
        &format!("Facilitator {} → {}", &facilitator_id[..8], action),
    );

    Ok(Json(serde_json::json!({ "ok": true, "action": action })))
}

/// POST /swift/v1/facilitators/:id/book
/// Book a session. Payment via USDC tx hash or Stripe (placeholder).
/// 8% platform fee. Optionally share Anky profile context.
#[derive(Deserialize)]
pub struct BookingRequest {
    #[serde(default)]
    pub payment_tx_hash: Option<String>,
    #[serde(default)]
    pub stripe_payment_id: Option<String>,
    /// If true, generate and share an anonymized summary of the user's writing patterns
    #[serde(default)]
    pub share_context: bool,
}

pub async fn book_facilitator(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<BookingRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let fac = {
        let db = state.db.lock().await;
        queries::get_facilitator(&db, &id)?
            .ok_or_else(|| AppError::NotFound("facilitator not found".into()))?
    };

    if fac.status != "approved" {
        return Err(AppError::NotFound("facilitator not found".into()));
    }

    let payment_method = if req.payment_tx_hash.is_some() {
        "usdc"
    } else if req.stripe_payment_id.is_some() {
        "stripe"
    } else {
        return Err(AppError::BadRequest(
            "payment_tx_hash or stripe_payment_id required".into(),
        ));
    };

    let amount = fac.session_rate_usd;
    let platform_fee = (amount * 0.08 * 100.0).round() / 100.0; // 8%, rounded to cents

    // Generate context if requested
    let shared_context = if req.share_context {
        let db = state.db.lock().await;
        queries::get_user_profile_summary(&db, &user_id)?
    } else {
        None
    };

    let booking_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().await;
        queries::insert_facilitator_booking(
            &db,
            &booking_id,
            &id,
            &user_id,
            amount,
            platform_fee,
            payment_method,
            req.payment_tx_hash.as_deref(),
            req.stripe_payment_id.as_deref(),
            req.share_context,
            shared_context.as_deref(),
        )?;
    }

    state.emit_log(
        "INFO",
        "booking",
        &format!(
            "Booking: {} → {} (${}, fee ${})",
            &user_id[..8],
            &fac.name,
            amount,
            platform_fee,
        ),
    );

    Ok(Json(serde_json::json!({
        "ok": true,
        "booking_id": booking_id,
        "facilitator_name": fac.name,
        "amount_usd": amount,
        "platform_fee_usd": platform_fee,
        "facilitator_receives_usd": amount - platform_fee,
        "booking_url": fac.booking_url,
        "contact_method": fac.contact_method,
    })))
}

/// GET /swift/v1/breathwork/history
pub async fn breathwork_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;
    let db = state.db.lock().await;
    let history = queries::get_user_breathwork_history(&db, &user_id)?;
    let items: Vec<_> = history
        .into_iter()
        .map(|(id, session_id, style, completed_at)| {
            serde_json::json!({
                "id": id,
                "session_id": session_id,
                "style": style,
                "completed_at": completed_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "history": items })))
}
