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
use std::fs;

fn video_public_url(path: &str) -> String {
    if let Some(rel) = path.strip_prefix("videos/") {
        format!("/videos/{}", rel)
    } else if let Some(rel) = path.strip_prefix("data/videos/") {
        format!("/data/videos/{}", rel)
    } else {
        format!("/videos/{}", path.trim_start_matches('/'))
    }
}

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
            let image_url = detail
                .image_path
                .as_ref()
                .map(|p| format!("https://anky.app/data/images/{}", p));
            let url = format!("https://anky.app/anky/{}", detail.id);

            // Only show writing to the owner
            let writing = if detail.origin == "written" {
                let db = state.db.lock().await;
                let owner = queries::get_anky_owner(&db, &id)?;
                let is_owner =
                    viewer_id.as_deref().is_some() && owner.as_deref() == viewer_id.as_deref();
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
                "image_model": detail.image_model,
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
        &format!(
            "API generate request: {} — {}",
            req.thinker_name, req.moment
        ),
    );

    let state_clone = state.clone();
    let name = req.thinker_name.clone();
    let moment = req.moment.clone();

    let anky_id = tokio::spawn(async move {
        crate::pipeline::stream_gen::generate_for_thinker(&state_clone, &name, &moment, None, None)
            .await
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
    let classification = crate::services::ollama::classify_and_enhance_prompt(
        &state.config.ollama_base_url,
        &state.config.ollama_model,
        &req.writing,
    )
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
pub struct PaidGenerateRequest {
    /// "flux" (default, free) or "gemini" (paid)
    pub model: Option<String>,
    // Direct prompt fields
    pub writing: Option<String>,
    pub enhanced_prompt: Option<String>,
    // Thinker portrait fields
    pub thinker_name: Option<String>,
    pub moment: Option<String>,
}

/// POST /api/v1/generate — anky generation
/// Model routing:
///   model="flux" (default) → Flux.1-dev + anky LoRA via ComfyUI, FREE
///   model="gemini"         → Gemini image pipeline, PAID ($0.25)
/// Payment (only required for gemini):
///   1. API key with free agent sessions → free
///   2. PAYMENT-SIGNATURE / x-payment header → wallet tx hash or x402
///   3. Nothing → 402 Payment Required
pub async fn generate_anky_paid(
    State(state): State<AppState>,
    headers: HeaderMap,
    api_key_info: Option<axum::Extension<ApiKeyInfo>>,
    Json(req): Json<PaidGenerateRequest>,
) -> Result<Response, AppError> {
    let resource_url = "https://anky.app/api/v1/generate";
    let use_flux = req.model.as_deref().unwrap_or("flux") != "gemini";

    let mut payment_method = String::new();
    let mut tx_hash: Option<String> = None;
    let mut api_key_str: Option<String> = None;
    let mut agent_id: Option<String> = None;

    if use_flux {
        // Flux is always free — check ComfyUI is available
        if !crate::services::comfyui::is_available().await {
            return Err(AppError::Internal(
                "Flux image server is not ready yet. Try again in a moment.".into(),
            ));
        }

        // Validate prompt with Ollama: must be about Anky
        let prompt_text = req
            .writing
            .as_deref()
            .or(req.thinker_name.as_deref())
            .unwrap_or("");
        let ollama_url = &state.config.ollama_base_url;
        if !crate::services::ollama::is_anky_prompt(ollama_url, prompt_text).await {
            return Ok((
                axum::http::StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "anky flux only generates images of Anky. your prompt doesn't seem to be about Anky — describe what Anky is doing, feeling, or becoming."
                })),
            ).into_response());
        }

        payment_method = "flux_free".into();
    } else {
        // Gemini requires payment
        if let Some(axum::Extension(ref key_info)) = api_key_info {
            api_key_str = Some(key_info.key.clone());
            let db = state.db.lock().await;
            if let Ok(Some(agent)) = queries::get_agent_by_key(&db, &key_info.key) {
                if agent.free_sessions_remaining > 0 {
                    queries::decrement_free_session(&db, &agent.id)?;
                    payment_method = "free_session".into();
                    agent_id = Some(agent.id);
                    drop(db);
                } else {
                    drop(db);
                }
            } else {
                drop(db);
            }
        }

        if payment_method.is_empty() {
            if let Some(sig) = headers
                .get("payment-signature")
                .or_else(|| headers.get("x-payment"))
                .and_then(|v| v.to_str().ok())
            {
                let sig = sig.trim();
                if sig.starts_with("0x")
                    && sig.len() == 66
                    && sig[2..].chars().all(|c| c.is_ascii_hexdigit())
                {
                    state.emit_log(
                        "INFO",
                        "payment",
                        &format!("Direct wallet payment: {}", sig),
                    );
                    tx_hash = Some(sig.to_string());
                    payment_method = "wallet".into();
                } else {
                    let facilitator = &state.config.x402_facilitator_url;
                    if facilitator.is_empty() {
                        return Err(AppError::Internal("x402 facilitator not configured".into()));
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

        if payment_method.is_empty() {
            return Ok(x402::payment_required_response(
                &state.config.treasury_address,
                resource_url,
            ));
        }
    }

    // Determine generation mode (thinker or direct writing)
    let is_thinker = req.thinker_name.is_some();

    let gen_record_id = uuid::Uuid::new_v4().to_string();
    state.emit_log(
        "INFO",
        "api",
        &format!(
            "Generate request (model={}, method={}): {}",
            if use_flux { "flux" } else { "gemini" },
            payment_method,
            if is_thinker {
                req.thinker_name.as_deref().unwrap_or("thinker")
            } else {
                "direct writing"
            }
        ),
    );

    let state_clone = state.clone();
    let payment_method_clone = payment_method.clone();
    let gen_id = gen_record_id.clone();
    let api_key_for_record = api_key_str.clone();
    let agent_id_for_record = agent_id.clone();
    let tx_hash_for_record = tx_hash.clone();

    let anky_id = if is_thinker {
        let thinker_name = req.thinker_name.clone().unwrap_or_default();
        let moment = req
            .moment
            .clone()
            .unwrap_or_else(|| "a quiet moment of deep thought".into());

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
                None,
                None,
                None,
                None,
                None,
                Some(&thinker_name),
                Some(&moment),
                "generating",
                "generated",
            )?;
        }

        let sc = state_clone.clone();
        let aid = anky_id.clone();
        let name = thinker_name.clone();
        let mom = moment.clone();
        tokio::spawn(async move {
            let result = if use_flux {
                // Build a simple prompt for thinker + Flux
                let prompt = format!("{} — {}", name, mom);
                crate::pipeline::image_gen::generate_image_only_flux(&sc, &aid, &prompt).await
            } else {
                crate::pipeline::stream_gen::generate_for_thinker(
                    &sc,
                    &name,
                    &mom,
                    None,
                    Some(&aid),
                )
                .await
                .map(|_| ())
            };
            if let Err(e) = result {
                tracing::error!("Thinker generation failed for {}: {}", &aid[..8], e);
                sc.emit_log("ERROR", "gen", &format!("Thinker generation failed: {}", e));
                let db = sc.db.lock().await;
                let _ = queries::mark_anky_failed(&db, &aid);
            }
        });

        anky_id
    } else {
        let writing = req.writing.clone().unwrap_or_default();

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
                &writing,
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
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                "generating",
                "generated",
            )?;
        }

        let sc = state_clone.clone();
        let aid = anky_id.clone();
        let w = writing.clone();
        tokio::spawn(async move {
            let result = if use_flux {
                crate::pipeline::image_gen::generate_image_only_flux(&sc, &aid, &w).await
            } else {
                crate::pipeline::image_gen::generate_image_only(&sc, &aid, &w, Some(&w)).await
            };
            if let Err(e) = result {
                tracing::error!("Generation failed for {}: {}", &aid[..8], e);
                sc.emit_log(
                    "ERROR",
                    "image_gen",
                    &format!("Generation failed for {}: {}", &aid[..8], e),
                );
                let db = sc.db.lock().await;
                let _ = queries::mark_anky_failed(&db, &aid);
            }
        });

        anky_id
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

    let writing = anky.writing_text.as_deref().unwrap_or("");
    let reflection = anky.reflection.as_deref().unwrap_or("");

    let mut messages: Vec<crate::services::ollama::OllamaChatMessage> = vec![
        crate::services::ollama::OllamaChatMessage {
            role: "system".into(),
            content: format!(
                "You are Anky, a consciousness companion continuing a conversation. The user just did a stream-of-consciousness writing session. You already reflected on their writing.\n\nBe warm, direct, precise. Reference their writing when relevant. Ask the question that cuts deepest. 2-3 paragraphs max.\n\nTHEIR WRITING:\n{}\n\nYOUR REFLECTION:\n{}",
                writing, reflection
            ),
        },
    ];
    for m in &req.history {
        messages.push(crate::services::ollama::OllamaChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        });
    }
    messages.push(crate::services::ollama::OllamaChatMessage {
        role: "user".into(),
        content: req.message.clone(),
    });

    let response_text = crate::services::ollama::chat_ollama(
        &state.config.ollama_base_url,
        &state.config.ollama_model,
        messages,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Chat failed: {}", e)))?;

    // Save updated conversation to DB
    let mut full_history: Vec<serde_json::Value> = req
        .history
        .iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();
    full_history.push(json!({"role": "user", "content": req.message}));
    full_history.push(json!({"role": "assistant", "content": response_text}));
    let conv_json = serde_json::to_string(&json!({
        "messages": full_history,
    }))
    .unwrap_or_default();
    {
        let db = state.db.lock().await;
        let _ = queries::update_anky_conversation(&db, &req.anky_id, &conv_json);
    }

    Ok(Json(json!({
        "response": response_text,
    })))
}

// --- Suggest Replies ---
#[derive(serde::Deserialize)]
pub struct SuggestRepliesRequest {
    pub anky_id: String,
    #[serde(default)]
    pub history: Vec<ChatMessage>,
}

