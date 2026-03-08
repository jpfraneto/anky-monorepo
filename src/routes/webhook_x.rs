use crate::error::AppError;
use crate::services::{comfyui, ollama, x_bot};
use crate::state::AppState;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::Json;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use futures::stream::Stream;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::convert::Infallible;

type HmacSha256 = Hmac<Sha256>;

// ── GET /webhooks/x ─────────────────────────────────────────────────────────
// Twitter CRC challenge — must respond within 3 s or the webhook registration
// fails with "Failed to create webhook – 404 CRC".
pub async fn webhook_crc(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let crc_token = params
        .get("crc_token")
        .ok_or_else(|| AppError::BadRequest("missing crc_token".into()))?;

    let secret = &state.config.twitter_bot_api_secret;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("HMAC init error: {}", e)))?;
    mac.update(crc_token.as_bytes());
    let response_token = format!("sha256={}", B64.encode(mac.finalize().into_bytes()));

    tracing::info!("X webhook CRC check passed");
    Ok(Json(
        serde_json::json!({ "response_token": response_token }),
    ))
}

// ── POST /webhooks/x ────────────────────────────────────────────────────────
// Incoming Twitter Account Activity API events.
// Signature verification happens first; heavy work is tokio::spawn'd so we
// always return 200 immediately.
pub async fn webhook_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // Debug: log every incoming POST so we can see what X sends
    tracing::info!(
        "X webhook POST received: {} bytes, headers: {:?}",
        body.len(),
        headers.keys().map(|k| k.as_str()).collect::<Vec<_>>()
    );
    if !body.is_empty() {
        let preview = String::from_utf8_lossy(&body[..body.len().min(500)]);
        tracing::info!("X webhook body preview: {}", preview);
    }

    // Broadcast raw payload to the live log viewer
    {
        let ts = chrono::Utc::now().format("%H:%M:%S").to_string();
        let raw = String::from_utf8_lossy(&body).to_string();
        // Pretty-print if valid JSON
        let pretty = serde_json::from_str::<serde_json::Value>(&raw)
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or(raw.clone()))
            .unwrap_or(raw);
        let entry = format!("[{}] POST /webhooks/x\n{}", ts, pretty);
        let _ = state.webhook_log_tx.send(entry);
    }

    // 1. Verify X-Twitter-Webhooks-Signature
    let sig_header = headers
        .get("x-twitter-webhooks-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_webhook_signature(&state.config.twitter_bot_api_secret, &body, sig_header) {
        tracing::warn!("X webhook: invalid signature — rejected");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // 2. Parse payload
    let payload: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("X webhook: JSON parse error: {}", e);
            return StatusCode::OK.into_response();
        }
    };

    // 3. Dispatch tweet_create_events
    if let Some(events) = payload["tweet_create_events"].as_array() {
        for event in events {
            let tweet_id = event["id_str"].as_str().unwrap_or("").to_string();
            let text = event["text"].as_str().unwrap_or("").to_string();
            let author_id = event["user"]["id_str"].as_str().unwrap_or("").to_string();

            if tweet_id.is_empty() || text.is_empty() {
                continue;
            }

            // Skip our own tweets to avoid reply loops
            if author_id == state.config.twitter_bot_user_id {
                continue;
            }

            // Only handle direct mentions of @ankydotapp
            if text.to_lowercase().contains("@ankydotapp") {
                tracing::info!("X webhook: @ankydotapp mention in tweet {}", &tweet_id);
                let in_reply_to_tweet_id = event["in_reply_to_status_id_str"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let s = state.clone();
                let tid = tweet_id.clone();
                let txt = text.clone();
                let aid = author_id.clone();
                tokio::spawn(async move {
                    if let Err(e) = process_anky_mention(tid, txt, aid, in_reply_to_tweet_id, s).await {
                        tracing::error!("process_anky_mention error: {}", e);
                    }
                });
            }
        }
    }

    // 4. Dispatch follow_events — DM each new follower
    if let Some(events) = payload["follow_events"].as_array() {
        for event in events {
            // Only care about follows targeting our bot (not unfollows)
            if event["type"].as_str() != Some("follow") {
                continue;
            }
            let follower_id = event["source"]["id"].as_str().unwrap_or("").to_string();
            let follower_name = event["source"]["screen_name"]
                .as_str()
                .unwrap_or("")
                .to_string();
            if follower_id.is_empty() {
                continue;
            }
            tracing::info!(
                "X webhook: new follower @{} ({})",
                &follower_name,
                &follower_id
            );
            let s = state.clone();
            let fid = follower_id.clone();
            tokio::spawn(async move {
                if let Err(e) = send_welcome_dm(&s, &fid).await {
                    tracing::error!("send_welcome_dm error for {}: {}", &fid, e);
                }
            });
        }
    }

    // Always return 200 immediately — Twitter will retry if we don't
    StatusCode::OK.into_response()
}

