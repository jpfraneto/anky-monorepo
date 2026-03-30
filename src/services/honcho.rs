/// Honcho user modeling API client.
///
/// Honcho builds persistent, evolving representations of users through
/// background reasoning over their messages. We send every writing to Honcho
/// and query its peer model before generation so artifacts become personally shaped.
///
/// API base: https://api.honcho.dev/v3/
/// All calls use a 10-second timeout. Failures are logged, never block user responses.
use crate::config::Config;
use crate::state::AppState;
use anyhow::{anyhow, Result};

/// Returns true when HONCHO_API_KEY is set and non-empty.
pub fn is_configured(config: &Config) -> bool {
    !config.honcho_api_key.is_empty()
}

/// Strip chars not matching `[a-zA-Z0-9_-]` so the id is safe for Honcho peer/session IDs.
fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?)
}

fn base_url() -> String {
    std::env::var("HONCHO_BASE_URL").unwrap_or_else(|_| "https://api.honcho.dev/v3".into())
}

/// Ensure a peer exists (idempotent create-or-get).
async fn ensure_peer(api_key: &str, workspace_id: &str, user_id: &str) -> Result<()> {
    let peer_id = sanitize_id(user_id);
    let url = format!("{}/workspaces/{}/peers", base_url(), workspace_id);
    let resp = client()?
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({ "id": peer_id }))
        .send()
        .await?;
    let status = resp.status();
    // 200/201 = created, 409/422 = already exists — both fine
    if status.is_success() || status.as_u16() == 409 || status.as_u16() == 422 {
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!(
            "ensure_peer {} failed ({}): {}",
            peer_id,
            status,
            body
        ))
    }
}

/// Ensure a session exists (idempotent).
async fn ensure_session(
    api_key: &str,
    workspace_id: &str,
    session_id: &str,
    peer_id: &str,
) -> Result<()> {
    let sid = sanitize_id(session_id);
    let pid = sanitize_id(peer_id);
    let url = format!("{}/workspaces/{}/sessions", base_url(), workspace_id);
    let resp = client()?
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "id": sid,
            "peers": { pid: {} }
        }))
        .send()
        .await?;
    let status = resp.status();
    if status.is_success() || status.as_u16() == 409 || status.as_u16() == 422 {
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!(
            "ensure_session {} failed ({}): {}",
            sid,
            status,
            body
        ))
    }
}

/// Send a writing to Honcho as a message in a session.
/// Ensures peer + session exist first, then posts the message.
/// Truncates to 25000 chars.
pub async fn send_writing(
    api_key: &str,
    workspace_id: &str,
    session_id: &str,
    peer_id: &str,
    text: &str,
) -> Result<()> {
    let pid = sanitize_id(peer_id);
    let sid = sanitize_id(session_id);

    ensure_peer(api_key, workspace_id, peer_id).await?;
    ensure_session(api_key, workspace_id, &sid, &pid).await?;

    let truncated: String = text.chars().take(25000).collect();
    let url = format!(
        "{}/workspaces/{}/sessions/{}/messages",
        base_url(),
        workspace_id,
        sid
    );
    let resp = client()?
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "messages": [{
                "content": truncated,
                "peer_id": pid
            }]
        }))
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!("send_writing failed ({}): {}", status, body))
    }
}

/// Get Honcho's peer context — its accumulated understanding of this user.
/// Returns None if no context is available yet.
pub async fn get_peer_context(
    api_key: &str,
    workspace_id: &str,
    user_id: &str,
) -> Result<Option<String>> {
    let pid = sanitize_id(user_id);
    let url = format!(
        "{}/workspaces/{}/peers/{}/context",
        base_url(),
        workspace_id,
        pid
    );
    let resp = client()?
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.trim().is_empty() || body.trim() == "null" || body.trim() == "{}" {
            Ok(None)
        } else {
            // Try to extract a representation string from the JSON response
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                // Honcho may return { "context": "..." } or { "representation": "..." }
                let ctx = v["context"]
                    .as_str()
                    .or_else(|| v["representation"].as_str())
                    .map(|s| s.to_string());
                if ctx.as_ref().map_or(true, |s| s.is_empty()) {
                    // Fall back to the full body as context
                    Ok(Some(body))
                } else {
                    Ok(ctx)
                }
            } else {
                Ok(Some(body))
            }
        }
    } else if resp.status().as_u16() == 404 {
        Ok(None)
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!("get_peer_context failed ({}): {}", status, body))
    }
}