pub async fn suggest_replies(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<SuggestRepliesRequest>,
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

    let writing = anky.writing_text.as_deref().unwrap_or("");
    let reflection = anky.reflection.as_deref().unwrap_or("");

    // Check if replies were pre-generated during the reflection stream
    let cached = anky.conversation_json.as_deref().and_then(|j| {
        let v: serde_json::Value = serde_json::from_str(j).ok()?;
        let r1 = v["pending_replies"][0].as_str()?.to_string();
        let r2 = v["pending_replies"][1].as_str()?.to_string();
        // Only use cache if no conversation history yet (first request)
        if req.history.is_empty() {
            Some((r1, r2))
        } else {
            None
        }
    });

    let (reply1, reply2) = if let Some(cached_replies) = cached {
        cached_replies
    } else {
        let history: Vec<(String, String)> = req
            .history
            .iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect();
        crate::services::ollama::generate_suggested_replies(
            &state.config.ollama_base_url,
            &state.config.ollama_model,
            writing,
            reflection,
            &history,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Suggest replies failed: {}", e)))?
    };

    // Save conversation state with pending suggestions
    let messages: Vec<serde_json::Value> = req
        .history
        .iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();
    let conv_json = serde_json::to_string(&json!({
        "messages": messages,
        "pending_replies": [reply1, reply2],
    }))
    .unwrap_or_default();
    {
        let db = state.db.lock().await;
        let _ = queries::update_anky_conversation(&db, &req.anky_id, &conv_json);
    }

    Ok(Json(json!({
        "reply1": reply1,
        "reply2": reply2,
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
    use crate::services::ollama::{chat_ollama, OllamaChatMessage};

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
        &state.config.ollama_model,
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
        return Err(AppError::BadRequest(
            "source must be 'human' or 'agent'".into(),
        ));
    }
    let db = state.db.lock().await;
    queries::insert_feedback(&db, &id, source, req.author.as_deref(), &req.content)?;
    drop(db);
    state.emit_log(
        "INFO",
        "feedback",
        &format!(
            "New feedback from {}: {}...",
            source,
            &req.content.chars().take(60).collect::<String>()
        ),
    );
    Ok(Json(json!({ "id": id, "saved": true })))
}

fn require_user_id(jar: &CookieJar) -> Result<String, AppError> {
    jar.get("anky_user_id")
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::Unauthorized("no user id".into()))
}

fn persist_checkpoint_record(
    conn: &rusqlite::Connection,
    session_id: &str,
    text: &str,
    elapsed: f64,
    session_token: Option<&str>,
) -> Result<String, AppError> {
    let word_count = text.split_whitespace().count() as i32;

    let prev = queries::get_latest_checkpoint(conn, session_id)?;
    let token = if let Some(ref prev) = prev {
        if elapsed < prev.elapsed_seconds {
            return Err(AppError::BadRequest("elapsed time cannot decrease".into()));
        }
        if let Some(ref prev_token) = prev.session_token {
            match session_token {
                Some(t) if t == prev_token => t.to_string(),
                Some(_) => return Err(AppError::BadRequest("session token mismatch".into())),
                None => return Err(AppError::BadRequest("session token required".into())),
            }
        } else {
            session_token.unwrap_or_default().to_string()
        }
    } else {
        uuid::Uuid::new_v4().to_string()
    };

    queries::insert_checkpoint(conn, session_id, text, elapsed, word_count, Some(&token))?;
    Ok(token)
}

// --- Checkpoint ---
#[derive(serde::Deserialize)]
pub struct CheckpointRequest {
    pub session_id: String,
    pub text: String,
    pub elapsed: f64,
    #[serde(default)]
    pub session_token: Option<String>,
}

pub async fn save_checkpoint(
    State(state): State<AppState>,
    Json(req): Json<CheckpointRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let word_count = req.text.split_whitespace().count() as i32;
    let db = state.db.lock().await;
    let token = persist_checkpoint_record(
        &db,
        &req.session_id,
        &req.text,
        req.elapsed,
        req.session_token.as_deref(),
    )?;
    queries::update_checkpoint_backed_writing_session(
        &db,
        &req.session_id,
        &req.text,
        req.elapsed,
        word_count,
        Some(&token),
    )?;
    drop(db);
    state.emit_log(
        "INFO",
        "checkpoint",
        &format!(
            "Checkpoint saved: {} ({} words, {:.0}s)",
            &req.session_id, word_count, req.elapsed
        ),
    );
    Ok(Json(json!({ "saved": true, "session_token": token })))
}

#[derive(serde::Deserialize)]
pub struct PauseWritingSessionRequest {
    pub session_id: String,
    pub text: String,
    pub elapsed: f64,
    #[serde(default)]
    pub session_token: Option<String>,
}

pub async fn pause_writing_session(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<PauseWritingSessionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.text.trim().is_empty() {
        return Err(AppError::BadRequest("cannot pause an empty session".into()));
    }

    let user_id = require_user_id(&jar)?;
    let word_count = req.text.split_whitespace().count() as i32;
    let db = state.db.lock().await;

    if let Some(existing) = queries::get_writing_session_state(&db, &req.session_id)? {
        if existing.user_id != user_id {
            return Err(AppError::Unauthorized(
                "that session belongs to another user".into(),
            ));
        }
        if existing.pause_used {
            return Err(AppError::BadRequest(
                "this session already used its pause".into(),
            ));
        }
        if existing.status == "completed" {
            return Err(AppError::BadRequest(
                "this session is already complete".into(),
            ));
        }
    }

    let token = persist_checkpoint_record(
        &db,
        &req.session_id,
        &req.text,
        req.elapsed,
        req.session_token.as_deref(),
    )?;
    queries::upsert_active_writing_session(
        &db,
        &req.session_id,
        &user_id,
        &req.text,
        req.elapsed,
        word_count,
        "paused",
        true,
        Some(&token),
    )?;
    drop(db);

    state.emit_log(
        "INFO",
        "writing",
        &format!("Paused session {} at {:.0}s", &req.session_id, req.elapsed),
    );

    Ok(Json(json!({
        "saved": true,
        "paused": true,
        "session_token": token,
    })))
}

pub async fn get_paused_writing_session(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = match jar.get("anky_user_id").map(|c| c.value().to_string()) {
        Some(uid) => uid,
        None => return Ok(Json(json!({ "paused_session": serde_json::Value::Null }))),
    };

    let db = state.db.lock().await;
    let session = queries::get_resumable_writing_session(&db, &user_id)?;

    Ok(Json(json!({
        "paused_session": session.map(|s| json!({
            "session_id": s.id,
            "text": s.content,
            "elapsed": s.duration_seconds,
            "word_count": s.word_count,
            "pause_used": s.pause_used,
            "status": s.status,
            "paused_at": s.paused_at,
            "resumed_at": s.resumed_at,
            "session_token": s.session_token,
        }))
    })))
}

#[derive(serde::Deserialize)]
pub struct ResumeWritingSessionRequest {
    pub session_id: String,
    pub text: String,
    pub elapsed: f64,
    #[serde(default)]
    pub session_token: Option<String>,
}

pub async fn resume_writing_session(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<ResumeWritingSessionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = require_user_id(&jar)?;
    let word_count = req.text.split_whitespace().count() as i32;
    let db = state.db.lock().await;

    let existing = queries::get_writing_session_state(&db, &req.session_id)?
        .ok_or_else(|| AppError::NotFound("paused session not found".into()))?;
    if existing.user_id != user_id {
        return Err(AppError::Unauthorized(
            "that session belongs to another user".into(),
        ));
    }
    if existing.status != "paused" && existing.status != "resumed" {
        return Err(AppError::BadRequest("that session is not resumable".into()));
    }

    queries::upsert_active_writing_session(
        &db,
        &req.session_id,
        &user_id,
        &req.text,
        req.elapsed,
        word_count,
        "resumed",
        true,
        req.session_token
            .as_deref()
            .or(existing.session_token.as_deref()),
    )?;
    drop(db);

    state.emit_log(
        "INFO",
        "writing",
        &format!("Resumed session {} at {:.0}s", &req.session_id, req.elapsed),
    );

    Ok(Json(json!({ "resumed": true })))
}

#[derive(serde::Deserialize)]
pub struct DiscardPausedSessionRequest {
    pub session_id: String,
}

pub async fn discard_paused_writing_session(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<DiscardPausedSessionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = require_user_id(&jar)?;
    let db = state.db.lock().await;
    queries::discard_resumable_writing_session(&db, &user_id, &req.session_id)?;
    drop(db);

    state.emit_log(
        "INFO",
        "writing",
        &format!("Discarded paused session {}", &req.session_id),
    );

    Ok(Json(json!({ "discarded": true })))
}

#[derive(serde::Deserialize)]
pub struct PrefetchMemoryRequest {
    pub text: String,
}

/// POST /api/prefetch-memory — pre-warm memory context during a writing session.
/// Called at ~5 minutes so the context is ready when the reflection is requested.
pub async fn prefetch_memory(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<PrefetchMemoryRequest>,
) -> Json<serde_json::Value> {
    let user_id = match jar.get("anky_user_id").map(|c| c.value().to_string()) {
        Some(uid) => uid,
        None => return Json(json!({ "ok": false, "reason": "no user id" })),
    };
    if req.text.split_whitespace().count() < 10 {
        return Json(json!({ "ok": false, "reason": "not enough text" }));
    }
    let db = state.db.clone();
    let ollama_url = state.config.ollama_base_url.clone();
    let cache = state.memory_cache.clone();
    let text = req.text.clone();
    let uid = user_id.clone();
    tokio::spawn(async move {
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            crate::memory::recall::build_memory_context(&db, &ollama_url, &uid, &text),
        )
        .await
        {
            Ok(Ok(ctx)) => {
                let formatted = ctx.format_for_prompt();
                let mut map = cache.lock().await;
                map.insert(uid.clone(), formatted);
                tracing::info!("memory pre-warmed for {}", &uid[..8.min(uid.len())]);
            }
            Ok(Err(e)) => tracing::warn!("prefetch_memory build error: {}", e),
            Err(_) => tracing::warn!("prefetch_memory timed out for {}", &uid[..8.min(uid.len())]),
        }
    });
    Json(json!({ "ok": true }))
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
pub async fn treasury_address(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({ "address": state.config.treasury_address }))
}

// --- Stream Reflection (SSE) ---
/// GET /api/stream-reflection/{id} — stream title+reflection from Claude via SSE.
/// If reflection already exists in DB, sends it immediately.
/// Otherwise, streams from Claude and saves to DB in the background.
pub async fn stream_reflection(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let user_id = jar.get("anky_user_id").map(|c| c.value().to_string());

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

    let has_existing = existing_reflection
        .as_ref()
        .map_or(false, |r| !r.is_empty());

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
        let ollama_url = state.config.ollama_base_url.clone();
        tokio::spawn(async move {
            // Check pre-warmed cache first (populated at minute 5 of writing session).
            // Fall back to building it now with a 5s timeout if not cached.
            let memory_ctx = if let Some(ref uid) = user_id {
                let cached = {
                    let mut cache = state_clone.memory_cache.lock().await;
                    cache.remove(uid)
                };
                if let Some(ctx) = cached {
                    tracing::info!("memory cache hit for {}", &uid[..8.min(uid.len())]);
                    Some(ctx)
                } else {
                    tracing::info!(
                        "memory cache miss for {}, building now",
                        &uid[..8.min(uid.len())]
                    );
                    tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        crate::memory::recall::build_memory_context(
                            &state_clone.db,
                            &ollama_url,
                            uid,
                            &writing_text,
                        ),
                    )
                    .await
                    .ok()
                    .and_then(|r| r.ok())
                    .map(|ctx| ctx.format_for_prompt())
                }
            } else {
                None
            };
            match crate::services::claude::stream_title_and_reflection(
                &api_key,
                &writing_text,
                tx,
                memory_ctx.as_deref(),
            )
            .await
            {
                Ok((full_text, input_tokens, output_tokens)) => {
                    let (title, reflection) =
                        crate::services::claude::parse_title_reflection(&full_text);
                    let cost =
                        crate::pipeline::cost::estimate_claude_cost(input_tokens, output_tokens);
                    {
                        let db = state_clone.db.lock().await;
                        if let Err(e) = queries::update_anky_title_reflection(
                            &db,
                            &anky_id,
                            &title,
                            &reflection,
                        ) {
                            tracing::error!(
                                "Failed to save reflection for {}: {}",
                                &anky_id[..8],
                                e
                            );
                        }
                        let _ = queries::insert_cost_record(
                            &db,
                            "claude",
                            "claude-sonnet-4-20250514",
                            input_tokens,
                            output_tokens,
                            cost,
                            Some(&anky_id),
                        );
                    }
                    state_clone.emit_log(
                        "INFO",
                        "stream",
                        &format!(
                            "Streamed reflection saved for {} (${:.4})",
                            &anky_id[..8],
                            cost
                        ),
                    );
                    // Proactively generate suggested replies in background so they're
                    // ready by the time the user finishes reading the reflection.
                    let sr_state = state_clone.clone();
                    let sr_anky_id = anky_id.clone();
                    let sr_writing = writing_text.clone();
                    let sr_reflection = reflection.clone();
                    tokio::spawn(async move {
                        match crate::services::ollama::generate_suggested_replies(
                            &sr_state.config.ollama_base_url,
                            &sr_state.config.ollama_model,
                            &sr_writing,
                            &sr_reflection,
                            &[],
                        )
                        .await
                        {
                            Ok((r1, r2)) => {
                                let conv_json = serde_json::to_string(&serde_json::json!({
                                    "messages": [],
                                    "pending_replies": [r1, r2],
                                }))
                                .unwrap_or_default();
                                let db = sr_state.db.lock().await;
                                let _ =
                                    queries::update_anky_conversation(&db, &sr_anky_id, &conv_json);
                                sr_state.emit_log(
                                    "INFO",
                                    "stream",
                                    &format!("Replies pre-generated for {}", &sr_anky_id[..8]),
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Pre-generating replies failed for {}: {}",
                                    &sr_anky_id[..8],
                                    e
                                );
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Stream reflection failed for {}: {}", &anky_id[..8], e);
                    state_clone.emit_log(
                        "ERROR",
                        "stream",
                        &format!("Stream failed for {}: {}", &anky_id[..8], e),
                    );
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
            )
            .await
            {
                s.emit_log(
                    "ERROR",
                    "retry",
                    &format!("Retry failed for {}: {}", &aid[..8], e),
                );
                let db = s.db.lock().await;
                let _ = queries::mark_anky_failed(&db, &aid);
            }
        });
    }

    Ok(Json(json!({ "retried": count })))
}

// ==================== Video Frame Generation ====================

const VIDEO_FRAME_COST_USD: f64 = 0.10;

#[derive(serde::Deserialize)]
pub struct VideoFrameRequest {
    pub prompt_id: String,
    pub prompt_text: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct CreateVideoRequest {
    pub prompt_id: String,
}

fn create_video_card_value(prompt_id: &str) -> Result<serde_json::Value, AppError> {
    let prompt = crate::create_videos::get_prompt(prompt_id)
        .ok_or_else(|| AppError::NotFound(format!("create-videos prompt {} not found", prompt_id)))?;
    let state = crate::create_videos::load_state(prompt_id)?;
    Ok(serde_json::to_value(state.to_card(&prompt))?)
}

async fn require_create_videos_user(state: &AppState, jar: &CookieJar) -> Result<String, AppError> {
    crate::routes::auth::get_auth_user(state, jar)
        .await
        .map(|user| user.user_id)
        .ok_or_else(|| AppError::Unauthorized("login required".into()))
}

fn persist_create_video_failure(
    prompt_id: &str,
    phase: &str,
    message: String,
) -> Result<(), AppError> {
    let mut state = crate::create_videos::load_state(prompt_id)?;
    match phase {
        "image" => {
            state.image_status = "failed".to_string();
            state.image_error = Some(message);
            if state.image_path.is_some() {
                state.video_status = if state.video_url.is_some() {
                    "complete".to_string()
                } else {
                    "ready".to_string()
                };
            } else {
                state.video_status = "locked".to_string();
            }
        }
        "video" => {
            state.video_status = "failed".to_string();
            state.video_error = Some(message);
        }
        _ => {}
    }
    state.touch();
    crate::create_videos::save_state(&state)?;
    Ok(())
}

/// GET /api/v1/create-videos/{id} — fetch prompt state for the marketing video creator.
pub async fn get_create_video_card(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({
        "card": create_video_card_value(&id)?
    })))
}

/// POST /api/v1/create-videos/image — generate the 16:9 seed image for a marketing concept.
pub async fn generate_create_video_image(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<CreateVideoRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = require_create_videos_user(&state, &jar).await?;
    let prompt = crate::create_videos::get_prompt(&req.prompt_id)
        .ok_or_else(|| AppError::NotFound(format!("create-videos prompt {} not found", req.prompt_id)))?;

    if state.config.gemini_api_key.is_empty() {
        return Err(AppError::Unavailable("Gemini API key not configured".into()));
    }

    let mut card_state = crate::create_videos::load_state(&prompt.id)?;
    if card_state.image_status == "generating" {
        return Err(AppError::BadRequest("image already generating".into()));
    }
    if card_state.video_status == "generating" {
        return Err(AppError::BadRequest(
            "wait for the current video generation to finish before regenerating the image".into(),
        ));
    }

    card_state.image_status = "generating".to_string();
    card_state.image_error = None;
    card_state.touch();
    crate::create_videos::save_state(&card_state)?;

    state.emit_log(
        "INFO",
        "create-videos",
        &format!("{} requested image generation for {}", user_id, prompt.id),
    );

    let references = crate::services::gemini::load_references(std::path::Path::new("src/public"));
    let image_result = match crate::services::gemini::generate_image_exact_with_aspect(
        &state.config.gemini_api_key,
        &prompt.image_prompt,
        &references,
        "16:9",
    )
    .await
    {
        Ok(result) => result,
        Err(err) => {
            let message = format!("Gemini error: {}", err);
            let _ = persist_create_video_failure(&prompt.id, "image", message.clone());
            return Err(AppError::Internal(message));
        }
    };

    let asset_stem = crate::create_videos::asset_stem(&prompt.id);
    let image_path = match crate::services::gemini::save_image(&image_result.base64, &asset_stem) {
        Ok(path) => path,
        Err(err) => {
            let message = format!("failed to save image: {}", err);
            let _ = persist_create_video_failure(&prompt.id, "image", message.clone());
            return Err(AppError::Internal(message));
        }
    };
    let image_jpeg_path = match crate::services::gemini::save_image_jpeg(
        &image_result.base64,
        &asset_stem,
    ) {
        Ok(path) => path,
        Err(err) => {
            let message = format!("failed to save image jpeg: {}", err);
            let _ = persist_create_video_failure(&prompt.id, "image", message.clone());
            return Err(AppError::Internal(message));
        }
    };

    card_state = crate::create_videos::load_state(&prompt.id)?;
    card_state.image_status = "complete".to_string();
    card_state.image_path = Some(image_path.clone());
    card_state.image_url = Some(crate::create_videos::image_public_url(&image_path));
    card_state.image_jpeg_path = Some(image_jpeg_path);
    card_state.video_status = "ready".to_string();
    card_state.video_path = None;
    card_state.video_url = None;
    card_state.video_request_id = None;
    card_state.image_error = None;
    card_state.video_error = None;
    card_state.touch();
    crate::create_videos::save_state(&card_state)?;

    {
        let db = state.db.lock().await;
        let _ = queries::insert_cost_record(
            &db,
            "gemini",
            "gemini-2.5-flash-image",
            0,
            0,
            0.04,
            Some(&prompt.id),
        );
    }

    state.emit_log(
        "INFO",
        "create-videos",
        &format!("Seed image ready for {}", prompt.id),
    );

    Ok(Json(json!({
        "ok": true,
        "card": create_video_card_value(&prompt.id)?
    })))
}

/// POST /api/v1/create-videos/video — animate a generated seed image into a 16:9 Grok clip.
pub async fn generate_create_video_clip(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<CreateVideoRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = require_create_videos_user(&state, &jar).await?;
    let prompt = crate::create_videos::get_prompt(&req.prompt_id)
        .ok_or_else(|| AppError::NotFound(format!("create-videos prompt {} not found", req.prompt_id)))?;

    if state.config.xai_api_key.is_empty() {
        return Err(AppError::Unavailable("XAI_API_KEY not configured".into()));
    }

    let mut card_state = crate::create_videos::load_state(&prompt.id)?;
    let image_jpeg_path = card_state
        .image_jpeg_path
        .clone()
        .ok_or_else(|| AppError::BadRequest("generate the image first".into()))?;

    if card_state.image_status != "complete" {
        return Err(AppError::BadRequest("generate the image first".into()));
    }

    if card_state.video_status == "generating" {
        return Ok(Json(json!({
            "ok": true,
            "card": create_video_card_value(&prompt.id)?
        })));
    }

    card_state.video_status = "generating".to_string();
    card_state.video_error = None;
    card_state.video_request_id = None;
    card_state.touch();
    crate::create_videos::save_state(&card_state)?;

    state.emit_log(
        "INFO",
        "create-videos",
        &format!("{} requested video generation for {}", user_id, prompt.id),
    );

    let state_clone = state.clone();
    let prompt_id = prompt.id.clone();
    let prompt_duration = prompt.duration_seconds;
    let video_prompt = prompt.video_prompt.clone();
    let image_url = crate::create_videos::image_absolute_url(&image_jpeg_path);

    tokio::spawn(async move {
        let request_id = match crate::services::grok::generate_video_from_image_with_aspect(
            &state_clone.config.xai_api_key,
            &video_prompt,
            prompt_duration,
            Some(&image_url),
            "16:9",
        )
        .await
        {
            Ok(request_id) => request_id,
            Err(err) => {
                let message = format!("Grok submit error: {}", err);
                let _ = persist_create_video_failure(&prompt_id, "video", message.clone());
                state_clone.emit_log("ERROR", "create-videos", &message);
                return;
            }
        };

        if let Ok(mut inner_state) = crate::create_videos::load_state(&prompt_id) {
            inner_state.video_request_id = Some(request_id.clone());
            inner_state.touch();
            let _ = crate::create_videos::save_state(&inner_state);
        }

        state_clone.emit_log(
            "INFO",
            "create-videos",
            &format!("Grok request {} started for {}", request_id, prompt_id),
        );

        let mut attempts = 0u32;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            attempts += 1;

            match crate::services::grok::poll_video(&state_clone.config.xai_api_key, &request_id)
                .await
            {
                Ok((status, maybe_url))
                    if status == "complete" || status == "done" || status == "succeeded" =>
                {
                    let Some(remote_video_url) = maybe_url else {
                        let message = "Grok completed without a downloadable video URL".to_string();
                        let _ = persist_create_video_failure(&prompt_id, "video", message.clone());
                        state_clone.emit_log("ERROR", "create-videos", &message);
                        return;
                    };

                    let output_path = crate::create_videos::video_output_path(&prompt_id);
                    if let Err(err) =
                        crate::services::grok::download_video(&remote_video_url, &output_path).await
                    {
                        let message = format!("video download failed: {}", err);
                        let _ = persist_create_video_failure(&prompt_id, "video", message.clone());
                        state_clone.emit_log("ERROR", "create-videos", &message);
                        return;
                    }

                    if let Ok(mut inner_state) = crate::create_videos::load_state(&prompt_id) {
                        inner_state.video_status = "complete".to_string();
                        inner_state.video_path = Some(output_path.clone());
                        inner_state.video_url = Some(crate::create_videos::video_public_url(
                            &crate::create_videos::video_filename(&prompt_id),
                        ));
                        inner_state.video_request_id = Some(request_id.clone());
                        inner_state.video_error = None;
                        inner_state.touch();
                        let _ = crate::create_videos::save_state(&inner_state);
                    }

                    {
                        let db = state_clone.db.lock().await;
                        let _ = queries::insert_cost_record(
                            &db,
                            "grok",
                            "grok-imagine-video",
                            0,
                            0,
                            prompt_duration as f64 * 0.05,
                            Some(&prompt_id),
                        );
                    }

                    state_clone.emit_log(
                        "INFO",
                        "create-videos",
                        &format!("Marketing video ready for {}", prompt_id),
                    );
                    return;
                }
                Ok((status, _))
                    if status == "failed"
                        || status == "error"
                        || status == "expired"
                        || status == "cancelled" =>
                {
                    let message = format!("Grok returned terminal status {}", status);
                    let _ = persist_create_video_failure(&prompt_id, "video", message.clone());
                    state_clone.emit_log("ERROR", "create-videos", &message);
                    return;
                }
                Ok((_status, _)) => {
                    if attempts >= 180 {
                        let message =
                            "Timed out waiting for Grok video generation after 15 minutes".to_string();
                        let _ = persist_create_video_failure(&prompt_id, "video", message.clone());
                        state_clone.emit_log("ERROR", "create-videos", &message);
                        return;
                    }
                }
                Err(err) => {
                    if attempts >= 180 {
                        let message = format!("Grok polling failed repeatedly: {}", err);
                        let _ = persist_create_video_failure(&prompt_id, "video", message.clone());
                        state_clone.emit_log("ERROR", "create-videos", &message);
                        return;
                    }
                }
            }
        }
    });

    Ok(Json(json!({
        "ok": true,
        "card": create_video_card_value(&prompt.id)?
    })))
}

/// POST /api/v1/generate/video-frame — generate a single video frame image (paid via x402)
pub async fn generate_video_frame(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<VideoFrameRequest>,
) -> Result<Response, AppError> {
    // Require wallet tx hash
    let tx_hash = headers
        .get("payment-signature")
        .or_else(|| headers.get("x-payment"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| {
            s.starts_with("0x") && s.len() >= 10 && s[2..].chars().all(|c| c.is_ascii_hexdigit())
        });

    if tx_hash.is_none() {
        return Ok((
            axum::http::StatusCode::PAYMENT_REQUIRED,
            Json(json!({
                "error": "payment required",
                "cost_usd": VIDEO_FRAME_COST_USD,
                "treasury": state.config.treasury_address,
            })),
        )
            .into_response());
    }

    let prompt_text = req
        .prompt_text
        .unwrap_or_else(|| format!("Video frame for prompt: {}", req.prompt_id));

    let gemini_key = &state.config.gemini_api_key;
    if gemini_key.is_empty() {
        return Err(AppError::Internal("Gemini API key not configured".into()));
    }

    let frame_id = uuid::Uuid::new_v4().to_string();
    let references = crate::services::gemini::load_references(std::path::Path::new("src/public"));

    state.emit_log(
        "INFO",
        "video",
        &format!(
            "Generating video frame: {} (tx={})",
            req.prompt_id,
            tx_hash.as_deref().unwrap_or("?")
        ),
    );

    let image_result = crate::services::gemini::generate_image_with_aspect(
        gemini_key,
        &prompt_text,
        &references,
        "1:1",
    )
    .await
    .map_err(|e| AppError::Internal(format!("Gemini error: {}", e)))?;

    let image_path = crate::services::gemini::save_image(&image_result.base64, &frame_id)
        .map_err(|e| AppError::Internal(format!("Save error: {}", e)))?;

    {
        let db = state.db.lock().await;
        let _ = queries::insert_cost_record(
            &db,
            "gemini",
            "gemini-2.5-flash-image",
            0,
            0,
            0.04,
            Some(&frame_id),
        );
    }

    state.emit_log(
        "INFO",
        "video",
        &format!("Video frame saved: {}", image_path),
    );

    Ok(Json(json!({
        "frame_id": frame_id,
        "image_path": image_path,
        "image_url": format!("/data/images/{}", image_path),
    }))
    .into_response())
}

// ==================== Video Pipeline (Grok) ====================

const VIDEO_GEN_COST_USD: f64 = 5.00;

#[derive(serde::Deserialize)]
pub struct VideoGenerateRequest {
    pub anky_id: String,
}

/// POST /api/v1/generate/video — generate an 88-second video from an anky's writing session.
///
/// Returns immediately after saving the project to DB. Script generation and
/// video rendering happen entirely in a background task so the browser never
/// times out. The frontend polls GET /api/v1/video/{id} for progress.
pub async fn generate_video(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<VideoGenerateRequest>,
) -> Result<Response, AppError> {
    // Try anky_user_id cookie first, fall back to Privy/session auth
    let user_id = if let Some(c) = jar.get("anky_user_id") {
        c.value().to_string()
    } else if let Some(auth_user) = crate::routes::auth::get_auth_user(&state, &jar).await {
        auth_user.user_id
    } else {
        return Err(AppError::BadRequest("login required".into()));
    };

    // Require payment — accept any 0x-prefixed hex hash (some wallets vary in length)
    let tx_hash = headers
        .get("payment-signature")
        .or_else(|| headers.get("x-payment"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| s.starts_with("0x") && s.len() >= 10);

    if tx_hash.is_none() {
        return Ok((
            axum::http::StatusCode::PAYMENT_REQUIRED,
            Json(json!({
                "error": "payment required",
                "cost_usd": VIDEO_GEN_COST_USD,
                "treasury": state.config.treasury_address,
            })),
        )
            .into_response());
    }
    let tx_hash = tx_hash.unwrap();

    // Guard against double-payment: if this anky already has an active project, return it
    {
        let db = state.db.lock().await;
        if let Some(existing_id) = queries::find_active_video_project_for_anky(&db, &req.anky_id)? {
            state.emit_log(
                "INFO",
                "video",
                &format!(
                    "Duplicate video request for anky {}, returning existing project {}",
                    &req.anky_id[..8],
                    &existing_id[..8]
                ),
            );
            return Ok(Json(json!({
                "project_id": existing_id,
                "status": "already_started",
                "message": "this anky already has a video in progress — no extra charge",
            }))
            .into_response());
        }
    }

    // Validate the anky exists and has writing text (fast, before saving project)
    {
        let db = state.db.lock().await;
        let anky = queries::get_anky_by_id(&db, &req.anky_id)?
            .ok_or_else(|| AppError::NotFound("anky not found".into()))?;
        if anky.writing_text.as_ref().map_or(true, |t| t.is_empty()) {
            return Err(AppError::BadRequest("no writing text for this anky".into()));
        }
    }

    let project_id = uuid::Uuid::new_v4().to_string();

    // Save project to DB immediately so the user always has a record even if
    // the browser disconnects. Status starts as 'pending' (script not yet generated).
    {
        let db = state.db.lock().await;
        queries::insert_video_project_pending(&db, &project_id, &user_id, &req.anky_id, &tx_hash)?;
    }

    state.emit_log(
        "INFO",
        "video",
        &format!(
            "Starting video generation for anky {} (project {})",
            &req.anky_id[..8],
            &project_id[..8]
        ),
    );

    // Return immediately — all heavy work happens in the background
    let s = state.clone();
    let pid = project_id.clone();
    let anky_id = req.anky_id.clone();
    let uid = user_id.clone();
    tokio::spawn(async move {
        // --- Phase 1: gather writing data + memory ---
        let video_ctx = {
            let db = s.db.lock().await;
            let anky = match queries::get_anky_by_id(&db, &anky_id) {
                Ok(Some(a)) => a,
                _ => {
                    let _ = queries::update_video_project_status(&db, &pid, "failed");
                    s.emit_log(
                        "ERROR",
                        "video",
                        &format!("Video {} failed: anky not found", &pid[..8]),
                    );
                    return;
                }
            };
            let writing_text = anky.writing_text.unwrap_or_default();
            let (fs, dur, wc): (Option<f64>, f64, i32) = db
                .query_row(
                    "SELECT w.flow_score, w.duration_seconds, w.word_count
                     FROM writing_sessions w JOIN ankys a ON a.writing_session_id = w.id
                     WHERE a.id = ?1",
                    rusqlite::params![&anky_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .unwrap_or((None, 480.0, writing_text.split_whitespace().count() as i32));

            crate::pipeline::video_gen::VideoContext {
                writing_text,
                anky_title: anky.title,
                anky_reflection: anky.reflection,
                flow_score: fs,
                duration_seconds: dur,
                word_count: wc,
                memory: None, // populated below
            }
        };

        // Build memory context outside the db lock
        let memory = match crate::memory::recall::build_memory_context(
            &s.db,
            &s.config.ollama_base_url,
            &uid,
            &video_ctx.writing_text,
        )
        .await
        {
            Ok(mem) if mem.session_count > 0 => {
                Some(crate::pipeline::video_gen::memory_context_to_video(&mem))
            }
            _ => None,
        };

        let video_ctx = crate::pipeline::video_gen::VideoContext {
            memory,
            ..video_ctx
        };

        // --- Phase 2: generate script with Claude ---
        let script_prompt_override = {
            let db = s.db.lock().await;
            queries::get_pipeline_prompt(&db, crate::pipeline::video_gen::VIDEO_SCRIPT_PROMPT_KEY)
                .ok()
                .flatten()
        };
        let script_result = match crate::pipeline::video_gen::generate_script(
            &s.config.anthropic_api_key,
            &video_ctx,
            script_prompt_override.as_deref(),
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                let db = s.db.lock().await;
                let _ = queries::update_video_project_status(&db, &pid, "failed");
                s.emit_log(
                    "ERROR",
                    "video",
                    &format!("Video {} script failed: {}", &pid[..8], e),
                );
                return;
            }
        };
        let mut script = script_result.script;

        // Record script cost
        let script_cost = crate::pipeline::cost::estimate_claude_cost(
            script_result.input_tokens,
            script_result.output_tokens,
        );
        {
            let db = s.db.lock().await;
            let _ = queries::insert_cost_record(
                &db,
                "claude",
                "claude-sonnet-4-20250514",
                script_result.input_tokens,
                script_result.output_tokens,
                script_cost,
                Some(&pid),
            );
        }

        let scene_count = script.scenes.len() as i32;
        let script_json = serde_json::to_string(&script).unwrap_or_default();

        // Update project with script data, transition to 'generating'
        {
            let db = s.db.lock().await;
            let _ = queries::update_video_project_script(&db, &pid, &script_json, scene_count);
        }

        s.emit_log(
            "INFO",
            "video",
            &format!(
                "Video {} script ready ({} scenes), starting generation",
                &pid[..8],
                scene_count
            ),
        );

        // --- Phase 3: generate images + videos ---
        match crate::pipeline::video_gen::generate_video_from_script(&s, &pid, &mut script).await {
            Ok(video_path) => {
                let updated_json = serde_json::to_string(&script).unwrap_or_default();
                let db = s.db.lock().await;
                let _ =
                    queries::update_video_project_complete(&db, &pid, &video_path, &updated_json);
                s.emit_log(
                    "INFO",
                    "video",
                    &format!("Video {} complete: {}", &pid[..8], video_path),
                );
            }
            Err(e) => {
                let db = s.db.lock().await;
                let _ = queries::update_video_project_status(&db, &pid, "failed");
                s.emit_log(
                    "ERROR",
                    "video",
                    &format!("Video {} failed: {}", &pid[..8], e),
                );
            }
        }
    });

    Ok(Json(json!({
        "project_id": project_id,
        "status": "pending",
        "message": "video generation started — generating script...",
    }))
    .into_response())
}

/// GET /api/v1/video/{id} — poll video project status.
pub async fn get_video_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project = {
        let db = state.db.lock().await;
        queries::get_video_project(&db, &id)?
            .ok_or_else(|| AppError::NotFound("video project not found".into()))?
    };

    let script: Option<serde_json::Value> = project
        .script_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let story_spine: Option<serde_json::Value> = project
        .story_spine
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(Json(json!({
        "id": project.id,
        "status": project.status,
        "current_step": project.current_step.unwrap_or_else(|| "script".to_string()),
        "total_scenes": project.total_scenes,
        "completed_scenes": project.completed_scenes,
        "video_path": project.video_path,
        "video_url": project.video_path.as_ref().map(|p| video_public_url(p)),
        "video_url_720p": project.video_path_720p.as_ref().map(|p| video_public_url(p)),
        "video_url_360p": project.video_path_360p.as_ref().map(|p| video_public_url(p)),
        "story_spine": story_spine,
        "script": script,
        "created_at": project.created_at,
    })))
}

/// POST /api/v1/video/{id}/resume — resume a failed video project from where it left off.
pub async fn resume_video_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project = {
        let db = state.db.lock().await;
        queries::get_video_project(&db, &id)?
            .ok_or_else(|| AppError::NotFound("video project not found".into()))?
    };

    if project.status != "failed" && project.status != "generating" {
        return Err(AppError::BadRequest(format!(
            "project status is '{}', can only resume 'failed' or stuck 'generating' projects",
            project.status
        )));
    }

    let script_json = project
        .script_json
        .ok_or_else(|| AppError::BadRequest("no script found for this project".into()))?;
    let mut script: crate::pipeline::video_gen::VideoScript = serde_json::from_str(&script_json)
        .map_err(|e| AppError::Internal(format!("failed to parse script: {}", e)))?;

    // Determine what step to resume from based on scene states
    let all_videos = script
        .scenes
        .iter()
        .all(|s| s.status == "complete" && !s.local_path.is_empty());

    let resume_from = if all_videos {
        "stitching"
    } else {
        "generating"
    };

    state.emit_log(
        "INFO",
        "video",
        &format!(
            "Resuming project {} from step '{}' (complete:{}/{})",
            &id[..8],
            resume_from,
            script
                .scenes
                .iter()
                .filter(|s| s.status == "complete")
                .count(),
            script.scenes.len(),
        ),
    );

    // Reset status to generating
    {
        let db = state.db.lock().await;
        let _ = queries::update_video_project_status(&db, &id, "generating");
    }

    let s = state.clone();
    let pid = id.clone();
    tokio::spawn(async move {
        let result = async {
            if resume_from == "generating" {
                // Sequential chain — skips completed scenes, extracts their frames for continuity
                crate::pipeline::video_gen::resume_from_generating(&s, &pid, &mut script).await
            } else {
                // All videos done, just stitch
                crate::pipeline::video_gen::resume_from_stitch(&s, &pid, &mut script).await
            }
        }
        .await;

        match result {
            Ok(video_path) => {
                let updated_json = serde_json::to_string(&script).unwrap_or_default();
                let db = s.db.lock().await;
                let _ =
                    queries::update_video_project_complete(&db, &pid, &video_path, &updated_json);
                s.emit_log(
                    "INFO",
                    "video",
                    &format!("Video {} resume complete: {}", &pid[..8], video_path),
                );
            }
            Err(e) => {
                let db = s.db.lock().await;
                let _ = queries::update_video_project_status(&db, &pid, "failed");
                s.emit_log(
                    "ERROR",
                    "video",
                    &format!("Video {} resume failed: {}", &pid[..8], e),
                );
            }
        }
    });

    Ok(Json(json!({
        "project_id": id,
        "resume_from": resume_from,
        "status": "generating",
    })))
}

#[derive(serde::Deserialize)]
pub struct SaveVideoPipelineConfigRequest {
    pub script_system_prompt: String,
    pub scene_image_prompt_template: String,
    pub scene_sound_prompt_template: String,
}

/// GET /api/v1/video/pipeline/config — current prompt templates + spend summary.
pub async fn get_video_pipeline_config(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = crate::routes::auth::get_auth_user(&state, &jar)
        .await
        .ok_or_else(|| AppError::BadRequest("login required".into()))?;

    let (script_prompt, image_template, sound_template, spend_7d, spend_all_time, recent_projects) = {
        let db = state.db.lock().await;
        let script_prompt =
            queries::get_pipeline_prompt(&db, crate::pipeline::video_gen::VIDEO_SCRIPT_PROMPT_KEY)?
                .unwrap_or_else(|| {
                    crate::pipeline::video_gen::default_script_system_prompt().to_string()
                });
        let image_template = queries::get_pipeline_prompt(
            &db,
            crate::pipeline::video_gen::VIDEO_IMAGE_PROMPT_TEMPLATE_KEY,
        )?
        .unwrap_or_else(|| crate::pipeline::video_gen::DEFAULT_IMAGE_PROMPT_TEMPLATE.to_string());
        let sound_template = queries::get_pipeline_prompt(
            &db,
            crate::pipeline::video_gen::VIDEO_SOUND_PROMPT_TEMPLATE_KEY,
        )?
        .unwrap_or_else(|| crate::pipeline::video_gen::DEFAULT_SOUND_PROMPT_TEMPLATE.to_string());
        let spend_7d = queries::get_video_service_spend(&db, &user.user_id, Some(7))?;
        let spend_all_time = queries::get_video_service_spend(&db, &user.user_id, None)?;
        let recent_projects = queries::get_recent_video_project_spend(&db, &user.user_id, 12)?;
        (
            script_prompt,
            image_template,
            sound_template,
            spend_7d,
            spend_all_time,
            recent_projects,
        )
    };

    Ok(Json(json!({
        "prompts": {
            "script_system_prompt": script_prompt,
            "scene_image_prompt_template": image_template,
            "scene_sound_prompt_template": sound_template,
        },
        "estimated_cost_model": {
            "claude_script": "token-based",
            "gemini_per_scene_usd": 0.04,
            "grok_per_second_usd": 0.05,
            "target_seconds": 88
        },
        "spend_7d": spend_7d.iter().map(|s| json!({
            "service": s.service,
            "model": s.model,
            "calls": s.calls,
            "total_cost_usd": s.total_cost_usd,
        })).collect::<Vec<_>>(),
        "spend_all_time": spend_all_time.iter().map(|s| json!({
            "service": s.service,
            "model": s.model,
            "calls": s.calls,
            "total_cost_usd": s.total_cost_usd,
        })).collect::<Vec<_>>(),
        "recent_projects": recent_projects.iter().map(|p| json!({
            "id": p.id,
            "status": p.status,
            "created_at": p.created_at,
            "total_cost_usd": p.total_cost_usd,
        })).collect::<Vec<_>>()
    })))
}

/// POST /api/v1/video/pipeline/config — update prompt templates used by the 8m→88s pipeline.
pub async fn save_video_pipeline_config(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<SaveVideoPipelineConfigRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = crate::routes::auth::get_auth_user(&state, &jar)
        .await
        .ok_or_else(|| AppError::BadRequest("login required".into()))?;

    if req.script_system_prompt.trim().is_empty()
        || req.scene_image_prompt_template.trim().is_empty()
        || req.scene_sound_prompt_template.trim().is_empty()
    {
        return Err(AppError::BadRequest(
            "all prompt fields are required".into(),
        ));
    }

    {
        let db = state.db.lock().await;
        queries::upsert_pipeline_prompt(
            &db,
            crate::pipeline::video_gen::VIDEO_SCRIPT_PROMPT_KEY,
            &req.script_system_prompt,
            Some(&user.user_id),
        )?;
        queries::upsert_pipeline_prompt(
            &db,
            crate::pipeline::video_gen::VIDEO_IMAGE_PROMPT_TEMPLATE_KEY,
            &req.scene_image_prompt_template,
            Some(&user.user_id),
        )?;
        queries::upsert_pipeline_prompt(
            &db,
            crate::pipeline::video_gen::VIDEO_SOUND_PROMPT_TEMPLATE_KEY,
            &req.scene_sound_prompt_template,
            Some(&user.user_id),
        )?;
    }

    Ok(Json(json!({
        "saved": true,
    })))
}

/// POST /api/v1/purge-cache — purge Cloudflare cache (admin only).
pub async fn purge_cache(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token = &state.config.cloudflare_api_token;
    let zone_id = &state.config.cloudflare_zone_id;

    if token.is_empty() || zone_id.is_empty() {
        return Err(AppError::Internal("Cloudflare credentials not configured. Set CLOUDFLARE_API_TOKEN and CLOUDFLARE_ZONE_ID.".into()));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "https://api.cloudflare.com/client/v4/zones/{}/purge_cache",
            zone_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({ "purge_everything": true }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("CF API error: {}", e)))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();

    if status.is_success() {
        state.emit_log("INFO", "cache", "Cloudflare cache purged");
        Ok(Json(json!({ "ok": true, "message": "cache purged" })))
    } else {
        Err(AppError::Internal(format!(
            "CF purge failed ({}): {}",
            status, body
        )))
    }
}

// ==================== OG Video Image ====================

/// GET /og/video — dynamically generate an OG image for the video page
pub async fn og_video_image() -> Result<Response, AppError> {
    use image::{Rgb, RgbImage};

    let width = 1200u32;
    let height = 630u32;

    // Create black background
    let mut img = RgbImage::from_pixel(width, height, Rgb([8, 8, 12]));

    // Draw a gold-ish rectangle border
    let gold = Rgb([212, 168, 83]);
    let white = Rgb([200, 200, 212]);
    for x in 40..1160 {
        img.put_pixel(x, 200, gold);
        img.put_pixel(x, 201, gold);
        img.put_pixel(x, 430, gold);
        img.put_pixel(x, 431, gold);
    }
    for y in 200..432 {
        img.put_pixel(40, y, gold);
        img.put_pixel(41, y, gold);
        img.put_pixel(1159, y, gold);
        img.put_pixel(1158, y, gold);
    }

    // Draw "ANKY" text as simple block letters (since we can't easily load fonts)
    // We'll use a simple approach: draw filled rectangles for each letter
    let letter_y = 250u32;
    let letter_h = 80u32;

    // A
    draw_rect(&mut img, 420, letter_y, 10, letter_h, gold);
    draw_rect(&mut img, 460, letter_y, 10, letter_h, gold);
    draw_rect(&mut img, 420, letter_y, 50, 10, gold);
    draw_rect(&mut img, 420, letter_y + 35, 50, 10, gold);

    // N
    draw_rect(&mut img, 490, letter_y, 10, letter_h, gold);
    draw_rect(&mut img, 540, letter_y, 10, letter_h, gold);
    draw_rect(&mut img, 490, letter_y, 60, 10, gold);

    // K
    draw_rect(&mut img, 570, letter_y, 10, letter_h, gold);
    draw_rect(&mut img, 580, letter_y + 35, 30, 10, gold);
    draw_rect(&mut img, 610, letter_y, 10, 35, gold);
    draw_rect(&mut img, 610, letter_y + 45, 10, 35, gold);

    // Y
    draw_rect(&mut img, 640, letter_y, 10, 45, gold);
    draw_rect(&mut img, 690, letter_y, 10, 45, gold);
    draw_rect(&mut img, 650, letter_y + 35, 40, 10, gold);
    draw_rect(&mut img, 665, letter_y + 35, 10, 45, gold);

    // "LEARN HOW TO FOCUS" in smaller blocks (just draw thin white line as separator)
    for x in 300..900 {
        img.put_pixel(x, 380, white);
    }

    // Encode to PNG
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgb8,
    )
    .map_err(|e| AppError::Internal(format!("PNG encode error: {}", e)))?;

    Ok((
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        [(axum::http::header::CACHE_CONTROL, "public, max-age=3600")],
        buf,
    )
        .into_response())
}

#[derive(Clone, Debug)]
struct DcaOgBuy {
    timestamp: String,
    signature: String,
    bought_anky: Option<f64>,
    spent_sol: Option<f64>,
}

fn trim_signature(sig: &str) -> String {
    if sig.len() <= 18 {
        return sig.to_string();
    }
    format!("{}...{}", &sig[..9], &sig[sig.len() - 7..])
}

fn trim_wallet(wallet: &str) -> String {
    if wallet.len() <= 16 {
        return wallet.to_string();
    }
    format!("{}...{}", &wallet[..8], &wallet[wallet.len() - 8..])
}

fn compact_timestamp(ts: &str) -> String {
    // Expected: 2026-02-25T18:02:02.113537+00:00
    if ts.len() >= 19 {
        let date = &ts[5..10];
        let time = &ts[11..19];
        format!("{} {}", date, time)
    } else {
        ts.to_string()
    }
}

fn parse_float_after_token(line: &str, token: &str) -> Option<f64> {
    let idx = line.find(token)?;
    let raw = line[idx + token.len()..]
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim();
    raw.parse::<f64>().ok()
}

fn parse_dca_recent_buys(path: &str, max_items: usize) -> Vec<DcaOgBuy> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut out = Vec::new();
    let marker = "swap_submitted signature=";
    let mut latest_buy_sol: Option<f64> = None;
    let mut latest_anky_out_raw: Option<f64> = None;
    let token_decimals = read_dca_env(".secrets/anky_dca.env")
        .get("ANKY_TOKEN_DECIMALS")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(6);
    let raw_scale = 10f64.powi(token_decimals as i32);

    for line in content.lines() {
        if let Some(v) = parse_float_after_token(line, "buy_sol=") {
            latest_buy_sol = Some(v);
        }
        if let Some(v) = parse_float_after_token(line, "quoted_out_anky_raw=") {
            latest_anky_out_raw = Some(v / raw_scale);
        }

        if let Some(idx) = line.find(marker) {
            let signature = line[idx + marker.len()..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if signature.is_empty() {
                continue;
            }

            let timestamp = if line.starts_with('[') {
                line.find(']')
                    .map(|end| line[1..end].to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "unknown".to_string()
            };

            out.push(DcaOgBuy {
                timestamp,
                signature,
                bought_anky: latest_anky_out_raw,
                spent_sol: latest_buy_sol,
            });

            // Reset quote context so rows stay paired to each swap cycle.
            latest_anky_out_raw = None;
        }
    }

    if out.len() > max_items {
        out = out.split_off(out.len() - max_items);
    }
    out
}

fn read_dca_env(path: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let content = fs::read_to_string(path).unwrap_or_default();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

async fn fetch_sol_balance_for_og(rpc_url: &str, wallet: &str) -> Option<f64> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [wallet, {"commitment": "confirmed"}]
    });
    let response = client
        .post(rpc_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(6))
        .send()
        .await
        .ok()?;
    let json: serde_json::Value = response.json().await.ok()?;
    let lamports = json["result"]["value"].as_i64()? as f64;
    Some(lamports / 1_000_000_000.0)
}

async fn fetch_anky_balance_for_og(rpc_url: &str, wallet: &str, mint: &str) -> Option<f64> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet,
            {"mint": mint},
            {"encoding": "jsonParsed", "commitment": "confirmed"}
        ]
    });
    let response = client
        .post(rpc_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(6))
        .send()
        .await
        .ok()?;
    let json: serde_json::Value = response.json().await.ok()?;
    let accounts = json["result"]["value"].as_array()?;
    let mut total = 0.0f64;
    for account in accounts {
        if let Some(amount) =
            account["account"]["data"]["parsed"]["info"]["tokenAmount"]["uiAmount"].as_f64()
        {
            total += amount;
        }
    }
    Some(total)
}

