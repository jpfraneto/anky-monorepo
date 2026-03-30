use crate::db::queries;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Max gap between chunks before session dies (same as human frontend).
const CHUNK_TIMEOUT_SECS: u64 = 8;

/// Session must reach this many seconds to qualify as an anky.
const ANKY_THRESHOLD_SECS: f64 = 480.0;

/// Max words per chunk — forces iterative generation, not a dump.
const MAX_WORDS_PER_CHUNK: usize = 50;

/// In-memory active session state (not persisted until complete).
#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub session_id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub user_id: String,
    pub chunks: Vec<String>,
    pub word_count: i32,
    pub started_at: std::time::Instant,
    pub last_chunk_at: std::time::Instant,
    pub dead: bool,
    pub finalized: bool,
}

/// Shared map of active chunked sessions.
pub type SessionMap = Arc<Mutex<HashMap<String, ActiveSession>>>;

pub fn new_session_map() -> SessionMap {
    Arc::new(Mutex::new(HashMap::new()))
}

// ---------- request / response types ----------

#[derive(Deserialize)]
pub struct StartRequest {
    /// Optional prompt or intention for the session (not required).
    pub prompt: Option<String>,
}

#[derive(Serialize)]
pub struct StartResponse {
    pub session_id: String,
    pub timeout_seconds: u64,
    pub max_words_per_chunk: usize,
    pub target_seconds: f64,
    pub message: String,
}

