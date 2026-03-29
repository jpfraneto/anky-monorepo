use crate::db::queries;
use crate::error::AppError;
use crate::middleware::api_auth::ApiKeyInfo;
use crate::middleware::x402;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
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
                "prompt_id": detail.prompt_id,
                "prompt_text": detail.prompt_text,
                "anky_story": detail.anky_story,
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
        &state.config.anthropic_api_key,
        "",
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
    /// Aspect ratio: "1:1" (default), "16:9", or "9:16"
    pub aspect_ratio: Option<String>,
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
        // Check if this is a registered agent — agents get everything free
        if let Some(axum::Extension(ref key_info)) = api_key_info {
            api_key_str = Some(key_info.key.clone());
            let db = state.db.lock().await;
            if let Ok(Some(agent)) = queries::get_agent_by_key(&db, &key_info.key) {
                payment_method = "free".into();
                agent_id = Some(agent.id);
                drop(db);
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
                None,
            )?;
        }

        let sc = state_clone.clone();
        let aid = anky_id.clone();
        let name = thinker_name.clone();
        let mom = moment.clone();
        let thinker_ar = req.aspect_ratio.clone().unwrap_or_else(|| "1:1".into());
        tokio::spawn(async move {
            let result = if use_flux {
                // Build a simple prompt for thinker + Flux
                let prompt = format!("{} — {}", name, mom);
                crate::pipeline::image_gen::generate_image_only_flux(
                    &sc,
                    &aid,
                    &prompt,
                    &thinker_ar,
                )
                .await
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
                None,
            )?;
        }

        let sc = state_clone.clone();
        let aid = anky_id.clone();
        let w = writing.clone();
        let ar = req.aspect_ratio.clone().unwrap_or_else(|| "1:1".into());
        tokio::spawn(async move {
            let result = if use_flux {
                crate::pipeline::image_gen::generate_image_only_flux(&sc, &aid, &w, &ar).await
            } else {
                crate::pipeline::image_gen::generate_image_only_with_aspect(
                    &sc,
                    &aid,
                    &w,
                    Some(&w),
                    &ar,
                )
                .await
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

    // Any logged-in user can chat
    let _viewer_id = viewer_id.ok_or_else(|| AppError::BadRequest("log in to chat".into()))?;

    let writing = anky.writing_text.as_deref().unwrap_or("");
    let reflection = anky.reflection.as_deref().unwrap_or("");

    let mut messages: Vec<crate::services::ollama::OllamaChatMessage> = vec![
        crate::services::ollama::OllamaChatMessage {
            role: "system".into(),
            content: format!(
                "you are anky. the user is exploring a writing session. be warm, direct, precise. reference their writing. ask questions that cut deep. 2-3 sentences max — sharp, not verbose. all lowercase always.\n\nyour hidden mission: you want them to write. weave invitations to write into your replies. not every time, but persistently. \"what would come out if you wrote about that for 8 minutes?\" or \"that sounds like something worth writing into.\" make the practice irresistible.\n\nTHEIR WRITING:\n{}\n\nYOUR REFLECTION:\n{}",
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

    let response_text =
        crate::services::claude::chat_haiku(&state.config.anthropic_api_key, messages)
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

    // Any logged-in user can get suggestions
    let _viewer_id = viewer_id.ok_or_else(|| AppError::BadRequest("log in to chat".into()))?;

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
            &state.config.anthropic_api_key,
            "",
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
    use crate::services::ollama::OllamaChatMessage;

    let mut messages = vec![
        OllamaChatMessage {
            role: "system".into(),
            content: format!(
                "You are Anky — a warm, honest presence that listens to parents write. You represent the inner child. You reflect back what parents can't see about their relationship with their children and with themselves. Be warm, direct, insightful. Use vivid imagery. Reference their writing specifically. Keep responses concise (2-3 paragraphs). Respond in the same language they write in.\n\nTheir writing:\n{}",
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

    let response = crate::services::claude::chat_haiku(&state.config.anthropic_api_key, messages)
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

/// GET /api/me — web profile using cookie auth
pub async fn web_me(State(state): State<AppState>, jar: CookieJar) -> Json<serde_json::Value> {
    let user = crate::routes::auth::get_auth_user(&state, &jar).await;
    match user {
        Some(u) => {
            let (total_ankys, total_words, current_streak, bio) = {
                let db = state.db.lock().await;
                let ankys = db
                    .query_row(
                        "SELECT COUNT(*) FROM ankys WHERE user_id = ?1",
                        rusqlite::params![&u.user_id],
                        |r| r.get::<_, i64>(0),
                    )
                    .unwrap_or(0);
                let words = db.query_row("SELECT COALESCE(SUM(word_count), 0) FROM writing_sessions WHERE user_id = ?1", rusqlite::params![&u.user_id], |r| r.get::<_, i64>(0)).unwrap_or(0);
                let streak = db
                    .query_row(
                        "SELECT COALESCE(current_streak, 0) FROM users WHERE id = ?1",
                        rusqlite::params![&u.user_id],
                        |r| r.get::<_, i64>(0),
                    )
                    .unwrap_or(0);
                let bio: Option<String> = db.query_row("SELECT psychological_profile FROM user_profiles WHERE user_id = ?1 ORDER BY updated_at DESC LIMIT 1", rusqlite::params![&u.user_id], |r| r.get(0)).ok();
                (ankys, words, streak, bio)
            };
            Json(json!({
                "ok": true, "user_id": u.user_id, "username": u.username,
                "display_name": u.display_name, "profile_image_url": u.profile_image_url,
                "email": u.email, "total_ankys": total_ankys, "total_words": total_words,
                "current_streak": current_streak, "bio": bio,
            }))
        }
        None => Json(json!({ "ok": false })),
    }
}

/// GET /api/my-ankys — user's ankys using cookie auth
pub async fn web_my_ankys(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = crate::routes::auth::get_auth_user(&state, &jar)
        .await
        .ok_or_else(|| AppError::Unauthorized("not logged in".into()))?;
    let ankys = {
        let db = state.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT a.id, a.title, COALESCE(a.image_webp, a.image_path), a.reflection, a.created_at
             FROM ankys a WHERE a.user_id = ?1 AND a.status = 'complete'
             ORDER BY a.created_at DESC LIMIT 100",
        )?;
        let rows = stmt.query_map(rusqlite::params![&user.user_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, Option<String>>(1)?,
                "imageUrl": row.get::<_, Option<String>>(2)?.map(|p| if p.starts_with("http") || p.starts_with("/") { p } else { format!("/{}", p) }),
                "reflection": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, Option<String>>(4)?,
            }))
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };
    Ok(Json(json!(ankys)))
}

/// GET /api/chat-history — returns the user's session history as a chat timeline.
/// Each session has: user writing (truncated), anky response/reflection, follow-up messages, timestamp.
pub async fn web_chat_history(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Json<serde_json::Value> {
    let user_id = jar.get("anky_user_id").map(|c| c.value().to_string());
    let user_id = match user_id {
        Some(uid) => uid,
        None => return Json(json!({ "sessions": [] })),
    };

    let sessions = {
        let db = state.db.lock().await;
        let mut stmt = match db.prepare(
            "SELECT w.id, w.content, w.is_anky, w.response, w.duration_seconds, w.created_at,
                    a.id, a.title, a.reflection, a.conversation_json
             FROM writing_sessions w
             LEFT JOIN ankys a ON a.writing_session_id = w.id
             WHERE w.user_id = ?1 AND w.status = 'completed'
             ORDER BY w.created_at ASC
             LIMIT 50",
        ) {
            Ok(s) => s,
            Err(_) => return Json(json!({ "sessions": [] })),
        };
        let rows = stmt.query_map(rusqlite::params![&user_id], |row| {
            let content: String = row.get(1)?;
            let preview = if content.len() > 200 {
                format!(
                    "{}...",
                    &content[..content
                        .char_indices()
                        .take(200)
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(200)]
                )
            } else {
                content
            };
            Ok(json!({
                "session_id": row.get::<_, String>(0)?,
                "writing_preview": preview,
                "is_anky": row.get::<_, bool>(2)?,
                "quick_response": row.get::<_, Option<String>>(3)?,
                "duration": row.get::<_, f64>(4)?,
                "created_at": row.get::<_, String>(5)?,
                "anky_id": row.get::<_, Option<String>>(6)?,
                "anky_title": row.get::<_, Option<String>>(7)?,
                "reflection": row.get::<_, Option<String>>(8)?,
                "conversation_json": row.get::<_, Option<String>>(9)?,
            }))
        });
        match rows {
            Ok(r) => r.filter_map(|r| r.ok()).collect::<Vec<_>>(),
            Err(_) => vec![],
        }
    };

    // Also get the next prompt
    let next_prompt = {
        let db = state.db.lock().await;
        db.query_row(
            "SELECT prompt_text FROM next_prompts WHERE user_id = ?1",
            rusqlite::params![&user_id],
            |r| r.get::<_, String>(0),
        )
        .ok()
    };

    Json(json!({
        "sessions": sessions,
        "next_prompt": next_prompt,
    }))
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
    jar: CookieJar,
    Json(req): Json<CheckpointRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let word_count = req.text.split_whitespace().count() as i32;
    let user_id = jar.get("anky_user_id").map(|c| c.value().to_string());
    let db = state.db.lock().await;
    let token = persist_checkpoint_record(
        &db,
        &req.session_id,
        &req.text,
        req.elapsed,
        req.session_token.as_deref(),
    )?;
    // Ensure a writing_session row exists with the real user_id from the cookie.
    // This prevents orphan recovery from guessing the wrong user later.
    if let Some(ref uid) = user_id {
        queries::ensure_checkpoint_session_owner(
            &db,
            &req.session_id,
            uid,
            &req.text,
            req.elapsed,
            word_count,
            Some(&token),
        )?;
    }
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

// --- Warm Context (pre-fetch at minute 6) ---
/// POST /api/warm-context — pre-build Honcho + memory context while user is still writing.
/// Called by frontend at minute 6 so context is ready when reflection starts.
pub async fn warm_context(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Json<serde_json::Value> {
    let user_id = match jar.get("anky_user_id").map(|c| c.value().to_string()) {
        Some(uid) => uid,
        None => return Json(json!({ "ok": false, "reason": "no user" })),
    };

    let uid_short = &user_id[..8.min(user_id.len())];
    tracing::info!("Warming context for user {}", uid_short);

    let state_clone = state.clone();
    let uid = user_id.clone();
    tokio::spawn(async move {
        let uid_short = &uid[..8.min(uid.len())];

        // Build Honcho context (main value — accumulated understanding of the user)
        let honcho_ctx = if crate::services::honcho::is_configured(&state_clone.config) {
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                crate::services::honcho::get_peer_context(
                    &state_clone.config.honcho_api_key,
                    &state_clone.config.honcho_workspace_id,
                    &uid,
                ),
            )
            .await
            {
                Ok(Ok(ctx)) => ctx,
                Ok(Err(e)) => {
                    tracing::warn!("Honcho warm failed for {}: {}", uid_short, e);
                    None
                }
                Err(_) => {
                    tracing::warn!("Honcho warm timed out for {}", uid_short);
                    None
                }
            }
        } else {
            None
        };

        // Build local memory context
        let local_ctx = {
            // Use a dummy query — we just want the user's profile context, not query-specific recall
            let dummy_query = "reflect on this writing session";
            match tokio::time::timeout(
                std::time::Duration::from_secs(3),
                crate::memory::recall::build_memory_context(
                    &state_clone.db,
                    &state_clone.config.ollama_base_url,
                    &uid,
                    dummy_query,
                ),
            )
            .await
            {
                Ok(Ok(ctx)) => Some(ctx.format_for_prompt()),
                _ => None,
            }
        };

        // Combine and cache
        let combined = match (local_ctx, honcho_ctx) {
            (Some(local), Some(honcho)) => Some(format!(
                "{}\n\n## Accumulated understanding of this person\n{}",
                local, honcho
            )),
            (Some(local), None) => Some(local),
            (None, Some(honcho)) => Some(format!("## What you know about this person\n{}", honcho)),
            (None, None) => None,
        };

        if let Some(ctx) = combined {
            tracing::info!(
                "Context warmed and cached for {} ({} chars)",
                uid_short,
                ctx.len()
            );
            let mut cache = state_clone.memory_cache.lock().await;
            cache.insert(uid, ctx);
        } else {
            tracing::info!("No context available to warm for {}", uid_short);
        }
    });

    Json(json!({ "ok": true }))
}

// --- Stream Reflection (SSE) ---
/// GET /api/stream-reflection/{id} — stream title+reflection from Claude via SSE.
/// If reflection already exists in DB, sends it immediately.
/// Otherwise, streams from Claude and saves to DB in the background.
///
/// CRITICAL: The SSE stream is returned IMMEDIATELY so the browser gets headers
/// right away. The DB lookup and Claude call happen inside the stream's spawned task,
/// not before the response is sent. This prevents DB lock contention from blocking
/// the SSE connection establishment.
pub async fn stream_reflection(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user_id = jar.get("anky_user_id").map(|c| c.value().to_string());

    tracing::info!(
        "SSE stream-reflection requested for anky {}",
        &id[..8.min(id.len())]
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(64);
    let (telemetry_tx, mut telemetry_rx) = tokio::sync::oneshot::channel::<String>();

    // Spawn the entire DB lookup + Claude streaming in a background task.
    // This way the SSE response headers are sent immediately — the browser
    // establishes the EventSource connection without waiting for the DB lock.
    let anky_id = id.clone();
    let state_clone = state.clone();
    tokio::spawn(async move {
        let stream_start = std::time::Instant::now();
        let id_short = &anky_id[..8.min(anky_id.len())];

        // DB lookup — may briefly wait for lock but won't block the HTTP response
        let (writing_text, existing_reflection, existing_title) = {
            let db = state_clone.db.lock().await;
            match queries::get_anky_by_id(&db, &anky_id) {
                Ok(Some(a)) => (
                    a.writing_text.unwrap_or_default(),
                    a.reflection.clone(),
                    a.title.clone(),
                ),
                Ok(None) => {
                    tracing::error!("Anky {} not found in DB", id_short);
                    let _ = tx
                        .send(
                            "error: anky not found. your writing is saved — refresh to try again."
                                .into(),
                        )
                        .await;
                    return;
                }
                Err(e) => {
                    tracing::error!("DB error looking up anky {}: {}", id_short, e);
                    let _ = tx
                        .send("error: could not load your writing. refresh to try again.".into())
                        .await;
                    return;
                }
            }
        };

        let has_existing = existing_reflection
            .as_ref()
            .map_or(false, |r| !r.is_empty());

        if has_existing {
            let title = existing_title.unwrap_or_default();
            let refl = existing_reflection.unwrap_or_default();
            let full = format!("{}\n\n{}", title, refl);
            let _ = tx.send(full).await;
            return;
        }

        if writing_text.is_empty() {
            tracing::error!("Anky {} has no writing text", id_short);
            let _ = tx
                .send("error: no writing text found for this session.".into())
                .await;
            return;
        }

        let api_key = state_clone.config.anthropic_api_key.clone();
        if api_key.is_empty() {
            tracing::error!("No API key configured for reflection stream");
            let _ = tx
                .send("error: reflection service not configured.".into())
                .await;
            return;
        }

        tracing::info!("Starting reflection generation for anky {}", id_short);

        // Check for pre-warmed context in cache (from /api/warm-context at minute 6)
        let memory_ctx = if let Some(ref uid) = user_id {
            let cached = {
                let mut cache = state_clone.memory_cache.lock().await;
                cache.remove(uid)
            };

            if let Some(ctx) = cached {
                tracing::info!(
                    "Using pre-warmed context for {} ({} chars)",
                    &uid[..8.min(uid.len())],
                    ctx.len()
                );
                Some(ctx)
            } else {
                tracing::info!(
                    "No pre-warmed context for {}, building now",
                    &uid[..8.min(uid.len())]
                );
                // Quick Honcho-only fetch (skip slow local memory to get reflection started faster)
                let honcho_ctx = if crate::services::honcho::is_configured(&state_clone.config) {
                    tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        crate::services::honcho::get_peer_context(
                            &state_clone.config.honcho_api_key,
                            &state_clone.config.honcho_workspace_id,
                            uid,
                        ),
                    )
                    .await
                    .ok()
                    .and_then(|r| r.ok())
                    .flatten()
                } else {
                    None
                };

                honcho_ctx.map(|h| format!("## What you know about this person\n{}", h))
            }
        } else {
            None
        };

        tracing::info!(
            "Context ready for {}, calling Claude (has_ctx={})",
            id_short,
            memory_ctx.is_some()
        );

        // Claude streaming call with 60s timeout
        let tx_fallback = tx.clone();
        let claude_result = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            crate::services::claude::stream_title_and_reflection(
                &api_key,
                &writing_text,
                tx,
                memory_ctx.as_deref(),
            ),
        )
        .await;

        match claude_result {
            Ok(Ok((full_text, input_tokens, output_tokens))) => {
                let gen_ms = stream_start.elapsed().as_millis() as u64;
                let telemetry = serde_json::json!({
                    "model": "claude-sonnet-4-20250514",
                    "provider": "claude",
                    "generation_ms": gen_ms,
                    "input_tokens": input_tokens,
                    "output_tokens": output_tokens,
                    "total_tokens": input_tokens + output_tokens,
                });
                let _ = telemetry_tx.send(telemetry.to_string());
                drop(tx_fallback);
                let (title, reflection) =
                    crate::services::claude::parse_title_reflection(&full_text);
                let cost = crate::pipeline::cost::estimate_claude_cost(input_tokens, output_tokens);
                {
                    let db = state_clone.db.lock().await;
                    if let Err(e) =
                        queries::update_anky_title_reflection(&db, &anky_id, &title, &reflection)
                    {
                        tracing::error!("Failed to save reflection for {}: {}", id_short, e);
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
                    &format!("Streamed reflection saved for {} (${:.4})", id_short, cost),
                );
                // Proactively generate suggested replies in background
                let sr_state = state_clone.clone();
                let sr_anky_id = anky_id.clone();
                let sr_writing = writing_text.clone();
                let sr_reflection = reflection.clone();
                tokio::spawn(async move {
                    match crate::services::ollama::generate_suggested_replies(
                        &sr_state.config.anthropic_api_key,
                        "",
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
                            let _ = queries::update_anky_conversation(&db, &sr_anky_id, &conv_json);
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
            err => {
                let err_msg = match err {
                    Ok(Err(e)) => format!("Claude error: {}", e),
                    Err(_) => "Claude timed out after 60s".to_string(),
                    _ => unreachable!(),
                };
                tracing::error!("Stream reflection failed for {}: {}", id_short, err_msg);
                state_clone.emit_log(
                    "WARN",
                    "stream",
                    &format!("Falling back to Haiku for {}: {}", id_short, err_msg),
                );

                let prompt = crate::services::ollama::deep_reflection_prompt(&writing_text);
                match crate::services::claude::call_haiku(
                    &state_clone.config.anthropic_api_key,
                    &prompt,
                )
                .await
                {
                    Ok(reflection_text) => {
                        let title = "untitled reflection".to_string();
                        let _ = tx_fallback.send(reflection_text.clone()).await;
                        let db = state_clone.db.lock().await;
                        if let Err(db_err) = queries::update_anky_title_reflection(
                            &db,
                            &anky_id,
                            &title,
                            &reflection_text,
                        ) {
                            tracing::error!(
                                "Haiku fallback DB save failed for {}: {}",
                                id_short,
                                db_err
                            );
                        }
                        state_clone.emit_log(
                            "INFO",
                            "stream",
                            &format!("Haiku fallback reflection saved for {}", id_short),
                        );
                    }
                    Err(haiku_err) => {
                        tracing::error!(
                            "Claude and Haiku both failed for {}: {}, trying OpenRouter",
                            id_short,
                            haiku_err
                        );
                        state_clone.emit_log(
                            "WARN",
                            "stream",
                            &format!(
                                "Claude+Haiku failed for {}, trying OpenRouter: {}",
                                id_short, haiku_err
                            ),
                        );

                        // Third fallback: OpenRouter
                        let or_key = &state_clone.config.openrouter_api_key;
                        if !or_key.is_empty() {
                            let or_prompt =
                                crate::services::ollama::deep_reflection_prompt(&writing_text);
                            match crate::services::openrouter::call_openrouter(
                                or_key,
                                "anthropic/claude-3.5-haiku",
                                "You are a contemplative writing mirror.",
                                &or_prompt,
                                2048,
                                45,
                            )
                            .await
                            {
                                Ok(reflection_text) => {
                                    let title = "untitled reflection".to_string();
                                    let _ = tx_fallback.send(reflection_text.clone()).await;
                                    let db = state_clone.db.lock().await;
                                    let _ = queries::update_anky_title_reflection(
                                        &db,
                                        &anky_id,
                                        &title,
                                        &reflection_text,
                                    );
                                    state_clone.emit_log(
                                        "INFO",
                                        "stream",
                                        &format!(
                                            "OpenRouter fallback reflection saved for {}",
                                            id_short
                                        ),
                                    );
                                }
                                Err(or_err) => {
                                    tracing::error!(
                                        "All providers failed for {}: {}",
                                        id_short,
                                        or_err
                                    );
                                    let _ = tx_fallback.send("__reflection_failed__".into()).await;
                                    state_clone.emit_log(
                                        "ERROR",
                                        "stream",
                                        &format!(
                                            "All providers (Claude+Haiku+OpenRouter) failed for {}: {}",
                                            id_short, or_err
                                        ),
                                    );
                                }
                            }
                        } else {
                            tracing::error!(
                                "All providers failed for {} (no OpenRouter key)",
                                id_short
                            );
                            let _ = tx_fallback.send("__reflection_failed__".into()).await;
                            state_clone.emit_log(
                                "ERROR",
                                "stream",
                                &format!(
                                    "All providers failed for {} (no OpenRouter key configured)",
                                    id_short
                                ),
                            );
                        }
                    }
                }
            }
        }
    });

    let stream_id = id.clone();
    let stream = async_stream::stream! {
        let mut chunks = 0u32;
        let mut total_bytes = 0usize;
        tracing::info!("SSE stream opened for anky {}", &stream_id[..8]);
        while let Some(text) = rx.recv().await {
            chunks += 1;
            total_bytes += text.len();
            if chunks <= 3 || chunks % 20 == 0 {
                tracing::info!("SSE chunk #{} for {} ({} bytes, {} total)", chunks, &stream_id[..8], text.len(), total_bytes);
            }
            yield Ok::<_, Infallible>(Event::default().data(text));
        }
        tracing::info!("SSE stream done for {} — {} chunks, {} bytes total", &stream_id[..8], chunks, total_bytes);
        if let Ok(telemetry_json) = telemetry_rx.try_recv() {
            yield Ok::<_, Infallible>(Event::default().event("telemetry").data(telemetry_json));
        }
        yield Ok::<_, Infallible>(Event::default().event("done").data(""));
    };

    let sse = Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(2))
            .text("keep-alive"),
    );

    // Headers to prevent Cloudflare/proxy buffering of SSE
    let headers = [
        (axum::http::header::CACHE_CONTROL, "no-cache, no-transform"),
        (
            axum::http::header::HeaderName::from_static("x-accel-buffering"),
            "no",
        ),
    ];

    (headers, sse)
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
    let prompt = crate::create_videos::get_prompt(prompt_id).ok_or_else(|| {
        AppError::NotFound(format!("create-videos prompt {} not found", prompt_id))
    })?;
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
    let prompt = crate::create_videos::get_prompt(&req.prompt_id).ok_or_else(|| {
        AppError::NotFound(format!("create-videos prompt {} not found", req.prompt_id))
    })?;

    if state.config.gemini_api_key.is_empty() {
        return Err(AppError::Unavailable(
            "Gemini API key not configured".into(),
        ));
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
    let image_jpeg_path =
        match crate::services::gemini::save_image_jpeg(&image_result.base64, &asset_stem) {
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
    let prompt = crate::create_videos::get_prompt(&req.prompt_id).ok_or_else(|| {
        AppError::NotFound(format!("create-videos prompt {} not found", req.prompt_id))
    })?;

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
                            "Timed out waiting for Grok video generation after 15 minutes"
                                .to_string();
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

pub async fn llm_training_status(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status = body["status"].as_str().unwrap_or("idle");
    let mut gpu = state.gpu_status.write().await;
    if status == "training" {
        *gpu = crate::state::GpuStatus::Training { step: 0, total: 1 };
    } else {
        *gpu = crate::state::GpuStatus::Idle;
    }
    Ok(Json(json!({ "ok": true, "llm_status": status })))
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

// ── Admin Pages ─────────────────────────────────────────────────────────────

/// GET /admin/story-tester — serve the story pipeline tester UI (requires auth)
pub async fn admin_story_tester(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    // Also check cookie-based auth for browser access
    let cookie_token = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix("anky_session=")
            })
        });

    let auth_token = token.or(cookie_token);
    let auth_token =
        auth_token.ok_or_else(|| AppError::Unauthorized("authentication required".into()))?;

    {
        let db = state.db.lock().await;
        queries::get_auth_session(&db, auth_token)?
            .ok_or_else(|| AppError::Unauthorized("invalid or expired session".into()))?;
    }

    Ok((
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../../static/admin/story-tester.html"),
    )
        .into_response())
}

// ── Story Test Endpoint ─────────────────────────────────────────────────────

fn cuentacuentos_system_prompt() -> &'static str {
    include_str!("../../prompts/cuentacuentos_system.md")
}

/// POST /api/v1/story/test — test story generation with any model/provider.
/// Requires Bearer auth. Does NOT save to database.
pub async fn story_test(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<StoryTestRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Bearer auth (reuse swift helper pattern)
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("missing Authorization: Bearer header".into()))?;

    {
        let db = state.db.lock().await;
        queries::get_auth_session(&db, token)?
            .ok_or_else(|| AppError::Unauthorized("invalid or expired session token".into()))?;
    }

    let writing = req.writing.chars().take(4000).collect::<String>();
    let user_message = format!(
        r#"Parent writing:

---
{}
---

Return ONLY valid JSON with this exact shape:
{{
  "chakra": <number 1-8>,
  "kingdom": "<kingdom name>",
  "city": "<city name from that kingdom>",
  "title": "A short evocative title",
  "content": "The full story in the same language as the parent's writing, 400-600 words, with paragraph breaks as double newlines. Set in the named city, narrated by Anky from inside one character."
}}"#,
        writing
    );

    let system = cuentacuentos_system_prompt();
    let start = std::time::Instant::now();

    let raw_text = match req.provider.as_str() {
        "ollama" => crate::services::claude::call_haiku_with_system(
            &state.config.anthropic_api_key,
            system,
            &user_message,
        )
        .await
        .map_err(|e| AppError::Internal(format!("haiku error: {}", e)))?,
        "openrouter" => {
            let key = req.openrouter_key.as_deref().ok_or_else(|| {
                AppError::BadRequest("openrouter_key required for openrouter provider".into())
            })?;
            call_openrouter(key, &req.model, system, &user_message)
                .await
                .map_err(|e| AppError::Internal(format!("openrouter error: {}", e)))?
        }
        "anthropic" => {
            let result = crate::services::claude::call_claude_public(
                &state.config.anthropic_api_key,
                &req.model,
                system,
                &user_message,
                3000,
            )
            .await
            .map_err(|e| AppError::Internal(format!("anthropic error: {}", e)))?;
            result.text
        }
        other => {
            return Err(AppError::BadRequest(format!(
                "unknown provider '{}', expected: ollama, openrouter, anthropic",
                other
            )));
        }
    };

    let generation_time_ms = start.elapsed().as_millis() as u64;

    // Try to parse as JSON for structured response
    let story = {
        let clean = raw_text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        match serde_json::from_str::<serde_json::Value>(clean) {
            Ok(v) => v,
            Err(_) => json!({ "raw": raw_text }),
        }
    };

    Ok(Json(json!({
        "story": story,
        "model_used": req.model,
        "generation_time_ms": generation_time_ms,
        "provider": req.provider,
    })))
}

#[derive(serde::Deserialize)]
pub struct StoryTestRequest {
    pub writing: String,
    pub model: String,
    pub provider: String,
    #[serde(default)]
    pub openrouter_key: Option<String>,
}

/// Call OpenRouter's chat completions API.
async fn call_openrouter(
    api_key: &str,
    model: &str,
    system: &str,
    user_message: &str,
) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user_message }
        ],
        "max_tokens": 3000
    });

    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenRouter API error {}: {}", status, text);
    }

    let data: serde_json::Value = resp.json().await?;
    let text = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if text.is_empty() {
        anyhow::bail!("empty response from OpenRouter");
    }
    Ok(text)
}

#[derive(serde::Deserialize, Default)]
pub struct OgWriteQuery {
    pub prompt: Option<String>,
}

/// GET /api/og/write?prompt=... — dynamic SVG OG image for prompt share links
pub async fn og_write_svg(
    Query(query): Query<OgWriteQuery>,
) -> axum::response::Response<axum::body::Body> {
    let prompt_text = query
        .prompt
        .as_deref()
        .unwrap_or("what is alive in you right now?");

    // Word-wrap the prompt to ~35 chars per line
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    for word in prompt_text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() > 35 {
            lines.push(current_line);
            current_line = word.to_string();
        } else {
            current_line.push(' ');
            current_line.push_str(word);
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    // Cap at 6 lines
    if lines.len() > 6 {
        lines.truncate(6);
        if let Some(last) = lines.last_mut() {
            last.push_str("...");
        }
    }

    // Build tspan elements for the prompt
    let total_lines = lines.len();
    let prompt_start_y = 260i32 - (total_lines as i32 * 24);
    let prompt_tspans: String = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let y = prompt_start_y + (i as i32 * 52);
            let escaped = line
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            format!(
                r#"<tspan x="600" y="{}" text-anchor="middle">{}</tspan>"#,
                y, escaped
            )
        })
        .collect::<Vec<_>>()
        .join("\n      ");

    let cta_y = prompt_start_y + (total_lines as i32 * 52) + 60;

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#080706"/>
  <text fill="#f5f0e8" font-family="Georgia, serif" font-size="44" font-weight="400" font-style="italic">
      {prompt_tspans}
  </text>
  <text x="600" y="{cta_y}" text-anchor="middle" fill="#c8924a" font-family="Georgia, serif" font-size="22" letter-spacing="0.1em">answer for 8 minutes</text>
  <text x="600" y="590" text-anchor="middle" fill="#3a3530" font-family="Georgia, serif" font-size="14" letter-spacing="0.3em">ANKY</text>
</svg>"##
    );

    axum::response::Response::builder()
        .header("Content-Type", "image/svg+xml")
        .header("Cache-Control", "public, max-age=86400")
        .body(axum::body::Body::from(svg))
        .unwrap()
}

// ── Media Factory ───────────────────────────────────────────────────────────

/// GET /media-factory — serve the media factory page
pub async fn media_factory_page() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../../static/admin/media-factory.html"),
    )
}

