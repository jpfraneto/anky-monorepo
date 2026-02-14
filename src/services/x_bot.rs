use anyhow::Result;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use base64::Engine;
use serde::Deserialize;

type HmacSha1 = Hmac<Sha1>;

#[derive(Deserialize, Debug)]
pub struct Mention {
    pub id: String,
    pub text: String,
    pub author_id: Option<String>,
}

#[derive(Deserialize)]
struct MentionsResponse {
    data: Option<Vec<Mention>>,
    includes: Option<MentionIncludes>,
    meta: Option<MentionMeta>,
}

#[derive(Deserialize)]
struct MentionIncludes {
    users: Option<Vec<MentionUser>>,
}

#[derive(Deserialize)]
pub struct MentionUser {
    pub id: String,
    pub username: String,
}

#[derive(Deserialize)]
struct MentionMeta {
    newest_id: Option<String>,
}

pub struct MentionResult {
    pub mentions: Vec<Mention>,
    pub users: Vec<MentionUser>,
    pub newest_id: Option<String>,
}

/// Fetch recent mentions of the bot user.
pub async fn fetch_mentions(
    bearer_token: &str,
    bot_user_id: &str,
    since_id: Option<&str>,
) -> Result<MentionResult> {
    let mut url = format!(
        "https://api.x.com/2/users/{}/mentions?tweet.fields=author_id&expansions=author_id&max_results=20",
        bot_user_id
    );
    if let Some(since) = since_id {
        url.push_str(&format!("&since_id={}", since));
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(bearer_token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Twitter mentions API error {}: {}", status, body);
    }

    let data: MentionsResponse = resp.json().await?;
    Ok(MentionResult {
        mentions: data.data.unwrap_or_default(),
        users: data.includes.and_then(|i| i.users).unwrap_or_default(),
        newest_id: data.meta.and_then(|m| m.newest_id),
    })
}

/// Post a reply tweet using OAuth 1.0a.
pub async fn reply_to_tweet(
    api_key: &str,
    api_secret: &str,
    access_token: &str,
    access_secret: &str,
    tweet_id: &str,
    text: &str,
) -> Result<String> {
    let url = "https://api.x.com/2/tweets";
    let body = serde_json::json!({
        "text": text,
        "reply": {
            "in_reply_to_tweet_id": tweet_id
        }
    });

    let auth_header = generate_oauth_header(
        "POST",
        url,
        api_key,
        api_secret,
        access_token,
        access_secret,
        &[],
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Twitter post tweet failed {}: {}", status, body_text);
    }

    #[derive(Deserialize)]
    struct TweetResponse {
        data: TweetData,
    }
    #[derive(Deserialize)]
    struct TweetData {
        id: String,
    }

    let data: TweetResponse = resp.json().await?;
    Ok(data.data.id)
}

/// Generate OAuth 1.0a Authorization header.
fn generate_oauth_header(
    method: &str,
    url: &str,
    consumer_key: &str,
    consumer_secret: &str,
    token: &str,
    token_secret: &str,
    extra_params: &[(&str, &str)],
) -> String {
    use std::collections::BTreeMap;
    use rand::Rng;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let nonce: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let mut params = BTreeMap::new();
    params.insert("oauth_consumer_key", consumer_key.to_string());
    params.insert("oauth_nonce", nonce.clone());
    params.insert("oauth_signature_method", "HMAC-SHA1".into());
    params.insert("oauth_timestamp", timestamp.clone());
    params.insert("oauth_token", token.to_string());
    params.insert("oauth_version", "1.0".into());

    for (k, v) in extra_params {
        params.insert(k, v.to_string());
    }

    // Build parameter string
    let param_string: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    // Build base string
    let base_string = format!(
        "{}&{}&{}",
        method.to_uppercase(),
        percent_encode(url),
        percent_encode(&param_string),
    );

    // Build signing key
    let signing_key = format!("{}&{}", percent_encode(consumer_secret), percent_encode(token_secret));

    // HMAC-SHA1
    let mut mac = HmacSha1::new_from_slice(signing_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(base_string.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    // Build Authorization header
    format!(
        r#"OAuth oauth_consumer_key="{}",oauth_nonce="{}",oauth_signature="{}",oauth_signature_method="HMAC-SHA1",oauth_timestamp="{}",oauth_token="{}",oauth_version="1.0""#,
        percent_encode(consumer_key),
        percent_encode(&nonce),
        percent_encode(&signature),
        percent_encode(&timestamp),
        percent_encode(token),
    )
}

fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}