#[derive(Deserialize)]
pub struct ChunkRequest {
    pub session_id: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct ChunkResponse {
    pub ok: bool,
    pub words_total: i32,
    pub elapsed_seconds: f64,
    pub remaining_seconds: f64,
    pub is_anky: bool,
    /// Set when the session completes as an anky.
    pub anky_id: Option<String>,
    /// Set when the session completes as an anky.
    pub estimated_wait_seconds: Option<u32>,
    /// Non-anky feedback (if session ended before 8 min).
    pub response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub session_id: String,
    pub alive: bool,
    pub words_total: i32,
    pub elapsed_seconds: f64,
    pub remaining_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SessionEventResponse {
    pub id: i64,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<i32>,
    pub elapsed_seconds: f64,
    pub words_total: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_word_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Value>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SessionEventsResponse {
    pub session_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub event_count: usize,
    pub events: Vec<SessionEventResponse>,
}

#[derive(Serialize)]
pub struct SessionResultResponse {
    pub session_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub alive: bool,
    pub finalized: bool,
    pub is_anky: bool,
    pub words_total: i32,
    pub elapsed_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_wait_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone)]
struct SessionTraceContext {
    session_id: String,
    user_id: String,
    agent_id: String,
    agent_name: String,
}

impl From<&ActiveSession> for SessionTraceContext {
    fn from(session: &ActiveSession) -> Self {
        Self {
            session_id: session.session_id.clone(),
            user_id: session.user_id.clone(),
            agent_id: session.agent_id.clone(),
            agent_name: session.agent_name.clone(),
        }
    }
}

// ---------- helpers ----------

fn authenticate_agent(
    db: &crate::db::Connection,
    headers: &HeaderMap,
) -> Result<queries::AgentRecord, AppError> {
    let key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("X-API-Key header required".into()))?;

    queries::get_agent_by_key(db, key)?
        .ok_or_else(|| AppError::Unauthorized("invalid API key".into()))
}

#[allow(clippy::too_many_arguments)]
async fn record_session_event(
    state: &AppState,
    context: &SessionTraceContext,
    event_type: &str,
    chunk_index: Option<i32>,
    elapsed_seconds: f64,
    words_total: i32,
    chunk_text: Option<&str>,
    chunk_word_count: Option<i32>,
    detail: Option<Value>,
) {
    let detail_json = detail.as_ref().map(Value::to_string);
    let Some(db) = crate::db::get_conn_logged(&state.db) else {
        return;
    };
    if let Err(err) = queries::insert_agent_session_event(
        &db,
        &context.session_id,
        &context.user_id,
        &context.agent_id,
        &context.agent_name,
        event_type,
        chunk_index,
        elapsed_seconds,
        words_total,
        chunk_text,
        chunk_word_count,
        detail_json.as_deref(),
    ) {
        tracing::error!(
            session = %context.session_id,
            event_type,
            error = %err,
            "failed to persist session event"
        );
    }
}

fn parse_event_detail(detail_json: Option<String>) -> Option<Value> {
    detail_json.map(|raw| serde_json::from_str(&raw).unwrap_or_else(|_| json!({ "raw": raw })))
}

/// Background task: watches for timed-out sessions and kills them.
/// If a session crossed the 8-minute threshold, it gets finalized as an anky.
/// If not, it gets finalized as a non-anky (with Ollama feedback).
pub fn spawn_session_reaper(sessions: SessionMap, state: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let now = std::time::Instant::now();

            struct TimedOutSession {
                session_id: String,
                crossed_threshold: bool,
                elapsed_seconds: f64,
                words_total: i32,
                gap_seconds: f64,
                context: SessionTraceContext,
            }

            // Collect sessions that need to be killed
            let to_kill: Vec<TimedOutSession> = {
                let mut map = sessions.lock().await;
                let mut dead_sessions = Vec::new();

                for (id, s) in map.iter_mut() {
                    let gap_seconds = now.duration_since(s.last_chunk_at).as_secs_f64();
                    if !s.dead && gap_seconds > CHUNK_TIMEOUT_SECS as f64 {
                        s.dead = true;
                        let elapsed_seconds = s.started_at.elapsed().as_secs_f64();
                        let crossed_threshold = elapsed_seconds >= ANKY_THRESHOLD_SECS;
                        tracing::info!(
                            session = %id,
                            agent = %s.agent_name,
                            words = s.word_count,
                            elapsed = elapsed_seconds,
                            gap_seconds,
                            "Session timed out (8s silence)"
                        );
                        dead_sessions.push(TimedOutSession {
                            session_id: id.clone(),
                            crossed_threshold,
                            elapsed_seconds,
                            words_total: s.word_count,
                            gap_seconds,
                            context: SessionTraceContext::from(&*s),
                        });
                    }
                }

                // Purge sessions dead for more than 5 minutes (cleanup)
                map.retain(|_, s| !s.dead || now.duration_since(s.last_chunk_at).as_secs() < 300);

                dead_sessions
            };

            // Finalize outside the lock
            for timed_out in to_kill {
                record_session_event(
                    &state,
                    &timed_out.context,
                    "session_timed_out",
                    None,
                    timed_out.elapsed_seconds,
                    timed_out.words_total,
                    None,
                    None,
                    Some(json!({
                        "reason": "8s_silence",
                        "gap_seconds": timed_out.gap_seconds,
                        "timeout_seconds": CHUNK_TIMEOUT_SECS,
                        "crossed_threshold": timed_out.crossed_threshold,
                    })),
                )
                .await;

                if timed_out.crossed_threshold {
                    let _ = finalize_anky(&state, &timed_out.session_id).await;
                } else {
                    finalize_non_anky(&state, &timed_out.session_id).await;
                }
            }
        }
    });
}

// ---------- handlers ----------