/// GET /og/dca — dynamically generate OG image for DCA page with latest buys
pub async fn og_dca_image() -> Result<Response, AppError> {
    use ab_glyph::{FontRef, PxScale};
    use image::{DynamicImage, Rgba, RgbaImage};
    use imageproc::drawing::draw_text_mut;

    let width = 1200u32;
    let height = 630u32;
    let mut canvas = RgbaImage::from_pixel(width, height, Rgba([5, 10, 12, 255]));

    // Neon-ish frame
    for x in 24..(width - 24) {
        canvas.put_pixel(x, 24, Rgba([42, 255, 160, 220]));
        canvas.put_pixel(x, 25, Rgba([42, 255, 160, 140]));
        canvas.put_pixel(x, height - 25, Rgba([42, 255, 160, 220]));
    }
    for y in 24..(height - 24) {
        canvas.put_pixel(24, y, Rgba([42, 255, 160, 220]));
        canvas.put_pixel(width - 25, y, Rgba([42, 255, 160, 220]));
    }

    // Panel bands
    for y in 90..130 {
        for x in 48..(width - 48) {
            canvas.put_pixel(x, y, Rgba([10, 35, 28, 255]));
        }
    }
    for y in 146..(height - 72) {
        for x in 48..(width - 48) {
            canvas.put_pixel(x, y, Rgba([7, 20, 16, 245]));
        }
    }

    let font_data = include_bytes!("../../static/fonts/Righteous-Regular.ttf");
    let font = FontRef::try_from_slice(font_data)
        .map_err(|e| AppError::Internal(format!("font error: {}", e)))?;

    let title_scale = PxScale::from(52.0);
    let header_scale = PxScale::from(22.0);
    let row_scale = PxScale::from(18.0);
    let row_detail_scale = PxScale::from(17.0);
    let brand_scale = PxScale::from(22.0);

    draw_text_mut(
        &mut canvas,
        Rgba([116, 255, 191, 255]),
        64,
        36,
        title_scale,
        &font,
        "$ANKY DCA LIVE",
    );
    draw_text_mut(
        &mut canvas,
        Rgba([175, 232, 210, 230]),
        64,
        96,
        header_scale,
        &font,
        "DCA wallet + balances + latest executed buys",
    );

    let env = read_dca_env(".secrets/anky_dca.env");
    let wallet = env
        .get("DCA_WALLET_PUBKEY")
        .cloned()
        .unwrap_or_else(|| "not-configured".to_string());
    let rpc_url = env
        .get("SOLANA_RPC_URL")
        .cloned()
        .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_string());
    let anky_mint = env
        .get("ANKY_TOKEN_MINT")
        .cloned()
        .unwrap_or_else(|| "6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump".to_string());

    let sol_balance = fetch_sol_balance_for_og(&rpc_url, &wallet).await;
    let anky_balance = fetch_anky_balance_for_og(&rpc_url, &wallet, &anky_mint).await;
    let summary = format!(
        "wallet {}   SOL {:.6}   $ANKY {:.2}",
        trim_wallet(&wallet),
        sol_balance.unwrap_or(0.0),
        anky_balance.unwrap_or(0.0)
    );
    draw_text_mut(
        &mut canvas,
        Rgba([132, 242, 214, 255]),
        64,
        136,
        header_scale,
        &font,
        &summary,
    );

    let buys = parse_dca_recent_buys("logs/anky_dca.log", 6);
    if buys.is_empty() {
        draw_text_mut(
            &mut canvas,
            Rgba([255, 196, 140, 240]),
            64,
            188,
            row_scale,
            &font,
            "no swap_submitted logs yet",
        );
    } else {
        let mut y = 184i32;
        for buy in buys {
            let ts = compact_timestamp(&buy.timestamp);
            let sig = trim_signature(&buy.signature);
            let x = buy.bought_anky.unwrap_or(0.0);
            let y_sol = buy.spent_sol.unwrap_or(0.0);
            let z_quote = if y_sol > 0.0 { x / y_sol } else { 0.0 };
            let line1 = format!("{}   {}", ts, sig);
            draw_text_mut(
                &mut canvas,
                Rgba([138, 255, 200, 250]),
                64,
                y,
                row_scale,
                &font,
                &line1,
            );

            let mut x_cursor = 64f32;
            let y2 = y + 22;
            let w = |txt: &str| {
                crate::pipeline::prompt_gen::measure_text_width_pub(&font, row_detail_scale, txt)
            };
            let draw = |canvas: &mut RgbaImage, txt: &str, color: Rgba<u8>, xc: &mut f32| {
                draw_text_mut(canvas, color, *xc as i32, y2, row_detail_scale, &font, txt);
                *xc += w(txt);
            };

            draw(
                &mut canvas,
                "bought ",
                Rgba([180, 236, 216, 235]),
                &mut x_cursor,
            );
            draw(
                &mut canvas,
                &format!("{:.2} $ANKY", x),
                Rgba([178, 112, 255, 255]),
                &mut x_cursor,
            );
            draw(
                &mut canvas,
                " for ",
                Rgba([180, 236, 216, 235]),
                &mut x_cursor,
            );
            draw(
                &mut canvas,
                &format!("{:.6} SOL", y_sol),
                Rgba([108, 177, 255, 255]),
                &mut x_cursor,
            );
            draw(
                &mut canvas,
                " at ",
                Rgba([180, 236, 216, 235]),
                &mut x_cursor,
            );
            draw(
                &mut canvas,
                &format!("{:.0} ANKY/SOL", z_quote),
                Rgba([178, 112, 255, 255]),
                &mut x_cursor,
            );

            y += 64;
            if y > 560 {
                break;
            }
        }
    }

    draw_text_mut(
        &mut canvas,
        Rgba([197, 235, 220, 220]),
        64,
        586,
        brand_scale,
        &font,
        "anky.app/dca",
    );

    let dynamic = DynamicImage::ImageRgba8(canvas);
    let mut buf = std::io::Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| AppError::Internal(format!("PNG encode error: {}", e)))?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "image/png"),
            (
                axum::http::header::CACHE_CONTROL,
                "public, max-age=60, s-maxage=60",
            ),
        ],
        buf.into_inner(),
    )
        .into_response())
}

