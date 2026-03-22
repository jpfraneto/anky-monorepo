/// Shared social reply context — Honcho peer lookup + interaction history.
/// Used by both webhook_x.rs and webhook_farcaster.rs to give Anky memory.
use crate::services::honcho;
use crate::state::AppState;

/// Everything Anky needs to know about someone before replying.
pub struct SocialContext {
    /// Honcho peer context — accumulated understanding from their writings
    pub peer_context: Option<String>,
    /// Past interactions: (their_text, anky_reply) most recent first
    pub interaction_history: Vec<(String, String)>,
}

/// Look up or create a social peer, fetch their Honcho context and interaction history.
/// Never blocks — returns empty context on any failure.
pub async fn fetch_social_context(
    state: &AppState,
    platform: &str,
    platform_user_id: &str,
    platform_username: &str,
) -> SocialContext {
    // 1. Upsert social_peers record and get honcho_peer_id + user_id
    let (honcho_peer_id, _user_id) =
        upsert_social_peer(state, platform, platform_user_id, platform_username).await;

    // 2. Fetch Honcho peer context if we have a peer ID
    let peer_context = if let Some(ref peer_id) = honcho_peer_id {
        if honcho::is_configured(&state.config) {
            match honcho::get_peer_context(
                &state.config.honcho_api_key,
                &state.config.honcho_workspace_id,
                peer_id,
            )
            .await
            {
                Ok(ctx) => ctx,
                Err(e) => {
                    tracing::debug!("Honcho peer context lookup failed for {}: {}", peer_id, e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        // Try looking up by the platform-specific ID as a fallback peer ID
        if honcho::is_configured(&state.config) {
            let fallback_id = format!("{}_{}", platform, platform_user_id);
            honcho::get_peer_context(
                &state.config.honcho_api_key,
                &state.config.honcho_workspace_id,
                &fallback_id,
            )
            .await
            .ok()
            .flatten()
        } else {
            None
        }
    };

    // 3. Fetch past interaction history from both tables
    let interaction_history = fetch_interaction_history(state, platform, platform_user_id).await;

    SocialContext {
        peer_context,
        interaction_history,
    }
}

/// Upsert a social_peers record. Returns (honcho_peer_id, user_id) if available.
async fn upsert_social_peer(
    state: &AppState,
    platform: &str,
    platform_user_id: &str,
    platform_username: &str,
) -> (Option<String>, Option<String>) {
    let db = state.db.lock().await;

    // Try to find existing
    let existing: Option<(Option<String>, Option<String>)> = db
        .query_row(
            "SELECT honcho_peer_id, user_id FROM social_peers WHERE platform = ?1 AND platform_user_id = ?2",
            rusqlite::params![platform, platform_user_id],
            |row| Ok((row.get(0).ok(), row.get(1).ok())),
        )
        .ok();

    if let Some((peer_id, user_id)) = existing {
        // Update last_seen and increment count
        let _ = db.execute(
            "UPDATE social_peers SET last_seen_at = datetime('now'), interaction_count = interaction_count + 1, platform_username = COALESCE(?3, platform_username) WHERE platform = ?1 AND platform_user_id = ?2",
            rusqlite::params![platform, platform_user_id, if platform_username.is_empty() { None } else { Some(platform_username) }],
        );
        return (peer_id, user_id);
    }

    // Create new peer — social platforms are not connected to the app,
    // so peer ID is always derived from platform + user_id
    let id = uuid::Uuid::new_v4().to_string();
    let honcho_peer_id = format!("{}_{}", platform, platform_user_id);

    let _ = db.execute(
        "INSERT OR IGNORE INTO social_peers (id, platform, platform_user_id, platform_username, honcho_peer_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            &id,
            platform,
            platform_user_id,
            if platform_username.is_empty() { None } else { Some(platform_username) },
            &honcho_peer_id,
        ],
    );

    (Some(honcho_peer_id), None)
}

/// Fetch past interactions with a specific user across platform-specific tables.
/// Returns (their_text, anky_reply) pairs, most recent first, up to 10.
async fn fetch_interaction_history(
    state: &AppState,
    platform: &str,
    platform_user_id: &str,
) -> Vec<(String, String)> {
    let db = state.db.lock().await;

    match platform {
        "x" => {
            // Query x_interactions table
            let mut stmt = match db.prepare(
                "SELECT tweet_text, result_text FROM x_interactions
                 WHERE x_user_id = ?1 AND result_text IS NOT NULL AND result_text != ''
                 ORDER BY created_at DESC LIMIT 10",
            ) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            stmt.query_map(rusqlite::params![platform_user_id], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, String>(1).unwrap_or_default(),
                ))
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        }
        "farcaster" => {
            // Query social_interactions table
            let mut stmt = match db.prepare(
                "SELECT post_text, reply_text FROM social_interactions
                 WHERE platform = 'farcaster' AND author_id = ?1 AND reply_text IS NOT NULL AND reply_text != ''
                 ORDER BY created_at DESC LIMIT 10",
            ) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            stmt.query_map(rusqlite::params![platform_user_id], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, String>(1).unwrap_or_default(),
                ))
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}