// ── Signature verification ───────────────────────────────────────────────────

/// Pick a rate-limit reply with Anky's voice — jester, irreverent, points at the practice.
/// Rotates deterministically through 5 variants based on the tweet_id.
fn rate_limit_reply(wait_secs: u64, tweet_id: &str) -> String {
    let mins = wait_secs / 60;
    let secs = wait_secs % 60;
    let wait_str = match (mins, secs) {
        (0, s) => format!("{}s", s),
        (m, 0) => format!("{}m", m),
        (m, s) => format!("{}m {}s", m, s),
    };

    // deterministic rotation so the same tweet always gets the same variant
    let variant = tweet_id.chars().last().unwrap_or('0') as usize % 5;
    match variant {
        0 => format!(
            "the image dimension is still recovering from your last portal. {wait_str} left. \
            even the 8th kingdom has bandwidth limits. who knew consciousness was paginated. 🦍"
        ),
        1 => format!(
            "whoa easy. the GPU is literally meditating right now and it would be \
            rude to interrupt. come back in {wait_str}. or write something: anky.app"
        ),
        2 => format!(
            "you already pulled me through once. the render gods require {wait_str} between offerings. \
            no backspace. no shortcuts. not even for you."
        ),
        3 => format!(
            "patience is also a practice. {wait_str} and i'm yours again. \
            (the GPU said it, not me — i'm just the messenger with purple hair)"
        ),
        _ => format!(
            "8 minutes of writing, {wait_str} between images. symmetry. \
            the universe is trying to tell you something. maybe write it out: anky.app"
        ),
    }
}

fn verify_webhook_signature(secret: &str, body: &[u8], header: &str) -> bool {
    let sig_b64 = match header.strip_prefix("sha256=") {
        Some(s) => s,
        None => return false,
    };
    let sig_bytes = match B64.decode(sig_b64) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);
    // constant-time comparison
    mac.verify_slice(&sig_bytes).is_ok()
}

