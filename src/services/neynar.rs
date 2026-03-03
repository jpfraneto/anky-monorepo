use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct FarcasterProfile {
    pub fid: u64,
    pub username: String,
    pub display_name: Option<String>,
    pub pfp_url: Option<String>,
}

/// Look up a Farcaster profile by wallet address via Neynar API.
/// Returns None if no Farcaster account is linked to this wallet.
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

    // Response format: { "<address>": [{ user object }] }
    let addr_lower = wallet_address.to_lowercase();
    let users = body
        .as_object()
        .and_then(|obj| {
            // Try both the exact address and lowercase
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
