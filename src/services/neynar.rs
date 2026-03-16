use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct FarcasterProfile {
    pub fid: u64,
    pub username: String,
    pub display_name: Option<String>,
    pub pfp_url: Option<String>,
}

/// A cast (Farcaster post) with the fields we care about.
#[derive(Debug, Clone)]
pub struct Cast {
    pub hash: String,
    pub author_fid: u64,
    pub author_username: String,
    pub text: String,
    pub parent_hash: Option<String>,
    pub parent_url: Option<String>,
    /// First image embed URL if present.
    pub image_url: Option<String>,
}

/// Result of publishing a cast.
#[derive(Debug)]
pub struct PublishResult {
    pub hash: String,
}

// ── Lookup ──────────────────────────────────────────────────────────────────

/// Look up a Farcaster profile by wallet address via Neynar API.
pub async fn lookup_wallet(
    api_key: &str,
    wallet_address: &str,
) -> Result<Option<FarcasterProfile>> {
    if api_key.is_empty() {
        return Ok(None);
    }

    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.neynar.com/v2/farcaster/user/bulk-by-address")
        .query(&[("addresses", wallet_address)])
        .header("x-api-key", api_key)
        .header("accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::debug!(
            "Neynar lookup failed ({}): {}",
            status,
            &body[..body.len().min(200)]
        );
        return Ok(None);
    }

    let body: serde_json::Value = resp.json().await?;

    let addr_lower = wallet_address.to_lowercase();
    let users = body
        .as_object()
        .and_then(|obj| {
            obj.get(&addr_lower)
                .or_else(|| obj.get(wallet_address))
                .or_else(|| obj.values().next())
        })
        .and_then(|v| v.as_array());

    let user = match users.and_then(|arr| arr.first()) {
        Some(u) => u,
        None => return Ok(None),
    };

    let fid = user.get("fid").and_then(|f| f.as_u64()).unwrap_or(0);
    let username = user
        .get("username")
        .and_then(|u| u.as_str())
        .unwrap_or("")
        .to_string();
    let display_name = user
        .get("display_name")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());
    let pfp_url = user
        .get("pfp_url")
        .and_then(|p| p.as_str())
        .map(|s| s.to_string());

    if fid == 0 || username.is_empty() {
        return Ok(None);
    }

    Ok(Some(FarcasterProfile {
        fid,
        username,
        display_name,
        pfp_url,
    }))
}

// ── Casting ─────────────────────────────────────────────────────────────────

/// Publish a cast (text reply to another cast).
pub async fn reply_to_cast(
    api_key: &str,
    signer_uuid: &str,
    parent_hash: &str,
    text: &str,
) -> Result<PublishResult> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "signer_uuid": signer_uuid,
        "text": text,
        "parent": parent_hash,
    });

    let resp = client
        .post("https://api.neynar.com/v2/farcaster/cast")
        .header("x-api-key", api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("neynar cast POST failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "Neynar cast error {}: {}",
            status,
            &err_body[..err_body.len().min(500)]
        );
    }

    let data: serde_json::Value = resp.json().await?;
    let hash = data["cast"]["hash"].as_str().unwrap_or("").to_string();

    Ok(PublishResult { hash })
}

/// Publish a cast with an image embed (reply to another cast).
pub async fn reply_to_cast_with_image(
    api_key: &str,
    signer_uuid: &str,
    parent_hash: &str,
    text: &str,
    image_url: &str,
) -> Result<PublishResult> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "signer_uuid": signer_uuid,
        "text": text,
        "parent": parent_hash,
        "embeds": [{ "url": image_url }],
    });

    let resp = client
        .post("https://api.neynar.com/v2/farcaster/cast")
        .header("x-api-key", api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("neynar cast+image POST failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "Neynar cast+image error {}: {}",
            status,
            &err_body[..err_body.len().min(500)]
        );
    }

    let data: serde_json::Value = resp.json().await?;
    let hash = data["cast"]["hash"].as_str().unwrap_or("").to_string();

    Ok(PublishResult { hash })
}