// ==================== Farcaster OG Embed ====================

/// GET /api/v1/og-embed — serves the latest anky's image with title + username overlay
/// as the Farcaster frame embed. Cloudflare caches via Cache-Control.
pub async fn og_embed_image(State(state): State<AppState>) -> Result<Response, AppError> {
    use ab_glyph::{FontRef, PxScale};
    use image::{DynamicImage, Rgba, RgbaImage};
    use imageproc::drawing::draw_text_mut;

    // Get latest complete anky with user info
    let embed = {
        let db = state.db.lock().await;
        queries::get_latest_anky_for_embed(&db)?
    };

    let embed = match embed {
        Some(e) => e,
        None => return og_video_image().await,
    };

    // Read the anky image from disk
    let full_path = if embed.image_path.starts_with('/') {
        std::path::PathBuf::from(&embed.image_path)
    } else {
        std::path::PathBuf::from("data/images").join(&embed.image_path)
    };

    let image_bytes = tokio::fs::read(&full_path).await.map_err(|e| {
        AppError::Internal(format!(
            "failed to read image {}: {}",
            full_path.display(),
            e
        ))
    })?;

    // Load and decode the anky image
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| AppError::Internal(format!("image decode error: {}", e)))?;
    let (width, height) = (img.width(), img.height());
    let mut canvas: RgbaImage = img.to_rgba8();

    // Load font
    let font_data = include_bytes!("../../static/fonts/Righteous-Regular.ttf");
    let font = FontRef::try_from_slice(font_data)
        .map_err(|e| AppError::Internal(format!("font error: {}", e)))?;

    let title = embed.title.as_deref().unwrap_or("anky");
    let username = &embed.display_username;

    // --- Draw dark gradient band at bottom ---
    let band_height = (height as f32 * 0.25) as u32;
    let band_start = height.saturating_sub(band_height);
    for y in band_start..height {
        let progress = (y - band_start) as f32 / band_height as f32;
        let alpha = (180.0 * progress) as u8;
        let a = alpha as f32 / 255.0;
        for x in 0..width {
            let pixel = canvas.get_pixel_mut(x, y);
            let r = ((pixel[0] as f32) * (1.0 - a)) as u8;
            let g = ((pixel[1] as f32) * (1.0 - a)) as u8;
            let b = ((pixel[2] as f32) * (1.0 - a)) as u8;
            *pixel = Rgba([r, g, b, 255]);
        }
    }

    // --- Draw title text ---
    let title_size = (width as f32 * 0.055).max(28.0);
    let title_scale = PxScale::from(title_size);
    let title_y = height.saturating_sub((band_height as f32 * 0.55) as u32) as i32;
    let title_x = (width as f32 * 0.04) as i32;
    let title_color = Rgba([255u8, 255, 255, 255]);
    draw_text_mut(
        &mut canvas,
        title_color,
        title_x,
        title_y,
        title_scale,
        &font,
        title,
    );

    // --- Draw username text below title ---
    let user_size = (width as f32 * 0.03).max(16.0);
    let user_scale = PxScale::from(user_size);
    let user_y = title_y + (title_size * 1.4) as i32;
    let user_text = format!("@{}", username);
    let user_color = Rgba([200u8, 200, 212, 220]);
    draw_text_mut(
        &mut canvas,
        user_color,
        title_x,
        user_y,
        user_scale,
        &font,
        &user_text,
    );

    // --- Draw "anky" branding in bottom-right ---
    let brand_size = (width as f32 * 0.035).max(14.0);
    let brand_scale = PxScale::from(brand_size);
    let brand_text = "anky.app";
    let brand_w =
        crate::pipeline::prompt_gen::measure_text_width_pub(&font, brand_scale, brand_text);
    let brand_x = (width as f32 - brand_w - width as f32 * 0.04) as i32;
    let brand_y = (height as f32 - brand_size * 1.8) as i32;
    let brand_color = Rgba([212u8, 168, 83, 200]);
    draw_text_mut(
        &mut canvas,
        brand_color,
        brand_x,
        brand_y,
        brand_scale,
        &font,
        brand_text,
    );

    // Encode to PNG
    let dynamic = DynamicImage::ImageRgba8(canvas);
    let mut buf = std::io::Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| AppError::Internal(format!("PNG encode error: {}", e)))?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "image/png"),
            (
                axum::http::header::CACHE_CONTROL,
                "public, max-age=300, s-maxage=300",
            ),
        ],
        buf.into_inner(),
    )
        .into_response())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReflectionCardBlockKind {
    Heading,
    Body,
}