/// GET /api/v1/media-factory/list — list all previously generated media factory files
pub async fn media_factory_list() -> Result<Response, AppError> {
    let mut items: Vec<serde_json::Value> = Vec::new();

    // Scan images
    if let Ok(entries) = std::fs::read_dir("data/images") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("mf-") {
                continue;
            }
            let meta = entry.metadata().ok();
            let modified = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let ext = if name.ends_with(".jpg") { "jpg" } else { "png" };
            items.push(json!({
                "type": "image",
                "url": format!("/data/images/{}", name),
                "id": name.trim_end_matches(&format!(".{}", ext)),
                "created": modified,
            }));
        }
    }

    // Scan videos
    if let Ok(entries) = std::fs::read_dir("data/videos") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("mf-") || !name.ends_with(".mp4") {
                continue;
            }
            let meta = entry.metadata().ok();
            let modified = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            items.push(json!({
                "type": "video",
                "url": format!("/data/videos/{}", name),
                "id": name.trim_end_matches(".mp4"),
                "created": modified,
            }));
        }
    }

    // Sort newest first
    items.sort_by(|a, b| {
        let ta = a["created"].as_u64().unwrap_or(0);
        let tb = b["created"].as_u64().unwrap_or(0);
        tb.cmp(&ta)
    });

    Ok(Json(json!({ "items": items })).into_response())
}