/// React to a cast (like).
pub async fn react_to_cast(api_key: &str, signer_uuid: &str, cast_hash: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "signer_uuid": signer_uuid,
        "reaction_type": "like",
        "target": cast_hash,
    });

    let resp = client
        .post("https://api.neynar.com/v2/farcaster/reaction")
        .header("x-api-key", api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        tracing::warn!(
            "Neynar react error {}: {}",
            status,
            &err_body[..err_body.len().min(200)]
        );
    }

    Ok(())
}

// ── Fetching ────────────────────────────────────────────────────────────────

/// Fetch a single cast by hash.
pub async fn fetch_cast(api_key: &str, cast_hash: &str) -> Result<Cast> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.neynar.com/v2/farcaster/cast")
        .query(&[("identifier", cast_hash), ("type", "hash")])
        .header("x-api-key", api_key)
        .header("accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "Neynar fetch cast error {}: {}",
            status,
            &err_body[..err_body.len().min(300)]
        );
    }

    let data: serde_json::Value = resp.json().await?;
    parse_cast(&data["cast"])
}

/// Fetch recent mention/reply notifications for the bot account and return the
/// associated casts in oldest-first order so backfills replay conversations
/// naturally.
pub async fn fetch_recent_interaction_casts(
    api_key: &str,
    fid: u64,
    limit: usize,
) -> Result<Vec<Cast>> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.neynar.com/v2/farcaster/notifications")
        .query(&[("fid", fid.to_string()), ("limit", limit.to_string())])
        .header("x-api-key", api_key)
        .header("accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "Neynar notifications error {}: {}",
            status,
            &err_body[..err_body.len().min(300)]
        );
    }

    let data: serde_json::Value = resp.json().await?;
    let mut casts = Vec::new();

    if let Some(notifications) = data["notifications"].as_array() {
        for notification in notifications {
            let kind = notification["type"].as_str().unwrap_or("");
            if !matches!(kind, "mention" | "reply") {
                continue;
            }

            let cast_value = &notification["cast"];
            if cast_value.is_null() {
                continue;
            }

            if let Ok(cast) = parse_cast(cast_value) {
                casts.push(cast);
            }
        }
    }

    casts.reverse();
    Ok(casts)
}

/// Walk the reply chain up to `depth` parents.
pub async fn fetch_conversation_chain(
    api_key: &str,
    cast_hash: &str,
    depth: u8,
) -> Result<Vec<(String, String)>> {
    let mut chain = Vec::new();
    let mut current_hash = cast_hash.to_string();

    for _ in 0..depth {
        match fetch_cast(api_key, &current_hash).await {
            Ok(cast) => {
                chain.push((cast.author_username.clone(), cast.text.clone()));
                match cast.parent_hash {
                    Some(ref ph) if !ph.is_empty() => current_hash = ph.clone(),
                    _ => break,
                }
            }
            Err(_) => break,
        }
    }

    chain.reverse(); // oldest first
    Ok(chain)
}