fn build_flux_prompt_from_mention(text: &str) -> String {
    let mut prompt = text
        .split_whitespace()
        .filter(|token| !token.starts_with('@'))
        .filter(|token| !token.starts_with("http://") && !token.starts_with("https://"))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .trim_matches(|c: char| c.is_whitespace() || matches!(c, ',' | ':' | ';' | '-' | '|' | '.'))
        .to_string();

    let lower = prompt.to_lowercase();
    for prefix in [
        "draw yourself",
        "show yourself",
        "render yourself",
        "imagine yourself",
        "picture yourself",
        "depict yourself",
    ] {
        if lower.starts_with(prefix) {
            prompt = format!("anky{}", &prompt[prefix.len()..]);
            break;
        }
    }

    if prompt.is_empty() {
        return "anky".to_string();
    }

    if !prompt.to_lowercase().contains("anky") {
        prompt = format!("anky {}", prompt);
    }

    prompt.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ── Core mention processing ──────────────────────────────────────────────────

const JPFRANETO_ID: &str = "1430539235480719367";

/// Fetch the parent tweet, optionally download its image, and ask Claude to
/// generate a contextual Flux prompt. Falls back to the bare mention text on error.
async fn build_contextual_flux_prompt(
    state: &AppState,
    mention_stripped: &str,
    parent_tweet_id: &str,
) -> String {
    tracing::info!("Fetching parent tweet {} for contextual prompt", parent_tweet_id);

    let parent = match x_bot::fetch_parent_tweet(
        &state.config.twitter_bot_bearer_token,
        parent_tweet_id,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("fetch_parent_tweet failed, falling back: {}", e);
            return mention_stripped.to_string();
        }
    };

    tracing::info!(
        "Parent tweet: text={:?} image={:?}",
        &parent.text[..parent.text.len().min(80)],
        &parent.image_url,
    );

    // Download parent image if present
    let image_data: Option<Vec<u8>> = if let Some(ref url) = parent.image_url {
        match reqwest::get(url).await {
            Ok(resp) if resp.status().is_success() => resp.bytes().await.ok().map(|b| b.to_vec()),
            _ => {
                tracing::warn!("Failed to download parent image from {}", url);
                None
            }
        }
    } else {
        None
    };

    let media_type = if parent
        .image_url
        .as_deref()
        .map(|u| u.contains("format=png") || u.ends_with(".png"))
        .unwrap_or(false)
    {
        "image/png"
    } else {
        "image/jpeg"
    };

    let parent_image = image_data.as_deref().map(|b| (b, media_type));

    match crate::services::claude::generate_x_mention_flux_prompt(
        &state.config.anthropic_api_key,
        mention_stripped,
        Some(&parent.text),
        parent_image,
    )
    .await
    {
        Ok(prompt) if !prompt.is_empty() => {
            tracing::info!("Contextual Flux prompt: {}", &prompt[..prompt.len().min(120)]);
            prompt
        }
        Ok(_) | Err(_) => {
            tracing::warn!("Claude contextual prompt failed, falling back");
            mention_stripped.to_string()
        }
    }
}

