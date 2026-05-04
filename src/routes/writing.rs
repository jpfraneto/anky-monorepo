use crate::error::AppError;
use crate::models::{WriteRequest, WriteResponse};
use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Html;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row as SqlxRow;
use std::time::{Duration, Instant};

const ANKYVERSE_START_MS: i64 = 1_691_658_000_000;
const DAY_MS: i64 = 86_400_000;
const ANKYVERSE_KINGDOMS: [&str; 8] = [
    "primordia",
    "emblazion",
    "chryseos",
    "eleasis",
    "voxlumis",
    "insightia",
    "claridium",
    "poiesis",
];

fn get_or_create_user_id(
    state: &AppState,
    jar: &CookieJar,
    token_header: Option<&str>,
) -> (String, Option<Cookie<'static>>) {
    // Prefer the authenticated session user over the visitor cookie.
    // This prevents writing sessions from being mislabeled to a stale visitor id
    // when the user is actually signed in.
    if let Some(user_id) = crate::routes::auth::authenticated_user_id_from_jar(state, jar) {
        return (user_id, None);
    }
    if let Some(user_id) = crate::routes::auth::visitor_id_from_jar(jar) {
        return (user_id, None);
    }
    // Farcaster webview fallback: cookie absent but localStorage token sent as header
    let id = if let Some(t) = token_header {
        let t = t.trim();
        // Accept only UUID-shaped tokens to prevent junk keys
        if t.len() == 36 && t.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            t.to_string()
        } else {
            uuid::Uuid::new_v4().to_string()
        }
    } else {
        uuid::Uuid::new_v4().to_string()
    };
    let cookie = crate::routes::auth::build_visitor_cookie(&id);
    (id, Some(cookie))
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AnkyProtocolSubmitRequest {
    pub session_hash: String,
    pub session: String,
    pub duration_seconds: i64,
    pub word_count: i32,
    pub kingdom: String,
    pub started_at: String,
    #[serde(default)]
    pub wallet_signature: Option<String>,
}

struct ParsedAnkyProtocolSession {
    text: String,
    keystroke_deltas: Vec<f64>,
    active_duration_ms: i64,
    word_count: i32,
}

fn kingdom_for_epoch_ms(epoch_ms: i64) -> &'static str {
    let day_index = (epoch_ms - ANKYVERSE_START_MS).div_euclid(DAY_MS);
    let idx = day_index.rem_euclid(ANKYVERSE_KINGDOMS.len() as i64) as usize;
    ANKYVERSE_KINGDOMS[idx]
}

fn parse_anky_protocol_session(session: &str) -> Result<ParsedAnkyProtocolSession, AppError> {
    if session.is_empty() {
        return Err(AppError::BadRequest("session is empty".into()));
    }
    if session.contains('\r') {
        return Err(AppError::BadRequest(
            "session must use \\n line endings only".into(),
        ));
    }

    let mut text = String::new();
    let mut keystroke_deltas = Vec::new();
    let mut active_duration_ms = 0_i64;

    for (idx, line) in session.split('\n').enumerate() {
        if line.is_empty() {
            return Err(AppError::BadRequest(format!(
                "session line {} is empty",
                idx + 1
            )));
        }

        let (ms_text, payload) = line.split_once(' ').ok_or_else(|| {
            AppError::BadRequest(format!(
                "session line {} must be formatted as '<ms> <char>'",
                idx + 1
            ))
        })?;

        let ms: i64 = ms_text.parse().map_err(|_| {
            AppError::BadRequest(format!("session line {} has an invalid ms value", idx + 1))
        })?;

        if ms < 0 {
            return Err(AppError::BadRequest(format!(
                "session line {} has a negative delta",
                idx + 1
            )));
        }
        if idx == 0 && ms != 0 {
            return Err(AppError::BadRequest(
                "the first session delta must be 0".into(),
            ));
        }

        active_duration_ms += ms;
        keystroke_deltas.push(ms as f64);

        if payload.is_empty() {
            text.push(' ');
        } else {
            text.push_str(payload);
        }
    }

    Ok(ParsedAnkyProtocolSession {
        word_count: text.split_whitespace().count() as i32,
        text,
        keystroke_deltas,
        active_duration_ms,
    })
}

#[derive(Debug, Clone)]
struct ProtocolAnkySnapshot {
    anky_id: String,
    user_id: String,
    writing_session_id: String,
    duration_seconds: i64,
    word_count: i32,
    title: Option<String>,
    reflection: Option<String>,
    image_url: Option<String>,
    solana_signature: Option<String>,
    reflection_status: String,
    image_status: String,
    solana_status: String,
    processing_job_state: String,
    last_error_stage: Option<String>,
    last_error_message: Option<String>,
    done_at: Option<String>,
    session_hash: String,
    session_payload: String,
}

#[derive(Default)]
struct ProtocolReplayState {
    title_sent: bool,
    reflection_complete_sent: bool,
    image_sent: bool,
    solana_sent: bool,
    done_sent: bool,
    error_sent: bool,
}

#[derive(Debug, Clone)]
struct ProtocolStreamMessage {
    event: &'static str,
    data: serde_json::Value,
}

fn protocol_event(event: &'static str, data: serde_json::Value) -> ProtocolStreamMessage {
    ProtocolStreamMessage { event, data }
}

fn derived_protocol_kingdom(started_at: &chrono::DateTime<chrono::Utc>) -> String {
    kingdom_for_epoch_ms(started_at.timestamp_millis()).to_string()
}

async fn protocol_submit_user_id(
    state: &AppState,
    headers: &HeaderMap,
    jar: &CookieJar,
) -> Result<String, AppError> {
    if headers.get("authorization").is_some() {
        return crate::routes::swift::bearer_auth(state, headers).await;
    }

    crate::routes::auth::authenticated_user_id_from_jar(state, jar).ok_or_else(|| {
        AppError::Unauthorized("an authenticated session is required for /api/anky/submit".into())
    })
}

fn validate_protocol_idempotency_headers(
    headers: &HeaderMap,
    session_hash: &str,
) -> Result<(), AppError> {
    for name in ["Idempotency-Key", "X-Anky-Session-Hash"] {
        if let Some(value) = headers.get(name).and_then(|v| v.to_str().ok()) {
            if value.trim() != session_hash {
                return Err(AppError::BadRequest(format!(
                    "{} must match session_hash",
                    name
                )));
            }
        }
    }
    Ok(())
}

async fn protocol_wallet_address(
    state: &AppState,
    user_id: &str,
) -> Result<Option<String>, AppError> {
    let db = crate::db::conn(&state.db)?;
    crate::db::queries::get_user_wallet(&db, user_id).map_err(AppError::from)
}

async fn load_protocol_snapshot(
    state: &AppState,
    user_id: &str,
    session_hash: &str,
) -> Result<Option<ProtocolAnkySnapshot>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
            a.id,
            a.user_id,
            a.writing_session_id,
            COALESCE(ws.duration_seconds::bigint, 0),
            COALESCE(ws.word_count, 0),
            a.title,
            a.reflection,
            a.image_path,
            a.solana_mint_tx,
            COALESCE(a.reflection_status, 'pending'),
            COALESCE(a.image_status, 'pending'),
            COALESCE(a.solana_status, 'pending'),
            COALESCE(a.processing_job_state, 'idle'),
            a.last_error_stage,
            a.last_error_message,
            a.done_at,
            a.session_hash,
            COALESCE(a.session_payload, '')
        FROM ankys a
        LEFT JOIN writing_sessions ws ON ws.id = a.writing_session_id
        WHERE a.user_id = $1
          AND a.session_hash = $2
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(session_hash)
    .fetch_optional(&state.db)
    .await?;

    Ok(row.map(|row| ProtocolAnkySnapshot {
        anky_id: row.get::<String, _>(0),
        user_id: row.get::<String, _>(1),
        writing_session_id: row.get::<String, _>(2),
        duration_seconds: row.get::<i64, _>(3),
        word_count: row.get::<i32, _>(4),
        title: row.get::<Option<String>, _>(5),
        reflection: row.get::<Option<String>, _>(6),
        image_url: row.get::<Option<String>, _>(7),
        solana_signature: row.get::<Option<String>, _>(8),
        reflection_status: row.get::<String, _>(9),
        image_status: row.get::<String, _>(10),
        solana_status: row.get::<String, _>(11),
        processing_job_state: row.get::<String, _>(12),
        last_error_stage: row.get::<Option<String>, _>(13),
        last_error_message: row.get::<Option<String>, _>(14),
        done_at: row.get::<Option<String>, _>(15),
        session_hash: row.get::<String, _>(16),
        session_payload: row.get::<String, _>(17),
    }))
}

