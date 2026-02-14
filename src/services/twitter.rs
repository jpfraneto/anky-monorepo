use anyhow::Result;
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Generate a PKCE code verifier and challenge.
pub fn generate_pkce() -> (String, String) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let verifier: String = (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            chars[idx] as char
        })
        .collect();

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);

    (verifier, challenge)
}

/// Generate a random state string.
pub fn generate_state() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            chars[idx] as char
        })
        .collect()
}

/// Build the Twitter OAuth 2.0 authorization URL.
pub fn authorization_url(
    client_id: &str,
    callback_url: &str,
    state: &str,
    challenge: &str,
    scopes: &[&str],
) -> String {
    let scope = scopes.join("%20");
    format!(
        "https://x.com/i/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        urlencoding(client_id),
        urlencoding(callback_url),
        scope,
        state,
        challenge,
    )
}

fn urlencoding(s: &str) -> String {
    s.replace(':', "%3A").replace('/', "%2F").replace('?', "%3F").replace('=', "%3D").replace('&', "%26")
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: Option<String>,
}

/// Exchange an authorization code for tokens.
pub async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    code_verifier: &str,
    callback_url: &str,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.x.com/2/oauth2/token")
        .basic_auth(client_id, Some(client_secret))
        .form(&[
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", callback_url),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Twitter token exchange failed {}: {}", status, body);
    }

    let tokens: TokenResponse = resp.json().await?;
    Ok(tokens)
}

/// Refresh an expired access token.
pub async fn refresh_token(
    client_id: &str,
    client_secret: &str,
    refresh: &str,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.x.com/2/oauth2/token")
        .basic_auth(client_id, Some(client_secret))
        .form(&[
            ("refresh_token", refresh),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Twitter token refresh failed {}: {}", status, body);
    }

    let tokens: TokenResponse = resp.json().await?;
    Ok(tokens)
}

#[derive(Deserialize, Debug)]
pub struct TwitterUser {
    pub id: String,
    pub username: String,
    pub name: String,
    pub profile_image_url: Option<String>,
}

#[derive(Deserialize)]
struct UserResponse {
    data: TwitterUser,
}

/// Get the authenticated user's profile.
pub async fn get_user(access_token: &str) -> Result<TwitterUser> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.x.com/2/users/me?user.fields=profile_image_url")
        .bearer_auth(access_token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Twitter get user failed {}: {}", status, body);
    }

    let data: UserResponse = resp.json().await?;
    Ok(data.data)
}
