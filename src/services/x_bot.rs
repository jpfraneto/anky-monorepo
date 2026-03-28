use anyhow::Result;
use base64::Engine;
use futures_util::StreamExt;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha1::Sha1;
use std::time::Duration;

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
    let resp = client.get(&url).bearer_auth(bearer_token).send().await?;

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

/// Post a thread of tweets, each replying to the previous one.
/// Returns the IDs of all posted tweets.
pub async fn reply_thread(
    api_key: &str,
    api_secret: &str,
    access_token: &str,
    access_secret: &str,
    initial_tweet_id: &str,
    slides: &[String],
) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    let mut parent_id = initial_tweet_id.to_string();

    for slide in slides {
        let id = reply_to_tweet(
            api_key,
            api_secret,
            access_token,
            access_secret,
            &parent_id,
            slide,
        )
        .await?;
        parent_id = id.clone();
        ids.push(id);
    }

    Ok(ids)
}

/// Like a tweet via the v2 API (used to acknowledge a mention is being processed).
pub async fn like_tweet(
    api_key: &str,
    api_secret: &str,
    access_token: &str,
    access_secret: &str,
    bot_user_id: &str,
    tweet_id: &str,
) -> Result<()> {
    let url = format!("https://api.twitter.com/2/users/{}/likes", bot_user_id);
    let auth_header = generate_oauth_header(
        "POST",
        &url,
        api_key,
        api_secret,
        access_token,
        access_secret,
        &[],
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "tweet_id": tweet_id }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Twitter like_tweet failed {}: {}", status, body);
    }

    tracing::info!("Liked tweet {}", tweet_id);
    Ok(())
}

/// Send a DM to a user via the v1.1 Direct Messages API.
/// Uses JSON body (not form-encoded), so body is NOT included in OAuth signature.
pub async fn send_dm(
    api_key: &str,
    api_secret: &str,
    access_token: &str,
    access_secret: &str,
    recipient_id: &str,
    text: &str,
) -> Result<()> {
    let url = "https://api.twitter.com/1.1/direct_messages/events/new.json";
    let auth_header = generate_oauth_header(
        "POST",
        url,
        api_key,
        api_secret,
        access_token,
        access_secret,
        &[],
    );

    let body = serde_json::json!({
        "event": {
            "type": "message_create",
            "message_create": {
                "target": { "recipient_id": recipient_id },
                "message_data": { "text": text }
            }
        }
    });

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
        anyhow::bail!("Twitter send_dm failed {}: {}", status, body_text);
    }

    tracing::info!("DM sent to user_id={}", recipient_id);
    Ok(())
}

/// Upload image bytes to Twitter v1.1 media/upload. Returns `media_id_string`.
pub async fn upload_media_v1(
    api_key: &str,
    api_secret: &str,
    access_token: &str,
    access_secret: &str,
    image_bytes: &[u8],
) -> Result<String> {
    let url = "https://upload.twitter.com/1.1/media/upload.json";

    // Multipart upload — body parts are NOT included in OAuth signature
    let auth_header = generate_oauth_header(
        "POST",
        url,
        api_key,
        api_secret,
        access_token,
        access_secret,
        &[],
    );

    let part = reqwest::multipart::Part::bytes(image_bytes.to_vec())
        .file_name("anky.png")
        .mime_str("image/png")
        .map_err(|e| anyhow::anyhow!("mime error: {}", e))?;
    let form = reqwest::multipart::Form::new().part("media", part);

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Authorization", auth_header)
        .multipart(form)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Twitter media upload failed: {}", body);
    }

    #[derive(Deserialize)]
    struct MediaResponse {
        media_id_string: String,
    }

    let data: MediaResponse = resp.json().await?;
    tracing::info!("Twitter media uploaded: id={}", data.media_id_string);
    Ok(data.media_id_string)
}