async fn upsert_protocol_submission(
    state: &AppState,
    user_id: &str,
    req: &AnkyProtocolSubmitRequest,
    parsed: &ParsedAnkyProtocolSession,
) -> Result<ProtocolAnkySnapshot, AppError> {
    let mut tx = state.db.begin().await?;

    sqlx::query("INSERT INTO users (id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    let writing_session_id = sqlx::query(
        r#"
        INSERT INTO writing_sessions (
            id,
            user_id,
            content,
            duration_seconds,
            word_count,
            is_anky,
            response,
            keystroke_deltas,
            flow_score,
            status,
            pause_used,
            session_token,
            session_hash
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            1,
            NULL,
            $6,
            NULL,
            'completed',
            0,
            NULL,
            $7
        )
        ON CONFLICT (user_id, session_hash) WHERE session_hash IS NOT NULL DO UPDATE SET
            content = EXCLUDED.content,
            duration_seconds = EXCLUDED.duration_seconds,
            word_count = EXCLUDED.word_count,
            is_anky = 1,
            keystroke_deltas = COALESCE(EXCLUDED.keystroke_deltas, writing_sessions.keystroke_deltas),
            status = 'completed'
        RETURNING id
        "#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(&parsed.text)
    .bind(req.duration_seconds as f64)
    .bind(req.word_count)
    .bind(
        serde_json::to_string(&parsed.keystroke_deltas)
            .map_err(|e| AppError::Internal(format!("keystroke serialize error: {}", e)))?,
    )
    .bind(&req.session_hash)
    .fetch_one(&mut *tx)
    .await?
    .get::<String, _>(0);

    let accepted_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let snapshot = sqlx::query(
        r#"
        INSERT INTO ankys (
            id,
            writing_session_id,
            user_id,
            image_prompt,
            reflection,
            title,
            image_path,
            caption,
            thinker_name,
            thinker_moment,
            status,
            origin,
            prompt_id,
            session_hash,
            session_payload,
            reflection_status,
            image_status,
            solana_status,
            processing_job_state,
            accepted_at
        ) VALUES (
            $1,
            $2,
            $3,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            'accepted',
            'protocol',
            NULL,
            $4,
            $5,
            'pending',
            'pending',
            'pending',
            'idle',
            $6
        )
        ON CONFLICT (user_id, session_hash) WHERE session_hash IS NOT NULL DO UPDATE SET
            writing_session_id = COALESCE(ankys.writing_session_id, EXCLUDED.writing_session_id),
            session_payload = EXCLUDED.session_payload,
            accepted_at = COALESCE(ankys.accepted_at, EXCLUDED.accepted_at)
        RETURNING
            id,
            writing_session_id,
            title,
            reflection,
            image_path,
            solana_mint_tx,
            COALESCE(reflection_status, 'pending'),
            COALESCE(image_status, 'pending'),
            COALESCE(solana_status, 'pending'),
            COALESCE(processing_job_state, 'idle'),
            last_error_stage,
            last_error_message,
            done_at,
            session_hash,
            COALESCE(session_payload, '')
        "#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&writing_session_id)
    .bind(user_id)
    .bind(&req.session_hash)
    .bind(&req.session)
    .bind(&accepted_at)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(ProtocolAnkySnapshot {
        anky_id: snapshot.get::<String, _>(0),
        user_id: user_id.to_string(),
        writing_session_id: snapshot.get::<String, _>(1),
        duration_seconds: req.duration_seconds,
        word_count: req.word_count,
        title: snapshot.get::<Option<String>, _>(2),
        reflection: snapshot.get::<Option<String>, _>(3),
        image_url: snapshot.get::<Option<String>, _>(4),
        solana_signature: snapshot.get::<Option<String>, _>(5),
        reflection_status: snapshot.get::<String, _>(6),
        image_status: snapshot.get::<String, _>(7),
        solana_status: snapshot.get::<String, _>(8),
        processing_job_state: snapshot.get::<String, _>(9),
        last_error_stage: snapshot.get::<Option<String>, _>(10),
        last_error_message: snapshot.get::<Option<String>, _>(11),
        done_at: snapshot.get::<Option<String>, _>(12),
        session_hash: snapshot.get::<String, _>(13),
        session_payload: snapshot.get::<String, _>(14),
    })
}

async fn try_claim_protocol_reflection(state: &AppState, anky_id: &str) -> Result<bool, AppError> {
    let updated = sqlx::query(
        r#"
        UPDATE ankys
        SET reflection_status = 'in_progress',
            status = 'generating',
            reflection_started_at = anky_now(),
            last_error_stage = NULL,
            last_error_message = NULL
        WHERE id = $1
          AND session_hash IS NOT NULL
          AND COALESCE(reflection_status, 'pending') IN ('pending', 'failed')
        "#,
    )
    .bind(anky_id)
    .execute(&state.db)
    .await?;

    Ok(updated.rows_affected() == 1)
}

async fn complete_protocol_reflection(
    state: &AppState,
    anky_id: &str,
    title: &str,
    reflection: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE ankys
        SET title = $2,
            reflection = $3,
            reflection_status = 'complete',
            status = 'generating',
            reflection_completed_at = anky_now(),
            last_error_stage = NULL,
            last_error_message = NULL
        WHERE id = $1
        "#,
    )
    .bind(anky_id)
    .bind(title)
    .bind(reflection)
    .execute(&state.db)
    .await?;
    Ok(())
}

async fn fail_protocol_stage(
    state: &AppState,
    anky_id: &str,
    stage: &str,
    message: &str,
) -> Result<(), AppError> {
    let stage_column = match stage {
        "claude" => "reflection_status",
        "image" => "image_status",
        "solana" => "solana_status",
        "persist" => "processing_job_state",
        _ => {
            return Err(AppError::Internal(format!(
                "unsupported protocol stage: {}",
                stage
            )));
        }
    };
    let sql = format!(
        "UPDATE ankys
         SET {stage_column} = 'failed',
             processing_job_state = CASE
                 WHEN $2 IN ('image', 'solana', 'persist') THEN 'failed'
                 ELSE processing_job_state
             END,
             status = 'generating',
             last_error_stage = $2,
             last_error_message = $3
         WHERE id = $1"
    );

    sqlx::query(&sql)
        .bind(anky_id)
        .bind(stage)
        .bind(message)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn ensure_protocol_processing_enqueued(
    state: &AppState,
    snapshot: &ProtocolAnkySnapshot,
    writing_text: &str,
) -> Result<bool, AppError> {
    if snapshot.reflection_status != "complete" {
        return Ok(false);
    }
    if snapshot.image_status == "complete"
        && (snapshot.solana_status == "complete" || snapshot.solana_status == "skipped")
    {
        return Ok(false);
    }

    let updated = sqlx::query(
        r#"
        UPDATE ankys
        SET processing_job_state = 'enqueued',
            status = 'generating',
            last_error_stage = NULL,
            last_error_message = NULL
        WHERE id = $1
          AND session_hash IS NOT NULL
          AND COALESCE(reflection_status, 'pending') = 'complete'
          AND (
              COALESCE(image_status, 'pending') <> 'complete'
              OR COALESCE(solana_status, 'pending') NOT IN ('complete', 'skipped')
          )
          AND COALESCE(processing_job_state, 'idle') NOT IN ('enqueued', 'in_progress')
        "#,
    )
    .bind(&snapshot.anky_id)
    .execute(&state.db)
    .await?;

    if updated.rows_affected() == 0 {
        return Ok(false);
    }

    let is_pro =
        sqlx::query_scalar::<_, i32>("SELECT COALESCE(is_pro, 0) FROM users WHERE id = $1")
            .bind(&snapshot.user_id)
            .fetch_optional(&state.db)
            .await?
            .unwrap_or(0)
            != 0;

    match crate::services::redis_queue::enqueue_job(
        &state.config.redis_url,
        &crate::state::GpuJob::AnkyImage {
            anky_id: snapshot.anky_id.clone(),
            session_id: snapshot.writing_session_id.clone(),
            user_id: snapshot.user_id.clone(),
            writing: writing_text.to_string(),
        },
        is_pro,
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(err) => {
            fail_protocol_stage(state, &snapshot.anky_id, "persist", &err.to_string()).await?;
            Err(err)
        }
    }
}

pub async fn maybe_enqueue_protocol_processing_for_anky(
    state: &AppState,
    anky_id: &str,
    writing_text: &str,
) -> Result<bool, AppError> {
    let Some(snapshot) = load_protocol_snapshot_by_anky_id(state, anky_id).await? else {
        return Ok(false);
    };

    ensure_protocol_processing_enqueued(state, &snapshot, writing_text).await
}

fn normalize_protocol_title(raw: &str) -> String {
    let title = crate::services::claude::parse_title_reflection(&format!("{}\n", raw)).0;
    if title.is_empty() {
        "untitled reflection".to_string()
    } else {
        title
    }
}

fn is_protocol_done(snapshot: &ProtocolAnkySnapshot) -> bool {
    snapshot.done_at.is_some()
        || (snapshot.image_status == "complete"
            && (snapshot.solana_status == "complete" || snapshot.solana_status == "skipped"))
}

async fn send_protocol_message(
    tx: &tokio::sync::mpsc::Sender<ProtocolStreamMessage>,
    message: ProtocolStreamMessage,
) {
    let _ = tx.send(message).await;
}

async fn emit_protocol_replay_events(
    tx: &tokio::sync::mpsc::Sender<ProtocolStreamMessage>,
    replay_state: &mut ProtocolReplayState,
    snapshot: &ProtocolAnkySnapshot,
) {
    if snapshot.reflection_status == "complete" {
        if !replay_state.title_sent {
            if let Some(title) = snapshot.title.clone().filter(|title| !title.is_empty()) {
                replay_state.title_sent = true;
                send_protocol_message(tx, protocol_event("title", json!({ "title": title }))).await;
            }
        }

        if !replay_state.reflection_complete_sent {
            if let Some(reflection) = snapshot.reflection.clone() {
                replay_state.reflection_complete_sent = true;
                send_protocol_message(
                    tx,
                    protocol_event("reflection_complete", json!({ "reflection": reflection })),
                )
                .await;
            }
        }
    }

    if snapshot.image_status == "complete" && !replay_state.image_sent {
        if let Some(image_url) = snapshot.image_url.clone() {
            replay_state.image_sent = true;
            send_protocol_message(
                tx,
                protocol_event("image_url", json!({ "image_url": image_url })),
            )
            .await;
        }
    }

    if matches!(snapshot.solana_status.as_str(), "complete" | "skipped")
        && !replay_state.solana_sent
    {
        if let Some(signature) = snapshot.solana_signature.clone() {
            replay_state.solana_sent = true;
            send_protocol_message(
                tx,
                protocol_event("solana", json!({ "signature": signature })),
            )
            .await;
        }
    }

    if is_protocol_done(snapshot) && !replay_state.done_sent {
        replay_state.done_sent = true;
        send_protocol_message(
            tx,
            protocol_event("done", json!({ "anky_id": snapshot.anky_id })),
        )
        .await;
    }

    if snapshot.last_error_stage.is_some()
        && !replay_state.error_sent
        && !is_protocol_done(snapshot)
    {
        replay_state.error_sent = true;
        send_protocol_message(
            tx,
            protocol_event(
                "error",
                json!({
                    "stage": snapshot.last_error_stage.clone().unwrap_or_else(|| "persist".to_string()),
                    "retryable": true,
                    "message": snapshot.last_error_message.clone().unwrap_or_else(|| "processing stalled".to_string()),
                }),
            ),
        )
        .await;
    }
}

async fn run_protocol_reflection_generation(
    state: AppState,
    anky_id: String,
    writing_text: String,
    tx: tokio::sync::mpsc::Sender<ProtocolStreamMessage>,
) {
    let (raw_tx, mut raw_rx) = tokio::sync::mpsc::channel::<String>(64);
    let config = state.config.clone();
    let reflection_handle = tokio::spawn(async move {
        crate::services::claude::stream_title_and_reflection_best(
            &config,
            &writing_text,
            raw_tx,
            None,
        )
        .await
    });

    let mut buffered_title = String::new();
    let mut title_sent = false;

    while let Some(chunk) = raw_rx.recv().await {
        if title_sent {
            if !chunk.is_empty() {
                send_protocol_message(
                    &tx,
                    protocol_event("reflection_chunk", json!({ "text": chunk })),
                )
                .await;
            }
            continue;
        }

        buffered_title.push_str(&chunk);
        if let Some(newline_pos) = buffered_title.find('\n') {
            let title = normalize_protocol_title(&buffered_title[..newline_pos]);
            send_protocol_message(&tx, protocol_event("title", json!({ "title": title }))).await;
            title_sent = true;

            let remainder = buffered_title[newline_pos + 1..].to_string();
            if !remainder.trim().is_empty() {
                send_protocol_message(
                    &tx,
                    protocol_event("reflection_chunk", json!({ "text": remainder })),
                )
                .await;
            }
            buffered_title.clear();
        }
    }

    match reflection_handle.await {
        Ok(Ok((full_text, _input_tokens, _output_tokens, _model, _provider))) => {
            let (title, reflection) = crate::services::claude::parse_title_reflection(&full_text);
            let title = if title.is_empty() {
                "untitled reflection".to_string()
            } else {
                title
            };

            if !title_sent {
                send_protocol_message(
                    &tx,
                    protocol_event("title", json!({ "title": title.clone() })),
                )
                .await;
            }

            if let Err(err) =
                complete_protocol_reflection(&state, &anky_id, &title, &reflection).await
            {
                let _ = fail_protocol_stage(&state, &anky_id, "persist", &err.to_string()).await;
                return;
            }
        }
        Ok(Err(err)) => {
            let _ = fail_protocol_stage(&state, &anky_id, "claude", &err.to_string()).await;
        }
        Err(err) => {
            let _ = fail_protocol_stage(&state, &anky_id, "claude", &err.to_string()).await;
        }
    }
}

async fn run_protocol_submit_stream(
    state: AppState,
    user_id: String,
    session_hash: String,
    writing_text: String,
    tx: tokio::sync::mpsc::Sender<ProtocolStreamMessage>,
) {
    let mut replay_state = ProtocolReplayState::default();
    let started = Instant::now();
    let mut reflection_started = false;

    let Some(initial_snapshot) = load_protocol_snapshot(&state, &user_id, &session_hash)
        .await
        .ok()
        .flatten()
    else {
        send_protocol_message(
            &tx,
            protocol_event(
                "error",
                json!({
                    "stage": "persist",
                    "retryable": true,
                    "message": "submit state not found"
                }),
            ),
        )
        .await;
        return;
    };

    send_protocol_message(
        &tx,
        protocol_event("accepted", json!({ "anky_id": initial_snapshot.anky_id })),
    )
    .await;

    loop {
        let snapshot = match load_protocol_snapshot(&state, &user_id, &session_hash).await {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => break,
            Err(err) => {
                send_protocol_message(
                    &tx,
                    protocol_event(
                        "error",
                        json!({
                            "stage": "persist",
                            "retryable": true,
                            "message": err.to_string(),
                        }),
                    ),
                )
                .await;
                break;
            }
        };

        emit_protocol_replay_events(&tx, &mut replay_state, &snapshot).await;
        if replay_state.done_sent || replay_state.error_sent {
            break;
        }

        if !reflection_started && snapshot.reflection_status != "complete" {
            match try_claim_protocol_reflection(&state, &snapshot.anky_id).await {
                Ok(true) => {
                    reflection_started = true;
                    replay_state.title_sent = true;
                    tokio::spawn(run_protocol_reflection_generation(
                        state.clone(),
                        snapshot.anky_id.clone(),
                        writing_text.clone(),
                        tx.clone(),
                    ));
                }
                Ok(false) => {}
                Err(err) => {
                    let _ =
                        fail_protocol_stage(&state, &snapshot.anky_id, "persist", &err.to_string())
                            .await;
                }
            }
        }

        if snapshot.reflection_status == "complete" {
            let _ = ensure_protocol_processing_enqueued(&state, &snapshot, &writing_text).await;
        }

        if started.elapsed() >= Duration::from_secs(180) {
            break;
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn load_protocol_snapshot_by_anky_id(
    state: &AppState,
    anky_id: &str,
) -> Result<Option<ProtocolAnkySnapshot>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
            a.id,
            a.user_id,
            a.writing_session_id,
            COALESCE(ws.duration_seconds::bigint, 0),
            COALESCE(ws.word_count, 0),
            a.title,
            a.reflection,
            a.image_path,
            a.solana_mint_tx,
            COALESCE(a.reflection_status, 'pending'),
            COALESCE(a.image_status, 'pending'),
            COALESCE(a.solana_status, 'pending'),
            COALESCE(a.processing_job_state, 'idle'),
            a.last_error_stage,
            a.last_error_message,
            a.done_at,
            COALESCE(a.session_hash, ''),
            COALESCE(a.session_payload, '')
        FROM ankys a
        LEFT JOIN writing_sessions ws ON ws.id = a.writing_session_id
        WHERE a.id = $1
          AND a.session_hash IS NOT NULL
        LIMIT 1
        "#,
    )
    .bind(anky_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(row.map(|row| ProtocolAnkySnapshot {
        anky_id: row.get::<String, _>(0),
        user_id: row.get::<String, _>(1),
        writing_session_id: row.get::<String, _>(2),
        duration_seconds: row.get::<i64, _>(3),
        word_count: row.get::<i32, _>(4),
        title: row.get::<Option<String>, _>(5),
        reflection: row.get::<Option<String>, _>(6),
        image_url: row.get::<Option<String>, _>(7),
        solana_signature: row.get::<Option<String>, _>(8),
        reflection_status: row.get::<String, _>(9),
        image_status: row.get::<String, _>(10),
        solana_status: row.get::<String, _>(11),
        processing_job_state: row.get::<String, _>(12),
        last_error_stage: row.get::<Option<String>, _>(13),
        last_error_message: row.get::<Option<String>, _>(14),
        done_at: row.get::<Option<String>, _>(15),
        session_hash: row.get::<String, _>(16),
        session_payload: row.get::<String, _>(17),
    }))
}

pub async fn resume_protocol_anky_job(
    state: &AppState,
    anky_id: &str,
    session_id: &str,
    user_id: &str,
    writing_text: &str,
) -> Result<bool, AppError> {
    let Some(initial_snapshot) = load_protocol_snapshot_by_anky_id(state, anky_id).await? else {
        return Ok(false);
    };

    let claimed = sqlx::query(
        r#"
        UPDATE ankys
        SET processing_job_state = 'in_progress',
            status = 'generating',
            last_error_stage = NULL,
            last_error_message = NULL
        WHERE id = $1
          AND session_hash IS NOT NULL
          AND COALESCE(processing_job_state, 'idle') IN ('enqueued', 'failed', 'idle')
        "#,
    )
    .bind(anky_id)
    .execute(&state.db)
    .await?;

    if claimed.rows_affected() == 0 {
        return Ok(true);
    }

    if initial_snapshot.reflection_status != "complete" {
        sqlx::query(
            "UPDATE ankys
             SET processing_job_state = 'idle'
             WHERE id = $1",
        )
        .bind(anky_id)
        .execute(&state.db)
        .await?;
        return Ok(true);
    }

    if initial_snapshot.image_status != "complete" {
        sqlx::query(
            "UPDATE ankys
             SET image_status = 'in_progress',
                 status = 'generating',
                 last_error_stage = NULL,
                 last_error_message = NULL
             WHERE id = $1",
        )
        .bind(anky_id)
        .execute(&state.db)
        .await?;

        if let Err(err) = crate::pipeline::image_gen::generate_anky_from_writing(
            state,
            anky_id,
            session_id,
            user_id,
            writing_text,
        )
        .await
        {
            fail_protocol_stage(state, anky_id, "image", &err.to_string()).await?;
            return Ok(true);
        }

        sqlx::query(
            r#"
            UPDATE ankys
            SET image_status = 'complete',
                image_completed_at = anky_now(),
                status = CASE
                    WHEN COALESCE(solana_status, 'pending') IN ('complete', 'skipped') THEN 'complete'
                    ELSE 'generating'
                END,
                last_error_stage = NULL,
                last_error_message = NULL
            WHERE id = $1
            "#,
        )
        .bind(anky_id)
        .execute(&state.db)
        .await?;
    }

    let Some(snapshot_after_image) = load_protocol_snapshot_by_anky_id(state, anky_id).await?
    else {
        return Ok(true);
    };

    if !matches!(
        snapshot_after_image.solana_status.as_str(),
        "complete" | "skipped"
    ) {
        if state.config.solana_mint_worker_url.is_empty() {
            sqlx::query(
                r#"
                UPDATE ankys
                SET solana_status = 'skipped',
                    solana_completed_at = anky_now(),
                    processing_job_state = 'complete',
                    status = 'complete',
                    done_at = COALESCE(done_at, anky_now()),
                    last_error_stage = NULL,
                    last_error_message = NULL
                WHERE id = $1
                "#,
            )
            .bind(anky_id)
            .execute(&state.db)
            .await?;
            return Ok(true);
        }

        sqlx::query(
            "UPDATE ankys
             SET solana_status = 'in_progress',
                 status = 'generating',
                 last_error_stage = NULL,
                 last_error_message = NULL
             WHERE id = $1",
        )
        .bind(anky_id)
        .execute(&state.db)
        .await?;

        if let Err(err) = crate::pipeline::image_gen::log_session_onchain(
            state,
            anky_id,
            &snapshot_after_image.writing_session_id,
            &snapshot_after_image.user_id,
            &snapshot_after_image.session_hash,
            snapshot_after_image.duration_seconds,
            snapshot_after_image.word_count,
        )
        .await
        {
            fail_protocol_stage(state, anky_id, "solana", &err.to_string()).await?;
            return Ok(true);
        }

        sqlx::query(
            r#"
            UPDATE ankys
            SET solana_status = 'complete',
                solana_completed_at = anky_now(),
                processing_job_state = 'complete',
                status = 'complete',
                done_at = COALESCE(done_at, anky_now()),
                last_error_stage = NULL,
                last_error_message = NULL
            WHERE id = $1
            "#,
        )
        .bind(anky_id)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query(
            r#"
            UPDATE ankys
            SET processing_job_state = 'complete',
                status = CASE
                    WHEN COALESCE(image_status, 'pending') = 'complete'
                     AND COALESCE(solana_status, 'pending') IN ('complete', 'skipped')
                    THEN 'complete'
                    ELSE status
                END,
                done_at = CASE
                    WHEN COALESCE(image_status, 'pending') = 'complete'
                     AND COALESCE(solana_status, 'pending') IN ('complete', 'skipped')
                    THEN COALESCE(done_at, anky_now())
                    ELSE done_at
                END
            WHERE id = $1
            "#,
        )
        .bind(anky_id)
        .execute(&state.db)
        .await?;
    }

    Ok(true)
}

pub async fn submit_anky_protocol(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<AnkyProtocolSubmitRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = protocol_submit_user_id(&state, &headers, &jar).await?;
    validate_protocol_idempotency_headers(&headers, &req.session_hash)?;

    let computed_hash = format!("{:x}", Sha256::digest(req.session.as_bytes()));
    if computed_hash != req.session_hash {
        return Err(AppError::BadRequest(
            "session hash mismatch: sha256(session) does not match session_hash".into(),
        ));
    }

    let parsed = parse_anky_protocol_session(&req.session)?;
    let expected_duration_seconds = (parsed.active_duration_ms as f64 / 1000.0).round() as i64;
    if req.duration_seconds != expected_duration_seconds {
        return Err(AppError::BadRequest(format!(
            "duration_seconds mismatch: expected {}, got {}",
            expected_duration_seconds, req.duration_seconds
        )));
    }
    if req.word_count != parsed.word_count {
        return Err(AppError::BadRequest(format!(
            "word_count mismatch: expected {}, got {}",
            parsed.word_count, req.word_count
        )));
    }

    let started_at = chrono::DateTime::parse_from_rfc3339(&req.started_at)
        .map_err(|_| AppError::BadRequest("started_at must be a valid ISO8601 timestamp".into()))?
        .with_timezone(&chrono::Utc);
    let expected_kingdom = derived_protocol_kingdom(&started_at);
    if req.kingdom.trim().to_lowercase() != expected_kingdom {
        return Err(AppError::BadRequest(format!(
            "kingdom mismatch: expected {}, got {}",
            expected_kingdom, req.kingdom
        )));
    }

    if let Some(signature) = req.wallet_signature.as_deref() {
        let wallet_address = protocol_wallet_address(&state, &user_id)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest(
                    "wallet_signature was provided but no connected wallet is available".into(),
                )
            })?;
        crate::services::wallet::verify_solana_signature(
            &wallet_address,
            &req.session_hash,
            signature,
        )?;
    }

    let snapshot = upsert_protocol_submission(&state, &user_id, &req, &parsed).await?;

    Ok(Json(json!({
        "ok": true,
        "accepted": true,
        "isAnky": true,
        "ankyId": snapshot.anky_id,
        "writingSessionId": snapshot.writing_session_id,
        "reflectionStatus": snapshot.reflection_status,
        "imageStatus": snapshot.image_status,
        "solanaStatus": snapshot.solana_status,
    })))
}

pub async fn process_writing(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<WriteRequest>,
) -> Result<(CookieJar, Json<WriteResponse>), AppError> {
    // Agent auth: if X-API-Key header is present, validate it.
    let api_key = headers.get("x-api-key").and_then(|v| v.to_str().ok());

    if api_key.is_some() {
        // Agents can no longer batch-submit writing. They must use the chunked session API,
        // which enforces the same 8-second timeout that humans face on the frontend.
        return Ok((
            jar,
            Json(WriteResponse {
                response: String::new(),
                duration: 0.0,
                is_anky: false,
                anky_id: None,
                wallet_address: None,
                estimated_wait_seconds: None,
                flow_score: None,
                error: Some(
                    "this endpoint no longer accepts agent submissions. \
                     the practice changed: you now write in real time, chunk by chunk, \
                     with the same 8-second timeout humans face. \
                     read the updated instructions at https://anky.app/skills \
                     and use POST /api/v1/session/start to begin."
                        .into(),
                ),
                anky_response: None,
                next_prompt: None,
                mood: None,
                model: None,
                provider: None,
                generation_ms: None,
                tokens_used: None,
                messages: None,
            }),
        ));
    }

    // Rate limit by IP (CF-Connecting-IP behind Cloudflare, fall back to cookie user id)
    let rate_key = headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            jar.get("anky_user_id")
                .map(|c| c.value().to_string())
                .unwrap_or_else(|| "anonymous".into())
        });

    if let Err(retry_after) = state.write_limiter.check(&rate_key).await {
        tracing::warn!(key = %rate_key, "Rate limited on /write");
        return Err(AppError::RateLimited(retry_after));
    }

    let token_header = headers
        .get("x-anky-user-token")
        .and_then(|v| v.to_str().ok());
    let (user_id, new_cookie) = get_or_create_user_id(&state, &jar, token_header);
    let jar = if let Some(c) = new_cookie {
        jar.add(c)
    } else {
        jar
    };

    let word_count = req.text.split_whitespace().count() as i32;

    // Look up user's preferred model
    let user_preferred_model = {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::get_user_settings(&db, &user_id)
            .ok()
            .map(|s| s.preferred_model)
    };

    // Very short writings (<10 words) — still give a live response via the light model
    if word_count < 10 {
        let nudge = crate::services::ollama::quick_nudge(
            &state.config,
            &req.text,
            user_preferred_model.as_deref(),
        )
        .await
        .unwrap_or_else(|_| {
            "breathe. i'm here with it.\nwhat is the one thing underneath this that wants to be said?"
                .into()
        });
        let messages = crate::services::ollama::two_line_reply_messages(&nudge);
        return Ok((
            jar,
            Json(WriteResponse {
                response: nudge,
                duration: req.duration,
                is_anky: false,
                anky_id: None,
                wallet_address: None,
                estimated_wait_seconds: None,
                flow_score: None,
                error: None,
                anky_response: None,
                next_prompt: None,
                mood: None,
                model: Some("live-nudge".into()),
                provider: Some("ollama".into()),
                generation_ms: None,
                tokens_used: None,
                messages: Some(messages),
            }),
        ));
    }

    // Compute flow score from keystroke deltas
    let flow_score = req
        .keystroke_deltas
        .as_ref()
        .map(|deltas| crate::db::queries::calculate_flow_score(deltas, req.duration, word_count));
    let keystroke_json = req
        .keystroke_deltas
        .as_ref()
        .map(|d| serde_json::to_string(d).unwrap_or_default());

    let mut is_anky = req.duration >= 480.0;

    // Downgrade: 8+ minutes but fewer than 300 words is not a real anky
    if is_anky && word_count < 300 {
        is_anky = false;
    }

    let session_id = req
        .session_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let mins = (req.duration / 60.0) as u32;
    let secs = (req.duration % 60.0) as u32;
    tracing::info!(
        user = %user_id,
        duration = format!("{}m{}s", mins, secs),
        words = word_count,
        is_anky = is_anky,
        "Processing writing session"
    );

    state.emit_log(
        "INFO",
        "writing",
        &format!(
            "New session: {}m{}s, {} words, anky={}",
            mins, secs, word_count, is_anky
        ),
    );

    // Save to DB FIRST — before Ollama call — so writing is never lost
    let wallet_address = {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::ensure_user(&db, &user_id)?;
        let mut wallet_address = crate::db::queries::get_user_wallet(&db, &user_id)?;
        if wallet_address.is_none() {
            let generated_wallet = crate::services::wallet::generate_custodial_wallet();
            crate::db::queries::set_generated_wallet(
                &db,
                &user_id,
                &generated_wallet.address,
                &generated_wallet.secret_key,
            )?;
            wallet_address = Some(generated_wallet.address);
        }
        let mut was_completed = false;
        if let Some(existing) = crate::db::queries::get_writing_session_state(&db, &session_id)? {
            if existing.user_id != user_id {
                // Allow claiming sessions that were auto-recovered with a guessed user_id
                // (e.g. "system", "recovered-unknown") — the real user is submitting now.
                let is_placeholder = existing.user_id == "system"
                    || existing.user_id == "recovered-unknown"
                    || existing.user_id.starts_with("recovered-");
                let same_session_token = existing
                    .session_token
                    .as_deref()
                    .zip(req.session_token.as_deref())
                    .map(|(existing_token, request_token)| existing_token == request_token)
                    .unwrap_or(false);
                if !is_placeholder && !same_session_token {
                    return Err(AppError::Unauthorized(
                        "that writing session belongs to another user".into(),
                    ));
                }
            }
            was_completed = existing.status == "completed";
        }

        crate::db::queries::upsert_completed_writing_session_with_flow(
            &db,
            &session_id,
            &user_id,
            &req.text,
            req.duration,
            word_count,
            is_anky,
            None, // response filled in after Ollama
            keystroke_json.as_deref(),
            flow_score,
            req.session_token.as_deref(),
        )?;
        // Update leaderboard stats
        if !was_completed {
            if let Some(fs) = flow_score {
                let _ = crate::db::queries::update_user_flow_stats(&db, &user_id, fs, is_anky);
            }
        }
        wallet_address
    };

    // Link to an Anky Now if now_slug was provided
    if let Some(ref now_slug) = req.now_slug {
        let db = crate::db::conn(&state.db)?;
        if let Ok(Some(now)) = crate::db::queries::get_now_by_slug(&db, now_slug) {
            let _ = crate::db::queries::insert_now_session(&db, &now.id, &session_id);
        }
    }

    // For anky sessions, return immediately — the frontend will open an SSE
    // connection to /api/stream-reflection/{anky_id} which handles the Claude
    // streaming call. No background call here to avoid duplicate API usage.
    let (response, resp_model, resp_provider, resp_gen_ms) = if is_anky {
        (
            "your anky is being born. the reflection is streaming...".to_string(),
            Some(state.config.openrouter_anky_model.clone()),
            Some(if state.config.openrouter_api_key.is_empty() {
                "claude".to_string()
            } else {
                "openrouter".to_string()
            }),
            None,
        )
    } else {
        let prompt = crate::services::ollama::quick_feedback_prompt(&req.text, req.duration);
        let gen_start = std::time::Instant::now();
        let (r, model_name, provider_name) = if !state.config.openrouter_api_key.is_empty() {
            let model_name = user_preferred_model
                .as_deref()
                .filter(|m| !m.is_empty() && *m != "default")
                .map(|m| m.to_string())
                .unwrap_or_else(|| state.config.openrouter_light_model.clone());
            match crate::services::openrouter::call_openrouter(
                &state.config.openrouter_api_key,
                &model_name,
                "",
                &prompt,
                700,
                60,
            )
            .await
            {
                Ok(text) => (text, model_name, "openrouter".to_string()),
                Err(e) => {
                    tracing::error!("OpenRouter quick feedback error: {}", e);
                    state.emit_log(
                        "ERROR",
                        "openrouter",
                        &format!("Quick feedback error: {}", e),
                    );
                    match crate::services::claude::call_haiku(
                        &state.config.anthropic_api_key,
                        &prompt,
                    )
                    .await
                    {
                        Ok(text) => (
                            text,
                            crate::services::claude::HAIKU_MODEL.to_string(),
                            "claude".to_string(),
                        ),
                        Err(err) => {
                            tracing::error!("Haiku error: {}", err);
                            state.emit_log("ERROR", "haiku", &format!("Haiku error: {}", err));
                            (
                                crate::services::ollama::quick_nudge(
                                    &state.config,
                                    &req.text,
                                    user_preferred_model.as_deref(),
                                )
                                .await
                                .unwrap_or_else(|_| {
                                    "breathe. i'm here with it.\nwhat is the one thing underneath this that wants to be said?"
                                        .into()
                                }),
                                "live-nudge".to_string(),
                                "fallback".to_string(),
                            )
                        }
                    }
                }
            }
        } else {
            match crate::services::claude::call_haiku(&state.config.anthropic_api_key, &prompt)
                .await
            {
                Ok(text) => (
                    text,
                    crate::services::claude::HAIKU_MODEL.to_string(),
                    "claude".to_string(),
                ),
                Err(e) => {
                    tracing::error!("Haiku error: {}", e);
                    state.emit_log("ERROR", "haiku", &format!("Haiku error: {}", e));
                    (
                        crate::services::ollama::quick_nudge(
                            &state.config,
                            &req.text,
                            user_preferred_model.as_deref(),
                        )
                        .await
                        .unwrap_or_else(|_| {
                            "breathe. i'm here with it.\nwhat is the one thing underneath this that wants to be said?"
                                .into()
                        }),
                        "live-nudge".to_string(),
                        "fallback".to_string(),
                    )
                }
            }
        };
        let r = crate::services::ollama::normalize_two_line_reply(&r);
        let gen_elapsed = gen_start.elapsed().as_millis() as u64;
        // Update the writing session with Ollama's response
        {
            let db = crate::db::conn(&state.db)?;
            let _ = db.execute(
                "UPDATE writing_sessions SET response = ?1 WHERE id = ?2",
                crate::params![&r, &session_id],
            );
        }
        (r, Some(model_name), Some(provider_name), Some(gen_elapsed))
    };

    // Handle inquiry: mark answered and generate next question
    if let Some(ref inquiry_id) = req.inquiry_id {
        if !inquiry_id.is_empty() {
            let db = crate::db::conn(&state.db)?;
            let _ =
                crate::db::queries::mark_inquiry_answered(&db, inquiry_id, &req.text, &session_id);

            // Spawn background task to generate next inquiry via Claude
            let state_bg = state.clone();
            let uid_bg = user_id.clone();
            tokio::spawn(async move {
                if let Err(e) = generate_next_inquiry(&state_bg, &uid_bg).await {
                    tracing::warn!("Failed to generate next inquiry: {}", e);
                }
            });
        }
    }

    let mut anky_id = None;

    // If it's an Anky, kick off background image generation
    if is_anky {
        let aid = uuid::Uuid::new_v4().to_string();
        {
            let db = crate::db::conn(&state.db)?;
            crate::db::queries::insert_anky(
                &db,
                &aid,
                &session_id,
                &user_id,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                "generating",
                "written",
                req.prompt_id.as_deref(),
            )?;
        }

        anky_id = Some(aid.clone());

        // Log writing session on-chain via spl-memo unless this is an unsigned
        // protocol submission, which the web treats as an unanchored session.
        let should_anchor = req.session_hash.is_none() || req.wallet_signature.is_some();
        if should_anchor {
            let log_state = state.clone();
            let log_aid = aid.clone();
            let log_sid = session_id.clone();
            let log_uid = user_id.clone();
            let log_text = req.text.clone();
            let explicit_session_hash = req.session_hash.clone();
            let log_duration = req.duration as i64;
            let log_words = word_count;
            tokio::spawn(async move {
                let session_hash = explicit_session_hash
                    .unwrap_or_else(|| format!("{:x}", Sha256::digest(log_text.as_bytes())));
                if let Err(e) = crate::pipeline::image_gen::log_session_onchain(
                    &log_state,
                    &log_aid,
                    &log_sid,
                    &log_uid,
                    &session_hash,
                    log_duration,
                    log_words,
                )
                .await
                {
                    tracing::warn!("session on-chain log failed: {}", e);
                }
            });
        } else {
            tracing::info!(session_id = %session_id, "Skipping on-chain anchor for unsigned protocol submission");
        }

        // Submit to GPU priority queue
        let is_pro = {
            let db = crate::db::conn(&state.db)?;
            crate::db::queries::is_user_pro(&db, &user_id).unwrap_or(false)
        };
        crate::services::redis_queue::enqueue_job(
            &state.config.redis_url,
            &crate::state::GpuJob::AnkyImage {
                anky_id: aid,
                session_id: session_id.clone(),
                user_id: user_id.clone(),
                writing: req.text.clone(),
            },
            is_pro,
        )
        .await?;
    }

    // anky_response is generated by the post-writing pipeline (event-driven),
    // not on-demand. The client polls /writing/{sessionId}/status to get it.

    // For web writes that aren't ankys, spawn the pipeline response generation
    if !is_anky {
        let resp_state = state.clone();
        let resp_uid = user_id.clone();
        let resp_sid = session_id.clone();
        let resp_text = req.text.clone();
        tokio::spawn(async move {
            crate::pipeline::guidance_gen::generate_anky_response(
                &resp_state,
                &resp_uid,
                &resp_sid,
                &resp_text,
            )
            .await;
        });
    }
    // For ankys, the full ritual lifecycle (which includes generate_anky_response)
    // is already triggered by the image pipeline completion.

    Ok((
        jar,
        Json(WriteResponse {
            messages: if is_anky {
                None
            } else {
                Some(crate::services::ollama::two_line_reply_messages(&response))
            },
            response,
            duration: req.duration,
            is_anky,
            anky_id,
            wallet_address,
            estimated_wait_seconds: if is_anky { Some(45) } else { None },
            flow_score,
            error: None,
            anky_response: None, // generated async — client polls or waits
            next_prompt: None,
            mood: None,
            model: resp_model,
            provider: resp_provider,
            generation_ms: resp_gen_ms,
            tokens_used: None,
        }),
    ))
}

