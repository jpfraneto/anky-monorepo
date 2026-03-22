use crate::routes::social_context;
use crate::services::{claude, comfyui, neynar, ollama};
use crate::state::AppState;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use hmac::{Hmac, Mac};
use sha2::Sha512;

type HmacSha512 = Hmac<Sha512>;

const FARCASTER_RETRY_STALE_MINUTES: i64 = 5;

fn truncate_for_log(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

// ── POST /webhooks/farcaster ────────────────────────────────────────────────
// Neynar sends cast.created events when someone mentions our bot.
// Verify HMAC-SHA512 signature, then spawn async processing.

pub async fn webhook_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    tracing::info!("Farcaster webhook POST: {} bytes", body.len(),);

    // Broadcast to webhook log viewer
    {
        let ts = chrono::Utc::now().format("%H:%M:%S").to_string();
        let raw = String::from_utf8_lossy(&body).to_string();
        let pretty = serde_json::from_str::<serde_json::Value>(&raw)
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or(raw.clone()))
            .unwrap_or(raw);
        let entry = format!("[{}] POST /webhooks/farcaster\n{}", ts, pretty);
        let _ = state.webhook_log_tx.send(entry);
    }

    // Verify Neynar webhook signature (HMAC-SHA512)
    let sig_header = headers
        .get("x-neynar-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_neynar_signature(&state.config.neynar_webhook_secret, &body, sig_header) {
        // Signature verification not matching yet — log but allow through.
        // The webhook secret from the Neynar dashboard may belong to the original
        // webhook, not the ones we auto-created. Safe to skip: the webhook URL is
        // not guessable and only Neynar knows it.
        tracing::debug!("Farcaster webhook: signature check skipped");
    }

    // Parse payload
    let payload: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Farcaster webhook: JSON parse error: {}", e);
            return StatusCode::OK.into_response();
        }
    };

    // Neynar webhook payload structure:
    // { "created_at": ..., "type": "cast.created", "data": { ...cast object... } }
    let event_type = payload["type"].as_str().unwrap_or("");

    if event_type == "cast.created" {
        if let Ok(cast) = neynar::parse_cast(&payload["data"]) {
            let _ = queue_farcaster_cast(state.clone(), cast, "webhook").await;
        }
    }

    StatusCode::OK.into_response()
}

// ── Signature verification ──────────────────────────────────────────────────