/// Post a v2 reply tweet that attaches an already-uploaded media_id.
pub async fn reply_with_media_v2(
    api_key: &str,
    api_secret: &str,
    access_token: &str,
    access_secret: &str,
    tweet_id: &str,
    text: &str,
    media_id: &str,
) -> Result<String> {
    let url = "https://api.x.com/2/tweets";
    let body = serde_json::json!({
        "text": text,
        "reply": { "in_reply_to_tweet_id": tweet_id },
        "media": { "media_ids": [media_id] }
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
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("reply_with_media_v2 failed {}: {}", status, body);
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
    tracing::info!(
        "Image reply posted to tweet {} as {}",
        tweet_id,
        data.data.id
    );
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
    use rand::Rng;
    use std::collections::BTreeMap;

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
    let signing_key = format!(
        "{}&{}",
        percent_encode(consumer_secret),
        percent_encode(token_secret)
    );

    // HMAC-SHA1
    let mut mac =
        HmacSha1::new_from_slice(signing_key.as_bytes()).expect("HMAC can take key of any size");
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

/// Delete a tweet by ID via the v2 API.
pub async fn delete_tweet(
    api_key: &str,
    api_secret: &str,
    access_token: &str,
    access_secret: &str,
    tweet_id: &str,
) -> Result<()> {
    let url = format!("https://api.x.com/2/tweets/{}", tweet_id);
    let auth_header = generate_oauth_header(
        "DELETE",
        &url,
        api_key,
        api_secret,
        access_token,
        access_secret,
        &[],
    );

    let client = reqwest::Client::new();
    let resp = client
        .delete(&url)
        .header("Authorization", auth_header)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("delete_tweet failed {}: {}", status, body);
    }

    tracing::info!("Deleted tweet {}", tweet_id);
    Ok(())
}

/// Parent tweet context fetched for reply mentions.
pub struct ParentTweet {
    pub text: String,
    pub image_url: Option<String>,
}

/// Fetch a tweet by ID and return its text plus the first image URL (if any).
pub async fn fetch_parent_tweet(bearer_token: &str, tweet_id: &str) -> Result<ParentTweet> {
    let url = format!(
        "https://api.twitter.com/2/tweets/{}?tweet.fields=text&expansions=attachments.media_keys&media.fields=url,preview_image_url,type",
        tweet_id
    );
    let client = reqwest::Client::new();
    let resp = client.get(&url).bearer_auth(bearer_token).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("fetch_parent_tweet failed {}: {}", status, body);
    }

    let data: serde_json::Value = resp.json().await?;
    let text = data["data"]["text"].as_str().unwrap_or("").to_string();

    let image_url = data["includes"]["media"].as_array().and_then(|media| {
        media.iter().find_map(|m| {
            if m["type"].as_str() == Some("photo") {
                m["url"].as_str().map(|s| s.to_string())
            } else {
                m["preview_image_url"].as_str().map(|s| s.to_string())
            }
        })
    });

    Ok(ParentTweet { text, image_url })
}

/// A tweet in a conversation chain — includes author info.
pub struct ConversationTweet {
    pub author_username: String,
    pub text: String,
}

/// Walk up the reply chain (max `depth` levels) and return tweets oldest-first.
/// Used to give Anky conversation context before replying.
pub async fn fetch_conversation_chain(
    bearer_token: &str,
    tweet_id: &str,
    depth: u8,
) -> Result<Vec<ConversationTweet>> {
    let mut chain = Vec::new();
    let mut current_id = tweet_id.to_string();

    for _ in 0..depth {
        let url = format!(
            "https://api.twitter.com/2/tweets/{}?tweet.fields=text,author_id,referenced_tweets&expansions=author_id&user.fields=username",
            current_id
        );
        let client = reqwest::Client::new();
        let resp = client.get(&url).bearer_auth(bearer_token).send().await?;

        if !resp.status().is_success() {
            break;
        }

        let data: serde_json::Value = resp.json().await?;
        let text = data["data"]["text"].as_str().unwrap_or("").to_string();
        let author_id = data["data"]["author_id"].as_str().unwrap_or("");

        // Resolve username from includes
        let username = data["includes"]["users"]
            .as_array()
            .and_then(|users| {
                users
                    .iter()
                    .find(|u| u["id"].as_str() == Some(author_id))
                    .and_then(|u| u["username"].as_str())
            })
            .unwrap_or("unknown")
            .to_string();

        chain.push(ConversationTweet {
            author_username: username,
            text,
        });

        // Find the parent tweet ID
        let parent_id = data["data"]["referenced_tweets"]
            .as_array()
            .and_then(|refs| {
                refs.iter()
                    .find(|r| r["type"].as_str() == Some("replied_to"))
                    .and_then(|r| r["id"].as_str())
                    .map(|s| s.to_string())
            });

        match parent_id {
            Some(pid) => current_id = pid,
            None => break,
        }
    }

    // Reverse so oldest is first
    chain.reverse();
    Ok(chain)
}

/// Ensure a filter rule for @ankydotapp mentions exists on the v2 Filtered Stream.
/// Idempotent — safe to call on every startup.
pub async fn ensure_mention_rule(bearer_token: &str) -> Result<()> {
    let client = reqwest::Client::new();

    // Check existing rules
    let resp = client
        .get("https://api.twitter.com/2/tweets/search/stream/rules")
        .bearer_auth(bearer_token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to list stream rules: {} {}", status, body);
    }

    let data: serde_json::Value = resp.json().await?;
    if let Some(rules) = data["data"].as_array() {
        for rule in rules {
            if rule["tag"].as_str() == Some("ankydotapp-mentions") {
                tracing::info!("Filtered stream rule already exists (id={})", rule["id"]);
                return Ok(());
            }
        }
    }

    // Create the rule
    let resp = client
        .post("https://api.twitter.com/2/tweets/search/stream/rules")
        .bearer_auth(bearer_token)
        .json(&serde_json::json!({
            "add": [{"value": "@ankydotapp -is:retweet", "tag": "ankydotapp-mentions"}]
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to create stream rule: {} {}", status, body);
    }

    let result: serde_json::Value = resp.json().await?;
    tracing::info!("Created filtered stream rule: {:?}", result["data"]);
    Ok(())
}

/// Connect to the X v2 Filtered Stream and process @ankydotapp mentions.
/// Returns Err when the stream disconnects (caller should reconnect with backoff).
pub async fn run_filtered_stream(bearer_token: &str, state: &crate::state::AppState) -> Result<()> {
    let client = reqwest::Client::new();

    let resp = client
        .get("https://api.twitter.com/2/tweets/search/stream")
        .bearer_auth(bearer_token)
        .query(&[
            ("tweet.fields", "author_id,created_at,referenced_tweets"),
            ("expansions", "author_id,referenced_tweets.id"),
            ("user.fields", "username"),
        ])
        .send()
        .await?;

    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        // Back off for 15 minutes before retrying
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!("Filtered stream 429 rate limited: {}", body);
        tokio::time::sleep(Duration::from_secs(900)).await;
        anyhow::bail!("rate limited (429), waited 15m");
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Filtered stream connect failed: {} {}", status, body);
    }

    tracing::info!("Connected to X v2 Filtered Stream");
    state.emit_log("INFO", "x_stream", "Connected to X v2 Filtered Stream");

    let mut stream = resp.bytes_stream();
    let mut pending = Vec::<u8>::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        pending.extend_from_slice(&chunk);

        while let Some(newline_pos) = pending.iter().position(|b| *b == b'\n') {
            let line_bytes: Vec<u8> = pending.drain(..=newline_pos).collect();
            let line = match std::str::from_utf8(&line_bytes) {
                Ok(s) => s.trim(),
                Err(e) => {
                    tracing::warn!("Stream utf8 decode error: {}", e);
                    continue;
                }
            };

            if line.is_empty() {
                continue; // heartbeat newline
            }

            let event: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        "Stream parse error: {} — {:?}",
                        e,
                        &line[..line.len().min(200)]
                    );
                    continue;
                }
            };

            let tweet_id = event["data"]["id"].as_str().unwrap_or("").to_string();
            let text = event["data"]["text"].as_str().unwrap_or("").to_string();
            let author_id = event["data"]["author_id"]
                .as_str()
                .unwrap_or("")
                .to_string();

            if tweet_id.is_empty() || text.is_empty() || author_id.is_empty() {
                continue;
            }

            // Skip our own tweets
            if author_id == state.config.twitter_bot_user_id {
                continue;
            }

            // Extract parent tweet ID if this is a reply
            let in_reply_to_tweet_id =
                event["data"]["referenced_tweets"]
                    .as_array()
                    .and_then(|refs| {
                        refs.iter()
                            .find(|r| r["type"].as_str() == Some("replied_to"))
                            .and_then(|r| r["id"].as_str())
                            .map(|s| s.to_string())
                    });

            // Resolve author username from includes
            let author_username = event["includes"]["users"]
                .as_array()
                .and_then(|users| {
                    users
                        .iter()
                        .find(|u| u["id"].as_str() == Some(&author_id))
                        .and_then(|u| u["username"].as_str())
                })
                .unwrap_or("")
                .to_string();

            tracing::info!(
                "Stream mention: tweet_id={} author=@{} ({}) reply_to={:?}",
                &tweet_id,
                &author_username,
                &author_id,
                &in_reply_to_tweet_id,
            );
            state.emit_log(
                "INFO",
                "x_stream",
                &format!(
                    "Mention from @{}: {}",
                    &author_username,
                    &text[..text.len().min(100)]
                ),
            );

            let s = state.clone();
            let tid = tweet_id.clone();
            let txt = text.clone();
            let aid = author_id.clone();
            let aun = author_username.clone();
            tokio::spawn(async move {
                if let Err(e) = crate::routes::webhook_x::process_anky_mention(
                    tid,
                    txt,
                    aid,
                    aun,
                    in_reply_to_tweet_id,
                    "filtered_stream",
                    s,
                )
                .await
                {
                    tracing::error!("process_anky_mention error: {}", e);
                }
            });
        }

        if pending.len() > 1024 * 1024 {
            tracing::warn!("Stream buffer exceeded 1MB without newline; dropping partial payload");
            pending.clear();
        }
    }

    anyhow::bail!("Filtered stream disconnected")
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
