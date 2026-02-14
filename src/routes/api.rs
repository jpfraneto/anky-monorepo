use crate::db::queries;
use crate::error::AppError;
use crate::middleware::api_auth::ApiKeyInfo;
use crate::middleware::x402;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde_json::json;
use std::convert::Infallible;

/// GET /api/v1/anky/{id} — fetch anky details (for polling after /write)
/// Writing text is only included if the requester's anky_user_id cookie matches the anky's owner.
pub async fn get_anky(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let viewer_id = jar.get("anky_user_id").map(|c| c.value().to_string());

    let anky = {
        let db = state.db.lock().await;
        queries::get_anky_by_id(&db, &id)?
    };

    match anky {
        Some(detail) => {
            let image_url = detail.image_path.as_ref().map(|p| format!("https://anky.app/data/images/{}", p));
            let url = format!("https://anky.app/anky/{}", detail.id);

            // Only show writing to the owner
            let writing = if detail.origin == "written" {
                let db = state.db.lock().await;
                let owner = queries::get_anky_owner(&db, &id)?;
                let is_owner = viewer_id.as_deref().is_some()
                    && owner.as_deref() == viewer_id.as_deref();
                if is_owner {
                    detail.writing_text
                } else {
                    None
                }
            } else {
                detail.writing_text
            };

            Ok(Json(json!({
                "id": detail.id,
                "status": detail.status,
                "title": detail.title,
                "reflection": detail.reflection,
                "image_url": image_url,
                "image_prompt": detail.image_prompt,
                "writing": writing,
                "url": url,
                "created_at": detail.created_at,
                "origin": detail.origin,
            })))
        }
        None => Err(AppError::NotFound(format!("anky {} not found", id))),
    }
}

#[derive(serde::Deserialize, Default)]
pub struct ListAnkysQuery {
    pub origin: Option<String>,
}

pub async fn list_ankys(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<ListAnkysQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ankys = {
        let db = state.db.lock().await;
        crate::db::queries::get_all_ankys(&db)?
    };

    let data: Vec<serde_json::Value> = ankys
        .iter()
        .filter(|a| {
            let origin_filter = query.origin.as_deref().unwrap_or("generated");
            a.origin == origin_filter
        })
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "title": a.title,
                "image_path": a.image_path.as_ref().map(|p| format!("/data/images/{}", p)),
                "thinker_name": a.thinker_name,
                "status": a.status,
                "created_at": a.created_at,
                "origin": a.origin,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "ankys": data })))
}

#[derive(serde::Deserialize)]
pub struct GenerateAnkyRequest {
    pub thinker_name: String,
    pub moment: String,
}

pub async fn generate_anky(
    State(state): State<AppState>,
    Json(req): Json<GenerateAnkyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.emit_log(
        "INFO",
        "api",
        &format!("API generate request: {} — {}", req.thinker_name, req.moment),
    );

    let state_clone = state.clone();
    let name = req.thinker_name.clone();
    let moment = req.moment.clone();

    let anky_id = tokio::spawn(async move {
        crate::pipeline::stream_gen::generate_for_thinker(&state_clone, &name, &moment, None, None).await
    })
    .await
    .map_err(|e| AppError::Internal(format!("Spawn error: {}", e)))?
    .map_err(|e| AppError::Internal(format!("Generation error: {}", e)))?;

    Ok(Json(serde_json::json!({
        "anky_id": anky_id,
        "status": "generating",
    })))
}

const GENERATE_COST_USD: f64 = 0.25;

#[derive(serde::Deserialize)]
pub struct CheckPromptRequest {
    pub writing: String,
}