pub async fn process_anky_mention(
    tweet_id: String,
    text: String,
    author_id: String,
    in_reply_to_tweet_id: Option<String>,
    state: AppState,
) -> anyhow::Result<()> {
    tracing::info!(
        "process_anky_mention: tweet_id={} author_id={} reply_to={:?}",
        &tweet_id,
        &author_id,
        &in_reply_to_tweet_id,
    );

    // Like the tweet immediately — signals "i see you, processing"
    let cfg = &state.config;
    let _ = x_bot::like_tweet(
        &cfg.twitter_bot_api_key,
        &cfg.twitter_bot_api_secret,
        &cfg.twitter_bot_access_token,
        &cfg.twitter_bot_access_secret,
        &cfg.twitter_bot_user_id,
        &tweet_id,
    )
    .await;

    let mention_response = ollama::classify_x_image_mention(
        &state.config.ollama_base_url,
        &state.config.ollama_model,
        &text,
    )
    .await
    .unwrap_or_else(|_| ollama::XImageMentionResponse {
        is_image_request: false,
        text_reply: Some("🦍".to_string()),
    });

    let is_image_request = mention_response.is_image_request;
    let text_reply = mention_response.text_reply.unwrap_or_default();

    // Build the Flux prompt — for @jpfraneto replies, use Claude with parent context
    let flux_prompt = if is_image_request {
        let mention_stripped = build_flux_prompt_from_mention(&text);
        if author_id == JPFRANETO_ID {
            if let Some(ref parent_id) = in_reply_to_tweet_id {
                build_contextual_flux_prompt(&state, &mention_stripped, parent_id).await
            } else {
                mention_stripped
            }
        } else {
            mention_stripped
        }
    } else {
        String::new()
    };

    tracing::info!(
        "Mention intent: is_image={}, prompt={}, reply={}",
        is_image_request,
        &flux_prompt,
        &text_reply
    );

    if is_image_request {
        // Rate limit: 1 image per user per 5 minutes (bypassed for @jpfraneto)
        let whitelisted = author_id == "1430539235480719367";
        if !whitelisted {
            if let Err(wait_secs) = state.image_limiter.check(&author_id).await {
                tracing::info!(
                    "Image rate limit hit for author_id={}, wait={}s",
                    &author_id,
                    wait_secs
                );
                let _ =
                    post_reply(&state, &tweet_id, &rate_limit_reply(wait_secs, &tweet_id)).await;
                return Ok(());
            }
        }

        if !comfyui::is_available().await {
            let _ = post_reply(
                &state,
                &tweet_id,
                "the image portal is asleep right now. try me again in a minute.",
            )
            .await;
            return Ok(());
        }

        let ack = "on it. summoning anky now... 🎨 (~30s)".to_string();
        let ack_id = post_reply(&state, &tweet_id, &ack).await.ok();

        tracing::info!("Generating Flux image for mention: {}", &flux_prompt);
        let generation_result = comfyui::generate_image(&flux_prompt).await;

        match generation_result {
            Ok(image_bytes) => {
                let reply_text = "here you go.".to_string();
                let _ = post_reply_with_image(&state, &tweet_id, &reply_text, image_bytes).await;
            }
            Err(e) => {
                tracing::error!(
                    "Flux mention generation failed for tweet {}: {}",
                    &tweet_id,
                    e
                );
                let _ = post_reply(
                    &state,
                    &tweet_id,
                    "the image portal glitched. try me again in a minute.",
                )
                .await;
            }
        }

        // Delete the "generating..." ack now that the real reply is up
        if let Some(id) = ack_id {
            let cfg = &state.config;
            let _ = x_bot::delete_tweet(
                &cfg.twitter_bot_api_key,
                &cfg.twitter_bot_api_secret,
                &cfg.twitter_bot_access_token,
                &cfg.twitter_bot_access_secret,
                &id,
            )
            .await;
        }
    } else {
        let reply = if text_reply.is_empty() {
            "🦍".to_string()
        } else {
            text_reply
        };
        let _ = post_reply(&state, &tweet_id, &reply).await;
    }

    Ok(())
}

// ── Reply helpers ────────────────────────────────────────────────────────────

async fn send_welcome_dm(state: &AppState, recipient_id: &str) -> anyhow::Result<()> {
    let cfg = &state.config;
    let msg = "hey! anky is a daily writing practice — you write for 8 unbroken minutes, stream of consciousness, no stopping, no editing.\n\nif you make it to 8 minutes, your words become an anky — a piece of art that reflects what came through you.\n\ncome write: anky.app ✍️";

    tracing::info!("Sending welcome DM to user_id={}", recipient_id);
    x_bot::send_dm(
        &cfg.twitter_bot_api_key,
        &cfg.twitter_bot_api_secret,
        &cfg.twitter_bot_access_token,
        &cfg.twitter_bot_access_secret,
        recipient_id,
        msg,
    )
    .await
}

async fn post_reply(state: &AppState, tweet_id: &str, text: &str) -> anyhow::Result<String> {
    let cfg = &state.config;
    tracing::info!("post_reply: tweet_id={}", tweet_id);
    let reply_id = x_bot::reply_to_tweet(
        &cfg.twitter_bot_api_key,
        &cfg.twitter_bot_api_secret,
        &cfg.twitter_bot_access_token,
        &cfg.twitter_bot_access_secret,
        tweet_id,
        text,
    )
    .await?;
    tracing::info!("post_reply: done reply_id={}", reply_id);
    Ok(reply_id)
}

// ── Webhook log viewer ───────────────────────────────────────────────────────