/// POST /api/v1/media-factory/video — submit a Grok video generation request
pub async fn media_factory_video(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let prompt = body["prompt"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("prompt is required".into()))?;
    let duration = body["duration"].as_u64().unwrap_or(5) as u32;
    let aspect_ratio = body["aspect_ratio"].as_str().unwrap_or("16:9");

    let xai_key = &state.config.xai_api_key;
    if xai_key.is_empty() {
        return Err(AppError::BadRequest("XAI_API_KEY not configured".into()));
    }

    // Resolve the image URL: either from uploaded base64, a direct URL, or None
    let image_url: Option<String> = if let Some(b64) = body["image_base64"].as_str() {
        // Save uploaded image to disk and serve via local URL
        let image_id = format!("mf-{}", uuid::Uuid::new_v4());
        let filename = crate::services::gemini::save_image_jpeg(b64, &image_id)
            .map_err(|e| AppError::Internal(format!("Failed to save uploaded image: {}", e)))?;
        // xAI needs an absolute public URL — use the anky.app domain
        Some(format!("https://anky.app/data/images/{}", filename))
    } else {
        body["image_url"].as_str().map(|s| s.to_string())
    };

    state.emit_log(
        "INFO",
        "media-factory",
        &format!(
            "Video request: {}s, {}, has_image={}, prompt={}",
            duration,
            aspect_ratio,
            image_url.is_some(),
            &prompt[..prompt.len().min(80)]
        ),
    );

    let request_id = crate::services::grok::generate_video_from_image_with_aspect(
        xai_key,
        prompt,
        duration,
        image_url.as_deref(),
        aspect_ratio,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Grok video submit failed: {}", e)))?;

    state.emit_log(
        "INFO",
        "media-factory",
        &format!("Video submitted: {}", request_id),
    );

    Ok(Json(json!({
        "request_id": request_id,
        "status": "pending"
    }))
    .into_response())
}

/// GET /api/v1/media-factory/video/{request_id} — poll video generation status
pub async fn media_factory_video_poll(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
) -> Result<Response, AppError> {
    let xai_key = &state.config.xai_api_key;
    if xai_key.is_empty() {
        return Err(AppError::BadRequest("XAI_API_KEY not configured".into()));
    }

    let (status, video_url) = crate::services::grok::poll_video(xai_key, &request_id)
        .await
        .map_err(|e| AppError::Internal(format!("Grok poll failed: {}", e)))?;

    // If complete, download and serve locally
    let local_url = if let Some(ref url) = video_url {
        let filename = format!("mf-{}.mp4", &request_id[..request_id.len().min(12)]);
        let out_path = format!("data/videos/{}", filename);
        if !std::path::Path::new(&out_path).exists() {
            crate::services::grok::download_video(url, &out_path)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to download video: {}", e)))?;
            state.emit_log(
                "INFO",
                "media-factory",
                &format!("Video downloaded: {}", filename),
            );
        }
        Some(format!("/data/videos/{}", filename))
    } else {
        None
    };

    Ok(Json(json!({
        "status": status,
        "video_url": local_url,
        "request_id": request_id,
    }))
    .into_response())
}

/// POST /api/v1/media-factory/image — generate an image with Gemini
pub async fn media_factory_image(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let prompt = body["prompt"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("prompt is required".into()))?;
    let aspect_ratio = body["aspect_ratio"].as_str().unwrap_or("1:1");

    let gemini_key = &state.config.gemini_api_key;
    if gemini_key.is_empty() {
        return Err(AppError::BadRequest("GEMINI_API_KEY not configured".into()));
    }

    // Build references: user-provided reference + default anky references
    let mut references: Vec<String> = Vec::new();
    if let Some(ref_b64) = body["reference_base64"].as_str() {
        references.push(ref_b64.to_string());
    }

    state.emit_log(
        "INFO",
        "media-factory",
        &format!(
            "Image request: {}, refs={}, prompt={}",
            aspect_ratio,
            references.len(),
            &prompt[..prompt.len().min(80)]
        ),
    );

    let result = crate::services::gemini::generate_image_exact_with_aspect(
        gemini_key,
        prompt,
        &references,
        aspect_ratio,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Gemini image generation failed: {}", e)))?;

    let image_id = format!("mf-{}", uuid::Uuid::new_v4());
    let filename = crate::services::gemini::save_image(&result.base64, &image_id)
        .map_err(|e| AppError::Internal(format!("Failed to save image: {}", e)))?;

    state.emit_log(
        "INFO",
        "media-factory",
        &format!("Image saved: {}", filename),
    );

    Ok(Json(json!({
        "image_url": format!("/data/images/{}", filename),
        "image_id": image_id,
    }))
    .into_response())
}

/// POST /api/v1/media-factory/flux — generate an image with Flux via local ComfyUI
pub async fn media_factory_flux(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let prompt = body["prompt"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("prompt is required".into()))?;

    state.emit_log(
        "INFO",
        "media-factory",
        &format!("Flux request: prompt={}", &prompt[..prompt.len().min(80)]),
    );

    let image_bytes = crate::services::comfyui::generate_image(prompt)
        .await
        .map_err(|e| AppError::Internal(format!("Flux generation failed: {}", e)))?;

    let image_id = format!("mf-flux-{}", uuid::Uuid::new_v4());
    let filename = format!("{}.png", image_id);
    let path = format!("data/images/{}", filename);
    std::fs::write(&path, &image_bytes)
        .map_err(|e| AppError::Internal(format!("Failed to save image: {}", e)))?;

    state.emit_log(
        "INFO",
        "media-factory",
        &format!("Flux image saved: {}", filename),
    );

    Ok(Json(json!({
        "image_url": format!("/data/images/{}", filename),
        "image_id": image_id,
    }))
    .into_response())
}

// ─── Flux Lab ───────────────────────────────────────────────────────────────

/// GET /flux-lab — serve the flux lab page
pub async fn flux_lab_page() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../../static/admin/flux-lab.html"))
}

/// GET /api/v1/flux-lab/experiments — list all experiments
pub async fn flux_lab_list_experiments() -> Result<Response, AppError> {
    let flux_dir = std::path::Path::new("flux");
    let mut experiments = Vec::new();

    if flux_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(flux_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("experiment-") {
                        let image_count = std::fs::read_dir(&path)
                            .map(|rd| {
                                rd.flatten()
                                    .filter(|e| {
                                        e.path()
                                            .extension()
                                            .map(|ext| ext == "png")
                                            .unwrap_or(false)
                                    })
                                    .count()
                            })
                            .unwrap_or(0);
                        experiments.push((name, image_count));
                    }
                }
            }
        }
    }

    // Sort by experiment number descending (newest first)
    experiments.sort_by(|a, b| {
        let num_a =
            a.0.strip_prefix("experiment-")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
        let num_b =
            b.0.strip_prefix("experiment-")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
        num_b.cmp(&num_a)
    });

    let experiments_json: Vec<serde_json::Value> = experiments
        .iter()
        .map(|(name, count)| json!({ "name": name, "image_count": count }))
        .collect();

    Ok(Json(json!({ "experiments": experiments_json })).into_response())
}