/// GET /api/writing/{sessionId}/status — web-accessible writing status (cookie auth)
pub async fn get_writing_status_web(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (user_id, _) = get_or_create_user_id(&state, &jar, None);
    let db = crate::db::conn(&state.db)?;

    let (anky_response, next_prompt, mood) = db
        .query_row(
            "SELECT anky_response, anky_next_prompt, anky_mood FROM writing_sessions WHERE id = ?1 AND user_id = ?2",
            crate::params![&session_id, &user_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0).unwrap_or(None),
                    row.get::<_, Option<String>>(1).unwrap_or(None),
                    row.get::<_, Option<String>>(2).unwrap_or(None),
                ))
            },
        )
        .unwrap_or((None, None, None));

    Ok(Json(serde_json::json!({
        "sessionId": session_id,
        "ankyResponse": anky_response,
        "nextPrompt": next_prompt,
        "mood": mood,
    })))
}

pub async fn get_writings(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let (user_id, _) = get_or_create_user_id(&state, &jar, None);

    let writings = {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::get_user_writings_with_ankys(&db, &user_id)?
    };

    let now = chrono::Utc::now();

    let mut ctx = tera::Context::new();
    ctx.insert(
        "writings",
        &serde_json::to_value(
            writings
                .iter()
                .map(|w| {
                    let first_line: String = w
                        .content
                        .lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(60)
                        .collect();
                    let duration_display = format!(
                        "{}m {}s",
                        (w.duration_seconds / 60.0) as u32,
                        (w.duration_seconds % 60.0) as u32
                    );
                    let relative_time = if let Ok(dt) =
                        chrono::NaiveDateTime::parse_from_str(&w.created_at, "%Y-%m-%d %H:%M:%S")
                    {
                        let diff = now.naive_utc() - dt;
                        if diff.num_days() > 0 {
                            format!("{}d ago", diff.num_days())
                        } else if diff.num_hours() > 0 {
                            format!("{}h ago", diff.num_hours())
                        } else {
                            format!("{}m ago", diff.num_minutes().max(1))
                        }
                    } else {
                        w.created_at.clone()
                    };
                    // Prefer Claude reflection over Ollama response for ankys
                    let display_response = if w.is_anky {
                        w.anky_reflection.as_ref().or(w.response.as_ref()).cloned()
                    } else {
                        w.response.clone()
                    };
                    serde_json::json!({
                        "id": w.id,
                        "content": w.content,
                        "first_line": first_line,
                        "duration_seconds": w.duration_seconds,
                        "duration_display": duration_display,
                        "word_count": w.word_count,
                        "is_anky": w.is_anky,
                        "response": display_response,
                        "created_at": w.created_at,
                        "relative_time": relative_time,
                        "anky_id": w.anky_id,
                        "anky_title": w.anky_title,
                        "anky_image_path": w.anky_image_path,
                        "conversation_json": w.conversation_json,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );

    let html = state.tera.render("writings.html", &ctx)?;
    Ok(Html(html))
}

/// GET /writing/{id} — public read-only view of a writing session (copyable text)
pub async fn view_writing(
    State(state): State<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Html<String>, AppError> {
    let db = crate::db::conn(&state.db)?;
    let (content, word_count, duration, created_at): (String, i32, f64, String) = db
        .query_row(
            "SELECT content, word_count, duration_seconds, created_at FROM writing_sessions WHERE id = ?1",
            crate::params![&session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .map_err(|_| AppError::NotFound("Writing session not found".into()))?;

    let mins = (duration / 60.0) as u32;
    let secs = (duration % 60.0) as u32;
    let escaped = content
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', "<br>");

    let html = format!(
        r#"<!DOCTYPE html>
<html><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>anky — writing session</title>
<style>
  body {{ font-family: Georgia, serif; max-width: 640px; margin: 2rem auto; padding: 0 1rem; background: #0a0a0a; color: #e0e0e0; }}
  .meta {{ color: #888; font-size: 0.85rem; margin-bottom: 1.5rem; }}
  .content {{ line-height: 1.7; white-space: pre-wrap; user-select: all; }}
  button {{ background: #333; color: #e0e0e0; border: 1px solid #555; padding: 0.5rem 1rem; border-radius: 4px; cursor: pointer; font-size: 0.9rem; }}
  button:hover {{ background: #444; }}
  .copied {{ color: #6f6; }}
</style>
</head><body>
<h2>anky</h2>
<div class="meta">{word_count} words &middot; {mins}m {secs}s &middot; {created_at}</div>
<button onclick="navigator.clipboard.writeText(document.getElementById('t').innerText).then(()=>{{this.textContent='copied!';this.classList.add('copied')}})">copy text</button>
<div id="t" class="content" style="margin-top:1.5rem">{escaped}</div>
</body></html>"#
    );
    Ok(Html(html))
}

/// Generate the next inquiry for a user based on their history.
async fn generate_next_inquiry(state: &AppState, user_id: &str) -> anyhow::Result<()> {
    let (history, lang) = {
        let db = crate::db::conn(&state.db)?;
        let h = crate::db::queries::get_inquiry_history(&db, user_id, 10)?;
        let l = crate::db::queries::get_inquiry_language(&db, user_id)?
            .unwrap_or_else(|| "en".to_string());
        (h, l)
    };

    // Build context from history
    let mut context = String::new();
    for (q, a, _) in history.iter().rev() {
        context.push_str(&format!("Q: {}\n", q));
        if let Some(answer) = a {
            let excerpt: String = answer.chars().take(500).collect();
            context.push_str(&format!("A: {}\n\n", excerpt));
        } else {
            context.push_str("A: [skipped/unanswered]\n\n");
        }
    }

    // Get psychological profile if available
    let profile = {
        let db = crate::db::conn(&state.db)?;
        let mut stmt =
            db.prepare("SELECT psychological_profile FROM user_profiles WHERE user_id = ?1")?;
        let mut rows = stmt.query_map(crate::params![user_id], |row| {
            row.get::<_, Option<String>>(0)
        })?;
        rows.next()
            .and_then(|r| r.ok())
            .flatten()
            .unwrap_or_default()
    };

    let system = format!(
        "You generate self-inquiry questions. Rules:\n\
         - Maximum 12 words. Shorter is better. 5-8 words is ideal.\n\
         - Ask about what the person DOES, not what they THINK. Behavior, not philosophy.\n\
         - Target the gap between who they perform being and who they actually are.\n\
         - No therapy-speak. No \"how does that make you feel.\" No \"tell me about.\"\n\
         - Be specific and concrete. \"What did you last lie about?\" not \"What is truth?\"\n\
         - The question should sting a little. It should be hard to answer honestly.\n\
         - Language: {}. Output ONLY the question. No quotes, no explanation.",
        lang
    );

    let user_msg = if context.is_empty() && profile.is_empty() {
        "Generate a sharp opening question for someone who just arrived. \
         Hit them where they live. No warm-up."
            .to_string()
    } else {
        let mut msg = String::new();
        if !profile.is_empty() {
            msg.push_str(&format!("PSYCHOLOGICAL PROFILE:\n{}\n\n", profile));
        }
        if !context.is_empty() {
            msg.push_str(&format!("CONVERSATION HISTORY:\n{}", context));
        }
        msg.push_str(
            "\nBased on what they revealed, ask the ONE question they're hoping you won't ask.",
        );
        msg
    };

    let question = crate::services::claude::call_haiku_with_system(
        &state.config.anthropic_api_key,
        &system,
        &user_msg,
    )
    .await?
    .trim()
    .to_string();

    if !question.is_empty() {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::create_inquiry(&db, user_id, &question, &lang)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::routes::session::new_session_map;
    use crate::routes::simulations::SlotTracker;
    use crate::services::stream::new_frame_buffer;
    use crate::state::{AppState, LiveState, RateLimiter};
    use redis::AsyncCommands;
    use std::collections::{HashMap, VecDeque};
    use std::sync::{Arc, LazyLock, Mutex as StdMutex};
    use tokio::sync::{broadcast, Mutex, RwLock};

    static TEST_LOCK: LazyLock<StdMutex<()>> = LazyLock::new(|| StdMutex::new(()));
    const TEST_DATABASE_URL: &str = "postgres://anky:anky@127.0.0.1:5432/anky";
    const TEST_REDIS_URL: &str = "redis://127.0.0.1:6379/15";

    async fn build_test_state() -> AppState {
        let mut config = Config::from_env().expect("config");
        config.database_url = TEST_DATABASE_URL.to_string();
        config.redis_url = TEST_REDIS_URL.to_string();
        config.solana_mint_worker_url.clear();
        config.solana_mint_worker_secret.clear();

        let db = crate::db::create_pool(TEST_DATABASE_URL)
            .await
            .expect("db pool");
        let (log_tx, _) = broadcast::channel(64);
        let (live_status_tx, _) = broadcast::channel(16);
        let (live_text_tx, _) = broadcast::channel(16);
        let (webhook_log_tx, _) = broadcast::channel(16);

        AppState {
            db,
            tera: Arc::new(tera::Tera::default()),
            i18n: Arc::new(crate::i18n::I18n::load_from_dir("locales").expect("i18n")),
            config: Arc::new(config),
            gpu_status: Arc::new(RwLock::new(crate::state::GpuStatus::Idle)),
            log_tx,
            live_state: Arc::new(RwLock::new(LiveState::default())),
            live_status_tx,
            live_text_tx,
            frame_buffer: new_frame_buffer(),
            write_limiter: RateLimiter::new(50, Duration::from_secs(60)),
            waiting_room: Arc::new(RwLock::new(VecDeque::new())),
            image_limiter: RateLimiter::new(50, Duration::from_secs(60)),
            webhook_log_tx,
            memory_cache: Arc::new(Mutex::new(HashMap::new())),
            sessions: new_session_map(),
            slot_tracker: SlotTracker::new(),
            log_history: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    async fn flush_test_redis() {
        let client = redis::Client::open(TEST_REDIS_URL).expect("redis client");
        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .expect("redis conn");
        redis::cmd("FLUSHDB")
            .query_async::<_, ()>(&mut conn)
            .await
            .expect("flushdb");
    }

    async fn queued_jobs_for_anky(anky_id: &str) -> Vec<String> {
        let client = redis::Client::open(TEST_REDIS_URL).expect("redis client");
        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .expect("redis conn");

        let mut payloads: Vec<String> = conn
            .lrange("anky:jobs:pro", 0, -1)
            .await
            .expect("lrange pro");
        let mut free_payloads: Vec<String> = conn
            .lrange("anky:jobs:free", 0, -1)
            .await
            .expect("lrange free");
        payloads.append(&mut free_payloads);

        payloads
            .into_iter()
            .filter(|payload| payload.contains(anky_id))
            .collect()
    }

    async fn cleanup_protocol_rows(state: &AppState, user_id: &str, session_hash: &str) {
        sqlx::query("DELETE FROM ankys WHERE user_id = $1 AND session_hash = $2")
            .bind(user_id)
            .bind(session_hash)
            .execute(&state.db)
            .await
            .expect("delete ankys");
        sqlx::query("DELETE FROM writing_sessions WHERE user_id = $1 AND session_hash = $2")
            .bind(user_id)
            .bind(session_hash)
            .execute(&state.db)
            .await
            .expect("delete writing_sessions");
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&state.db)
            .await
            .expect("delete users");
    }

    fn build_session(text: &str) -> String {
        let mut lines = Vec::new();
        for (idx, ch) in text.chars().enumerate() {
            let delta = if idx == 0 { 0 } else { 100 };
            if ch == ' ' {
                lines.push(format!("{} ", delta));
            } else {
                lines.push(format!("{} {}", delta, ch));
            }
        }
        lines.join("\n")
    }

    fn build_protocol_request(text: &str) -> AnkyProtocolSubmitRequest {
        let started_at = chrono::DateTime::parse_from_rfc3339("2026-04-14T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let session = build_session(text);
        let session_hash = format!("{:x}", Sha256::digest(session.as_bytes()));
        let word_count = text.split_whitespace().count() as i32;
        let duration_seconds =
            ((text.chars().count().saturating_sub(1) as f64) * 0.1).round() as i64;

        AnkyProtocolSubmitRequest {
            session_hash,
            session,
            duration_seconds,
            word_count,
            kingdom: derived_protocol_kingdom(&started_at),
            started_at: started_at.to_rfc3339(),
            wallet_signature: None,
        }
    }

    async fn collect_replay_events(
        snapshot: &ProtocolAnkySnapshot,
    ) -> Vec<(String, serde_json::Value)> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ProtocolStreamMessage>(16);
        let mut replay_state = ProtocolReplayState::default();
        emit_protocol_replay_events(&tx, &mut replay_state, snapshot).await;
        drop(tx);

        let mut events = Vec::new();
        while let Some(message) = rx.recv().await {
            events.push((message.event.to_string(), message.data));
        }
        events
    }

    #[tokio::test(flavor = "current_thread")]
    async fn first_submit_creates_one_logical_anky() {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = build_test_state().await;
        flush_test_redis().await;

        let user_id = format!("test-protocol-{}", uuid::Uuid::new_v4());
        let req = build_protocol_request("a quiet honest sentence");
        let parsed = parse_anky_protocol_session(&req.session).expect("parsed");

        let snapshot = upsert_protocol_submission(&state, &user_id, &req, &parsed)
            .await
            .expect("upsert");

        let loaded = load_protocol_snapshot(&state, &user_id, &req.session_hash)
            .await
            .expect("load")
            .expect("snapshot");

        let anky_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ankys WHERE user_id = $1 AND session_hash = $2",
        )
        .bind(&user_id)
        .bind(&req.session_hash)
        .fetch_one(&state.db)
        .await
        .expect("anky count");
        let writing_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM writing_sessions WHERE user_id = $1 AND session_hash = $2",
        )
        .bind(&user_id)
        .bind(&req.session_hash)
        .fetch_one(&state.db)
        .await
        .expect("writing count");

        assert_eq!(snapshot.anky_id, loaded.anky_id);
        assert_eq!(loaded.reflection_status, "pending");
        assert_eq!(loaded.image_status, "pending");
        assert_eq!(loaded.solana_status, "pending");
        assert_eq!(anky_count, 1);
        assert_eq!(writing_count, 1);

        cleanup_protocol_rows(&state, &user_id, &req.session_hash).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn duplicate_submit_before_completion_reuses_same_anky() {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = build_test_state().await;
        flush_test_redis().await;

        let user_id = format!("test-protocol-{}", uuid::Uuid::new_v4());
        let req = build_protocol_request("duplicate before completion");
        let parsed = parse_anky_protocol_session(&req.session).expect("parsed");

        let first = upsert_protocol_submission(&state, &user_id, &req, &parsed)
            .await
            .expect("first");
        let second = upsert_protocol_submission(&state, &user_id, &req, &parsed)
            .await
            .expect("second");

        let anky_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ankys WHERE user_id = $1 AND session_hash = $2",
        )
        .bind(&user_id)
        .bind(&req.session_hash)
        .fetch_one(&state.db)
        .await
        .expect("anky count");

        assert_eq!(first.anky_id, second.anky_id);
        assert_eq!(anky_count, 1);

        cleanup_protocol_rows(&state, &user_id, &req.session_hash).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn duplicate_submit_after_completion_replays_stored_artifacts() {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = build_test_state().await;
        flush_test_redis().await;

        let user_id = format!("test-protocol-{}", uuid::Uuid::new_v4());
        let req = build_protocol_request("replay the finished anky");
        let parsed = parse_anky_protocol_session(&req.session).expect("parsed");
        let snapshot = upsert_protocol_submission(&state, &user_id, &req, &parsed)
            .await
            .expect("upsert");

        sqlx::query(
            "UPDATE ankys
             SET title = 'already there',
                 reflection = 'the stored reflection',
                 image_path = 'https://cdn.example/anky.webp',
                 solana_mint_tx = 'sig-123',
                 reflection_status = 'complete',
                 image_status = 'complete',
                 solana_status = 'complete',
                 processing_job_state = 'complete',
                 done_at = anky_now(),
                 last_error_stage = NULL,
                 last_error_message = NULL
             WHERE id = $1",
        )
        .bind(&snapshot.anky_id)
        .execute(&state.db)
        .await
        .expect("complete anky");

        let loaded = load_protocol_snapshot(&state, &user_id, &req.session_hash)
            .await
            .expect("load")
            .expect("snapshot");
        let events = collect_replay_events(&loaded).await;
        let event_names: Vec<String> = events.into_iter().map(|(event, _)| event).collect();

        assert_eq!(
            event_names,
            vec![
                "title".to_string(),
                "reflection_complete".to_string(),
                "image_url".to_string(),
                "solana".to_string(),
                "done".to_string()
            ]
        );

        cleanup_protocol_rows(&state, &user_id, &req.session_hash).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn partial_failure_retry_reenqueues_once_and_resumes_missing_stage() {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = build_test_state().await;
        flush_test_redis().await;

        let user_id = format!("test-protocol-{}", uuid::Uuid::new_v4());
        let req = build_protocol_request("resume the missing solana stage");
        let parsed = parse_anky_protocol_session(&req.session).expect("parsed");
        let snapshot = upsert_protocol_submission(&state, &user_id, &req, &parsed)
            .await
            .expect("upsert");

        sqlx::query(
            "UPDATE ankys
             SET title = 'complete enough',
                 reflection = 'reflection is already done',
                 image_path = 'https://cdn.example/existing.webp',
                 reflection_status = 'complete',
                 image_status = 'complete',
                 solana_status = 'failed',
                 processing_job_state = 'failed',
                 last_error_stage = 'solana',
                 last_error_message = 'worker timeout'
             WHERE id = $1",
        )
        .bind(&snapshot.anky_id)
        .execute(&state.db)
        .await
        .expect("mark partial failure");

        let retry_snapshot = load_protocol_snapshot(&state, &user_id, &req.session_hash)
            .await
            .expect("load")
            .expect("snapshot");

        assert!(
            ensure_protocol_processing_enqueued(&state, &retry_snapshot, &parsed.text)
                .await
                .expect("enqueue")
        );
        assert!(
            !ensure_protocol_processing_enqueued(&state, &retry_snapshot, &parsed.text)
                .await
                .expect("dedupe enqueue")
        );
        assert_eq!(queued_jobs_for_anky(&snapshot.anky_id).await.len(), 1);

        resume_protocol_anky_job(
            &state,
            &snapshot.anky_id,
            &snapshot.writing_session_id,
            &user_id,
            &parsed.text,
        )
        .await
        .expect("resume");

        let resumed = load_protocol_snapshot(&state, &user_id, &req.session_hash)
            .await
            .expect("load resumed")
            .expect("resumed snapshot");

        assert_eq!(resumed.image_status, "complete");
        assert_eq!(resumed.solana_status, "skipped");
        assert!(resumed.done_at.is_some());

        cleanup_protocol_rows(&state, &user_id, &req.session_hash).await;
        flush_test_redis().await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn concurrent_duplicate_submits_collapse_to_one_row() {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = build_test_state().await;
        flush_test_redis().await;

        let user_id = format!("test-protocol-{}", uuid::Uuid::new_v4());
        let req = build_protocol_request("concurrent duplicates should collapse");

        let mut handles = Vec::new();
        for _ in 0..8 {
            let state = state.clone();
            let user_id = user_id.clone();
            let req = req.clone();
            handles.push(tokio::spawn(async move {
                let parsed = parse_anky_protocol_session(&req.session).expect("parsed");
                upsert_protocol_submission(&state, &user_id, &req, &parsed)
                    .await
                    .expect("upsert")
                    .anky_id
            }));
        }

        let mut anky_ids = Vec::new();
        for handle in handles {
            anky_ids.push(handle.await.expect("join"));
        }
        anky_ids.sort();
        anky_ids.dedup();

        let anky_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ankys WHERE user_id = $1 AND session_hash = $2",
        )
        .bind(&user_id)
        .bind(&req.session_hash)
        .fetch_one(&state.db)
        .await
        .expect("anky count");
        let writing_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM writing_sessions WHERE user_id = $1 AND session_hash = $2",
        )
        .bind(&user_id)
        .bind(&req.session_hash)
        .fetch_one(&state.db)
        .await
        .expect("writing count");

        assert_eq!(anky_ids.len(), 1);
        assert_eq!(anky_count, 1);
        assert_eq!(writing_count, 1);

        cleanup_protocol_rows(&state, &user_id, &req.session_hash).await;
    }
}