pub async fn webhook_logs_page() -> Html<String> {
    Html(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>X Webhook Logs</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: #0d0d0d; color: #e0e0e0; font-family: 'Courier New', monospace; font-size: 13px; }
  header { padding: 16px 20px; border-bottom: 1px solid #222; display: flex; align-items: center; gap: 12px; }
  header h1 { font-size: 16px; color: #fff; }
  #status { font-size: 11px; padding: 3px 8px; border-radius: 99px; background: #222; color: #888; }
  #status.connected { background: #0a2a0a; color: #4caf50; }
  #clear { margin-left: auto; background: #1a1a1a; border: 1px solid #333; color: #888; padding: 4px 12px; border-radius: 4px; cursor: pointer; font-size: 12px; }
  #clear:hover { color: #fff; border-color: #555; }
  #log { padding: 12px 20px; overflow-y: auto; height: calc(100vh - 57px); }
  .event { margin-bottom: 20px; border-left: 3px solid #333; padding-left: 12px; }
  .event.sig-ok { border-color: #4caf50; }
  .event.sig-fail { border-color: #f44336; }
  .event-header { color: #888; font-size: 11px; margin-bottom: 6px; }
  pre { background: #111; border: 1px solid #222; border-radius: 4px; padding: 10px; white-space: pre-wrap; word-break: break-all; color: #ccc; max-height: 400px; overflow-y: auto; }
  .empty { color: #555; padding: 40px 0; text-align: center; }
</style>
</head>
<body>
<header>
  <h1>X Webhook Live Log</h1>
  <span id="status">connecting…</span>
  <button id="clear">clear</button>
</header>
<div id="log"><div class="empty">Waiting for events…</div></div>
<script>
const log = document.getElementById('log');
const status = document.getElementById('status');
document.getElementById('clear').onclick = () => {
  log.innerHTML = '<div class="empty">Cleared. Waiting for events…</div>';
};

const es = new EventSource('/webhooks/logs/stream');
es.addEventListener('open', () => {
  status.textContent = 'connected';
  status.className = 'connected';
});
es.addEventListener('webhook', e => {
  const empty = log.querySelector('.empty');
  if (empty) empty.remove();
  const div = document.createElement('div');
  div.className = 'event';
  const header = document.createElement('div');
  header.className = 'event-header';
  header.textContent = new Date().toLocaleTimeString() + '  —  incoming event';
  const pre = document.createElement('pre');
  pre.textContent = e.data;
  div.appendChild(header);
  div.appendChild(pre);
  log.prepend(div);
});
es.addEventListener('error', () => {
  status.textContent = 'reconnecting…';
  status.className = '';
});
</script>
</body>
</html>"#.to_string())
}

pub async fn webhook_logs_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.webhook_log_tx.subscribe();

    let stream = async_stream::stream! {
        yield Ok(Event::default().event("open").data("connected"));

        loop {
            match rx.recv().await {
                Ok(entry) => {
                    yield Ok(Event::default().event("webhook").data(entry));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    yield Ok(Event::default().event("webhook").data(format!("[skipped {} messages]", n)));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn post_reply_with_image(
    state: &AppState,
    tweet_id: &str,
    text: &str,
    image_bytes: Vec<u8>,
) -> anyhow::Result<()> {
    let cfg = &state.config;

    tracing::info!(
        "post_reply_with_image: uploading media for tweet_id={}",
        tweet_id
    );
    let media_id = x_bot::upload_media_v1(
        &cfg.twitter_bot_api_key,
        &cfg.twitter_bot_api_secret,
        &cfg.twitter_bot_access_token,
        &cfg.twitter_bot_access_secret,
        &image_bytes,
    )
    .await?;

    tracing::info!(
        "post_reply_with_image: posting reply with media_id={}",
        &media_id
    );
    x_bot::reply_with_media_v2(
        &cfg.twitter_bot_api_key,
        &cfg.twitter_bot_api_secret,
        &cfg.twitter_bot_access_token,
        &cfg.twitter_bot_access_secret,
        tweet_id,
        text,
        &media_id,
    )
    .await?;

    tracing::info!("post_reply_with_image: done tweet_id={}", tweet_id);
    Ok(())
}