#[derive(Clone)]
struct ReflectionCardBlock {
    kind: ReflectionCardBlockKind,
    text: String,
}

#[derive(Clone)]
struct ReflectionCardWrappedBlock {
    kind: ReflectionCardBlockKind,
    lines: Vec<String>,
}

#[derive(Clone, Copy)]
struct ReflectionCardConfig {
    safe_x: u32,
    top_y: u32,
    image_size: u32,
    gap: u32,
    reflection_gap: u32,
    footer_pad: u32,
    title_start: f32,
    title_min: f32,
    title_max_lines: usize,
    body_start: f32,
    body_min: f32,
    brand_size: f32,
}

struct ReflectionCardLayout {
    config: ReflectionCardConfig,
    title_lines: Vec<String>,
    title_scale: f32,
    title_line_height: u32,
    wrapped_blocks: Vec<ReflectionCardWrappedBlock>,
    body_scale: f32,
    body_line_height: u32,
    heading_scale: f32,
    heading_line_height: u32,
    paragraph_gap: u32,
    heading_gap: u32,
}

fn reflection_card_slug(input: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
        if slug.len() >= 64 {
            break;
        }
    }
    slug.trim_matches('-').to_string()
}

fn reflection_card_blend(dst: &mut image::Rgba<u8>, src: image::Rgba<u8>) {
    let alpha = src[3] as f32 / 255.0;
    let inv = 1.0 - alpha;
    dst[0] = ((src[0] as f32 * alpha) + (dst[0] as f32 * inv)) as u8;
    dst[1] = ((src[1] as f32 * alpha) + (dst[1] as f32 * inv)) as u8;
    dst[2] = ((src[2] as f32 * alpha) + (dst[2] as f32 * inv)) as u8;
    dst[3] = 255;
}

