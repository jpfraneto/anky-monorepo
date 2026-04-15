/// Sealed writing sessions — encrypted envelopes from the iOS app.
///
/// The backend stores ciphertext + encrypted keys as opaque blobs.
/// It NEVER decrypts any sealed session data. The backend is blind.
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::swift::bearer_auth;

// ===== Request / Response types =====

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealSessionRequest {
    pub session_id: String,
    pub ciphertext: String,
    pub nonce: String,
    pub tag: String,
    pub user_encrypted_key: String,
    pub anky_encrypted_key: String,
    pub session_hash: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SealSessionResponse {
    pub sealed: bool,
    pub session_hash: String,
    pub stored_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedSessionEntry {
    pub session_id: String,
    pub ciphertext: String,
    pub nonce: String,
    pub tag: String,
    pub user_encrypted_key: String,
    pub session_hash: String,
    pub sealed_at: i64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct SealedSessionsListResponse {
    pub sessions: Vec<SealedSessionEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResponse {
    pub exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sealed_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ciphertext_size_bytes: Option<i64>,
}

// ===== Handlers =====

/// POST /api/sessions/seal — store an encrypted writing session envelope.
pub async fn seal_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SealSessionRequest>,
) -> Result<Json<SealSessionResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let b64 = base64::engine::general_purpose::STANDARD;

    // Decode all base64 fields
    let ciphertext_bytes = b64
        .decode(&req.ciphertext)
        .map_err(|e| AppError::BadRequest(format!("invalid base64 ciphertext: {}", e)))?;
    let nonce_bytes = b64
        .decode(&req.nonce)
        .map_err(|e| AppError::BadRequest(format!("invalid base64 nonce: {}", e)))?;
    let tag_bytes = b64
        .decode(&req.tag)
        .map_err(|e| AppError::BadRequest(format!("invalid base64 tag: {}", e)))?;
    let user_key_bytes = b64
        .decode(&req.user_encrypted_key)
        .map_err(|e| AppError::BadRequest(format!("invalid base64 userEncryptedKey: {}", e)))?;
    let anky_key_bytes = b64
        .decode(&req.anky_encrypted_key)
        .map_err(|e| AppError::BadRequest(format!("invalid base64 ankyEncryptedKey: {}", e)))?;

    // Verify the hash: sha256(ciphertext_bytes) must equal session_hash
    let computed_hash = hex::encode(Sha256::digest(&ciphertext_bytes));
    if computed_hash != req.session_hash {
        return Err(AppError::BadRequest(
            "ciphertext hash mismatch: sha256(ciphertext) does not match sessionHash".into(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    let metadata_json = req.metadata.as_ref().map(|m| m.to_string());

    // Store in database
    {
        let db = crate::db::conn(&state.db)?;
        db.execute(
            "INSERT INTO sealed_sessions (id, user_id, session_id, ciphertext, nonce, tag, user_encrypted_key, anky_encrypted_key, session_hash, metadata_json, sealed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            crate::params![
                id,
                user_id,
                req.session_id,
                ciphertext_bytes,
                nonce_bytes,
                tag_bytes,
                user_key_bytes,
                anky_key_bytes,
                req.session_hash,
                metadata_json,
                now
            ],
        )
        .map_err(|e| AppError::Internal(format!("DB insert sealed_session: {}", e)))?;
    }

    // Store on disk for future Arweave upload
    let dir = format!("data/sealed/{}", user_id);
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Internal(format!("create sealed dir: {}", e)))?;
    let path = format!("{}/{}.sealed", dir, req.session_hash);
    let envelope = serde_json::json!({
        "sessionId": req.session_id,
        "ciphertext": req.ciphertext,
        "nonce": req.nonce,
        "tag": req.tag,
        "userEncryptedKey": req.user_encrypted_key,
        "ankyEncryptedKey": req.anky_encrypted_key,
        "sessionHash": req.session_hash,
        "metadata": req.metadata,
        "sealedAt": now,
    });
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&envelope).unwrap_or_default(),
    )
    .map_err(|e| AppError::Internal(format!("write sealed file: {}", e)))?;

    tracing::info!(
        user = %user_id,
        session = %req.session_id,
        hash = %req.session_hash,
        size = ciphertext_bytes.len(),
        "Sealed session stored"
    );

    Ok(Json(SealSessionResponse {
        sealed: true,
        session_hash: req.session_hash,
        stored_at: now,
    }))
}

/// GET /swift/v2/sealed-sessions — list the authenticated user's sealed sessions.
/// Does NOT return ankyEncryptedKey — that field is for the enclave only.
pub async fn list_sealed_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SealedSessionsListResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let db = crate::db::conn(&state.db)?;
    let mut stmt = db
        .prepare(
            "SELECT session_id, ciphertext, nonce, tag, user_encrypted_key, session_hash, sealed_at, metadata_json
             FROM sealed_sessions
             WHERE user_id = ?1
             ORDER BY sealed_at DESC",
        )
        .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;

    let rows = stmt
        .query_map(crate::params![user_id], |row| {
            let ciphertext_bytes: Vec<u8> = row.get(1)?;
            let nonce_bytes: Vec<u8> = row.get(2)?;
            let tag_bytes: Vec<u8> = row.get(3)?;
            let user_key_bytes: Vec<u8> = row.get(4)?;
            let metadata_str: Option<String> = row.get(7)?;

            let b64 = base64::engine::general_purpose::STANDARD;
            Ok(SealedSessionEntry {
                session_id: row.get(0)?,
                ciphertext: b64.encode(&ciphertext_bytes),
                nonce: b64.encode(&nonce_bytes),
                tag: b64.encode(&tag_bytes),
                user_encrypted_key: b64.encode(&user_key_bytes),
                session_hash: row.get(5)?,
                sealed_at: row.get(6)?,
                metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            })
        })
        .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;

    let sessions: Vec<SealedSessionEntry> = rows.filter_map(|r| r.ok()).collect();

    Ok(Json(SealedSessionsListResponse { sessions }))
}

/// GET /api/verify/{session_hash} — public verification endpoint.
/// Returns existence + size only. Never exposes ciphertext, keys, or user info.
pub async fn verify_sealed_session(
    State(state): State<AppState>,
    Path(session_hash): Path<String>,
) -> Result<Json<VerifyResponse>, AppError> {
    let db = crate::db::conn(&state.db)?;

    let result: Result<(i64, Vec<u8>), _> = db.query_row(
        "SELECT sealed_at, ciphertext FROM sealed_sessions WHERE session_hash = ?1 LIMIT 1",
        crate::params![session_hash],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok((sealed_at, ciphertext_bytes)) => Ok(Json(VerifyResponse {
            exists: true,
            sealed_at: Some(sealed_at),
            ciphertext_size_bytes: Some(ciphertext_bytes.len() as i64),
        })),
        Err(_) => Ok(Json(VerifyResponse {
            exists: false,
            sealed_at: None,
            ciphertext_size_bytes: None,
        })),
    }
}

// ===== Query helpers for other modules =====

/// Look up the session_hash for a sealed session by session_id.
/// Used by mirror metadata to include hash as a cNFT attribute.
pub fn get_session_hash_by_session_id(
    db: &crate::db::Connection,
    session_id: &str,
) -> Option<String> {
    db.query_row(
        "SELECT session_hash FROM sealed_sessions WHERE session_id = ?1 LIMIT 1",
        crate::params![session_id],
        |row| row.get(0),
    )
    .ok()
}

/// POST /api/sessions/seal-browser — store an encrypted session from the browser.
/// Uses cookie auth instead of bearer token. Simpler payload (no separate key wrapping).
/// The ciphertext is encrypted directly to the enclave's public key.
pub async fn seal_session_browser(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Json(req): Json<BrowserSealRequest>,
) -> Result<Json<SealSessionResponse>, AppError> {
    let user_id = crate::routes::auth::visitor_id_from_jar(&jar)
        .unwrap_or_else(|| "anonymous-browser".into());

    let b64 = base64::engine::general_purpose::STANDARD;

    let ciphertext_bytes = b64
        .decode(&req.sealed.ciphertext)
        .map_err(|e| AppError::BadRequest(format!("invalid ciphertext: {}", e)))?;
    let nonce_bytes = b64
        .decode(&req.sealed.nonce)
        .map_err(|e| AppError::BadRequest(format!("invalid nonce: {}", e)))?;
    let tag_bytes = b64
        .decode(&req.sealed.tag)
        .map_err(|e| AppError::BadRequest(format!("invalid tag: {}", e)))?;
    let ephemeral_pk = b64
        .decode(&req.sealed.ephemeral_public_key)
        .map_err(|e| AppError::BadRequest(format!("invalid ephemeral key: {}", e)))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    // Store in sealed_sessions table — backend is blind to content
    {
        let db = crate::db::conn(&state.db)?;
        db.execute(
            "INSERT INTO sealed_sessions (id, user_id, session_id, ciphertext, nonce, tag, user_encrypted_key, anky_encrypted_key, session_hash, metadata_json, sealed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            crate::params![
                id,
                user_id,
                req.session_id,
                ciphertext_bytes,
                nonce_bytes,
                tag_bytes,
                ephemeral_pk,  // store ephemeral PK in user_encrypted_key field
                ephemeral_pk,  // same — browser flow doesn't separate keys
                req.sealed.session_hash,
                serde_json::json!({"source": "browser", "duration": req.duration, "word_count": req.word_count}).to_string(),
                now
            ],
        )
        .map_err(|e| AppError::Internal(format!("DB insert browser sealed session: {}", e)))?;
    }

    // Store on disk for future Arweave batch upload
    let dir = format!("data/sealed/{}", user_id);
    std::fs::create_dir_all(&dir).ok();
    let path = format!("{}/{}.sealed", dir, req.sealed.session_hash);
    let envelope = serde_json::json!({
        "sessionId": req.session_id,
        "ephemeralPublicKey": req.sealed.ephemeral_public_key,
        "ciphertext": req.sealed.ciphertext,
        "nonce": req.sealed.nonce,
        "tag": req.sealed.tag,
        "sessionHash": req.sealed.session_hash,
        "source": "browser",
        "sealedAt": now,
    });
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&envelope).unwrap_or_default(),
    )
    .ok();

    tracing::info!(
        user = %user_id,
        session = %req.session_id,
        hash = %req.sealed.session_hash,
        size = ciphertext_bytes.len(),
        "Browser sealed session stored"
    );

    Ok(Json(SealSessionResponse {
        sealed: true,
        session_hash: req.sealed.session_hash,
        stored_at: now,
    }))
}