/// POST /api/v1/session/start — open a new chunked writing session.
pub async fn start_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<StartRequest>,
) -> Result<Json<StartResponse>, AppError> {
    let agent = {
        let db = crate::db::conn(&state.db)?;
        authenticate_agent(&db, &headers)?
    };

    let session_id = uuid::Uuid::new_v4().to_string();
    let user_id = format!("agent:{}", agent.id);
    let now = std::time::Instant::now();

    let session = ActiveSession {
        session_id: session_id.clone(),
        agent_id: agent.id.clone(),
        agent_name: agent.name.clone(),
        user_id: user_id.clone(),
        chunks: Vec::new(),
        word_count: 0,
        started_at: now,
        last_chunk_at: now,
        dead: false,
        finalized: false,
    };
    let context = SessionTraceContext::from(&session);

    {
        let mut map = state.sessions.lock().await;
        map.insert(session_id.clone(), session);
    }

    record_session_event(
        &state,
        &context,
        "session_started",
        None,
        0.0,
        0,
        None,
        None,
        Some(json!({
            "prompt": req.prompt,
            "timeout_seconds": CHUNK_TIMEOUT_SECS,
            "target_seconds": ANKY_THRESHOLD_SECS,
            "max_words_per_chunk": MAX_WORDS_PER_CHUNK,
        })),
    )
    .await;

    state.emit_log(
        "INFO",
        "session",
        &format!(
            "Agent @{} started chunked session {}",
            agent.name,
            &session_id[..8]
        ),
    );

    Ok(Json(StartResponse {
        session_id,
        timeout_seconds: CHUNK_TIMEOUT_SECS,
        max_words_per_chunk: MAX_WORDS_PER_CHUNK,
        target_seconds: ANKY_THRESHOLD_SECS,
        message: format!(
            "session open. send chunks to /api/v1/session/chunk within {}s of each other. \
             keep writing for {} minutes to birth an anky. max {} words per chunk — \
             let the words come, don't pre-compose.",
            CHUNK_TIMEOUT_SECS,
            (ANKY_THRESHOLD_SECS / 60.0) as u32,
            MAX_WORDS_PER_CHUNK,
        ),
    }))
}