fn reflection_card_fill_background(img: &mut image::RgbaImage) {
    let top = [8.0f32, 9.0, 16.0];
    let mid = [18.0f32, 18.0, 35.0];
    let bottom = [6.0f32, 7.0, 11.0];
    let height = img.height().max(1);
    let width = img.width().max(1);

    for y in 0..height {
        let t = y as f32 / (height - 1) as f32;
        let (a, b, local_t) = if t < 0.46 {
            (top, mid, t / 0.46)
        } else {
            (mid, bottom, (t - 0.46) / 0.54)
        };
        let r = a[0] + (b[0] - a[0]) * local_t;
        let g = a[1] + (b[1] - a[1]) * local_t;
        let bch = a[2] + (b[2] - a[2]) * local_t;
        for x in 0..width {
            let drift = x as f32 / (width - 1) as f32;
            let glow = (1.0 - ((drift - 0.72).abs() * 1.6)).max(0.0) * 10.0;
            img.put_pixel(
                x,
                y,
                image::Rgba([
                    (r + glow * 0.14).min(255.0) as u8,
                    (g + glow * 0.09).min(255.0) as u8,
                    (bch + glow * 0.28).min(255.0) as u8,
                    255,
                ]),
            );
        }
    }
}

fn reflection_card_draw_soft_circle(
    img: &mut image::RgbaImage,
    cx: i32,
    cy: i32,
    radius: f32,
    color: image::Rgba<u8>,
) {
    let min_x = ((cx as f32 - radius).floor() as i32).max(0);
    let max_x = ((cx as f32 + radius).ceil() as i32).min(img.width() as i32 - 1);
    let min_y = ((cy as f32 - radius).floor() as i32).max(0);
    let max_y = ((cy as f32 + radius).ceil() as i32).min(img.height() as i32 - 1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - cx as f32;
            let dy = y as f32 - cy as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > radius {
                continue;
            }
            let falloff = (1.0 - dist / radius).powf(1.8);
            let mut tinted = color;
            tinted[3] = ((color[3] as f32) * falloff) as u8;
            let pixel = img.get_pixel_mut(x as u32, y as u32);
            reflection_card_blend(pixel, tinted);
        }
    }
}