#[derive(Debug, Deserialize)]
pub struct BrowserSealRequest {
    pub session_id: String,
    pub sealed: BrowserSealedData,
    pub duration: Option<f64>,
    pub word_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct BrowserSealedData {
    pub session_hash: String,
    pub ephemeral_public_key: String,
    pub nonce: String,
    pub tag: String,
    pub ciphertext: String,
}

/// GET /api/anky/public-key — returns the enclave's encryption public key.
/// The iOS app uses this to encrypt session data to the enclave.
pub async fn get_enclave_public_key(State(state): State<AppState>) -> axum::response::Response {
    use axum::response::IntoResponse;

    if state.config.enclave_url.is_empty() {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "enclave not configured"})),
        )
            .into_response();
    }

    match reqwest::get(format!("{}/public-key", state.config.enclave_url)).await {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(data) => Json(data).into_response(),
            Err(e) => (
                axum::http::StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("enclave response parse error: {}", e)})),
            )
                .into_response(),
        },
        Ok(resp) => (
            axum::http::StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": format!("enclave returned {}", resp.status())})),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": format!("enclave unreachable: {}", e)})),
        )
            .into_response(),
    }
}

/// Look up the session_hash for a sealed session by user_id (most recent).
/// Fallback for mirrors that don't have a direct session_id link.
pub fn get_latest_session_hash_by_user(
    db: &crate::db::Connection,
    user_id: &str,
) -> Option<String> {
    db.query_row(
        "SELECT session_hash FROM sealed_sessions WHERE user_id = ?1 ORDER BY sealed_at DESC LIMIT 1",
        crate::params![user_id],
        |row| row.get(0),
    )
    .ok()
}