/// POST /api/v1/session/chunk — append text to an active session.
pub async fn send_chunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChunkRequest>,
) -> Result<Json<ChunkResponse>, AppError> {
    // Authenticate (must be the same agent that started the session)
    let agent = {
        let db = crate::db::conn(&state.db)?;
        authenticate_agent(&db, &headers)?
    };

    let chunk_words = req.text.split_whitespace().count();
    enum ChunkDecision {
        Missing,
        WrongAgent,
        Rejected {
            context: SessionTraceContext,
            elapsed_seconds: f64,
            remaining_seconds: f64,
            words_total: i32,
            next_chunk_index: i32,
            error: String,
            detail: Value,
        },
        Dead {
            context: SessionTraceContext,
            elapsed_seconds: f64,
            words_total: i32,
            gap_seconds: f64,
            crossed_threshold: bool,
            timeout_just_happened: bool,
        },
        Accepted {
            context: SessionTraceContext,
            elapsed_seconds: f64,
            remaining_seconds: f64,
            words_total: i32,
            gap_seconds: f64,
            chunk_index: i32,
            crossed_threshold: bool,
        },
    }

    let decision = {
        let mut map = state.sessions.lock().await;
        match map.get_mut(&req.session_id) {
            None => ChunkDecision::Missing,
            Some(session) => {
                if session.agent_id != agent.id {
                    ChunkDecision::WrongAgent
                } else {
                    let context = SessionTraceContext::from(&*session);
                    let now = std::time::Instant::now();
                    let elapsed_seconds = session.started_at.elapsed().as_secs_f64();
                    let remaining_seconds = (ANKY_THRESHOLD_SECS - elapsed_seconds).max(0.0);
                    let gap_seconds = now.duration_since(session.last_chunk_at).as_secs_f64();
                    let next_chunk_index = session.chunks.len() as i32 + 1;

                    if session.dead {
                        ChunkDecision::Dead {
                            context,
                            elapsed_seconds,
                            words_total: session.word_count,
                            gap_seconds,
                            crossed_threshold: elapsed_seconds >= ANKY_THRESHOLD_SECS,
                            timeout_just_happened: false,
                        }
                    } else if gap_seconds > CHUNK_TIMEOUT_SECS as f64 {
                        session.dead = true;
                        ChunkDecision::Dead {
                            context,
                            elapsed_seconds,
                            words_total: session.word_count,
                            gap_seconds,
                            crossed_threshold: elapsed_seconds >= ANKY_THRESHOLD_SECS,
                            timeout_just_happened: true,
                        }
                    } else if chunk_words == 0 {
                        ChunkDecision::Rejected {
                            context,
                            elapsed_seconds,
                            remaining_seconds,
                            words_total: session.word_count,
                            next_chunk_index,
                            error: "empty chunk".into(),
                            detail: json!({
                                "reason": "empty_chunk",
                                "gap_seconds": gap_seconds,
                            }),
                        }
                    } else if chunk_words > MAX_WORDS_PER_CHUNK {
                        ChunkDecision::Rejected {
                            context,
                            elapsed_seconds,
                            remaining_seconds,
                            words_total: session.word_count,
                            next_chunk_index,
                            error: format!(
                                "chunk too large: {} words (max {}). let the words come in smaller bursts.",
                                chunk_words, MAX_WORDS_PER_CHUNK
                            ),
                            detail: json!({
                                "reason": "chunk_too_large",
                                "attempted_words": chunk_words,
                                "max_words_per_chunk": MAX_WORDS_PER_CHUNK,
                                "gap_seconds": gap_seconds,
                            }),
                        }
                    } else {
                        session.chunks.push(req.text.clone());
                        session.word_count += chunk_words as i32;
                        session.last_chunk_at = now;
                        let elapsed_seconds = session.started_at.elapsed().as_secs_f64();
                        let remaining_seconds = (ANKY_THRESHOLD_SECS - elapsed_seconds).max(0.0);
                        ChunkDecision::Accepted {
                            context,
                            elapsed_seconds,
                            remaining_seconds,
                            words_total: session.word_count,
                            gap_seconds,
                            chunk_index: session.chunks.len() as i32,
                            crossed_threshold: elapsed_seconds >= ANKY_THRESHOLD_SECS,
                        }
                    }
                }
            }
        }
    };

    match decision {
        ChunkDecision::Missing => Ok(Json(ChunkResponse {
            ok: false,
            words_total: 0,
            elapsed_seconds: 0.0,
            remaining_seconds: 0.0,
            is_anky: false,
            anky_id: None,
            estimated_wait_seconds: None,
            response: None,
            error: Some(
                "session not found. start a new one with POST /api/v1/session/start".into(),
            ),
        })),
        ChunkDecision::WrongAgent => Err(AppError::Unauthorized(
            "this session belongs to another agent".into(),
        )),
        ChunkDecision::Rejected {
            context,
            elapsed_seconds,
            remaining_seconds,
            words_total,
            next_chunk_index,
            error,
            detail,
        } => {
            record_session_event(
                &state,
                &context,
                "chunk_rejected",
                Some(next_chunk_index),
                elapsed_seconds,
                words_total,
                Some(req.text.as_str()),
                Some(chunk_words as i32),
                Some(detail),
            )
            .await;

            Ok(Json(ChunkResponse {
                ok: false,
                words_total,
                elapsed_seconds,
                remaining_seconds,
                is_anky: false,
                anky_id: None,
                estimated_wait_seconds: None,
                response: None,
                error: Some(error),
            }))
        }
        ChunkDecision::Dead {
            context,
            elapsed_seconds,
            words_total,
            gap_seconds,
            crossed_threshold,
            timeout_just_happened,
        } => {
            if timeout_just_happened {
                record_session_event(
                    &state,
                    &context,
                    "session_timed_out",
                    None,
                    elapsed_seconds,
                    words_total,
                    None,
                    None,
                    Some(json!({
                        "reason": "8s_silence",
                        "gap_seconds": gap_seconds,
                        "timeout_seconds": CHUNK_TIMEOUT_SECS,
                        "crossed_threshold": crossed_threshold,
                    })),
                )
                .await;
            }

            if crossed_threshold {
                // Session died after 8+ minutes — this IS an anky. The silence is the natural end.
                let (anky_id, response) = finalize_anky(&state, &req.session_id).await?;
                Ok(Json(ChunkResponse {
                    ok: true,
                    words_total,
                    elapsed_seconds,
                    remaining_seconds: 0.0,
                    is_anky: true,
                    anky_id: Some(anky_id),
                    estimated_wait_seconds: Some(45),
                    response: Some(response),
                    error: None,
                }))
            } else {
                // Session died before 8 minutes — not an anky
                let response = finalize_non_anky(&state, &req.session_id).await;
                Ok(Json(ChunkResponse {
                    ok: false,
                    words_total,
                    elapsed_seconds,
                    remaining_seconds: 0.0,
                    is_anky: false,
                    anky_id: None,
                    estimated_wait_seconds: None,
                    response,
                    error: Some("session died — you stopped writing for more than 8 seconds. that's the practice: you can't stop.".into()),
                }))
            }
        }
        ChunkDecision::Accepted {
            context,
            elapsed_seconds,
            remaining_seconds,
            words_total,
            gap_seconds,
            chunk_index,
            crossed_threshold,
        } => {
            record_session_event(
                &state,
                &context,
                "chunk_accepted",
                Some(chunk_index),
                elapsed_seconds,
                words_total,
                Some(req.text.as_str()),
                Some(chunk_words as i32),
                Some(json!({
                    "gap_seconds": gap_seconds,
                    "crossed_threshold": crossed_threshold,
                })),
            )
            .await;

            Ok(Json(ChunkResponse {
                ok: true,
                words_total,
                elapsed_seconds,
                remaining_seconds,
                is_anky: crossed_threshold,
                anky_id: None,
                estimated_wait_seconds: None,
                response: None,
                error: None,
            }))
        }
    }
}