/// POST /api/v1/check-prompt — classify a prompt before payment
pub async fn check_prompt(
    State(state): State<AppState>,
    Json(req): Json<CheckPromptRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let api_key = &state.config.anthropic_api_key;
    if api_key.is_empty() {
        return Err(AppError::Internal("API key not configured".into()));
    }

    let classification = crate::services::claude::classify_and_enhance_prompt(api_key, &req.writing)
        .await
        .map_err(|e| AppError::Internal(format!("Classification failed: {}", e)))?;

    if classification.is_image_request {
        Ok(Json(json!({
            "status": "ready",
            "enhanced_prompt": classification.enhanced_prompt.unwrap_or_default(),
        })))
    } else {
        Ok(Json(json!({
            "status": "needs_revision",
            "message": classification.feedback.unwrap_or_else(|| "Please describe a visual scene or concept for your Anky image.".into()),
        })))
    }
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum PaidGenerateRequest {
    Direct { writing: String, enhanced_prompt: Option<String> },
    Thinker { thinker_name: String, moment: String },
}

/// POST /api/v1/generate — paid anky generation
/// Payment flow:
///   1. API key with free agent sessions → free
///   2. API key with balance >= $0.10 → deduct
///   3. PAYMENT-SIGNATURE header → verify via x402 facilitator
///   4. Nothing → 402 Payment Required
pub async fn generate_anky_paid(
    State(state): State<AppState>,
    headers: HeaderMap,
    api_key_info: Option<axum::Extension<ApiKeyInfo>>,
    Json(req): Json<PaidGenerateRequest>,
) -> Result<Response, AppError> {
    let resource_url = "https://anky.app/api/v1/generate";
    let mut payment_method = String::new();
    let mut tx_hash: Option<String> = None;
    let mut api_key_str: Option<String> = None;
    let mut agent_id: Option<String> = None;

    if let Some(axum::Extension(ref key_info)) = api_key_info {
        api_key_str = Some(key_info.key.clone());

        // Check if this is an agent with free sessions
        let db = state.db.lock().await;
        if let Ok(Some(agent)) = queries::get_agent_by_key(&db, &key_info.key) {
            if agent.free_sessions_remaining > 0 {
                queries::decrement_free_session(&db, &agent.id)?;
                payment_method = "free_session".into();
                agent_id = Some(agent.id);
                drop(db);
            } else {
                drop(db);
                // Fall through to balance check
            }
        } else {
            drop(db);
        }

        // If not free, try balance deduction
        if payment_method.is_empty() {
            if key_info.balance_usd >= GENERATE_COST_USD {
                let db = state.db.lock().await;
                queries::deduct_balance(&db, &key_info.key, GENERATE_COST_USD)?;
                drop(db);
                payment_method = "balance".into();
            } else {
                // API key present but insufficient balance — still try x402 below
            }
        }
    }

    // If no API key payment, check for payment header
    if payment_method.is_empty() {
        if let Some(sig) = headers
            .get("payment-signature")
            .or_else(|| headers.get("x-payment"))
            .and_then(|v| v.to_str().ok())
        {
            let sig = sig.trim();
            // Raw tx hash from wallet (0x + 64 hex chars) — accept directly
            if sig.starts_with("0x") && sig.len() == 66 && sig[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                state.emit_log("INFO", "payment", &format!("Direct wallet payment: {}", sig));
                tx_hash = Some(sig.to_string());
                payment_method = "wallet".into();
            } else {
                // x402 facilitator flow
                let facilitator = &state.config.x402_facilitator_url;
                if facilitator.is_empty() {
                    return Err(AppError::Internal(
                        "x402 facilitator not configured".into(),
                    ));
                }
                match x402::verify_x402_payment(facilitator, sig, resource_url).await {
                    Ok(hash) => {
                        tx_hash = Some(hash);
                        payment_method = "x402".into();
                    }
                    Err(reason) => {
                        return Ok((
                            axum::http::StatusCode::PAYMENT_REQUIRED,
                            Json(json!({
                                "error": "payment verification failed",
                                "reason": reason
                            })),
                        )
                            .into_response());
                    }
                }
            }
        }
    }

    // No payment at all → return 402
    if payment_method.is_empty() {
        return Ok(x402::payment_required_response(
            &state.config.treasury_address,
            resource_url,
        ));
    }

    // Payment accepted — start generation
    let gen_record_id = uuid::Uuid::new_v4().to_string();

    state.emit_log(
        "INFO",
        "api",
        &format!(
            "Paid generate request (method={}): {:?}",
            payment_method,
            match &req {
                PaidGenerateRequest::Direct { .. } => "direct writing",
                PaidGenerateRequest::Thinker { thinker_name, .. } => thinker_name.as_str(),
            }
        ),
    );

    let state_clone = state.clone();
    let payment_method_clone = payment_method.clone();
    let gen_id = gen_record_id.clone();
    let api_key_for_record = api_key_str.clone();
    let agent_id_for_record = agent_id.clone();
    let tx_hash_for_record = tx_hash.clone();

    // Create the anky record synchronously, then spawn background generation
    let anky_id = match req {
        PaidGenerateRequest::Direct { ref writing, ref enhanced_prompt } => {
            let session_id = uuid::Uuid::new_v4().to_string();
            let anky_id = uuid::Uuid::new_v4().to_string();
            let user_id = "api-user";
            let word_count = writing.split_whitespace().count() as i32;

            {
                let db = state.db.lock().await;
                queries::ensure_user(&db, user_id)?;
                queries::insert_writing_session(
                    &db,
                    &session_id,
                    user_id,
                    writing,
                    480.0,
                    word_count,
                    true,
                    None,
                )?;
                queries::insert_anky(
                    &db,
                    &anky_id,
                    &session_id,
                    user_id,
                    None, None, None, None, None, None, None,
                    "generating",
                    "generated",
                )?;
            }

            let sc = state_clone.clone();
            let aid = anky_id.clone();
            let w = writing.clone();
            let ep = enhanced_prompt.clone();
            tokio::spawn(async move {
                if let Err(e) = crate::pipeline::image_gen::generate_image_only(&sc, &aid, &w, ep.as_deref()).await {
                    tracing::error!("Generation failed for {}: {}", &aid[..8], e);
                    sc.emit_log("ERROR", "image_gen", &format!("Generation failed for {}: {}", &aid[..8], e));
                    let db = sc.db.lock().await;
                    let _ = queries::mark_anky_failed(&db, &aid);
                }
            });

            anky_id
        }
        PaidGenerateRequest::Thinker {
            ref thinker_name,
            ref moment,
        } => {
            // Pre-create anky record so we can return the ID immediately
            let anky_id = uuid::Uuid::new_v4().to_string();
            let placeholder_session = uuid::Uuid::new_v4().to_string();

            {
                let db = state.db.lock().await;
                queries::ensure_user(&db, "system")?;
                queries::insert_anky(
                    &db,
                    &anky_id,
                    &placeholder_session,
                    "system",
                    None, None, None, None, None,
                    Some(thinker_name),
                    Some(moment),
                    "generating",
                    "generated",
                )?;
            }

            let sc = state_clone.clone();
            let aid = anky_id.clone();
            let name = thinker_name.clone();
            let mom = moment.clone();
            tokio::spawn(async move {
                match crate::pipeline::stream_gen::generate_for_thinker(&sc, &name, &mom, None, Some(&aid)).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Thinker generation failed for {}: {}", &aid[..8], e);
                        sc.emit_log("ERROR", "stream_gen", &format!("Thinker generation failed for {}: {}", &aid[..8], e));
                        let db = sc.db.lock().await;
                        let _ = queries::mark_anky_failed(&db, &aid);
                    }
                }
            });

            anky_id
        }
    };

    // Record generation
    {
        let db = state.db.lock().await;
        let _ = queries::insert_generation_record(
            &db,
            &gen_id,
            &anky_id,
            api_key_for_record.as_deref(),
            agent_id_for_record.as_deref(),
            &payment_method_clone,
            if payment_method_clone == "free_session" {
                0.0
            } else {
                GENERATE_COST_USD
            },
            tx_hash_for_record.as_deref(),
        );
    }

    let url = format!("https://anky.app/anky/{}", anky_id);

    let response = json!({
        "anky_id": anky_id,
        "status": "generating",
        "payment_method": payment_method,
        "url": url,
    });

    Ok(Json(response).into_response())
}