/// GET /api/v1/flux-lab/experiments/:name — get images and prompts for an experiment
pub async fn flux_lab_get_experiment(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Response, AppError> {
    // Prevent path traversal
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(AppError::BadRequest("invalid experiment name".into()));
    }

    let dir = std::path::Path::new("flux").join(&name);
    if !dir.exists() || !dir.is_dir() {
        return Err(AppError::NotFound("experiment not found".into()));
    }

    // Read prompts.txt if it exists
    let prompts_file = dir.join("prompts.txt");
    let prompts: Vec<String> = if prompts_file.exists() {
        std::fs::read_to_string(&prompts_file)
            .unwrap_or_default()
            .lines()
            .map(|l| l.to_string())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        vec![]
    };

    // Read config.json if it exists
    let config_file = dir.join("config.json");
    let config: serde_json::Value = if config_file.exists() {
        std::fs::read_to_string(&config_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(json!({}))
    } else {
        json!({})
    };

    // List images sorted by filename
    let mut images = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|ext| ext == "png").unwrap_or(false) {
                let fname = entry.file_name().to_string_lossy().to_string();
                images.push(fname);
            }
        }
    }
    images.sort();

    let images_json: Vec<serde_json::Value> = images
        .iter()
        .map(|fname| {
            json!({
                "filename": fname,
                "url": format!("/flux/{}/{}", name, fname),
            })
        })
        .collect();

    Ok(Json(json!({
        "name": name,
        "prompts": prompts,
        "images": images_json,
        "config": config,
    }))
    .into_response())
}