/// Chat with Honcho about a specific peer — ask structured questions.
/// Used for profile generation (every 5th session).
pub async fn chat_about_peer(
    api_key: &str,
    workspace_id: &str,
    user_id: &str,
    query: &str,
) -> Result<String> {
    let pid = sanitize_id(user_id);
    let url = format!(
        "{}/workspaces/{}/peers/{}/chat",
        base_url(),
        workspace_id,
        pid
    );
    let resp = client()?
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "query": query,
            "reasoning_level": "low"
        }))
        .send()
        .await?;

    if resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        // Response may be { "response": "..." } or plain text
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            Ok(v["response"]
                .as_str()
                .or_else(|| v["content"].as_str())
                .unwrap_or(&body)
                .to_string())
        } else {
            Ok(body)
        }
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!("chat_about_peer failed ({}): {}", status, body))
    }
}

/// Backfill all historical writing sessions to Honcho.
/// Queries every writing_session with content, groups by user, and sends each
/// writing chronologically so Honcho builds the full user model from scratch.
/// Skips sessions that have no content. Logs progress every 50 sessions.
/// Runs once at startup if Honcho is configured.
pub async fn backfill_all_writings(state: &AppState) {
    if !is_configured(&state.config) {
        return;
    }

    let api_key = state.config.honcho_api_key.clone();
    let workspace_id = state.config.honcho_workspace_id.clone();

    // Pull all writing sessions ordered by created_at ASC so Honcho sees them chronologically
    let sessions: Vec<(String, String, String)> = {
        let Some(db) = crate::db::get_conn_logged(&state.db) else {
            return;
        };
        let mut stmt = match db.prepare(
            "SELECT id, user_id, content
             FROM writing_sessions
             WHERE content IS NOT NULL AND content != ''
             ORDER BY created_at ASC",
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Honcho backfill query failed: {}", e);
                return;
            }
        };
        let rows = stmt
            .query_map(crate::params![], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap_or_else(|_| panic!("honcho backfill query failed"));
        rows.filter_map(|r| r.ok()).collect()
    };

    let total = sessions.len();
    if total == 0 {
        tracing::info!("Honcho backfill: no writing sessions to send");
        return;
    }

    tracing::info!(
        "Honcho backfill: sending {} historical writing sessions",
        total
    );
    state.emit_log(
        "INFO",
        "honcho",
        &format!("Starting Honcho backfill of {} writing sessions", total),
    );

    let mut sent = 0usize;
    let mut failed = 0usize;

    for (session_id, user_id, content) in &sessions {
        match send_writing(&api_key, &workspace_id, session_id, user_id, content).await {
            Ok(()) => {
                sent += 1;
            }
            Err(e) => {
                failed += 1;
                tracing::warn!(
                    "Honcho backfill failed for session {}: {}",
                    &session_id[..8.min(session_id.len())],
                    e
                );
            }
        }

        if (sent + failed) % 50 == 0 {
            tracing::info!(
                "Honcho backfill progress: {}/{} sent, {} failed",
                sent,
                total,
                failed
            );
            state.emit_log(
                "INFO",
                "honcho",
                &format!(
                    "Backfill progress: {}/{} sent, {} failed",
                    sent, total, failed
                ),
            );
        }

        // Small delay to avoid hammering the API
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    tracing::info!(
        "Honcho backfill complete: {}/{} sent, {} failed",
        sent,
        total,
        failed
    );
    state.emit_log(
        "INFO",
        "honcho",
        &format!(
            "Backfill complete: {}/{} sent, {} failed",
            sent, total, failed
        ),
    );
}
