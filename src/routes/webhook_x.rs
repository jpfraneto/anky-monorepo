use crate::error::AppError;
use crate::services::{claude, comfyui, hermes, ollama, x_bot};
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
            let author_username = event["user"]["screen_name"]
                .as_str()
                .unwrap_or("")
                .to_string();

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
                let aun = author_username.clone();
                tokio::spawn(async move {
                    if let Err(e) = process_anky_mention(
                        tid,
                        txt,
                        aid,
                        aun,
                        in_reply_to_tweet_id,
                        "account_activity_webhook",
                        s,
                    )
                    .await
                    {
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

async fn upsert_x_interaction(
    state: &AppState,
    tweet_id: &str,
    author_id: &str,
    author_username: &str,
    text: &str,
    parent_tweet_id: Option<&str>,
    source: &str,
    status: &str,
    classification: Option<&str>,
    tag: Option<&str>,
    extracted_content: Option<&str>,
    result_text: Option<&str>,
    reply_tweet_id: Option<&str>,
    error_message: Option<&str>,
) -> anyhow::Result<()> {
    let db = state.db.lock().await;
    db.execute(
        "INSERT INTO x_interactions (
            id, tweet_id, x_user_id, x_username, tweet_text, status, classification,
            reply_tweet_id, source, parent_tweet_id, tag, extracted_content,
            result_text, error_message, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7,
            ?8, ?9, ?10, ?11, ?12,
            ?13, ?14, datetime('now')
        )
        ON CONFLICT(tweet_id) DO UPDATE SET
            x_user_id = COALESCE(excluded.x_user_id, x_interactions.x_user_id),
            x_username = COALESCE(excluded.x_username, x_interactions.x_username),
            tweet_text = COALESCE(excluded.tweet_text, x_interactions.tweet_text),
            status = excluded.status,
            classification = COALESCE(excluded.classification, x_interactions.classification),
            reply_tweet_id = COALESCE(excluded.reply_tweet_id, x_interactions.reply_tweet_id),
            source = COALESCE(excluded.source, x_interactions.source),
            parent_tweet_id = COALESCE(excluded.parent_tweet_id, x_interactions.parent_tweet_id),
            tag = COALESCE(excluded.tag, x_interactions.tag),
            extracted_content = COALESCE(excluded.extracted_content, x_interactions.extracted_content),
            result_text = COALESCE(excluded.result_text, x_interactions.result_text),
            error_message = CASE
                WHEN excluded.error_message IS NOT NULL THEN excluded.error_message
                WHEN excluded.status IN ('evolution_done', 'replied_text', 'replied_with_image', 'rate_limited', 'image_unavailable') THEN NULL
                ELSE x_interactions.error_message
            END,
            updated_at = datetime('now')",
        rusqlite::params![
            tweet_id,
            tweet_id,
            author_id,
            if author_username.is_empty() {
                None::<&str>
            } else {
                Some(author_username)
            },
            if text.is_empty() { None::<&str> } else { Some(text) },
            status,
            classification,
            reply_tweet_id,
            source,
            parent_tweet_id,
            tag,
            extracted_content,
            result_text,
            error_message,
        ],
    )?;
    Ok(())
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
    tracing::info!(
        "Fetching parent tweet {} for contextual prompt",
        parent_tweet_id
    );

    let parent =
        match x_bot::fetch_parent_tweet(&state.config.twitter_bot_bearer_token, parent_tweet_id)
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
            tracing::info!(
                "Contextual Flux prompt: {}",
                &prompt[..prompt.len().min(120)]
            );
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
    author_username: String,
    in_reply_to_tweet_id: Option<String>,
    source: &'static str,
    state: AppState,
) -> anyhow::Result<()> {
    tracing::info!(
        "process_anky_mention: tweet_id={} author=@{} ({}) reply_to={:?} source={}",
        &tweet_id,
        &author_username,
        &author_id,
        &in_reply_to_tweet_id,
        source,
    );

    let parsed_tag = hermes::parse_tag(&text);
    let initial_classification = if parsed_tag.is_some() {
        Some("tagged_task")
    } else {
        None
    };
    let initial_tag = parsed_tag.as_ref().map(|(tag, _)| tag.as_str());
    let initial_content = parsed_tag.as_ref().map(|(_, content)| content.as_str());

    if let Err(e) = upsert_x_interaction(
        &state,
        &tweet_id,
        &author_id,
        &author_username,
        &text,
        in_reply_to_tweet_id.as_deref(),
        source,
        "received",
        initial_classification,
        initial_tag,
        initial_content,
        None,
        None,
        None,
    )
    .await
    {
        tracing::warn!("x_interactions upsert failed for {}: {}", &tweet_id, e);
    }

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

    // ── JP-only: tagged task dispatch to Hermes agent ────────────────────
    if author_id == JPFRANETO_ID {
        if let Some((tag, content)) = parsed_tag.clone() {
            tracing::info!(
                "Tagged task from JP: [{}] {} (tweet {})",
                &tag,
                &content[..content.len().min(80)],
                &tweet_id
            );

            // Ack immediately so JP knows it's being processed
            let ack = format!(
                "got it. [{}] task received. running the agent now... 🦍",
                tag.to_lowercase()
            );
            let ack_id = post_reply(&state, &tweet_id, &ack).await.ok();

            let _ = upsert_x_interaction(
                &state,
                &tweet_id,
                &author_id,
                &author_username,
                &text,
                in_reply_to_tweet_id.as_deref(),
                source,
                "evolution_running",
                Some("tagged_task"),
                Some(&tag),
                Some(&content),
                None,
                None,
                None,
            )
            .await;

            // Dispatch to Hermes bridge (async, may take minutes)
            let task = hermes::HermesTask {
                tag: tag.clone(),
                content: content.clone(),
                source_tweet_id: tweet_id.clone(),
                author: format!("@{}", author_username),
            };

            // Save task to DB as "running"
            let task_db_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
            {
                let db = state.db.lock().await;
                let _ = db.execute(
                    "INSERT INTO x_evolution_tasks (id, tweet_id, tag, content, author, status) VALUES (?1, ?2, ?3, ?4, ?5, 'running')",
                    rusqlite::params![&task_db_id, &tweet_id, &tag, &content, &format!("@{}", author_username)],
                );
            }

            match hermes::dispatch_task(&task).await {
                Ok(result) => {
                    let summary = result
                        .summary
                        .unwrap_or_else(|| "task processed, no summary returned".to_string());

                    // Update DB with result
                    {
                        let db = state.db.lock().await;
                        let _ = db.execute(
                            "UPDATE x_evolution_tasks SET status = 'done', summary = ?1, completed_at = datetime('now') WHERE id = ?2",
                            rusqlite::params![&summary, &task_db_id],
                        );
                    }

                    // Truncate for tweet length (280 - ~20 for mention overhead)
                    let reply = if summary.len() > 260 {
                        format!("{}...", &summary[..257])
                    } else {
                        summary.clone()
                    };
                    tracing::info!("Hermes task {} done: {}", &tweet_id, &reply);
                    let reply_id = match post_reply(&state, &tweet_id, &reply).await {
                        Ok(id) => Some(id),
                        Err(e) => {
                            tracing::error!(
                                "Failed to post Hermes completion reply for {}: {}",
                                &tweet_id,
                                e
                            );
                            None
                        }
                    };
                    let _ = upsert_x_interaction(
                        &state,
                        &tweet_id,
                        &author_id,
                        &author_username,
                        &text,
                        in_reply_to_tweet_id.as_deref(),
                        source,
                        "evolution_done",
                        Some("tagged_task"),
                        Some(&tag),
                        Some(&content),
                        Some(&summary),
                        reply_id.as_deref(),
                        None,
                    )
                    .await;
                }
                Err(e) => {
                    tracing::error!("Hermes dispatch failed for tweet {}: {}", &tweet_id, e);
                    let bridge_error = format!("Hermes dispatch failed: {}", e);
                    // Update DB with error
                    {
                        let db = state.db.lock().await;
                        let _ = db.execute(
                            "UPDATE x_evolution_tasks SET status = 'error', summary = ?1, completed_at = datetime('now') WHERE id = ?2",
                            rusqlite::params![&format!("error: {}", e), &task_db_id],
                        );
                    }
                    let error_reply =
                        "the bridge is down or the agent errored out. JP check the logs. 🦍";
                    let reply_id = post_reply(&state, &tweet_id, error_reply).await.ok();
                    let _ = upsert_x_interaction(
                        &state,
                        &tweet_id,
                        &author_id,
                        &author_username,
                        &text,
                        in_reply_to_tweet_id.as_deref(),
                        source,
                        "evolution_error",
                        Some("tagged_task"),
                        Some(&tag),
                        Some(&content),
                        Some(error_reply),
                        reply_id.as_deref(),
                        Some(&bridge_error),
                    )
                    .await;
                }
            }

            // Delete the ack now that the real reply is up
            if let Some(id) = ack_id {
                let _ = x_bot::delete_tweet(
                    &cfg.twitter_bot_api_key,
                    &cfg.twitter_bot_api_secret,
                    &cfg.twitter_bot_access_token,
                    &cfg.twitter_bot_access_secret,
                    &id,
                )
                .await;
            }

            return Ok(());
        }
    }

    // Step 1: Classify — image request or text reply?
    // Still use Ollama for fast classification (it's good at this binary task)
    let mention_response = ollama::classify_x_image_mention(
        &state.config.ollama_base_url,
        &state.config.ollama_model,
        &text,
    )
    .await
    .unwrap_or_else(|_| ollama::XImageMentionResponse {
        is_image_request: false,
        text_reply: None,
    });

    let is_image_request = mention_response.is_image_request;

    tracing::info!(
        "Mention classified: is_image={} for tweet {}",
        is_image_request,
        &tweet_id
    );

    let _ = upsert_x_interaction(
        &state,
        &tweet_id,
        &author_id,
        &author_username,
        &text,
        in_reply_to_tweet_id.as_deref(),
        source,
        "classified",
        Some(if is_image_request {
            "image_request"
        } else {
            "text_reply"
        }),
        initial_tag,
        initial_content,
        None,
        None,
        None,
    )
    .await;

    if is_image_request {
        // ── Image generation path — contextual prompts for everyone ──────
        let mention_stripped = build_flux_prompt_from_mention(&text);
        let flux_prompt = if let Some(ref parent_id) = in_reply_to_tweet_id {
            build_contextual_flux_prompt(&state, &mention_stripped, parent_id).await
        } else {
            mention_stripped
        };

        // Rate limit: 1 image per user per 5 minutes (bypassed for @jpfraneto)
        let whitelisted = author_id == JPFRANETO_ID;
        if !whitelisted {
            if let Err(wait_secs) = state.image_limiter.check(&author_id).await {
                tracing::info!(
                    "Image rate limit hit for author_id={}, wait={}s",
                    &author_id,
                    wait_secs
                );
                let rate_limited_text = rate_limit_reply(wait_secs, &tweet_id);
                let reply_id = post_reply(&state, &tweet_id, &rate_limited_text).await.ok();
                let _ = upsert_x_interaction(
                    &state,
                    &tweet_id,
                    &author_id,
                    &author_username,
                    &text,
                    in_reply_to_tweet_id.as_deref(),
                    source,
                    "rate_limited",
                    Some("image_request"),
                    initial_tag,
                    initial_content,
                    Some(&rate_limited_text),
                    reply_id.as_deref(),
                    None,
                )
                .await;
                return Ok(());
            }
        }

        if !comfyui::is_available().await {
            let unavailable_text =
                "the image portal is asleep right now. try me again in a minute.";
            let reply_id = post_reply(&state, &tweet_id, unavailable_text).await.ok();
            let _ = upsert_x_interaction(
                &state,
                &tweet_id,
                &author_id,
                &author_username,
                &text,
                in_reply_to_tweet_id.as_deref(),
                source,
                "image_unavailable",
                Some("image_request"),
                initial_tag,
                initial_content,
                Some(unavailable_text),
                reply_id.as_deref(),
                None,
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
                let reply_id = post_reply_with_image(&state, &tweet_id, &reply_text, image_bytes)
                    .await
                    .ok();
                let _ = upsert_x_interaction(
                    &state,
                    &tweet_id,
                    &author_id,
                    &author_username,
                    &text,
                    in_reply_to_tweet_id.as_deref(),
                    source,
                    "replied_with_image",
                    Some("image_request"),
                    initial_tag,
                    initial_content,
                    Some(&reply_text),
                    reply_id.as_deref(),
                    None,
                )
                .await;
            }
            Err(e) => {
                tracing::error!(
                    "Flux mention generation failed for tweet {}: {}",
                    &tweet_id,
                    e
                );
                let glitch_text = "the image portal glitched. try me again in a minute.";
                let reply_id = post_reply(&state, &tweet_id, glitch_text).await.ok();
                let generation_error = format!("Flux mention generation failed: {}", e);
                let _ = upsert_x_interaction(
                    &state,
                    &tweet_id,
                    &author_id,
                    &author_username,
                    &text,
                    in_reply_to_tweet_id.as_deref(),
                    source,
                    "image_generation_error",
                    Some("image_request"),
                    initial_tag,
                    initial_content,
                    Some(glitch_text),
                    reply_id.as_deref(),
                    Some(&generation_error),
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
        // ── Reply path — Claude decides text-only or text+image ──────────

        // 1. Fetch conversation chain (up to 3 parent tweets)
        let conversation_context = if let Some(ref parent_id) = in_reply_to_tweet_id {
            x_bot::fetch_conversation_chain(&state.config.twitter_bot_bearer_token, parent_id, 3)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let context_pairs: Vec<(String, String)> = conversation_context
            .iter()
            .map(|t| (t.author_username.clone(), t.text.clone()))
            .collect();

        // 2. Check if Anky already replied in this thread (conversation memory)
        let prior_reply = {
            let db = state.db.lock().await;
            if let Some(ref parent_id) = in_reply_to_tweet_id {
                db.query_row(
                    "SELECT anky_reply_text FROM x_conversations WHERE tweet_id = ?1 OR parent_tweet_id = ?1 ORDER BY created_at DESC LIMIT 1",
                    rusqlite::params![parent_id],
                    |row| row.get::<_, String>(0),
                ).ok()
            } else {
                None
            }
        };

        // 3. Fetch image from the mention tweet or its parent (for vision)
        //    Try the mention's parent first (the tweet they replied to with @ankydotapp),
        //    since that's most likely to have the relevant image.
        let tweet_image_data: Option<(Vec<u8>, String)> = {
            let target_id = in_reply_to_tweet_id.as_deref().unwrap_or(&tweet_id);
            match x_bot::fetch_parent_tweet(&state.config.twitter_bot_bearer_token, target_id).await
            {
                Ok(parent) => {
                    if let Some(ref url) = parent.image_url {
                        match reqwest::get(url).await {
                            Ok(resp) if resp.status().is_success() => {
                                let media_type =
                                    if url.contains("format=png") || url.ends_with(".png") {
                                        "image/png".to_string()
                                    } else {
                                        "image/jpeg".to_string()
                                    };
                                resp.bytes().await.ok().map(|b| (b.to_vec(), media_type))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        };

        if tweet_image_data.is_some() {
            tracing::info!("Vision: found image in tweet chain for {}", &tweet_id);
        }

        // 4. Generate reply with Claude — it decides text or text+image
        let username = if author_username.is_empty() {
            None
        } else {
            Some(author_username.as_str())
        };

        let image_ref: Option<(&[u8], &str)> = tweet_image_data
            .as_ref()
            .map(|(bytes, mt)| (bytes.as_slice(), mt.as_str()));

        let anky_reply = claude::generate_anky_reply(
            &state.config.anthropic_api_key,
            &text,
            username,
            &context_pairs,
            prior_reply.as_deref(),
            image_ref,
        )
        .await;

        let reply_text = match anky_reply {
            Ok(claude::AnkyReply::TextWithImage {
                ref text,
                ref flux_prompt,
            }) => {
                // Anky wants to reply with an image — try to generate it
                tracing::info!(
                    "Anky chose image reply for tweet {}: prompt={}",
                    &tweet_id,
                    &flux_prompt[..flux_prompt.len().min(100)]
                );

                if comfyui::is_available().await {
                    match comfyui::generate_image(flux_prompt).await {
                        Ok(image_bytes) => {
                            let reply_id =
                                post_reply_with_image(&state, &tweet_id, text, image_bytes)
                                    .await
                                    .ok();
                            let _ = upsert_x_interaction(
                                &state,
                                &tweet_id,
                                &author_id,
                                &author_username,
                                &text,
                                in_reply_to_tweet_id.as_deref(),
                                source,
                                "replied_with_image",
                                Some("text_reply"),
                                initial_tag,
                                initial_content,
                                Some(text),
                                reply_id.as_deref(),
                                None,
                            )
                            .await;
                            text.clone()
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Proactive image gen failed, falling back to text: {}",
                                e
                            );
                            let reply_id = post_reply(&state, &tweet_id, text).await.ok();
                            let fallback_error = format!(
                                "Proactive image generation failed, fell back to text: {}",
                                e
                            );
                            let _ = upsert_x_interaction(
                                &state,
                                &tweet_id,
                                &author_id,
                                &author_username,
                                &text,
                                in_reply_to_tweet_id.as_deref(),
                                source,
                                "replied_text",
                                Some("text_reply"),
                                initial_tag,
                                initial_content,
                                Some(text),
                                reply_id.as_deref(),
                                Some(&fallback_error),
                            )
                            .await;
                            text.clone()
                        }
                    }
                } else {
                    // GPU busy — just send the text part
                    tracing::info!("GPU unavailable for proactive image, sending text only");
                    let reply_id = post_reply(&state, &tweet_id, text).await.ok();
                    let _ = upsert_x_interaction(
                        &state,
                        &tweet_id,
                        &author_id,
                        &author_username,
                        &text,
                        in_reply_to_tweet_id.as_deref(),
                        source,
                        "replied_text",
                        Some("text_reply"),
                        initial_tag,
                        initial_content,
                        Some(text),
                        reply_id.as_deref(),
                        None,
                    )
                    .await;
                    text.clone()
                }
            }
            Ok(claude::AnkyReply::Text(ref reply)) if !reply.is_empty() => {
                let reply_id = post_reply(&state, &tweet_id, reply).await.ok();
                let _ = upsert_x_interaction(
                    &state,
                    &tweet_id,
                    &author_id,
                    &author_username,
                    &text,
                    in_reply_to_tweet_id.as_deref(),
                    source,
                    "replied_text",
                    Some("text_reply"),
                    initial_tag,
                    initial_content,
                    Some(reply),
                    reply_id.as_deref(),
                    None,
                )
                .await;
                reply.clone()
            }
            Ok(_) => {
                let fallback = "🦍".to_string();
                let reply_id = post_reply(&state, &tweet_id, &fallback).await.ok();
                let _ = upsert_x_interaction(
                    &state,
                    &tweet_id,
                    &author_id,
                    &author_username,
                    &text,
                    in_reply_to_tweet_id.as_deref(),
                    source,
                    "replied_text",
                    Some("text_reply"),
                    initial_tag,
                    initial_content,
                    Some(&fallback),
                    reply_id.as_deref(),
                    None,
                )
                .await;
                fallback
            }
            Err(e) => {
                tracing::error!("Claude reply generation failed: {}", e);
                // Fallback to Ollama
                let fallback = ollama::classify_x_image_mention(
                    &state.config.ollama_base_url,
                    &state.config.ollama_model,
                    &text,
                )
                .await
                .ok()
                .and_then(|r| r.text_reply)
                .unwrap_or_else(|| "🦍".to_string());
                let reply_id = post_reply(&state, &tweet_id, &fallback).await.ok();
                let claude_error = format!("Claude reply generation failed: {}", e);
                let _ = upsert_x_interaction(
                    &state,
                    &tweet_id,
                    &author_id,
                    &author_username,
                    &text,
                    in_reply_to_tweet_id.as_deref(),
                    source,
                    "replied_text",
                    Some("text_reply"),
                    initial_tag,
                    initial_content,
                    Some(&fallback),
                    reply_id.as_deref(),
                    Some(&claude_error),
                )
                .await;
                fallback
            }
        };

        tracing::info!("Anky reply for tweet {}: {}", &tweet_id, &reply_text);

        // 4. Save to conversation memory
        {
            let db = state.db.lock().await;
            let _ = db.execute(
                "INSERT OR REPLACE INTO x_conversations (tweet_id, author_id, author_username, parent_tweet_id, mention_text, anky_reply_text)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    &tweet_id,
                    &author_id,
                    &author_username,
                    &in_reply_to_tweet_id,
                    &text,
                    &reply_text,
                ],
            );
        }
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
) -> anyhow::Result<String> {
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
    let reply_id = x_bot::reply_with_media_v2(
        &cfg.twitter_bot_api_key,
        &cfg.twitter_bot_api_secret,
        &cfg.twitter_bot_access_token,
        &cfg.twitter_bot_access_secret,
        tweet_id,
        text,
        &media_id,
    )
    .await?;

    tracing::info!(
        "post_reply_with_image: done tweet_id={} reply_id={}",
        tweet_id,
        &reply_id
    );
    Ok(reply_id)
}