fn reflection_card_stroke_rect(
    img: &mut image::RgbaImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    color: image::Rgba<u8>,
) {
    if w == 0 || h == 0 {
        return;
    }
    for dx in 0..w {
        let top = img.get_pixel_mut(x + dx, y);
        reflection_card_blend(top, color);
        let bottom = img.get_pixel_mut(x + dx, y + h - 1);
        reflection_card_blend(bottom, color);
    }
    for dy in 0..h {
        let left = img.get_pixel_mut(x, y + dy);
        reflection_card_blend(left, color);
        let right = img.get_pixel_mut(x + w - 1, y + dy);
        reflection_card_blend(right, color);
    }
}

fn reflection_card_is_inside_rounded_rect(x: u32, y: u32, size: u32, radius: u32) -> bool {
    if x >= size || y >= size {
        return false;
    }
    let r = radius as i32;
    let xi = x as i32;
    let yi = y as i32;
    let edge = size as i32 - r - 1;

    let dx = if xi < r {
        r - xi
    } else if xi > edge {
        xi - edge
    } else {
        0
    };
    let dy = if yi < r {
        r - yi
    } else if yi > edge {
        yi - edge
    } else {
        0
    };
    dx == 0 || dy == 0 || dx * dx + dy * dy <= r * r
}

fn reflection_card_draw_cover_image(
    canvas: &mut image::RgbaImage,
    src: &image::DynamicImage,
    x: u32,
    y: u32,
    size: u32,
) {
    use image::GenericImageView;

    let (src_w, src_h) = src.dimensions();
    if src_w == 0 || src_h == 0 || size == 0 {
        return;
    }

    let scale = (size as f32 / src_w as f32).max(size as f32 / src_h as f32);
    let new_w = (src_w as f32 * scale).ceil() as u32;
    let new_h = (src_h as f32 * scale).ceil() as u32;
    let resized = image::imageops::resize(
        &src.to_rgba8(),
        new_w,
        new_h,
        image::imageops::FilterType::Lanczos3,
    );
    let crop_x = new_w.saturating_sub(size) / 2;
    let crop_y = new_h.saturating_sub(size) / 2;
    let radius = (size as f32 * 0.12) as u32;

    for dy in 0..size {
        for dx in 0..size {
            if !reflection_card_is_inside_rounded_rect(dx, dy, size, radius) {
                continue;
            }
            let px = resized.get_pixel(crop_x + dx, crop_y + dy);
            canvas.put_pixel(x + dx, y + dy, *px);
        }
    }

    reflection_card_stroke_rect(canvas, x, y, size, size, image::Rgba([196, 160, 255, 48]));
}

fn reflection_card_clean_inline(text: &str) -> String {
    text.replace("***", "")
        .replace("**", "")
        .replace("__", "")
        .replace('*', "")
        .replace('_', "")
        .replace('`', "")
        .trim()
        .to_string()
}

fn reflection_card_known_heading(text: &str) -> Option<String> {
    let cleaned = reflection_card_clean_inline(text)
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_ascii_lowercase();
    match cleaned.as_str() {
        "what i see" => Some("What I see".to_string()),
        "do this today" => Some("Do this today".to_string()),
        _ => None,
    }
}

fn reflection_card_extract_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed
        .strip_prefix("### ")
        .or_else(|| trimmed.strip_prefix("## "))
        .or_else(|| trimmed.strip_prefix("# "))
    {
        return Some(reflection_card_known_heading(rest).unwrap_or_else(|| {
            reflection_card_clean_inline(rest)
                .trim_end_matches(':')
                .trim()
                .to_string()
        }));
    }
    if let Some(inner) = trimmed
        .strip_prefix("**")
        .and_then(|s| s.strip_suffix("**"))
        .or_else(|| {
            trimmed
                .strip_prefix("__")
                .and_then(|s| s.strip_suffix("__"))
        })
    {
        let cleaned = reflection_card_clean_inline(inner);
        if cleaned.len() <= 48 || reflection_card_known_heading(&cleaned).is_some() {
            return Some(reflection_card_known_heading(&cleaned).unwrap_or(cleaned));
        }
    }
    reflection_card_known_heading(trimmed)
}

fn reflection_card_blocks(text: &str) -> Vec<ReflectionCardBlock> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();

    let flush_paragraph = |blocks: &mut Vec<ReflectionCardBlock>, paragraph: &mut Vec<String>| {
        if paragraph.is_empty() {
            return;
        }
        let joined = paragraph.join(" ");
        let cleaned = reflection_card_clean_inline(&joined);
        if !cleaned.is_empty() {
            blocks.push(ReflectionCardBlock {
                kind: ReflectionCardBlockKind::Body,
                text: cleaned,
            });
        }
        paragraph.clear();
    };

    for raw_line in text.replace('\r', "").lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            flush_paragraph(&mut blocks, &mut paragraph);
            continue;
        }
        if let Some(heading) = reflection_card_extract_heading(line) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(ReflectionCardBlock {
                kind: ReflectionCardBlockKind::Heading,
                text: heading,
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(ReflectionCardBlock {
                kind: ReflectionCardBlockKind::Body,
                text: format!("• {}", reflection_card_clean_inline(rest)),
            });
            continue;
        }
        paragraph.push(line.to_string());
    }
    flush_paragraph(&mut blocks, &mut paragraph);

    if blocks.is_empty() {
        blocks.push(ReflectionCardBlock {
            kind: ReflectionCardBlockKind::Body,
            text: String::new(),
        });
    }
    blocks
}

fn reflection_card_wrap_word(
    font: &ab_glyph::FontRef<'_>,
    scale: ab_glyph::PxScale,
    word: &str,
    max_width: f32,
) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        let mut trial = current.clone();
        trial.push(ch);
        let width = crate::pipeline::prompt_gen::measure_text_width_pub(font, scale, &trial);
        if !current.is_empty() && width > max_width {
            parts.push(current);
            current = ch.to_string();
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    if parts.is_empty() {
        parts.push(word.to_string());
    }
    parts
}

fn reflection_card_wrap_text(
    font: &ab_glyph::FontRef<'_>,
    scale: ab_glyph::PxScale,
    text: &str,
    max_width: f32,
) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut tokens = Vec::new();
    for word in words {
        let width = crate::pipeline::prompt_gen::measure_text_width_pub(font, scale, word);
        if width > max_width {
            tokens.extend(reflection_card_wrap_word(font, scale, word, max_width));
        } else {
            tokens.push(word.to_string());
        }
    }

    let mut lines = Vec::new();
    let mut current = tokens[0].clone();
    for token in tokens.into_iter().skip(1) {
        let test = format!("{} {}", current, token);
        let width = crate::pipeline::prompt_gen::measure_text_width_pub(font, scale, &test);
        if width <= max_width {
            current = test;
        } else {
            lines.push(current);
            current = token;
        }
    }
    lines.push(current);
    lines
}

fn reflection_card_fit_layout(
    title: &str,
    text: &str,
    title_font: &ab_glyph::FontRef<'_>,
    body_font: &ab_glyph::FontRef<'_>,
) -> Result<ReflectionCardLayout, AppError> {
    let blocks = reflection_card_blocks(text);
    let configs = [
        ReflectionCardConfig {
            safe_x: 72,
            top_y: 84,
            image_size: 320,
            gap: 40,
            reflection_gap: 72,
            footer_pad: 150,
            title_start: 76.0,
            title_min: 34.0,
            title_max_lines: 4,
            body_start: 28.0,
            body_min: 18.0,
            brand_size: 34.0,
        },
        ReflectionCardConfig {
            safe_x: 62,
            top_y: 72,
            image_size: 280,
            gap: 32,
            reflection_gap: 60,
            footer_pad: 138,
            title_start: 68.0,
            title_min: 30.0,
            title_max_lines: 5,
            body_start: 26.0,
            body_min: 16.0,
            brand_size: 32.0,
        },
        ReflectionCardConfig {
            safe_x: 52,
            top_y: 60,
            image_size: 236,
            gap: 28,
            reflection_gap: 50,
            footer_pad: 126,
            title_start: 60.0,
            title_min: 28.0,
            title_max_lines: 5,
            body_start: 24.0,
            body_min: 14.0,
            brand_size: 30.0,
        },
        ReflectionCardConfig {
            safe_x: 42,
            top_y: 48,
            image_size: 184,
            gap: 22,
            reflection_gap: 40,
            footer_pad: 116,
            title_start: 50.0,
            title_min: 24.0,
            title_max_lines: 6,
            body_start: 22.0,
            body_min: 12.0,
            brand_size: 28.0,
        },
    ];

    for config in configs {
        let title_x = config.safe_x + config.image_size + config.gap;
        let title_width = 1080u32.saturating_sub(config.safe_x + title_x) as f32;
        let title_height = config.image_size as f32;
        let mut fitted_title = None;

        let mut title_size = config.title_start;
        while title_size >= config.title_min {
            let scale = ab_glyph::PxScale::from(title_size);
            let lines = reflection_card_wrap_text(title_font, scale, title, title_width);
            let line_height = (title_size * 1.04).round() as u32;
            if lines.len() <= config.title_max_lines
                && (lines.len() as f32 * line_height as f32) <= title_height
            {
                fitted_title = Some((lines, title_size, line_height));
                break;
            }
            title_size -= 2.0;
        }

        let Some((title_lines, title_scale, title_line_height)) = fitted_title else {
            continue;
        };

        let reflection_top = config.top_y + config.image_size + config.reflection_gap;
        let reflection_width = (1080u32 - config.safe_x * 2) as f32;
        let reflection_height = 1920u32.saturating_sub(reflection_top + config.footer_pad) as f32;

        let mut body_size = config.body_start;
        while body_size >= config.body_min {
            let body_scale = ab_glyph::PxScale::from(body_size);
            let heading_size = (body_size + 8.0).min(body_size * 1.4);
            let heading_scale = ab_glyph::PxScale::from(heading_size);
            let body_line_height = (body_size * 1.28).round() as u32;
            let heading_line_height = (heading_size * 1.05).round() as u32;
            let paragraph_gap = (body_size * 0.72).round().max(12.0) as u32;
            let heading_gap = (body_size * 0.56).round().max(10.0) as u32;

            let mut wrapped_blocks = Vec::new();
            let mut total_height = 0u32;
            for block in &blocks {
                let (font, scale, line_height, gap) = match block.kind {
                    ReflectionCardBlockKind::Heading => {
                        (title_font, heading_scale, heading_line_height, heading_gap)
                    }
                    ReflectionCardBlockKind::Body => {
                        (body_font, body_scale, body_line_height, paragraph_gap)
                    }
                };
                let lines = reflection_card_wrap_text(font, scale, &block.text, reflection_width);
                total_height = total_height.saturating_add(line_height * lines.len() as u32);
                total_height = total_height.saturating_add(gap);
                wrapped_blocks.push(ReflectionCardWrappedBlock {
                    kind: block.kind,
                    lines,
                });
            }
            total_height = total_height.saturating_sub(paragraph_gap.min(total_height));

            if total_height as f32 <= reflection_height {
                return Ok(ReflectionCardLayout {
                    config,
                    title_lines,
                    title_scale,
                    title_line_height,
                    wrapped_blocks,
                    body_scale: body_size,
                    body_line_height,
                    heading_scale: heading_size,
                    heading_line_height,
                    paragraph_gap,
                    heading_gap,
                });
            }

            body_size -= 2.0;
        }
    }

    Err(AppError::Internal(
        "reflection card could not fit on one screen".into(),
    ))
}

