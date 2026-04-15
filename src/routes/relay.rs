use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::json;

/// POST /api/v1/relay
///
/// Receives an encrypted .anky session from the client, uploads it to Irys (Arweave),
/// and anchors the session hash + arweave tx to Solana.
///
/// The client encrypts the session with the enclave's public key.
/// This endpoint never sees the plaintext writing.
///
/// Request body:
/// {
///   "encrypted": { "ephemeralPublicKey", "nonce", "tag", "ciphertext", "sessionHash" },
///   "writer_pubkey": "base58..."
/// }
///
/// Response:
/// {
///   "hash": "hex...",
///   "arweave_tx": "...",
///   "solana_tx": "...",
///   "explorer_url": "...",
///   "arweave_url": "..."
/// }
pub async fn relay_session(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let encrypted = body
        .get("encrypted")
        .ok_or_else(|| AppError::BadRequest("missing encrypted field".into()))?;
    let session_hash = encrypted
        .get("sessionHash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing sessionHash".into()))?;
    let writer_pubkey = body
        .get("writer_pubkey")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing writer_pubkey".into()))?;

    // 1. Upload encrypted payload to Irys (Arweave) via HTTP
    let arweave_tx = upload_to_irys(&state, encrypted, session_hash).await?;

    // 2. Anchor to Solana: build tx, sign as fee payer, submit
    let solana_tx = anchor_to_solana(&state, session_hash, &arweave_tx, writer_pubkey).await?;

    let explorer_url = format!(
        "https://explorer.solana.com/tx/{}?cluster=devnet",
        solana_tx
    );
    let arweave_url = format!("https://gateway.irys.xyz/{}", arweave_tx);

    tracing::info!(
        "session anchored: hash={} arweave={} solana={}",
        session_hash,
        arweave_tx,
        solana_tx
    );

    Ok(Json(json!({
        "hash": session_hash,
        "arweave_tx": arweave_tx,
        "solana_tx": solana_tx,
        "explorer_url": explorer_url,
        "arweave_url": arweave_url,
    })))
}

/// Upload encrypted JSON to Irys devnet via their HTTP upload endpoint.
/// Irys devnet is free for small uploads (<100KB).
async fn upload_to_irys(
    _state: &AppState,
    encrypted: &serde_json::Value,
    session_hash: &str,
) -> Result<String, AppError> {
    let payload = serde_json::to_string(encrypted)
        .map_err(|e| AppError::Internal(format!("json serialize: {e}")))?;

    // For devnet, Irys accepts unsigned uploads under 100KB for free
    // POST https://devnet.irys.xyz/tx with the data
    let client = reqwest::Client::new();
    let resp = client
        .post("https://devnet.irys.xyz/tx/solana")
        .header("Content-Type", "application/octet-stream")
        .header("x-tag-Content-Type", "application/vnd.anky+json")
        .header("x-tag-App-Name", "anky")
        .header("x-tag-Session-Hash", session_hash)
        .body(payload)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("irys upload: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "irys upload failed ({}): {}",
            status, body
        )));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("irys response parse: {e}")))?;

    let tx_id = resp_json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("irys response missing id".into()))?
        .to_string();

    Ok(tx_id)
}

/// Build the anchor_session instruction, sign with fee payer, and submit via Solana RPC.
/// This uses raw JSON-RPC calls — no Solana SDK dependency needed.
async fn anchor_to_solana(
    _state: &AppState,
    session_hash: &str,
    arweave_tx: &str,
    writer_pubkey: &str,
) -> Result<String, AppError> {
    // For the MVP, we delegate to the Node.js relay script which already handles
    // transaction building and signing. This avoids adding solana-sdk to the Rust deps.
    //
    // POST to a local Node.js relay process that:
    //   1. Builds the anchor_session instruction
    //   2. Signs with the fee payer keypair
    //   3. Submits to Solana
    //   4. Returns the tx signature
    //
    // The Node.js relay runs alongside the Rust server on localhost:3456
    let client = reqwest::Client::new();
    let resp = client
        .post("http://localhost:3456/relay")
        .json(&json!({
            "session_hash": session_hash,
            "arweave_tx": arweave_tx,
            "writer_pubkey": writer_pubkey,
        }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("solana relay: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "solana relay failed ({}): {}",
            status, body
        )));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("solana relay parse: {e}")))?;

    let sig = resp_json
        .get("signature")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("relay response missing signature".into()))?
        .to_string();

    Ok(sig)
}