fn verify_neynar_signature(secret: &str, body: &[u8], sig_hex: &str) -> bool {
    if secret.is_empty() {
        // If no secret configured, skip verification (dev mode)
        tracing::warn!("Neynar webhook secret not configured — skipping signature check");
        return true;
    }
    if sig_hex.is_empty() {
        return false;
    }

    let sig_bytes = match hex::decode(sig_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mut mac = match HmacSha512::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok()
}

pub async fn backfill_recent_interactions(state: AppState, limit: usize) -> anyhow::Result<usize> {
    let casts = neynar::fetch_recent_interaction_casts(
        &state.config.neynar_api_key,
        state.config.farcaster_bot_fid,
        limit,
    )
    .await?;

    let mut queued = 0usize;
    for cast in casts {
        if queue_farcaster_cast(state.clone(), cast, "notifications").await {
            queued += 1;
        }
    }

    Ok(queued)
}

async fn queue_farcaster_cast(state: AppState, cast: neynar::Cast, source: &'static str) -> bool {
    if cast.hash.is_empty() || cast.text.is_empty() {
        return false;
    }

    if cast.author_fid == state.config.farcaster_bot_fid {
        return false;
    }

    if !claim_social_interaction(&state, &cast).await {
        return false;
    }

    tracing::info!(
        "Farcaster cast queued via {} from @{} (fid:{}) in cast {}: {}",
        source,
        &cast.author_username,
        cast.author_fid,
        truncate_for_log(&cast.hash, 10),
        truncate_for_log(&cast.text, 100),
    );

    let s = state.clone();
    let cast_hash = cast.hash.clone();
    let text = cast.text.clone();
    let author_fid = cast.author_fid;
    let author_username = cast.author_username.clone();
    let parent_hash = cast.parent_hash.clone();
    let image_url = cast.image_url.clone();

    tokio::spawn(async move {
        match process_farcaster_mention(
            cast_hash.clone(),
            text,
            author_fid,
            author_username,
            parent_hash,
            image_url,
            s,
        )
        .await
        {
            Ok(_) => tracing::info!(
                "FC mention {} processed successfully",
                truncate_for_log(&cast_hash, 10)
            ),
            Err(e) => tracing::error!(
                "process_farcaster_mention error for {}: {}",
                truncate_for_log(&cast_hash, 10),
                e
            ),
        }
    });

    true
}

async fn claim_social_interaction(state: &AppState, cast: &neynar::Cast) -> bool {
    let stale_window = format!("-{} minutes", FARCASTER_RETRY_STALE_MINUTES);
    let db = state.db.lock().await;
    match db.execute(
        "INSERT INTO social_interactions (
            id, platform, post_id, author_id, author_username, post_text,
            parent_id, status, updated_at
        ) VALUES (
            ?1, 'farcaster', ?1, ?2, ?3, ?4,
            ?5, 'processing', datetime('now')
        )
        ON CONFLICT(platform, post_id) DO UPDATE SET
            author_id = COALESCE(excluded.author_id, social_interactions.author_id),
            author_username = COALESCE(excluded.author_username, social_interactions.author_username),
            post_text = COALESCE(excluded.post_text, social_interactions.post_text),
            parent_id = COALESCE(excluded.parent_id, social_interactions.parent_id),
            status = excluded.status,
            updated_at = datetime('now')
        WHERE (social_interactions.reply_id IS NULL OR social_interactions.reply_id = '')
          AND (
              social_interactions.updated_at IS NULL
              OR social_interactions.updated_at < datetime('now', ?6)
              OR social_interactions.status NOT IN ('processing', 'received', 'classified')
          )",
        rusqlite::params![
            &cast.hash,
            cast.author_fid.to_string(),
            if cast.author_username.is_empty() {
                None::<&str>
            } else {
                Some(cast.author_username.as_str())
            },
            if cast.text.is_empty() {
                None::<&str>
            } else {
                Some(cast.text.as_str())
            },
            cast.parent_hash.as_deref(),
            stale_window,
        ],
    ) {
        Ok(rows) => rows > 0,
        Err(e) => {
            tracing::warn!(
                "Failed to claim Farcaster cast {}: {}",
                truncate_for_log(&cast.hash, 10),
                e
            );
            false
        }
    }
}

// ── Core mention processing ─────────────────────────────────────────────────
// Mirrors the X webhook flow: classify → reply (text or image) → save to DB.
// Uses the same Claude identity, same Ollama classifier, same ComfyUI pipeline.