// ===== Sealed Write: the unified privacy-preserving write path =====
//
// Flow:
// 1. iOS encrypts writing on-device with enclave's X25519 public key
// 2. iOS computes session_hash = SHA256(plaintext) locally
// 3. iOS sends sealed envelope + session_hash + duration + word_count
// 4. Backend stores the sealed envelope (blind — never sees plaintext)
// 5. Backend logs session_hash on-chain via spl-memo
// 6. Backend relays the sealed envelope to the enclave
// 7. Enclave decrypts, calls OpenRouter for reflection + image prompt
// 8. Enclave returns {reflection, image_prompt, title} — NO plaintext
// 9. Backend creates anky record with enclave outputs
// 10. Backend generates image from the enclave's prompt
//
// The backend NEVER sees the writing. The session_hash on-chain was computed
// by the iOS app. The enclave is the only entity that sees plaintext.

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedWriteRequest {
    pub session_id: String,
    pub ciphertext: String,           // base64 — encrypted writing
    pub nonce: String,                // base64
    pub tag: String,                  // base64
    pub ephemeral_public_key: String, // base64 — ephemeral X25519 pubkey
    pub session_hash: String,         // SHA256(plaintext), computed by iOS
    pub duration: f64,                // seconds
    pub word_count: i32,
    #[serde(default)]
    pub user_encrypted_key: Option<String>, // optional — user's iCloud key copy
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedWriteResponse {
    pub ok: bool,
    pub session_id: String,
    pub session_hash: String,
    pub anky_id: Option<String>,
    pub is_anky: bool,
    pub solana_tx: Option<String>,
}

/// Response expected from the enclave's POST /process-writing endpoint.
#[derive(Debug, Deserialize)]
pub struct EnclaveProcessResponse {
    pub reflection: String,
    pub image_prompt: String,
    pub title: String,
    pub hash_verified: bool, // enclave confirms SHA256(decrypted) matches session_hash
}

/// POST /api/sealed-write — the unified sealed write endpoint.
/// Stores the encrypted envelope, logs on-chain, and relays to enclave for processing.
pub async fn sealed_write(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SealedWriteRequest>,
) -> Result<Json<SealedWriteResponse>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let b64 = base64::engine::general_purpose::STANDARD;

    // Decode and validate base64 fields
    let ciphertext_bytes = b64
        .decode(&req.ciphertext)
        .map_err(|e| AppError::BadRequest(format!("invalid ciphertext: {}", e)))?;
    let nonce_bytes = b64
        .decode(&req.nonce)
        .map_err(|e| AppError::BadRequest(format!("invalid nonce: {}", e)))?;
    let tag_bytes = b64
        .decode(&req.tag)
        .map_err(|e| AppError::BadRequest(format!("invalid tag: {}", e)))?;
    let ephemeral_bytes = b64
        .decode(&req.ephemeral_public_key)
        .map_err(|e| AppError::BadRequest(format!("invalid ephemeral key: {}", e)))?;

    let is_anky = req.duration >= 480.0 && req.word_count >= 50;

    // Step 1: Store the sealed envelope (backend is blind)
    let sealed_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    let user_key_bytes = req
        .user_encrypted_key
        .as_ref()
        .and_then(|k| b64.decode(k).ok())
        .unwrap_or_default();
    {
        let db = crate::db::conn(&state.db)?;
        db.execute(
            "INSERT INTO sealed_sessions (id, user_id, session_id, ciphertext, nonce, tag, user_encrypted_key, anky_encrypted_key, session_hash, metadata_json, sealed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            crate::params![
                sealed_id,
                user_id,
                req.session_id,
                ciphertext_bytes,
                nonce_bytes,
                tag_bytes,
                user_key_bytes,
                ephemeral_bytes,
                req.session_hash,
                serde_json::json!({
                    "duration": req.duration,
                    "word_count": req.word_count,
                    "is_anky": is_anky,
                }).to_string(),
                now
            ],
        )
        .map_err(|e| AppError::Internal(format!("DB insert sealed: {}", e)))?;
    }

    // Store on disk
    let dir = format!("data/sealed/{}", user_id);
    std::fs::create_dir_all(&dir).ok();
    let path = format!("{}/{}.sealed", dir, req.session_hash);
    let envelope = serde_json::json!({
        "sessionId": req.session_id,
        "ciphertext": req.ciphertext,
        "nonce": req.nonce,
        "tag": req.tag,
        "ephemeralPublicKey": req.ephemeral_public_key,
        "sessionHash": req.session_hash,
        "sealedAt": now,
    });
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&envelope).unwrap_or_default(),
    )
    .ok();

    tracing::info!(
        user = %user_id,
        session = %req.session_id,
        hash = %req.session_hash,
        duration = req.duration,
        words = req.word_count,
        is_anky = is_anky,
        "Sealed write received"
    );

    // Step 2: Log session_hash on-chain via spl-memo (fire-and-forget)
    let solana_tx = if is_anky && !state.config.solana_mint_worker_url.is_empty() {
        let anky_id = uuid::Uuid::new_v4().to_string();

        // Create the anky record first (reflection/image filled in by enclave pipeline)
        {
            let db = crate::db::conn(&state.db)?;
            crate::db::queries::insert_anky(
                &db,
                &anky_id,
                &req.session_id,
                &user_id,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                "processing",
                "sealed",
                None,
            )?;
        }

        // Log on-chain
        let log_state = state.clone();
        let log_aid = anky_id.clone();
        let log_sid = req.session_id.clone();
        let log_uid = user_id.clone();
        let log_hash = req.session_hash.clone();
        let log_duration = req.duration as i64;
        let log_words = req.word_count;
        tokio::spawn(async move {
            if let Err(e) = crate::pipeline::image_gen::log_session_onchain(
                &log_state,
                &log_aid,
                &log_sid,
                &log_uid,
                &log_hash,
                log_duration,
                log_words,
            )
            .await
            {
                tracing::warn!("sealed session on-chain log failed: {}", e);
            }
        });

        // Step 3: Relay to enclave for processing (fire-and-forget)
        if !state.config.enclave_url.is_empty() {
            let enc_state = state.clone();
            let enc_anky_id = anky_id.clone();
            let enc_ciphertext = req.ciphertext.clone();
            let enc_nonce = req.nonce.clone();
            let enc_tag = req.tag.clone();
            let enc_ephemeral = req.ephemeral_public_key.clone();
            let enc_hash = req.session_hash.clone();
            let enc_user_id = user_id.clone();
            let enc_session_id = req.session_id.clone();
            tokio::spawn(async move {
                if let Err(e) = relay_to_enclave_and_process(
                    &enc_state,
                    &enc_anky_id,
                    &enc_session_id,
                    &enc_user_id,
                    &enc_ciphertext,
                    &enc_nonce,
                    &enc_tag,
                    &enc_ephemeral,
                    &enc_hash,
                )
                .await
                {
                    tracing::warn!(
                        "enclave processing failed for {}: {}",
                        &enc_anky_id[..8.min(enc_anky_id.len())],
                        e
                    );
                    // Mark anky as failed so retry worker can pick it up
                    if let Ok(db) = crate::db::conn(&enc_state.db) {
                        let _ = db.execute(
                            "UPDATE ankys SET status = 'enclave_failed' WHERE id = ?1",
                            crate::params![enc_anky_id],
                        );
                    }
                }
            });
        }

        // Also create writing_sessions record for consistency
        {
            let db = crate::db::conn(&state.db)?;
            crate::db::queries::upsert_completed_writing_session_with_flow(
                &db,
                &req.session_id,
                &user_id,
                "[sealed — plaintext not available to backend]",
                req.duration,
                req.word_count,
                true,
                None,
                None,
                None,
                None,
            )?;
        }

        Some(anky_id)
    } else {
        // Sub-threshold write — just store the sealed envelope, no anky
        {
            let db = crate::db::conn(&state.db)?;
            crate::db::queries::upsert_completed_writing_session_with_flow(
                &db,
                &req.session_id,
                &user_id,
                "[sealed — plaintext not available to backend]",
                req.duration,
                req.word_count,
                false,
                None,
                None,
                None,
                None,
            )?;
        }
        None
    };

    Ok(Json(SealedWriteResponse {
        ok: true,
        session_id: req.session_id,
        session_hash: req.session_hash,
        anky_id: solana_tx.clone(),
        is_anky,
        solana_tx: None, // tx signature available via polling
    }))
}