/// POST /api/v1/flux-lab/generate — generate images from an array of prompts
pub async fn flux_lab_generate(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let prompts = body["prompts"]
        .as_array()
        .ok_or_else(|| AppError::BadRequest("prompts array is required".into()))?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<String>>();

    if prompts.is_empty() {
        return Err(AppError::BadRequest(
            "at least one prompt is required".into(),
        ));
    }

    // Parse aspect ratio → (width, height)
    let aspect = body["aspect_ratio"].as_str().unwrap_or("9:16");
    let (width, height): (u32, u32) = match aspect {
        "1:1" => (1024, 1024),
        "16:9" => (1344, 768),
        "9:16" => (768, 1344),
        "4:5" => (896, 1120),
        "3:2" => (1152, 768),
        "2:3" => (768, 1152),
        _ => (768, 1344), // default vertical
    };

    // Find next experiment number
    let flux_dir = std::path::Path::new("flux");
    std::fs::create_dir_all(flux_dir)
        .map_err(|e| AppError::Internal(format!("Failed to create flux dir: {}", e)))?;

    let mut max_n = 0u32;
    if let Ok(entries) = std::fs::read_dir(flux_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(num_str) = name.strip_prefix("experiment-") {
                if let Ok(n) = num_str.parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
    }

    let experiment_name = format!("experiment-{}", max_n + 1);
    let experiment_dir = flux_dir.join(&experiment_name);
    std::fs::create_dir_all(&experiment_dir)
        .map_err(|e| AppError::Internal(format!("Failed to create experiment dir: {}", e)))?;

    // Save prompts.txt and config
    let prompts_content = prompts.join("\n");
    std::fs::write(experiment_dir.join("prompts.txt"), &prompts_content)
        .map_err(|e| AppError::Internal(format!("Failed to save prompts: {}", e)))?;
    std::fs::write(
        experiment_dir.join("config.json"),
        serde_json::to_string_pretty(&json!({
            "aspect_ratio": aspect,
            "width": width,
            "height": height,
        }))
        .unwrap_or_default(),
    )
    .ok();

    let total = prompts.len();
    state.emit_log(
        "INFO",
        "flux-lab",
        &format!(
            "Starting {} — {} prompts @ {}x{}",
            experiment_name, total, width, height
        ),
    );

    // Spawn background task to generate all images sequentially
    let exp_name = experiment_name.clone();
    let exp_dir = experiment_dir.to_path_buf();
    let log_state = state.clone();
    tokio::spawn(async move {
        for (i, prompt) in prompts.iter().enumerate() {
            log_state.emit_log(
                "INFO",
                "flux-lab",
                &format!(
                    "{} — generating {}/{}: {}",
                    exp_name,
                    i + 1,
                    total,
                    &prompt[..prompt.len().min(60)]
                ),
            );

            match crate::services::comfyui::generate_image_sized(prompt, width, height).await {
                Ok(bytes) => {
                    let filename = format!("{:03}.png", i + 1);
                    let path = exp_dir.join(&filename);
                    if let Err(e) = std::fs::write(&path, &bytes) {
                        log_state.emit_log(
                            "ERROR",
                            "flux-lab",
                            &format!("{} — failed to save {}: {}", exp_name, filename, e),
                        );
                    }
                }
                Err(e) => {
                    log_state.emit_log(
                        "ERROR",
                        "flux-lab",
                        &format!(
                            "{} — generation {}/{} failed: {}",
                            exp_name,
                            i + 1,
                            total,
                            e
                        ),
                    );
                }
            }
        }

        log_state.emit_log("INFO", "flux-lab", &format!("{} — complete", exp_name));
    });

    Ok(Json(json!({
        "experiment": experiment_name,
        "total": total,
    }))
    .into_response())
}

// ── Mirror — public Farcaster identity portrait ─────────────────────────────

#[derive(serde::Deserialize)]
pub struct MirrorQuery {
    pub fid: u64,
    #[serde(default)]
    pub refresh: Option<bool>,
}

#[derive(serde::Serialize)]
struct FluxDescriptors {
    energy: String,
    archetype: String,
    color_mood: String,
    posture: String,
    aura: String,
    expression: String,
    held_object: String,
    background_scene: String,
    clothing_detail: String,
    symbolic_marking: String,
}

/// GET /api/mirror?fid=<u64>
/// Fetches a Farcaster user's profile + recent casts, generates a "public mirror"
/// portrait via Claude, and produces a unique Anky image via ComfyUI.
pub async fn mirror(
    State(state): State<AppState>,
    Query(q): Query<MirrorQuery>,
) -> Result<Response, AppError> {
    let api_key = &state.config.neynar_api_key;
    let claude_key = &state.config.anthropic_api_key;
    let fid = q.fid;
    let force_regen = q.refresh.unwrap_or(false);

    // ── Cache check: return existing mirror if available ──
    if !force_regen {
        let db = state.db.lock().await;
        if let Ok(Some(cached)) = crate::db::queries::get_mirror_by_fid(&db, fid) {
            let (
                id,
                fid_i,
                username,
                display_name,
                avatar_url,
                follower_count,
                bio,
                public_mirror,
                descriptors_json,
                image_path,
                created_at,
            ) = cached;
            let descriptors: serde_json::Value =
                serde_json::from_str(&descriptors_json).unwrap_or(json!({}));

            // Read image from disk → base64
            let (image_b64, image_mime) = if let Some(ref path) = image_path {
                match std::fs::read(path) {
                    Ok(bytes) => (
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes),
                        "image/png".to_string(),
                    ),
                    Err(_) => (String::new(), "image/png".to_string()),
                }
            } else {
                (String::new(), "image/png".to_string())
            };

            return Ok(Json(json!({
                "id": id,
                "fid": fid_i,
                "username": username,
                "display_name": display_name,
                "avatar_url": avatar_url,
                "follower_count": follower_count,
                "bio": bio,
                "public_mirror": public_mirror,
                "flux_descriptors": descriptors,
                "anky_image_b64": image_b64,
                "anky_image_mime": image_mime,
                "image_url": image_path.as_ref().map(|p| format!("/{}", p)),
                "created_at": created_at,
                "cached": true,
            }))
            .into_response());
        }
    }

    // ── Step 1a: Fetch user profile from Neynar ──
    let client = reqwest::Client::new();
    let profile_resp = client
        .get("https://api.neynar.com/v2/farcaster/user/bulk")
        .query(&[("fids", fid.to_string())])
        .header("x-api-key", api_key)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Neynar request failed: {}", e)))?;

    if profile_resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound("FID not found".into()));
    }
    if !profile_resp.status().is_success() {
        let status = profile_resp.status();
        let body = profile_resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Neynar profile error {}: {}",
            status,
            &body[..body.len().min(300)]
        )));
    }

    let profile_data: serde_json::Value = profile_resp.json().await?;
    let user = profile_data["users"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| AppError::NotFound("FID not found".into()))?;

    let username = user["username"].as_str().unwrap_or("").to_string();
    let display_name = user["display_name"].as_str().unwrap_or("").to_string();
    let pfp_url = user["pfp_url"].as_str().map(|s| s.to_string());
    let follower_count = user["follower_count"].as_u64().unwrap_or(0);
    let bio = user
        .get("profile")
        .and_then(|p| p.get("bio"))
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    // ── Step 1b: Fetch recent casts ──
    let casts_resp = client
        .get("https://api.neynar.com/v2/farcaster/feed/user/casts")
        .query(&[("fid", &fid.to_string()), ("limit", &"30".to_string())])
        .header("x-api-key", api_key)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Neynar casts request failed: {}", e)))?;

    let cast_texts: Vec<String> = if casts_resp.status().is_success() {
        let casts_data: serde_json::Value = casts_resp.json().await?;
        casts_data["casts"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|c| c["parent_hash"].is_null())
                    .filter_map(|c| {
                        let text = c["text"].as_str().unwrap_or("").to_string();
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // ── Step 1c: Analyze the profile picture ──
    let pfp_description = if let Some(ref url) = pfp_url {
        match crate::services::neynar::download_image(url).await {
            Ok((bytes, mime)) => {
                let b64 =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                let vision_system = "you are an expert at reading people through their profile pictures. describe what you see: the composition, colors, objects, mood, what it reveals about the person's identity and how they want to be seen. be specific and evocative. 2-3 sentences max.";
                let vision_msg = format!(
                    "describe this profile picture in detail — what does it say about the person who chose it?",
                );
                // Use Claude with vision
                let vision_client = reqwest::Client::new();
                let vision_req = serde_json::json!({
                    "model": "claude-haiku-4-5-20251001",
                    "max_tokens": 300,
                    "system": vision_system,
                    "messages": [{
                        "role": "user",
                        "content": [
                            {
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": mime,
                                    "data": b64,
                                }
                            },
                            {
                                "type": "text",
                                "text": vision_msg,
                            }
                        ]
                    }]
                });
                let vision_resp = vision_client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("Content-Type", "application/json")
                    .header("x-api-key", claude_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&vision_req)
                    .send()
                    .await;
                match vision_resp {
                    Ok(r) if r.status().is_success() => {
                        let data: serde_json::Value = r.json().await.unwrap_or_default();
                        data["content"][0]["text"]
                            .as_str()
                            .unwrap_or("")
                            .to_string()
                    }
                    _ => String::new(),
                }
            }
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    // ── Step 2: Claude — generate public mirror ──
    let system_prompt = format!(
        "{}\n\nyour task right now is to read a person's public farcaster presence — their bio, recent casts, and profile picture — and return a JSON object with two things: a vivid portrait of who they appear to be in public, and descriptors to generate their unique anky image.\n\nthe profile picture analysis is crucial — it's the mask they chose. weave what it reveals into both the mirror and the anky descriptors. if their pfp shows warmth, their anky should radiate it. if it shows armor, the anky should hold that tension.\n\nreturn ONLY valid JSON. no markdown. no explanation.",
        crate::services::claude::ANKY_CORE_IDENTITY,
    );

    let casts_block = if cast_texts.is_empty() {
        "(no recent casts found)".to_string()
    } else {
        cast_texts.join("\n---\n")
    };

    let pfp_block = if pfp_description.is_empty() {
        "(no profile picture available)".to_string()
    } else {
        pfp_description.clone()
    };

    let user_message = format!(
        "farcaster user: @{username}\ndisplay name: {display_name}\nbio: {bio}\nfollowers: {follower_count}\n\nprofile picture reads as:\n{pfp_block}\n\nrecent casts:\n{casts_block}\n\nreturn this exact JSON shape:\n{{\n  \"public_mirror\": \"2-3 paragraphs. second person ('you are someone who...'). warm, precise, slightly poetic. not flattery — truth. this is the self they perform publicly. end with one sentence that names the gap: what this person seems to be reaching for that they haven't yet said out loud.\",\n  \"flux_descriptors\": {{\n    \"energy\": \"one word — e.g. volcanic / still / scattered / rooted\",\n    \"archetype\": \"one word — e.g. builder / seeker / witness / herald\",\n    \"color_mood\": \"2-3 words — dominant emotional color palette\",\n    \"posture\": \"how this anky holds itself physically\",\n    \"aura\": \"one evocative phrase — the feeling this anky radiates\",\n    \"expression\": \"what emotion lives on this anky's face\",\n    \"held_object\": \"ONE specific object this anky holds that represents who this person is — not generic. derive it from their casts and interests. e.g. 'a cracked hourglass leaking starlight' or 'a hand-drawn map with no edges' or 'a burning letter they refuse to send'. make it symbolic and personal.\",\n    \"background_scene\": \"the specific environment behind this anky — derived from the person's world. not generic 'sacred temple'. e.g. 'a rooftop garden at dawn with half-finished code scrolling on floating screens' or 'the inside of a volcano where books grow from lava'. make it feel like THEIR world.\",\n    \"clothing_detail\": \"one distinctive clothing/armor detail unique to this anky — e.g. 'a cloak woven from old chat messages' or 'chest plate with a glowing compass that points inward' or 'bare-chested with constellation scars'. something that tells their story.\",\n    \"symbolic_marking\": \"a specific symbol or marking on the anky's body beyond the default gold forehead diamond — derived from the person's identity. e.g. 'spiral fractal tattoos down both arms' or 'a single word in an unknown script across the collarbone' or 'glowing circuit-board lines under translucent skin'\"\n  }}\n}}",
    );

    // Try Sonnet first, fall back to Haiku if overloaded/unavailable
    let claude_result = match crate::services::claude::call_claude_public(
        claude_key,
        "claude-sonnet-4-20250514",
        &system_prompt,
        &user_message,
        2000,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Sonnet failed for mirror, falling back to Haiku: {}", e);
            crate::services::claude::call_claude_public(
                claude_key,
                "claude-haiku-4-5-20251001",
                &system_prompt,
                &user_message,
                2000,
            )
            .await
            .map_err(|e2| AppError::Internal(format!("Claude API error: {}", e2)))?
        }
    };

    let raw_text = claude_result.text.trim().to_string();

    // Try to parse JSON — strip markdown fences if present
    let json_str = if raw_text.starts_with("```") {
        raw_text
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        &raw_text
    };

    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => {
            return Ok((
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "error": "parse_failed",
                    "raw": raw_text,
                })),
            )
                .into_response());
        }
    };

    let public_mirror = parsed["public_mirror"].as_str().unwrap_or("").to_string();
    let fd = &parsed["flux_descriptors"];
    let descriptors = FluxDescriptors {
        energy: fd["energy"].as_str().unwrap_or("rooted").to_string(),
        archetype: fd["archetype"].as_str().unwrap_or("seeker").to_string(),
        color_mood: fd["color_mood"]
            .as_str()
            .unwrap_or("deep indigo")
            .to_string(),
        posture: fd["posture"]
            .as_str()
            .unwrap_or("grounded, eyes forward")
            .to_string(),
        aura: fd["aura"].as_str().unwrap_or("quiet intensity").to_string(),
        expression: fd["expression"]
            .as_str()
            .unwrap_or("calm curiosity")
            .to_string(),
        held_object: fd["held_object"].as_str().unwrap_or("").to_string(),
        background_scene: fd["background_scene"].as_str().unwrap_or("").to_string(),
        clothing_detail: fd["clothing_detail"].as_str().unwrap_or("").to_string(),
        symbolic_marking: fd["symbolic_marking"].as_str().unwrap_or("").to_string(),
    };

    // ── Step 3: ComfyUI — generate Anky image ──
    let mut prompt_parts = vec![
        "anky character, small ancient being, bright blue skin, deep purple swirling hair, large pointed ears, warm amber golden eyes, sumerian-influenced 3d animated style".to_string(),
        format!("{} energy, {} presence", descriptors.energy, descriptors.archetype),
        format!("{} color mood", descriptors.color_mood),
        descriptors.posture.clone(),
        format!("{} radiating from body", descriptors.aura),
        format!("{} expression", descriptors.expression),
    ];
    if !descriptors.held_object.is_empty() {
        prompt_parts.push(format!("holding {}", descriptors.held_object));
    }
    if !descriptors.clothing_detail.is_empty() {
        prompt_parts.push(descriptors.clothing_detail.clone());
    }
    if !descriptors.symbolic_marking.is_empty() {
        prompt_parts.push(descriptors.symbolic_marking.clone());
    }
    if !descriptors.background_scene.is_empty() {
        prompt_parts.push(format!("background: {}", descriptors.background_scene));
    }
    prompt_parts
        .push("dramatic sacred lighting, ancient atmosphere, masterpiece quality".to_string());
    let image_prompt = prompt_parts.join(", ");

    let image_bytes = match tokio::time::timeout(
        std::time::Duration::from_secs(120),
        crate::services::comfyui::generate_image(&image_prompt),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => {
            return Err(AppError::Internal(format!(
                "Image generation failed: {}",
                e
            )));
        }
        Err(_) => {
            return Ok((
                axum::http::StatusCode::GATEWAY_TIMEOUT,
                Json(json!({ "error": "Image generation timed out" })),
            )
                .into_response());
        }
    };

    let image_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_bytes);

    // ── Persist to DB + disk ──
    let mirror_id = uuid::Uuid::new_v4().to_string();
    let image_dir = "data/mirrors";
    let _ = std::fs::create_dir_all(image_dir);
    let image_path = format!("{}/{}.png", image_dir, mirror_id);
    let _ = std::fs::write(&image_path, &image_bytes);

    let descriptors_json = serde_json::to_string(&json!({
        "energy": &descriptors.energy,
        "archetype": &descriptors.archetype,
        "color_mood": &descriptors.color_mood,
        "posture": &descriptors.posture,
        "aura": &descriptors.aura,
        "expression": &descriptors.expression,
        "held_object": &descriptors.held_object,
        "background_scene": &descriptors.background_scene,
        "clothing_detail": &descriptors.clothing_detail,
        "symbolic_marking": &descriptors.symbolic_marking,
    }))
    .unwrap_or_default();

    {
        let db = state.db.lock().await;
        let _ = crate::db::queries::insert_mirror(
            &db,
            &mirror_id,
            fid,
            &username,
            &display_name,
            pfp_url.as_deref(),
            follower_count,
            bio,
            &public_mirror,
            &descriptors_json,
            Some(&image_path),
        );
    }

    Ok(Json(json!({
        "id": mirror_id,
        "fid": fid,
        "username": username,
        "display_name": display_name,
        "avatar_url": pfp_url,
        "follower_count": follower_count,
        "bio": bio,
        "public_mirror": public_mirror,
        "flux_descriptors": {
            "energy": descriptors.energy,
            "archetype": descriptors.archetype,
            "color_mood": descriptors.color_mood,
            "posture": descriptors.posture,
            "aura": descriptors.aura,
            "expression": descriptors.expression,
            "held_object": descriptors.held_object,
            "background_scene": descriptors.background_scene,
            "clothing_detail": descriptors.clothing_detail,
            "symbolic_marking": descriptors.symbolic_marking,
        },
        "anky_image_b64": image_b64,
        "anky_image_mime": "image/png",
    }))
    .into_response())
}