/// Parse a Neynar cast JSON object into our Cast struct.
pub fn parse_cast(v: &serde_json::Value) -> Result<Cast> {
    let hash = v["hash"].as_str().unwrap_or("").to_string();
    let author_fid = v["author"]["fid"].as_u64().unwrap_or(0);
    let author_username = v["author"]["username"].as_str().unwrap_or("").to_string();
    let text = v["text"].as_str().unwrap_or("").to_string();
    let parent_hash = v["parent_hash"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let parent_url = v["parent_url"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // Extract first image embed
    let image_url = v["embeds"].as_array().and_then(|embeds| {
        embeds.iter().find_map(|e| {
            let url = e["url"].as_str()?;
            // Check if it looks like an image
            if url.contains("imagedelivery")
                || url.ends_with(".png")
                || url.ends_with(".jpg")
                || url.ends_with(".jpeg")
                || url.ends_with(".gif")
                || url.ends_with(".webp")
                || e["metadata"]["content_type"]
                    .as_str()
                    .map(|ct| ct.starts_with("image/"))
                    .unwrap_or(false)
            {
                Some(url.to_string())
            } else {
                None
            }
        })
    });

    if hash.is_empty() {
        anyhow::bail!("cast has no hash");
    }

    Ok(Cast {
        hash,
        author_fid,
        author_username,
        text,
        parent_hash,
        parent_url,
        image_url,
    })
}

// ── Webhook subscription ────────────────────────────────────────────────────

/// Create a Neynar webhook subscription to receive cast mentions.
/// Call this once during setup or on startup if the webhook doesn't exist.
pub async fn ensure_webhook(api_key: &str, webhook_url: &str, bot_fid: u64) -> Result<String> {
    if api_key.is_empty() || bot_fid == 0 {
        anyhow::bail!("neynar_api_key or farcaster_bot_fid not configured");
    }

    // First check if webhook already exists
    let client = reqwest::Client::new();
    let list_resp = client
        .get("https://api.neynar.com/v2/farcaster/webhook/list/")
        .header("x-api-key", api_key)
        .header("accept", "application/json")
        .send()
        .await?;

    let subscription = serde_json::json!({
        "cast.created": {
            "mentioned_fids": [bot_fid],
            "parent_author_fids": [bot_fid],
        }
    });

    if list_resp.status().is_success() {
        let data: serde_json::Value = list_resp.json().await?;
        if let Some(webhooks) = data["webhooks"].as_array() {
            // Find any webhook pointing to our URL
            for wh in webhooks {
                let target = wh["target_url"].as_str().unwrap_or("");
                if target == webhook_url {
                    let id = wh["webhook_id"].as_str().unwrap_or("").to_string();
                    // Update subscription to ensure it includes both mentions and replies
                    let update_body = serde_json::json!({
                        "name": "anky-farcaster-bot",
                        "url": webhook_url,
                        "webhook_id": id,
                        "subscription": subscription,
                    });
                    let update_resp = client
                        .put("https://api.neynar.com/v2/farcaster/webhook/")
                        .header("x-api-key", api_key)
                        .header("content-type", "application/json")
                        .json(&update_body)
                        .send()
                        .await;
                    match update_resp {
                        Ok(r) if r.status().is_success() => {
                            tracing::info!("Neynar webhook updated (mentions + replies): {}", id);
                        }
                        Ok(r) => {
                            let err = r.text().await.unwrap_or_default();
                            tracing::warn!(
                                "Neynar webhook update failed ({}), using as-is",
                                &err[..err.len().min(200)]
                            );
                        }
                        Err(e) => tracing::warn!("Neynar webhook update error: {}, using as-is", e),
                    }
                    return Ok(id);
                }
            }
            // No matching webhook but some exist — log what's there
            let count = webhooks.len();
            tracing::info!("Found {} Neynar webhooks, none matching our URL", count);
        }
    }

    // Try to create — may fail if limit reached, which is fine if one already exists
    let body = serde_json::json!({
        "name": "anky-farcaster-bot",
        "url": webhook_url,
        "subscription": subscription,
    });

    let resp = client
        .post("https://api.neynar.com/v2/farcaster/webhook")
        .header("x-api-key", api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        if err_body.contains("LimitReached") {
            tracing::warn!(
                "Neynar webhook limit reached — a webhook likely already exists via dashboard"
            );
            return Ok("existing-via-dashboard".into());
        }
        anyhow::bail!(
            "Neynar webhook create error {}: {}",
            status,
            &err_body[..err_body.len().min(500)]
        );
    }

    let data: serde_json::Value = resp.json().await?;
    let webhook_id = data["webhook"]["webhook_id"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    tracing::info!(
        "Neynar webhook created (mentions + replies): {}",
        webhook_id
    );
    Ok(webhook_id)
}

/// Download an image from a URL and return (bytes, media_type).
pub async fn download_image(url: &str) -> Result<(Vec<u8>, String)> {
    let resp = reqwest::get(url).await?;
    if !resp.status().is_success() {
        anyhow::bail!("image download failed: {}", resp.status());
    }
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let bytes = resp.bytes().await?.to_vec();
    Ok((bytes, content_type))
}

/// Save an image to disk and return the public URL path.
/// Used for embedding generated images in casts (Farcaster needs a URL, not raw bytes).
pub fn save_image_for_embed(image_bytes: &[u8], cast_hash: &str) -> Result<String> {
    let dir = "data/images/farcaster";
    std::fs::create_dir_all(dir)?;
    let filename = format!("{}.png", &cast_hash[..cast_hash.len().min(16)]);
    let path = format!("{}/{}", dir, filename);
    std::fs::write(&path, image_bytes)?;
    // Return the public URL (served via /data/images)
    Ok(format!(
        "https://anky.app/data/images/farcaster/{}",
        filename
    ))
}