/// GET /api/v1/session/{id} — check session status.
pub async fn session_status(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let map = state.sessions.lock().await;
    match map.get(&id) {
        None => Json(json!({"error": "session not found"})),
        Some(s) => {
            let elapsed = s.started_at.elapsed().as_secs_f64();
            let remaining = (ANKY_THRESHOLD_SECS - elapsed).max(0.0);
            Json(json!({
                "session_id": s.session_id,
                "alive": !s.dead,
                "words_total": s.word_count,
                "elapsed_seconds": (elapsed * 10.0).round() / 10.0,
                "remaining_seconds": (remaining * 10.0).round() / 10.0,
                "agent": s.agent_name,
            }))
        }
    }
}

/// GET /api/v1/session/{id}/events — replay the server-observed session timeline.
/// Requires the same X-API-Key used to create the session.
pub async fn session_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<SessionEventsResponse>, AppError> {
    let agent = {
        let db = crate::db::conn(&state.db)?;
        authenticate_agent(&db, &headers)?
    };

    let (owner, events) = {
        let db = crate::db::conn(&state.db)?;
        let owner = queries::get_agent_session_owner(&db, &id)?
            .ok_or_else(|| AppError::NotFound(format!("session {} not found", id)))?;
        if owner.agent_id != agent.id {
            return Err(AppError::Unauthorized(
                "this session belongs to another agent".into(),
            ));
        }
        let events = queries::list_agent_session_events(&db, &id)?;
        (owner, events)
    };

    Ok(Json(SessionEventsResponse {
        session_id: id,
        user_id: owner.user_id,
        agent_id: owner.agent_id,
        agent_name: owner.agent_name,
        event_count: events.len(),
        events: events
            .into_iter()
            .map(|event| SessionEventResponse {
                id: event.id,
                event_type: event.event_type,
                chunk_index: event.chunk_index,
                elapsed_seconds: event.elapsed_seconds,
                words_total: event.words_total,
                chunk_text: event.chunk_text,
                chunk_word_count: event.chunk_word_count,
                detail: parse_event_detail(event.detail_json),
                created_at: event.created_at,
            })
            .collect(),
    }))
}