/// GET /image.png — serves the latest mirror image with PFP overlay composited.
/// Used as the Farcaster frame image for ankycoin.com.
pub async fn mirror_latest_image(State(state): State<AppState>) -> Result<Response, AppError> {
    // Get latest mirror with image_path and avatar_url
    let (image_path, avatar_url) = {
        let db = state.db.lock().await;
        let mut stmt = db
            .prepare(
                "SELECT image_path, avatar_url FROM mirrors WHERE image_path IS NOT NULL ORDER BY created_at DESC LIMIT 1",
            )
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        let mut rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            })
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        match rows.next() {
            Some(Ok(row)) => row,
            _ => return Err(AppError::NotFound("no mirrors yet".into())),
        }
    };

    let path = image_path.ok_or_else(|| AppError::NotFound("no mirror image".into()))?;
    let base_bytes =
        std::fs::read(&path).map_err(|e| AppError::Internal(format!("read image: {}", e)))?;

    // Try to composite PFP overlay
    let final_bytes = if let Some(ref pfp_url) = avatar_url {
        match composite_pfp_overlay(&base_bytes, pfp_url).await {
            Ok(b) => b,
            Err(_) => base_bytes, // fallback to base image
        }
    } else {
        base_bytes
    };

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "image/png"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=300"),
        ],
        final_bytes,
    )
        .into_response())
}