// --- Chat with Anky ---
#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub anky_id: String,
    pub message: String,
    #[serde(default)]
    pub history: Vec<ChatMessage>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub async fn chat_with_anky(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<ChatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let viewer_id = jar.get("anky_user_id").map(|c| c.value().to_string());

    let anky = {
        let db = state.db.lock().await;
        queries::get_anky_by_id(&db, &req.anky_id)?
    };

    let anky = anky.ok_or_else(|| AppError::NotFound("anky not found".into()))?;

    // Verify ownership
    if let Some(ref vid) = viewer_id {
        let db = state.db.lock().await;
        let owner = queries::get_anky_owner(&db, &req.anky_id)?;
        if owner.as_deref() != Some(vid.as_str()) {
            return Err(AppError::BadRequest("not your anky".into()));
        }
    } else {
        return Err(AppError::BadRequest("not authenticated".into()));
    }

    let api_key = &state.config.anthropic_api_key;
    if api_key.is_empty() {
        return Err(AppError::Internal("API key not configured".into()));
    }

    let writing = anky.writing_text.as_deref().unwrap_or("");
    let reflection = anky.reflection.as_deref().unwrap_or("");

    let history: Vec<(String, String)> = req.history
        .iter()
        .map(|m| (m.role.clone(), m.content.clone()))
        .collect();

    let result = crate::services::claude::chat_about_writing(
        api_key, writing, reflection, &history, &req.message,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Chat failed: {}", e)))?;

    Ok(Json(json!({
        "response": result.text,
    })))
}

// --- Quick Chat (Ollama, for non-anky sessions) ---
#[derive(serde::Deserialize)]
pub struct QuickChatRequest {
    pub writing: String,
    pub message: String,
    #[serde(default)]
    pub history: Vec<ChatMessage>,
}

pub async fn chat_quick(
    State(state): State<AppState>,
    Json(req): Json<QuickChatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    use crate::services::ollama::{OllamaChatMessage, chat_ollama};

    let mut messages = vec![
        OllamaChatMessage {
            role: "system".into(),
            content: format!(
                "You are Anky, a consciousness companion. The user wrote a stream of consciousness session (less than 8 minutes). Be warm, direct, insightful. Reference their writing. Keep responses concise (2-3 paragraphs). Help them see patterns and encourage them to write for the full 8 minutes next time.\n\nTheir writing:\n{}",
                req.writing
            ),
        },
    ];

    // Add conversation history
    for msg in &req.history {
        messages.push(OllamaChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
    }

    // Add the new message
    messages.push(OllamaChatMessage {
        role: "user".into(),
        content: req.message.clone(),
    });

    let response = chat_ollama(
        &state.config.ollama_base_url,
        "llama3.1:latest",
        messages,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Chat failed: {}", e)))?;

    Ok(Json(json!({ "response": response })))
}

// --- Feedback ---
#[derive(serde::Deserialize)]
pub struct FeedbackRequest {
    pub content: String,
    pub source: Option<String>,
    pub author: Option<String>,
}

pub async fn submit_feedback(
    State(state): State<AppState>,
    Json(req): Json<FeedbackRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let source = req.source.as_deref().unwrap_or("human");
    if source != "human" && source != "agent" {
        return Err(AppError::BadRequest("source must be 'human' or 'agent'".into()));
    }
    let db = state.db.lock().await;
    queries::insert_feedback(&db, &id, source, req.author.as_deref(), &req.content)?;
    drop(db);
    state.emit_log("INFO", "feedback", &format!("New feedback from {}: {}...", source, &req.content.chars().take(60).collect::<String>()));
    Ok(Json(json!({ "id": id, "saved": true })))
}

// --- Checkpoint ---
#[derive(serde::Deserialize)]
pub struct CheckpointRequest {
    pub session_id: String,
    pub text: String,
    pub elapsed: f64,
}

pub async fn save_checkpoint(
    State(state): State<AppState>,
    Json(req): Json<CheckpointRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let word_count = req.text.split_whitespace().count() as i32;
    let db = state.db.lock().await;
    queries::insert_checkpoint(&db, &req.session_id, &req.text, req.elapsed, word_count)?;
    drop(db);
    state.emit_log(
        "INFO",
        "checkpoint",
        &format!("Checkpoint saved: {} ({} words, {:.0}s)", &req.session_id, word_count, req.elapsed),
    );
    Ok(Json(json!({ "saved": true })))
}

// --- Cost Estimate ---
pub async fn cost_estimate(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({
        "cost_per_anky": GENERATE_COST_USD,
        "base_cost": GENERATE_COST_USD,
        "protocol_fee_pct": 0,
    })))
}

// --- Treasury Address ---
pub async fn treasury_address(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(json!({ "address": state.config.treasury_address }))
}

// --- Stream Reflection (SSE) ---
/// GET /api/stream-reflection/{id} — stream title+reflection from Claude via SSE.
/// If reflection already exists in DB, sends it immediately.
/// Otherwise, streams from Claude and saves to DB in the background.
pub async fn stream_reflection(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let (writing_text, existing_reflection, existing_title) = {
        let db = state.db.lock().await;
        let anky = queries::get_anky_by_id(&db, &id)?;
        match anky {
            Some(a) => (
                a.writing_text.unwrap_or_default(),
                a.reflection.clone(),
                a.title.clone(),
            ),
            None => return Err(AppError::NotFound("anky not found".into())),
        }
    };

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(64);

    let has_existing = existing_reflection.as_ref().map_or(false, |r| !r.is_empty());

    if has_existing {
        // Already have reflection — send it immediately
        let title = existing_title.unwrap_or_default();
        let refl = existing_reflection.unwrap_or_default();
        let full = format!("{}\n\n{}", title, refl);
        tokio::spawn(async move {
            let _ = tx.send(full).await;
        });
    } else if writing_text.is_empty() {
        drop(tx);
        return Err(AppError::BadRequest("no writing text found".into()));
    } else {
        let api_key = state.config.anthropic_api_key.clone();
        if api_key.is_empty() {
            drop(tx);
            return Err(AppError::Internal("API key not configured".into()));
        }

        let anky_id = id.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            match crate::services::claude::stream_title_and_reflection(&api_key, &writing_text, tx).await {
                Ok((full_text, input_tokens, output_tokens)) => {
                    let (title, reflection) = crate::services::claude::parse_title_reflection(&full_text);
                    let cost = crate::pipeline::cost::estimate_claude_cost(input_tokens, output_tokens);
                    let db = state_clone.db.lock().await;
                    if let Err(e) = queries::update_anky_title_reflection(&db, &anky_id, &title, &reflection) {
                        tracing::error!("Failed to save reflection for {}: {}", &anky_id[..8], e);
                    }
                    let _ = queries::insert_cost_record(&db, "claude", "claude-sonnet-4-20250514", input_tokens, output_tokens, cost, Some(&anky_id));
                    state_clone.emit_log("INFO", "stream", &format!("Streamed reflection saved for {} (${:.4})", &anky_id[..8], cost));
                }
                Err(e) => {
                    tracing::error!("Stream reflection failed for {}: {}", &anky_id[..8], e);
                    state_clone.emit_log("ERROR", "stream", &format!("Stream failed for {}: {}", &anky_id[..8], e));
                }
            }
        });
    }

    let stream = async_stream::stream! {
        while let Some(text) = rx.recv().await {
            yield Ok::<_, Infallible>(Event::default().data(text));
        }
        yield Ok::<_, Infallible>(Event::default().event("done").data(""));
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

// --- Retry Failed Ankys ---
pub async fn retry_failed(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let failed = {
        let db = state.db.lock().await;
        queries::get_failed_ankys(&db)?
    };

    if failed.is_empty() {
        return Ok(Json(json!({ "retried": 0, "message": "no failed ankys" })));
    }

    let count = failed.len();
    state.emit_log("INFO", "retry", &format!("Retrying {} failed ankys", count));

    for (anky_id, session_id, writing) in failed {
        let s = state.clone();
        let aid = anky_id.clone();
        let sid = session_id.clone();
        let text = writing.clone();
        tokio::spawn(async move {
            s.emit_log("INFO", "retry", &format!("Retrying anky {}", &aid[..8]));
            if let Err(e) = crate::pipeline::image_gen::generate_anky_from_writing(
                &s, &aid, &sid, "retry", &text,
            ).await {
                s.emit_log("ERROR", "retry", &format!("Retry failed for {}: {}", &aid[..8], e));
                let db = s.db.lock().await;
                let _ = queries::mark_anky_failed(&db, &aid);
            }
        });
    }

    Ok(Json(json!({ "retried": count })))
}