/// Relay a sealed envelope to the enclave for processing, then update the anky
/// with the enclave's outputs (reflection, image prompt, title).
async fn relay_to_enclave_and_process(
    state: &AppState,
    anky_id: &str,
    session_id: &str,
    user_id: &str,
    ciphertext: &str,
    nonce: &str,
    tag: &str,
    ephemeral_public_key: &str,
    session_hash: &str,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    // Call enclave's /process-writing endpoint
    let resp = client
        .post(format!("{}/process-writing", state.config.enclave_url))
        .json(&serde_json::json!({
            "ciphertext": ciphertext,
            "nonce": nonce,
            "tag": tag,
            "ephemeral_public_key": ephemeral_public_key,
            "session_hash": session_hash,
        }))
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        anyhow::bail!("enclave error: {}", err);
    }

    let result: EnclaveProcessResponse = resp.json().await?;

    if !result.hash_verified {
        anyhow::bail!("enclave hash verification failed — ciphertext may be tampered");
    }

    tracing::info!(
        anky = %&anky_id[..8.min(anky_id.len())],
        title = %result.title,
        "Enclave processed sealed writing"
    );

    // Update the anky record with enclave outputs
    {
        let db = crate::db::conn(&state.db)?;
        db.execute(
            "UPDATE ankys SET title = ?1, reflection = ?2, status = 'generating_image' WHERE id = ?3",
            crate::params![result.title, result.reflection, anky_id],
        )?;
    }

    // Enqueue image generation using the enclave's image prompt
    let is_pro = {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::is_user_pro(&db, user_id).unwrap_or(false)
    };
    crate::services::redis_queue::enqueue_job(
        &state.config.redis_url,
        &crate::state::GpuJob::AnkyImageFromPrompt {
            anky_id: anky_id.to_string(),
            session_id: session_id.to_string(),
            user_id: user_id.to_string(),
            image_prompt: result.image_prompt,
        },
        is_pro,
    )
    .await?;

    Ok(())
}