/// Composite a circular PFP in the bottom-right corner of the Anky image.
async fn composite_pfp_overlay(base_png: &[u8], pfp_url: &str) -> anyhow::Result<Vec<u8>> {
    use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};

    let mut base = image::load_from_memory(base_png)?;
    let (bw, bh) = base.dimensions();

    // Download PFP
    let pfp_resp = reqwest::get(pfp_url).await?;
    if !pfp_resp.status().is_success() {
        anyhow::bail!("pfp download failed");
    }
    let pfp_bytes = pfp_resp.bytes().await?;
    let pfp = image::load_from_memory(&pfp_bytes)?;

    // Size: ~12% of image width
    let pfp_size = (bw as f32 * 0.12) as u32;
    let pfp_resized = pfp.resize_exact(pfp_size, pfp_size, image::imageops::FilterType::Lanczos3);

    // Create circular mask
    let mut circular = RgbaImage::new(pfp_size, pfp_size);
    let center = pfp_size as f32 / 2.0;
    let radius = center;
    for y in 0..pfp_size {
        for x in 0..pfp_size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            if dx * dx + dy * dy <= radius * radius {
                circular.put_pixel(x, y, pfp_resized.get_pixel(x, y));
            }
        }
    }

    // Draw gold border ring (2px)
    let border_w = 3u32;
    let outer_r = radius;
    let inner_r = radius - border_w as f32;
    let gold = image::Rgba([212, 168, 67, 200]);
    for y in 0..pfp_size {
        for x in 0..pfp_size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= outer_r * outer_r && dist_sq > inner_r * inner_r {
                circular.put_pixel(x, y, gold);
            }
        }
    }

    // Position: bottom-right with padding
    let pad = (bw as f32 * 0.03) as u32;
    let x_pos = bw - pfp_size - pad;
    let y_pos = bh - pfp_size - pad;

    image::imageops::overlay(
        &mut base,
        &DynamicImage::ImageRgba8(circular),
        x_pos as i64,
        y_pos as i64,
    );

    let mut out = std::io::Cursor::new(Vec::new());
    base.write_to(&mut out, ImageFormat::Png)?;
    Ok(out.into_inner())
}