async fn render_anky_reflection_card_bytes(
    state: &AppState,
    id: &str,
) -> Result<(Vec<u8>, String), AppError> {
    use ab_glyph::{FontRef, PxScale};
    use image::{DynamicImage, Rgba, RgbaImage};
    use imageproc::drawing::draw_text_mut;

    let anky = {
        let db = state.db.lock().await;
        queries::get_anky_by_id(&db, id)?
    }
    .ok_or_else(|| AppError::NotFound(format!("anky {} not found", id)))?;

    if anky.status != "complete" {
        return Err(AppError::BadRequest("anky is not ready yet".into()));
    }

    let image_path = anky
        .image_path
        .clone()
        .ok_or_else(|| AppError::BadRequest("anky does not have an image yet".into()))?;
    let title = anky.title.clone().unwrap_or_else(|| "untitled".to_string());
    let reflection_text = anky
        .reflection
        .clone()
        .or(anky.image_prompt.clone())
        .unwrap_or_default();
    if reflection_text.trim().is_empty() {
        return Err(AppError::BadRequest(
            "anky does not have reflection text yet".into(),
        ));
    }

    let full_path = if image_path.starts_with('/') {
        std::path::PathBuf::from(&image_path)
    } else {
        std::path::PathBuf::from("data/images").join(&image_path)
    };
    let image_bytes = tokio::fs::read(&full_path).await.map_err(|e| {
        AppError::Internal(format!(
            "failed to read image {}: {}",
            full_path.display(),
            e
        ))
    })?;
    let source = image::load_from_memory(&image_bytes)
        .map_err(|e| AppError::Internal(format!("image decode error: {}", e)))?;

    let righteous_data = include_bytes!("../../static/fonts/Righteous-Regular.ttf");
    let title_font = FontRef::try_from_slice(righteous_data)
        .map_err(|e| AppError::Internal(format!("font error: {}", e)))?;

    let body_font_bytes =
        std::fs::read("/usr/share/fonts/liberation-mono-fonts/LiberationMono-Regular.ttf")
            .unwrap_or_else(|_| righteous_data.to_vec());
    let body_font = FontRef::try_from_slice(&body_font_bytes)
        .map_err(|e| AppError::Internal(format!("body font error: {}", e)))?;

    let layout = reflection_card_fit_layout(&title, &reflection_text, &title_font, &body_font)?;

    let mut canvas = RgbaImage::from_pixel(1080, 1920, Rgba([8, 9, 16, 255]));
    reflection_card_fill_background(&mut canvas);
    reflection_card_draw_soft_circle(&mut canvas, 900, 220, 260.0, Rgba([123, 47, 247, 42]));
    reflection_card_draw_soft_circle(&mut canvas, 170, 1720, 220.0, Rgba([196, 160, 255, 22]));

    reflection_card_draw_cover_image(
        &mut canvas,
        &DynamicImage::ImageRgba8(source.to_rgba8()),
        layout.config.safe_x,
        layout.config.top_y,
        layout.config.image_size,
    );

    let title_x = layout.config.safe_x + layout.config.image_size + layout.config.gap;
    let title_y = layout.config.top_y
        + ((layout.config.image_size as i32
            - (layout.title_lines.len() as i32 * layout.title_line_height as i32))
            .max(0) as u32
            / 2);
    let title_scale = PxScale::from(layout.title_scale);
    for (idx, line) in layout.title_lines.iter().enumerate() {
        draw_text_mut(
            &mut canvas,
            Rgba([246, 236, 255, 255]),
            title_x as i32,
            (title_y + idx as u32 * layout.title_line_height) as i32,
            title_scale,
            &title_font,
            line,
        );
    }

    let mut cursor_y =
        layout.config.top_y + layout.config.image_size + layout.config.reflection_gap;
    let heading_scale = PxScale::from(layout.heading_scale);
    let body_scale = PxScale::from(layout.body_scale);
    for block in &layout.wrapped_blocks {
        match block.kind {
            ReflectionCardBlockKind::Heading => {
                for line in &block.lines {
                    draw_text_mut(
                        &mut canvas,
                        Rgba([196, 160, 255, 255]),
                        layout.config.safe_x as i32,
                        cursor_y as i32,
                        heading_scale,
                        &title_font,
                        line,
                    );
                    cursor_y += layout.heading_line_height;
                }
                cursor_y += layout.heading_gap;
            }
            ReflectionCardBlockKind::Body => {
                for line in &block.lines {
                    draw_text_mut(
                        &mut canvas,
                        Rgba([236, 233, 247, 255]),
                        layout.config.safe_x as i32,
                        cursor_y as i32,
                        body_scale,
                        &body_font,
                        line,
                    );
                    cursor_y += layout.body_line_height;
                }
                cursor_y += layout.paragraph_gap;
            }
        }
    }

    let brand_text = "https://anky.app";
    let brand_scale = PxScale::from(layout.config.brand_size);
    let brand_width =
        crate::pipeline::prompt_gen::measure_text_width_pub(&title_font, brand_scale, brand_text);
    let brand_x = ((1080.0 - brand_width) / 2.0).round() as i32;
    draw_text_mut(
        &mut canvas,
        Rgba([160, 111, 255, 255]),
        brand_x,
        1832,
        brand_scale,
        &title_font,
        brand_text,
    );

    let dynamic = DynamicImage::ImageRgba8(canvas);
    let mut buf = std::io::Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| AppError::Internal(format!("PNG encode error: {}", e)))?;

    Ok((buf.into_inner(), reflection_card_slug(&title)))
}

/// GET /api/anky-card/{id} — render a phone-sized downloadable reflection card image.
pub async fn anky_reflection_card_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let (bytes, slug) = render_anky_reflection_card_bytes(&state, &id).await?;
    let filename = if slug.is_empty() {
        "anky-reflection".to_string()
    } else {
        slug
    };
    let disposition =
        axum::http::HeaderValue::from_str(&format!("inline; filename=\"{}.png\"", filename))
            .map_err(|e| AppError::Internal(format!("header error: {}", e)))?;

    let mut response = Response::new(bytes.into());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("image/png"),
    );
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("public, max-age=300, s-maxage=300"),
    );
    response
        .headers_mut()
        .insert(axum::http::header::CONTENT_DISPOSITION, disposition);
    Ok(response)
}

// ==================== Studio Video Upload ====================

/// POST /api/v1/studio/upload — multipart: video (WebM blob) + metadata (JSON)
pub async fn upload_studio_video(
    State(state): State<AppState>,
    jar: CookieJar,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = jar.get("anky_user_id").map(|c| c.value().to_string());

    let video_id = uuid::Uuid::new_v4().to_string();
    let mut video_data: Option<Vec<u8>> = None;
    let mut metadata: Option<serde_json::Value> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "video" => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("video read error: {}", e)))?;
                video_data = Some(bytes.to_vec());
            }
            "metadata" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("metadata read error: {}", e)))?;
                metadata = Some(serde_json::from_str(&text).unwrap_or_else(|_| json!({})));
            }
            _ => {}
        }
    }

    let video_bytes = video_data.ok_or_else(|| AppError::BadRequest("no video field".into()))?;
    let meta = metadata.unwrap_or_else(|| json!({}));

    // Ensure flat videos directory exists
    tokio::fs::create_dir_all("videos")
        .await
        .map_err(|e| AppError::Internal(format!("mkdir error: {}", e)))?;

    let file_path = format!("{}.webm", video_id);
    let full_path = format!("videos/{}", file_path);
    tokio::fs::write(&full_path, &video_bytes)
        .await
        .map_err(|e| AppError::Internal(format!("write error: {}", e)))?;

    let duration = meta["duration_seconds"].as_f64().unwrap_or(0.0);
    let title = meta["title"].as_str();
    let scene_data = meta.get("scenes").map(|s| s.to_string());

    {
        let db = state.db.lock().await;
        queries::insert_video_recording(
            &db,
            &video_id,
            user_id.as_deref(),
            title,
            &file_path,
            duration,
            scene_data.as_deref(),
        )?;
    }

    let size_mb = video_bytes.len() as f64 / (1024.0 * 1024.0);
    state.emit_log(
        "INFO",
        "studio",
        &format!(
            "Video uploaded: {} ({:.1}MB, {:.0}s)",
            &video_id[..8],
            size_mb,
            duration
        ),
    );

    Ok(Json(json!({
        "video_id": video_id,
        "file_path": file_path,
        "size_mb": format!("{:.1}", size_mb),
        "status": "uploaded",
    })))
}

// ==================== Feed ====================

#[derive(serde::Deserialize, Default)]
pub struct FeedQuery {
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_per_page")]
    pub per_page: i32,
}

fn default_page() -> i32 {
    1
}
fn default_per_page() -> i32 {
    20
}

/// GET /api/v1/feed?page=1&per_page=20
pub async fn get_feed(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Query(query): axum::extract::Query<FeedQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let viewer_id = jar.get("anky_user_id").map(|c| c.value().to_string());
    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 50);

    let items = {
        let db = state.db.lock().await;
        queries::get_feed(&db, viewer_id.as_deref(), page, per_page)?
    };

    let data: Vec<serde_json::Value> = items
        .iter()
        .map(|item| {
            let img = item
                .image_webp
                .as_ref()
                .map(|p| format!("/data/images/{}", p))
                .or_else(|| {
                    item.image_path
                        .as_ref()
                        .map(|p| format!("/data/images/{}", p))
                });
            json!({
                "id": item.id,
                "title": item.title,
                "image_url": img,
                "thinker_name": item.thinker_name,
                "created_at": item.created_at,
                "like_count": item.like_count,
                "user_liked": item.user_liked,
            })
        })
        .collect();

    Ok(Json(json!(data)))
}

// ==================== Likes ====================

/// POST /api/v1/anky/:id/like — toggle like
pub async fn toggle_like(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = if let Some(c) = jar.get("anky_user_id") {
        c.value().to_string()
    } else if let Some(auth_user) = crate::routes::auth::get_auth_user(&state, &jar).await {
        auth_user.user_id
    } else {
        return Err(AppError::BadRequest("login required".into()));
    };

    let (liked, count) = {
        let db = state.db.lock().await;
        let liked = queries::toggle_like(&db, &user_id, &id)?;
        let count = queries::get_like_count(&db, &id)?;
        (liked, count)
    };

    Ok(Json(json!({
        "liked": liked,
        "like_count": count,
    })))
}

/// POST /api/memory/backfill — backfill memory for all existing writing sessions
pub async fn memory_backfill(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let anthropic_key = state.config.anthropic_api_key.clone();
    let ollama_url = state.config.ollama_base_url.clone();

    if anthropic_key.is_empty() {
        return Err(AppError::Internal(
            "Anthropic API key not configured".into(),
        ));
    }

    state.emit_log("INFO", "memory", "Starting memory backfill...");

    let s = state.clone();
    tokio::spawn(async move {
        let (processed, total) =
            crate::pipeline::memory_pipeline::backfill_memories(&s, &ollama_url, &anthropic_key)
                .await;
        s.emit_log(
            "INFO",
            "memory",
            &format!(
                "Backfill complete: {}/{} sessions processed",
                processed, total
            ),
        );
    });

    Ok(Json(json!({ "status": "backfill_started" })))
}

fn draw_rect(img: &mut image::RgbImage, x: u32, y: u32, w: u32, h: u32, color: image::Rgb<u8>) {
    for dx in 0..w {
        for dy in 0..h {
            if x + dx < img.width() && y + dy < img.height() {
                img.put_pixel(x + dx, y + dy, color);
            }
        }
    }
}