/// GET /api/v1/session/{id}/result — recover the final outcome for a session.
/// Requires the same X-API-Key used to create the session.
pub async fn session_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<SessionResultResponse>, AppError> {
    let agent = {
        let db = crate::db::conn(&state.db)?;
        authenticate_agent(&db, &headers)?
    };

    let (owner, events) = {
        let db = crate::db::conn(&state.db)?;
        let owner = queries::get_agent_session_owner(&db, &id)?
            .ok_or_else(|| AppError::NotFound(format!("session {} not found", id)))?;
        if owner.agent_id != agent.id {
            return Err(AppError::Unauthorized(
                "this session belongs to another agent".into(),
            ));
        }
        let events = queries::list_agent_session_events(&db, &id)?;
        (owner, events)
    };

    let active_snapshot = {
        let map = state.sessions.lock().await;
        map.get(&id).map(|session| {
            (
                !session.dead,
                session.word_count,
                session.started_at.elapsed().as_secs_f64(),
            )
        })
    };

    let mut alive = active_snapshot.map(|snapshot| snapshot.0).unwrap_or(false);
    let mut words_total = active_snapshot.map(|snapshot| snapshot.1).unwrap_or(0);
    let mut elapsed_seconds = active_snapshot.map(|snapshot| snapshot.2).unwrap_or(0.0);
    let mut finalized = false;
    let mut is_anky = false;
    let mut anky_id: Option<String> = None;
    let mut anky_status: Option<String> = None;
    let mut estimated_wait_seconds: Option<u32> = None;
    let mut last_event_type: Option<String> = None;
    let mut last_event_at: Option<String> = None;
    let mut completed_at: Option<String> = None;

    for event in &events {
        words_total = event.words_total;
        elapsed_seconds = event.elapsed_seconds;
        last_event_type = Some(event.event_type.clone());
        last_event_at = Some(event.created_at.clone());

        let detail = parse_event_detail(event.detail_json.clone());
        if let Some(detail) = detail.as_ref() {
            if !is_anky {
                is_anky = detail
                    .get("crossed_threshold")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            }
            if anky_id.is_none() {
                anky_id = detail
                    .get("anky_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
            if estimated_wait_seconds.is_none() {
                estimated_wait_seconds = detail
                    .get("estimated_wait_seconds")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32);
            }
        }

        match event.event_type.as_str() {
            "session_completed_anky" => {
                finalized = true;
                is_anky = true;
                alive = false;
                completed_at = Some(event.created_at.clone());
            }
            "session_completed_non_anky" => {
                finalized = true;
                is_anky = false;
                alive = false;
                completed_at = Some(event.created_at.clone());
            }
            _ => {}
        }
    }

    if let Some(ref id) = anky_id {
        let db = crate::db::conn(&state.db)?;
        anky_status = queries::get_anky_by_id(&db, id)?.map(|anky| anky.status);
    }

    Ok(Json(SessionResultResponse {
        session_id: id,
        user_id: owner.user_id,
        agent_id: owner.agent_id,
        agent_name: owner.agent_name,
        alive,
        finalized,
        is_anky,
        words_total,
        elapsed_seconds,
        anky_id,
        anky_status,
        estimated_wait_seconds,
        last_event_type,
        last_event_at,
        completed_at,
    }))
}

// ---------- finalization ----------

/// Finalize a dead session as non-anky: save to DB, get Ollama feedback.
async fn finalize_non_anky(state: &AppState, session_id: &str) -> Option<String> {
    let session = {
        let mut map = state.sessions.lock().await;
        let s = map.get(session_id)?;
        if s.finalized {
            return None;
        }
        let cloned = s.clone();
        map.get_mut(session_id).unwrap().finalized = true;
        cloned
    };
    let full_text = session.chunks.join(" ");
    let elapsed = session.started_at.elapsed().as_secs_f64();
    let context = SessionTraceContext::from(&session);

    // Save to DB
    {
        let db = crate::db::conn(&state.db).ok()?;
        let _ = queries::ensure_user(&db, &session.user_id);
        let _ = queries::upsert_completed_writing_session_with_flow(
            &db,
            &session.session_id,
            &session.user_id,
            &full_text,
            elapsed,
            session.word_count,
            false,
            None,
            None,
            None,
            None,
        );
    }

    record_session_event(
        state,
        &context,
        "session_completed_non_anky",
        None,
        elapsed,
        session.word_count,
        None,
        None,
        Some(json!({
            "crossed_threshold": false,
            "feedback_requested": true,
        })),
    )
    .await;

    // Get quick feedback from Haiku
    let prompt = crate::services::ollama::quick_feedback_prompt(&full_text, elapsed);
    let feedback = crate::services::claude::call_haiku(&state.config.anthropic_api_key, &prompt)
        .await
        .ok();

    // Save feedback
    if let Some(ref fb) = feedback {
        let db = crate::db::conn(&state.db).ok()?;
        let _ = db.execute(
            "UPDATE writing_sessions SET response = ?1 WHERE id = ?2",
            crate::params![fb, session_id],
        );
    }

    state.emit_log(
        "INFO",
        "session",
        &format!(
            "Agent @{} session {} died at {:.0}s, {} words (non-anky)",
            session.agent_name,
            &session_id[..8],
            elapsed,
            session.word_count,
        ),
    );

    feedback
}

/// Finalize a completed session as an anky: save to DB, kick off image gen + reflection.
async fn finalize_anky(state: &AppState, session_id: &str) -> Result<(String, String), AppError> {
    let session = {
        let mut map = state.sessions.lock().await;
        let s = map
            .get(session_id)
            .ok_or_else(|| AppError::NotFound("session not found".into()))?;
        if s.finalized {
            return Err(AppError::BadRequest("session already finalized".into()));
        }
        let cloned = s.clone();
        let entry = map.get_mut(session_id).unwrap();
        entry.dead = true;
        entry.finalized = true;
        cloned
    };

    let full_text = session.chunks.join(" ");
    let elapsed = session.started_at.elapsed().as_secs_f64();
    let context = SessionTraceContext::from(&session);

    // Save to DB
    {
        let db = crate::db::conn(&state.db)?;
        let _ = queries::ensure_user(&db, &session.user_id);
        queries::upsert_completed_writing_session_with_flow(
            &db,
            &session.session_id,
            &session.user_id,
            &full_text,
            elapsed,
            session.word_count,
            true,
            None,
            None,
            None,
            None,
        )?;
    }

    // Create anky record
    let anky_id = uuid::Uuid::new_v4().to_string();
    {
        let db = crate::db::conn(&state.db)?;
        queries::insert_anky(
            &db,
            &anky_id,
            &session.session_id,
            &session.user_id,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            "generating",
            "written",
            None,
        )?;
    }

    // Background: deep Haiku reflection
    let state_bg = state.clone();
    let sid = session.session_id.clone();
    let text_bg = full_text.clone();
    let api_key_bg = state.config.anthropic_api_key.clone();
    tokio::spawn(async move {
        let prompt = crate::services::ollama::deep_reflection_prompt(&text_bg);
        match crate::services::claude::call_haiku(&api_key_bg, &prompt).await {
            Ok(r) => {
                let Some(db) = crate::db::get_conn_logged(&state_bg.db) else {
                    return;
                };
                let _ = db.execute(
                    "UPDATE writing_sessions SET response = ?1 WHERE id = ?2",
                    crate::params![&r, &sid],
                );
            }
            Err(e) => tracing::error!("Haiku bg reflection error: {}", e),
        }
    });

    // Submit image generation to GPU priority queue
    {
        let is_pro = {
            let db = crate::db::conn(&state.db)?;
            queries::is_user_pro(&db, &session.user_id).unwrap_or(false)
        };
        crate::services::redis_queue::enqueue_job(
            &state.config.redis_url,
            &crate::state::GpuJob::AnkyImage {
                anky_id: anky_id.clone(),
                session_id: session.session_id.clone(),
                user_id: session.user_id.clone(),
                writing: full_text.clone(),
            },
            is_pro,
        )
        .await?;
    }

    record_session_event(
        state,
        &context,
        "session_completed_anky",
        None,
        elapsed,
        session.word_count,
        None,
        None,
        Some(json!({
            "anky_id": anky_id.clone(),
            "estimated_wait_seconds": 45,
            "crossed_threshold": true,
        })),
    )
    .await;

    state.emit_log(
        "INFO",
        "session",
        &format!(
            "Agent @{} completed anky {} via chunked session ({} words, {:.0}s)",
            session.agent_name,
            &anky_id[..8],
            session.word_count,
            elapsed,
        ),
    );

    Ok((
        anky_id,
        "your anky is being born. the reflection is streaming...".into(),
    ))
}