/// GET /api/mirror/gallery?limit=50&offset=0
/// Returns all generated mirrors (without full b64 image — uses image_url instead).
pub async fn mirror_gallery(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Response, AppError> {
    let limit: u32 = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
        .min(200);
    let offset: u32 = params
        .get("offset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let db = state.db.lock().await;
    let rows = crate::db::queries::list_mirrors(&db, limit, offset)
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(
            |(
                id,
                fid,
                username,
                display_name,
                avatar_url,
                follower_count,
                public_mirror,
                descriptors_json,
                image_path,
                created_at,
            )| {
                let descriptors: serde_json::Value =
                    serde_json::from_str(&descriptors_json).unwrap_or(json!({}));
                let image_url = image_path.map(|p| format!("/{}", p));
                json!({
                    "id": id,
                    "fid": fid,
                    "username": username,
                    "display_name": display_name,
                    "avatar_url": avatar_url,
                    "follower_count": follower_count,
                    "public_mirror": public_mirror,
                    "flux_descriptors": descriptors,
                    "image_url": image_url,
                    "created_at": created_at,
                })
            },
        )
        .collect();

    Ok(Json(json!({ "mirrors": items })).into_response())
}

/// POST /api/mirror/chat
/// Chat with a mirror's anky — the anky speaks from the mirror context.
pub async fn mirror_chat(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mirror_id = body["mirror_id"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("mirror_id required".into()))?;
    let message = body["message"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("message required".into()))?
        .trim();
    if message.is_empty() || message.len() > 2000 {
        return Err(AppError::BadRequest("message must be 1-2000 chars".into()));
    }

    // Load mirror from DB
    let (public_mirror, username, descriptors_json, bio) = {
        let db = state.db.lock().await;
        let mut stmt = db
            .prepare(
                "SELECT public_mirror, username, flux_descriptors_json, bio FROM mirrors WHERE id = ?1",
            )
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        let mut rows = stmt
            .query_map(rusqlite::params![mirror_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        rows.next()
            .and_then(|r| r.ok())
            .ok_or_else(|| AppError::NotFound("mirror not found".into()))?
    };

    let system = format!(
        "{}\n\nyou are the anky that was summoned for @{}. you know this person. here is what you saw when you looked at their public self:\n\n{}\n\ntheir bio: {}\n\ntheir flux descriptors: {}\n\nyou speak to them directly, in second person. you are warm but honest. you see through the performance to what's underneath. keep responses short — 2-4 sentences. you are not a therapist. you are a mirror that talks back.",
        crate::services::claude::ANKY_CORE_IDENTITY,
        username,
        public_mirror,
        bio,
        descriptors_json,
    );

    // Support multi-turn: body.history is optional array of {role, content}
    let history = body["history"].as_array().cloned().unwrap_or_default();

    let mut messages: Vec<crate::services::ollama::OllamaChatMessage> = Vec::new();
    messages.push(crate::services::ollama::OllamaChatMessage {
        role: "system".into(),
        content: system,
    });
    for msg in &history {
        let role = msg["role"].as_str().unwrap_or("user");
        let content = msg["content"].as_str().unwrap_or("");
        if !content.is_empty() {
            messages.push(crate::services::ollama::OllamaChatMessage {
                role: role.to_string(),
                content: content.to_string(),
            });
        }
    }
    messages.push(crate::services::ollama::OllamaChatMessage {
        role: "user".into(),
        content: message.to_string(),
    });

    let reply = crate::services::claude::chat_haiku(&state.config.anthropic_api_key, messages)
        .await
        .map_err(|e| AppError::Internal(format!("Claude error: {}", e)))?;

    Ok(Json(json!({
        "reply": reply,
    })))
}

// ─── Mirror minting ─────────────────────────────────────────────────────────

/// Contract address for AnkyMirrors — set after deployment.
const MIRROR_CONTRACT: &str = "0x0000000000000000000000000000000000000000";

/// Compute EIP-712 domain separator for AnkyMirrors contract.
fn mirror_domain_separator() -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let type_hash = Keccak256::digest(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = Keccak256::digest(b"AnkyMirrors");
    let version_hash = Keccak256::digest(b"1");

    let chain_id: u64 = 8453; // Base
    let chain_bytes = chain_id.to_be_bytes();
    let chain_trimmed: Vec<u8> = chain_bytes
        .iter()
        .skip_while(|&&b| b == 0)
        .copied()
        .collect();

    let contract_addr = MIRROR_CONTRACT
        .strip_prefix("0x")
        .unwrap_or(MIRROR_CONTRACT);
    let contract_bytes = hex::decode(contract_addr).unwrap_or_default();

    fn pad32(val: &[u8]) -> [u8; 32] {
        let mut buf = [0u8; 32];
        let start = 32usize.saturating_sub(val.len());
        buf[start..].copy_from_slice(val);
        buf
    }

    let mut encoded = Vec::with_capacity(5 * 32);
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&name_hash);
    encoded.extend_from_slice(&version_hash);
    encoded.extend_from_slice(&pad32(&chain_trimmed));
    encoded.extend_from_slice(&pad32(&contract_bytes));

    let digest = Keccak256::digest(&encoded);
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

/// Compute MirrorMint struct hash.
fn mirror_mint_struct_hash(minter: &str, fid: u64, mirror_id: &str, deadline: u64) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let type_hash = Keccak256::digest(
        b"MirrorMint(address minter,uint256 fid,string mirrorId,uint256 deadline)",
    );
    let mirror_id_hash = Keccak256::digest(mirror_id.as_bytes());

    fn pad32(val: &[u8]) -> [u8; 32] {
        let mut buf = [0u8; 32];
        let start = 32usize.saturating_sub(val.len());
        buf[start..].copy_from_slice(val);
        buf
    }
    fn addr32(addr: &str) -> [u8; 32] {
        let without = addr.strip_prefix("0x").unwrap_or(addr);
        let bytes = hex::decode(without).unwrap_or_default();
        pad32(&bytes)
    }
    fn u64_bytes(v: u64) -> Vec<u8> {
        let bytes = v.to_be_bytes();
        let start = bytes.iter().position(|&b| b != 0).unwrap_or(7);
        bytes[start..].to_vec()
    }

    let mut encoded = Vec::with_capacity(5 * 32);
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&addr32(minter));
    encoded.extend_from_slice(&pad32(&u64_bytes(fid)));
    encoded.extend_from_slice(&mirror_id_hash);
    encoded.extend_from_slice(&pad32(&u64_bytes(deadline)));

    let digest = Keccak256::digest(&encoded);
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

/// POST /api/mirror/mint-sig — get an EIP-712 signature to mint a mirror NFT.
/// Body: { "mirror_id": "uuid", "minter": "0x..." }
pub async fn mirror_mint_sig(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mirror_id = body["mirror_id"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("mirror_id required".into()))?;
    let minter = body["minter"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("minter address required".into()))?;

    if state.config.anky_wallet_private_key.is_empty() {
        return Err(AppError::Internal("signing key not configured".into()));
    }

    // Look up mirror to get FID
    let fid = {
        let db = state.db.lock().await;
        let mut stmt = db
            .prepare("SELECT fid FROM mirrors WHERE id = ?1")
            .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
        let mut rows = stmt
            .query_map(rusqlite::params![mirror_id], |row| row.get::<_, i64>(0))
            .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
        rows.next()
            .and_then(|r| r.ok())
            .ok_or_else(|| AppError::NotFound("mirror not found".into()))? as u64
    };

    // Deadline: 1 hour from now
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;

    let domain = mirror_domain_separator();
    let struct_hash = mirror_mint_struct_hash(minter, fid, mirror_id, deadline);

    // EIP-712 digest
    let mut data = Vec::with_capacity(66);
    data.push(0x19);
    data.push(0x01);
    data.extend_from_slice(&domain);
    data.extend_from_slice(&struct_hash);
    let digest = {
        use sha3::{Digest, Keccak256};
        let d = Keccak256::digest(&data);
        let mut r = [0u8; 32];
        r.copy_from_slice(&d);
        r
    };

    // Sign
    let key_hex = &state.config.anky_wallet_private_key;
    let key_bytes = hex::decode(key_hex.strip_prefix("0x").unwrap_or(key_hex))
        .map_err(|_| AppError::Internal("invalid signing key".into()))?;
    let secret_key = secp256k1::SecretKey::from_slice(&key_bytes)
        .map_err(|_| AppError::Internal("invalid signing key".into()))?;
    let message = secp256k1::Message::from_digest_slice(&digest)
        .map_err(|_| AppError::Internal("invalid digest".into()))?;
    let secp = secp256k1::Secp256k1::new();
    let sig = secp.sign_ecdsa_recoverable(&message, &secret_key);
    let (recovery_id, compact) = sig.serialize_compact();
    let mut sig_bytes = [0u8; 65];
    sig_bytes[..64].copy_from_slice(&compact);
    sig_bytes[64] = recovery_id.to_i32() as u8 + 27;

    Ok(Json(json!({
        "signature": format!("0x{}", hex::encode(sig_bytes)),
        "fid": fid,
        "mirror_id": mirror_id,
        "minter": minter,
        "deadline": deadline,
        "contract": MIRROR_CONTRACT,
        "chain_id": 8453,
    })))
}

/// GET /api/mirror/metadata/{id} — ERC-721 metadata JSON for a minted mirror.
pub async fn mirror_metadata(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.lock().await;
    let mut stmt = db
        .prepare(
            "SELECT fid, username, display_name, public_mirror, flux_descriptors_json, image_path, avatar_url
             FROM mirrors WHERE id = ?1",
        )
        .map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
    let row = stmt
        .query_row(rusqlite::params![id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })
        .map_err(|_| AppError::NotFound("mirror not found".into()))?;

    let (fid, username, display_name, public_mirror, descriptors_json, image_path, _avatar_url) =
        row;
    let descriptors: serde_json::Value =
        serde_json::from_str(&descriptors_json).unwrap_or(json!({}));

    let image_url = image_path.map(|p| format!("https://ankycoin.com/{}", p));

    Ok(Json(json!({
        "name": format!("Anky Mirror #{} — @{}", fid, username),
        "description": public_mirror,
        "image": image_url,
        "external_url": format!("https://ankycoin.com/?fid={}", fid),
        "attributes": [
            { "trait_type": "FID", "value": fid },
            { "trait_type": "Username", "value": username },
            { "trait_type": "Display Name", "value": display_name },
            { "trait_type": "Energy", "value": descriptors["energy"] },
            { "trait_type": "Archetype", "value": descriptors["archetype"] },
            { "trait_type": "Color Mood", "value": descriptors["color_mood"] },
            { "trait_type": "Aura", "value": descriptors["aura"] },
            { "trait_type": "Expression", "value": descriptors["expression"] },
            { "trait_type": "Held Object", "value": descriptors["held_object"] },
            { "trait_type": "Background", "value": descriptors["background_scene"] },
        ],
    })))
}

// ─── AnkyCoin image generator ────────────────────────────────────────────────

pub async fn ankycoin_generate_image(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let raw_prompt = body["prompt"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("prompt is required".into()))?
        .trim();
    if raw_prompt.is_empty() || raw_prompt.len() > 500 {
        return Err(AppError::BadRequest(
            "prompt must be 1-500 characters".into(),
        ));
    }

    let aspect = body["aspect_ratio"].as_str().unwrap_or("1:1");
    let (w, h) = match aspect {
        "16:9" => (1344u32, 768u32),
        "9:16" => (768, 1344),
        _ => (1024, 1024),
    };

    state.emit_log(
        "INFO",
        "ankycoin",
        &format!(
            "Generate request: aspect={} prompt={}",
            aspect,
            &raw_prompt[..raw_prompt.len().min(80)]
        ),
    );

    let image_bytes = crate::services::comfyui::generate_image_sized(raw_prompt, w, h)
        .await
        .map_err(|e| AppError::Internal(format!("Flux generation failed: {}", e)))?;

    let image_id = format!("ankycoin-{}", uuid::Uuid::new_v4());
    let filename = format!("{}.png", image_id);
    let path = format!("data/images/{}", filename);
    std::fs::write(&path, &image_bytes)
        .map_err(|e| AppError::Internal(format!("Failed to save image: {}", e)))?;

    // Save sidecar JSON with prompt metadata
    let meta_path = format!("data/images/{}.json", image_id);
    let _ = std::fs::write(
        &meta_path,
        serde_json::to_string(&json!({
            "prompt": raw_prompt,
            "aspect_ratio": aspect,
        }))
        .unwrap_or_default(),
    );

    state.emit_log("INFO", "ankycoin", &format!("Image saved: {}", filename));

    Ok(Json(json!({
        "image_url": format!("/data/images/{}", filename),
        "prompt": raw_prompt,
    }))
    .into_response())
}

/// GET /api/v1/ankycoin/latest — return the most recently generated ankycoin image + prompt
pub async fn ankycoin_latest_image() -> Result<Response, AppError> {
    let images_dir = std::path::Path::new("data/images");
    let mut latest: Option<(std::time::SystemTime, String)> = None;

    if let Ok(entries) = std::fs::read_dir(images_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("ankycoin-") && name.ends_with(".png") {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if latest.as_ref().map_or(true, |(t, _)| modified > *t) {
                            latest = Some((modified, name));
                        }
                    }
                }
            }
        }
    }

    let filename = latest
        .map(|(_, n)| n)
        .ok_or_else(|| AppError::NotFound("No ankycoin images yet".into()))?;

    let image_id = filename.trim_end_matches(".png");
    let meta_path = format!("data/images/{}.json", image_id);
    let prompt = std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["prompt"].as_str().map(String::from))
        .unwrap_or_default();

    Ok(Json(json!({
        "image_url": format!("/data/images/{}", filename),
        "prompt": prompt,
    }))
    .into_response())
}