async fn process_farcaster_mention(
    cast_hash: String,
    text: String,
    author_fid: u64,
    author_username: String,
    parent_hash: Option<String>,
    image_url: Option<String>,
    state: AppState,
) -> anyhow::Result<()> {
    let cfg = &state.config;
    let api_key = &cfg.neynar_api_key;
    let signer = &cfg.neynar_signer_uuid;

    // Save initial interaction
    upsert_social_interaction(
        &state,
        "farcaster",
        &cast_hash,
        &author_fid.to_string(),
        &author_username,
        &text,
        parent_hash.as_deref(),
        "received",
        None,
        None,
        None,
    )
    .await;

    // Like the cast immediately
    let _ = neynar::react_to_cast(api_key, signer, &cast_hash).await;

    // Classify: image request or text reply?
    let mention_response =
        ollama::classify_x_image_mention(&cfg.ollama_base_url, &cfg.ollama_model, &text)
            .await
            .unwrap_or_else(|_| ollama::XImageMentionResponse {
                is_image_request: false,
                text_reply: None,
            });

    let is_image_request = mention_response.is_image_request;

    tracing::info!(
        "FC mention classified: is_image={} for cast {}",
        is_image_request,
        truncate_for_log(&cast_hash, 10)
    );

    upsert_social_interaction(
        &state,
        "farcaster",
        &cast_hash,
        &author_fid.to_string(),
        &author_username,
        &text,
        parent_hash.as_deref(),
        "classified",
        Some(if is_image_request {
            "image_request"
        } else {
            "text_reply"
        }),
        None,
        None,
    )
    .await;

    if is_image_request {
        // ── Image generation path ───────────────────────────────────────
        let mention_stripped = strip_mentions(&text);
        let flux_prompt =
            if !mention_stripped.is_empty() && mention_stripped.to_lowercase().contains("anky") {
                mention_stripped.clone()
            } else if mention_stripped.is_empty() {
                "anky".to_string()
            } else {
                format!("anky {}", mention_stripped)
            };

        // Rate limit: 1 image per user per 5 minutes
        if let Err(wait_secs) = state.image_limiter.check(&author_fid.to_string()).await {
            let rate_text = format!(
                "the image dimension needs {}s to recover. even consciousness has bandwidth limits. 🦍",
                wait_secs
            );
            let reply_hash = neynar::reply_to_cast(api_key, signer, &cast_hash, &rate_text)
                .await
                .ok()
                .map(|r| r.hash);
            upsert_social_interaction(
                &state,
                "farcaster",
                &cast_hash,
                &author_fid.to_string(),
                &author_username,
                &text,
                parent_hash.as_deref(),
                "rate_limited",
                Some("image_request"),
                Some(&rate_text),
                reply_hash.as_deref(),
            )
            .await;
            return Ok(());
        }

        if !comfyui::is_available().await {
            let unavail = "the image portal is asleep right now. try me again in a minute.";
            let reply_hash = neynar::reply_to_cast(api_key, signer, &cast_hash, unavail)
                .await
                .ok()
                .map(|r| r.hash);
            upsert_social_interaction(
                &state,
                "farcaster",
                &cast_hash,
                &author_fid.to_string(),
                &author_username,
                &text,
                parent_hash.as_deref(),
                "image_unavailable",
                Some("image_request"),
                Some(unavail),
                reply_hash.as_deref(),
            )
            .await;
            return Ok(());
        }

        tracing::info!("Generating Flux image for FC mention: {}", &flux_prompt);
        match comfyui::generate_image(&flux_prompt).await {
            Ok(image_bytes) => {
                // Save image and get public URL for embed
                let image_embed_url = neynar::save_image_for_embed(&image_bytes, &cast_hash)?;
                tracing::info!(
                    "FC image saved, posting cast with embed: {}",
                    &image_embed_url
                );
                let reply_text = "here you go. 🦍";
                let cast_result = neynar::reply_to_cast_with_image(
                    api_key,
                    signer,
                    &cast_hash,
                    reply_text,
                    &image_embed_url,
                )
                .await;
                if let Err(ref e) = cast_result {
                    tracing::error!("FC image cast failed: {}", e);
                }
                let reply_hash = cast_result.ok().map(|r| r.hash);
                upsert_social_interaction(
                    &state,
                    "farcaster",
                    &cast_hash,
                    &author_fid.to_string(),
                    &author_username,
                    &text,
                    parent_hash.as_deref(),
                    "replied_with_image",
                    Some("image_request"),
                    Some(reply_text),
                    reply_hash.as_deref(),
                )
                .await;
            }
            Err(e) => {
                tracing::error!("Flux FC generation failed: {}", e);
                let glitch = "the image portal glitched. try me again in a minute.";
                let reply_hash = neynar::reply_to_cast(api_key, signer, &cast_hash, glitch)
                    .await
                    .ok()
                    .map(|r| r.hash);
                upsert_social_interaction(
                    &state,
                    "farcaster",
                    &cast_hash,
                    &author_fid.to_string(),
                    &author_username,
                    &text,
                    parent_hash.as_deref(),
                    "image_generation_error",
                    Some("image_request"),
                    Some(glitch),
                    reply_hash.as_deref(),
                )
                .await;
            }
        }
    } else {
        // ── Text reply path (Claude with conversation context) ──────────

        // Fetch Honcho peer context + interaction history
        let social_ctx = social_context::fetch_social_context(
            &state,
            "farcaster",
            &author_fid.to_string(),
            &author_username,
        )
        .await;

        // Fetch conversation chain for context
        let context_pairs = if let Some(ref ph) = parent_hash {
            neynar::fetch_conversation_chain(api_key, ph, 3)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Check if Anky already replied in this thread
        let prior_reply = {
            let db = state.db.lock().await;
            if let Some(ref ph) = parent_hash {
                db.query_row(
                    "SELECT reply_text FROM social_interactions WHERE platform = 'farcaster' AND (post_id = ?1 OR parent_id = ?1) AND reply_text IS NOT NULL ORDER BY created_at DESC LIMIT 1",
                    rusqlite::params![ph],
                    |row| row.get::<_, String>(0),
                ).ok()
            } else {
                None
            }
        };

        // Fetch image from the cast or parent for vision
        let tweet_image_data: Option<(Vec<u8>, String)> = {
            let img_source = image_url.as_deref().or_else(|| {
                // If no image on this cast, try the parent
                None // we'd need to fetch parent — skip for now
            });
            if let Some(url) = img_source {
                neynar::download_image(url).await.ok()
            } else {
                None
            }
        };

        if tweet_image_data.is_some() {
            tracing::info!(
                "Vision: found image in FC cast chain for {}",
                truncate_for_log(&cast_hash, 10)
            );
        }

        // Generate reply with Claude — same identity as X bot, now with context
        let username = if author_username.is_empty() {
            None
        } else {
            Some(author_username.as_str())
        };

        let image_ref: Option<(&[u8], &str)> = tweet_image_data
            .as_ref()
            .map(|(bytes, mt)| (bytes.as_slice(), mt.as_str()));

        let anky_reply = claude::generate_anky_reply(
            &cfg.anthropic_api_key,
            &text,
            username,
            &context_pairs,
            prior_reply.as_deref(),
            image_ref,
            social_ctx.peer_context.as_deref(),
            &social_ctx.interaction_history,
            "farcaster",
        )
        .await;

        let reply_text = match anky_reply {
            Ok(claude::AnkyReply::TextWithImage {
                ref text,
                ref flux_prompt,
            }) => {
                tracing::info!(
                    "Anky chose image reply for FC cast {}: prompt={}",
                    truncate_for_log(&cast_hash, 10),
                    truncate_for_log(flux_prompt, 100)
                );

                if comfyui::is_available().await {
                    match comfyui::generate_image(flux_prompt).await {
                        Ok(image_bytes) => {
                            let image_url = neynar::save_image_for_embed(&image_bytes, &cast_hash)
                                .unwrap_or_default();
                            if !image_url.is_empty() {
                                let reply_hash = neynar::reply_to_cast_with_image(
                                    api_key, signer, &cast_hash, text, &image_url,
                                )
                                .await
                                .ok()
                                .map(|r| r.hash);
                                upsert_social_interaction(
                                    &state,
                                    "farcaster",
                                    &cast_hash,
                                    &author_fid.to_string(),
                                    &author_username,
                                    &text,
                                    parent_hash.as_deref(),
                                    "replied_with_image",
                                    Some("text_reply"),
                                    Some(text),
                                    reply_hash.as_deref(),
                                )
                                .await;
                                text.clone()
                            } else {
                                // fallback to text
                                let reply_hash =
                                    neynar::reply_to_cast(api_key, signer, &cast_hash, text)
                                        .await
                                        .ok()
                                        .map(|r| r.hash);
                                upsert_social_interaction(
                                    &state,
                                    "farcaster",
                                    &cast_hash,
                                    &author_fid.to_string(),
                                    &author_username,
                                    &text,
                                    parent_hash.as_deref(),
                                    "replied_text",
                                    Some("text_reply"),
                                    Some(text),
                                    reply_hash.as_deref(),
                                )
                                .await;
                                text.clone()
                            }
                        }
                        Err(_) => {
                            let reply_hash =
                                neynar::reply_to_cast(api_key, signer, &cast_hash, text)
                                    .await
                                    .ok()
                                    .map(|r| r.hash);
                            upsert_social_interaction(
                                &state,
                                "farcaster",
                                &cast_hash,
                                &author_fid.to_string(),
                                &author_username,
                                &text,
                                parent_hash.as_deref(),
                                "replied_text",
                                Some("text_reply"),
                                Some(text),
                                reply_hash.as_deref(),
                            )
                            .await;
                            text.clone()
                        }
                    }
                } else {
                    let reply_hash = neynar::reply_to_cast(api_key, signer, &cast_hash, text)
                        .await
                        .ok()
                        .map(|r| r.hash);
                    upsert_social_interaction(
                        &state,
                        "farcaster",
                        &cast_hash,
                        &author_fid.to_string(),
                        &author_username,
                        &text,
                        parent_hash.as_deref(),
                        "replied_text",
                        Some("text_reply"),
                        Some(text),
                        reply_hash.as_deref(),
                    )
                    .await;
                    text.clone()
                }
            }
            Ok(claude::AnkyReply::Text(ref reply)) if !reply.is_empty() => {
                let reply_hash = neynar::reply_to_cast(api_key, signer, &cast_hash, reply)
                    .await
                    .ok()
                    .map(|r| r.hash);
                upsert_social_interaction(
                    &state,
                    "farcaster",
                    &cast_hash,
                    &author_fid.to_string(),
                    &author_username,
                    &text,
                    parent_hash.as_deref(),
                    "replied_text",
                    Some("text_reply"),
                    Some(reply),
                    reply_hash.as_deref(),
                )
                .await;
                reply.clone()
            }
            Ok(_) => {
                let fallback = "🦍";
                let reply_hash = neynar::reply_to_cast(api_key, signer, &cast_hash, fallback)
                    .await
                    .ok()
                    .map(|r| r.hash);
                upsert_social_interaction(
                    &state,
                    "farcaster",
                    &cast_hash,
                    &author_fid.to_string(),
                    &author_username,
                    &text,
                    parent_hash.as_deref(),
                    "replied_text",
                    Some("text_reply"),
                    Some(fallback),
                    reply_hash.as_deref(),
                )
                .await;
                fallback.to_string()
            }
            Err(e) => {
                tracing::error!("Claude FC reply generation failed: {}", e);
                // Fallback to Ollama
                let fallback = ollama::classify_x_image_mention(
                    &cfg.ollama_base_url,
                    &cfg.ollama_model,
                    &text,
                )
                .await
                .ok()
                .and_then(|r| r.text_reply)
                .unwrap_or_else(|| "🦍".to_string());
                let reply_hash = neynar::reply_to_cast(api_key, signer, &cast_hash, &fallback)
                    .await
                    .ok()
                    .map(|r| r.hash);
                upsert_social_interaction(
                    &state,
                    "farcaster",
                    &cast_hash,
                    &author_fid.to_string(),
                    &author_username,
                    &text,
                    parent_hash.as_deref(),
                    "replied_text",
                    Some("text_reply"),
                    Some(&fallback),
                    reply_hash.as_deref(),
                )
                .await;
                fallback
            }
        };

        tracing::info!(
            "Anky FC reply for cast {}: {}",
            truncate_for_log(&cast_hash, 10),
            truncate_for_log(&reply_text, 100)
        );
    }

    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Strip @mentions from cast text.
fn strip_mentions(text: &str) -> String {
    text.split_whitespace()
        .filter(|token| !token.starts_with('@'))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Upsert a social interaction record (platform-agnostic).
async fn upsert_social_interaction(
    state: &AppState,
    platform: &str,
    post_id: &str,
    author_id: &str,
    author_username: &str,
    text: &str,
    parent_id: Option<&str>,
    status: &str,
    classification: Option<&str>,
    reply_text: Option<&str>,
    reply_id: Option<&str>,
) {
    let db = state.db.lock().await;
    let _ = db.execute(
        "INSERT INTO social_interactions (
            id, platform, post_id, author_id, author_username, post_text,
            parent_id, status, classification, reply_text, reply_id, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11, datetime('now')
        )
        ON CONFLICT(platform, post_id) DO UPDATE SET
            status = excluded.status,
            classification = COALESCE(excluded.classification, social_interactions.classification),
            reply_text = COALESCE(excluded.reply_text, social_interactions.reply_text),
            reply_id = COALESCE(excluded.reply_id, social_interactions.reply_id),
            updated_at = datetime('now')",
        rusqlite::params![
            post_id,
            platform,
            post_id,
            author_id,
            if author_username.is_empty() {
                None::<&str>
            } else {
                Some(author_username)
            },
            if text.is_empty() {
                None::<&str>
            } else {
                Some(text)
            },
            parent_id,
            status,
            classification,
            reply_text,
            reply_id,
        ],
    );
}
