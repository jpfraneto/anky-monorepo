use crate::db::queries;
use crate::routes::auth;
use crate::services::stream;
use crate::state::{AppState, LiveStatusEvent, LiveTextEvent, QueueEntry};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde_json::json;
use std::convert::Infallible;

/// GET /ws/live — WebSocket handler for GO LIVE.
/// Only one writer at a time. Requires authenticated user with username.
pub async fn ws_live(
    ws: WebSocketUpgrade,
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Authenticate
    let user = auth::get_auth_user(&state, &jar).await;
    let username = user.as_ref().and_then(|u| u.username.clone());

    match username {
        Some(uname) => ws.on_upgrade(move |socket| handle_live_socket(socket, state, uname)),
        None => {
            // Reject — can't go live without username
            ws.on_upgrade(move |socket| async move {
                let mut s = socket;
                let _ = s.send(Message::Text(
                    json!({"type":"error","message":"login required — set a username in /settings"}).to_string().into(),
                )).await;
            })
        }
    }
}

/// Max live session duration: 8 minutes (480 seconds)
const MAX_LIVE_SECS: u64 = 480;

async fn handle_live_socket(mut socket: WebSocket, state: AppState, username: String) {
    let writer_id = uuid::Uuid::new_v4().to_string();

    // Try to claim the live slot
    {
        let mut live = state.live_state.write().await;
        if live.is_live {
            let _ = socket
                .send(Message::Text(
                    json!({"type":"error","message":"slot occupied"})
                        .to_string()
                        .into(),
                ))
                .await;
            drop(socket);
            return;
        }
        live.is_live = true;
        live.writer_id = Some(writer_id.clone());
        live.writer_username = Some(username.clone());
        live.writer_type = Some("human".to_string());
        live.started_at = Some(std::time::Instant::now());
    }

    // Broadcast that we went live
    let _ = state.live_status_tx.send(LiveStatusEvent::WentLive {
        writer_id: writer_id.clone(),
        writer_username: username.clone(),
        writer_type: "human".to_string(),
    });
    state.emit_log(
        "INFO",
        "live",
        &format!("@{} went live on stream", username),
    );

    // Set initial live frame
    stream::update_live_frame(
        &state.frame_buffer,
        &username,
        "writing in progress...",
        0,
        0.0,
        1.0,
        0.0,
    )
    .await;

    // Broadcast initial live text event
    let _ = state.live_text_tx.send(LiveTextEvent {
        content: "writing in progress...".to_string(),
        words: 0,
        elapsed: 0.0,
        idle_ratio: 1.0,
        progress: 0.0,
        is_live: true,
        writer_username: Some(username.clone()),
        writer_type: Some("human".to_string()),
        congrats: None,
    });

    // Confirm to client
    let _ = socket
        .send(Message::Text(
            json!({"type":"live","writer_id":writer_id,"writer_username":username})
                .to_string()
                .into(),
        ))
        .await;

    // 8-minute hard stop timer
    let deadline = tokio::time::sleep(std::time::Duration::from_secs(MAX_LIVE_SECS));
    tokio::pin!(deadline);
    let mut hit_8_min = false;

    // Read messages from client, with 8-minute deadline
    loop {
        tokio::select! {
            _ = &mut deadline => {
                // 8 minutes reached — force stop
                hit_8_min = true;
                let _ = socket.send(Message::Text(
                    json!({"type":"anky_complete","message":"8 minutes reached — you wrote an anky!"}).to_string().into(),
                )).await;
                break;
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            if parsed.get("type").and_then(|t| t.as_str()) == Some("text") {
                                if let Some(content) = parsed.get("content").and_then(|c| c.as_str()) {
                                    let words = parsed.get("words").and_then(|v| v.as_i64()).unwrap_or(0);
                                    let elapsed = parsed.get("elapsed").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                    let idle_ratio = parsed.get("idle_ratio").and_then(|v| v.as_f64()).unwrap_or(1.0);
                                    let progress = parsed.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);

                                    // Update Rust-rendered frame
                                    stream::update_live_frame(
                                        &state.frame_buffer,
                                        &username,
                                        content,
                                        words, elapsed, idle_ratio, progress,
                                    ).await;

                                    // Broadcast to overlay SSE clients
                                    let _ = state.live_text_tx.send(LiveTextEvent {
                                        content: content.to_string(),
                                        words,
                                        elapsed,
                                        idle_ratio,
                                        progress,
                                        is_live: true,
                                        writer_username: Some(username.clone()),
                                        writer_type: Some("human".to_string()),
                                        congrats: None,
                                    });
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    // Release the live slot
    {
        let mut live = state.live_state.write().await;
        if live.writer_id.as_deref() == Some(&writer_id) {
            live.is_live = false;
            live.writer_id = None;
            live.writer_username = None;
            live.writer_type = None;
            live.started_at = None;
        }
    }

    if hit_8_min {
        // Show congratulations for 8 seconds
        state.emit_log(
            "INFO",
            "live",
            &format!("@{} completed an anky! (8 min)", username),
        );
        {
            state.live_state.write().await.showing_congrats = true;
        }
        stream::set_congrats_frame(&state.frame_buffer, &username).await;
        let _ = state.live_status_tx.send(LiveStatusEvent::Congrats {
            writer_username: username.clone(),
        });
        let _ = state.live_text_tx.send(LiveTextEvent {
            content: String::new(),
            words: 0,
            elapsed: 480.0,
            idle_ratio: 0.0,
            progress: 1.0,
            is_live: false,
            writer_username: Some(username.clone()),
            writer_type: Some("human".to_string()),
            congrats: Some(true),
        });
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    } else {
        state.emit_log("INFO", "live", &format!("@{} ended live session", username));
    }

    {
        state.live_state.write().await.showing_congrats = false;
    }
    let _ = state.live_status_tx.send(LiveStatusEvent::WentIdle);
    stream::set_idle_frame(&state.frame_buffer).await;

    // Broadcast idle text event
    let _ = state.live_text_tx.send(LiveTextEvent {
        content: String::new(),
        words: 0,
        elapsed: 0.0,
        idle_ratio: 0.0,
        progress: 0.0,
        is_live: false,
        writer_username: None,
        writer_type: None,
        congrats: None,
    });

    // Try next queued writer (spawn to avoid recursive async cycle)
    tokio::spawn(async move {
        try_next_from_queue(state).await;
    });
}

/// Compute per-word delays that spread the text over 480 seconds with natural rhythm.
/// Punctuation at the end of a word increases its weight (longer pause after it).
fn compute_typing_rhythm(words: &[&str]) -> Vec<f64> {
    if words.is_empty() {
        return vec![];
    }

    let mut weights: Vec<f64> = Vec::with_capacity(words.len());
    for (i, word) in words.iter().enumerate() {
        let last_char = word.chars().last().unwrap_or(' ');
        let mut w: f64 = 1.0;

        // Punctuation rhythm
        match last_char {
            '.' | '!' | '?' => w = 3.5, // end of sentence — long pause
            ',' | ';' => w = 2.0,       // clause break — medium pause
            ':' => w = 2.5,             // colon — slightly longer
            '-' | '—' => w = 1.8,       // dash — slight pause
            _ => {}
        }

        // Ellipsis gets extra weight
        if word.ends_with("...") || word.ends_with("…") {
            w = 4.0;
        }

        // Check if next word starts a new paragraph (double newline was collapsed,
        // but we can detect capitalized word after sentence-ending punctuation)
        if i + 1 < words.len() {
            let next = words[i + 1];
            if (last_char == '.' || last_char == '!' || last_char == '?')
                && next
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
            {
                w = w.max(4.5); // paragraph-like break
            }
        }

        weights.push(w);
    }

    // Normalize weights so they sum to exactly 480 seconds
    let total_weight: f64 = weights.iter().sum();
    let target_secs = MAX_LIVE_SECS as f64;
    let scale = target_secs / total_weight;

    weights.iter().map(|w| w * scale).collect()
}

/// Perform an agent session on the livestream (extracted for reuse by queue).
async fn perform_agent_session(state: AppState, agent_name: String, text: String) {
    let writer_id = uuid::Uuid::new_v4().to_string();
    let wid = writer_id.clone();

    // Claim the live slot
    {
        let mut live = state.live_state.write().await;
        live.is_live = true;
        live.writer_id = Some(writer_id.clone());
        live.writer_username = Some(agent_name.clone());
        live.writer_type = Some("agent".to_string());
        live.started_at = Some(std::time::Instant::now());
    }

    // Broadcast that we went live
    let _ = state.live_status_tx.send(LiveStatusEvent::WentLive {
        writer_id: writer_id.clone(),
        writer_username: agent_name.clone(),
        writer_type: "agent".to_string(),
    });
    state.emit_log("INFO", "live", &format!("Agent @{} went live", agent_name));

    let words: Vec<&str> = text.split_whitespace().collect();
    let total_words = words.len();
    let delays = compute_typing_rhythm(&words);
    let mut accumulated = String::new();
    let start_time = std::time::Instant::now();

    state.emit_log(
        "INFO",
        "live",
        &format!(
            "Agent @{}: performing {} words over 8 minutes with rhythm",
            agent_name, total_words
        ),
    );

    for (i, word) in words.iter().enumerate() {
        if !accumulated.is_empty() {
            accumulated.push(' ');
        }
        accumulated.push_str(word);

        let elapsed = start_time.elapsed().as_secs_f64();
        let progress = (elapsed / MAX_LIVE_SECS as f64).min(1.0);
        let word_count = (i + 1) as i64;

        // Update frame
        stream::update_live_frame_typed(
            &state.frame_buffer,
            &agent_name,
            &accumulated,
            word_count,
            elapsed,
            1.0,
            progress,
            "agent",
        )
        .await;

        // Broadcast SSE
        let _ = state.live_text_tx.send(LiveTextEvent {
            content: accumulated.clone(),
            words: word_count,
            elapsed,
            idle_ratio: 1.0,
            progress,
            is_live: true,
            writer_username: Some(agent_name.clone()),
            writer_type: Some("agent".to_string()),
            congrats: None,
        });

        // Wait the rhythm-computed delay for this word
        let delay_secs = delays.get(i).copied().unwrap_or(0.3);
        tokio::time::sleep(std::time::Duration::from_secs_f64(delay_secs)).await;
    }

    // Release slot
    {
        let mut live = state.live_state.write().await;
        if live.writer_id.as_deref() == Some(&wid) {
            live.is_live = false;
            live.writer_id = None;
            live.writer_username = None;
            live.writer_type = None;
            live.started_at = None;
        }
    }

    // Always show congrats — the agent performed for the full 8 minutes
    state.emit_log(
        "INFO",
        "live",
        &format!(
            "Agent @{} completed an anky on stream! ({} words, {:.0}s)",
            agent_name,
            total_words,
            start_time.elapsed().as_secs_f64()
        ),
    );
    {
        state.live_state.write().await.showing_congrats = true;
    }
    stream::set_congrats_frame(&state.frame_buffer, &agent_name).await;
    let _ = state.live_status_tx.send(LiveStatusEvent::Congrats {
        writer_username: agent_name.clone(),
    });
    let _ = state.live_text_tx.send(LiveTextEvent {
        content: String::new(),
        words: 0,
        elapsed: 480.0,
        idle_ratio: 0.0,
        progress: 1.0,
        is_live: false,
        writer_username: Some(agent_name.clone()),
        writer_type: Some("agent".to_string()),
        congrats: Some(true),
    });
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;

    {
        state.live_state.write().await.showing_congrats = false;
    }
    let _ = state.live_status_tx.send(LiveStatusEvent::WentIdle);
    stream::set_idle_frame(&state.frame_buffer).await;

    let _ = state.live_text_tx.send(LiveTextEvent {
        content: String::new(),
        words: 0,
        elapsed: 0.0,
        idle_ratio: 0.0,
        progress: 0.0,
        is_live: false,
        writer_username: None,
        writer_type: None,
        congrats: None,
    });

    // Try next queued writer
    try_next_from_queue(state).await;
}

/// POST /api/v1/live/write — Agent live-writing endpoint.
/// Accepts full text, performs it on the livestream over 8 minutes with natural typing rhythm.
/// If the slot is occupied, the agent gets queued and auto-performs when their turn comes.
pub async fn agent_live_write(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // Authenticate agent via X-API-Key header
    let api_key = match headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        Some(k) => k.to_string(),
        None => return Json(json!({"error": "X-API-Key header required"})),
    };

    let agent = {
        let db = state.db.lock().await;
        queries::get_agent_by_key(&db, &api_key).ok().flatten()
    };

    let agent = match agent {
        Some(a) => a,
        None => return Json(json!({"error": "invalid API key"})),
    };

    let text = match body.get("text").and_then(|t| t.as_str()) {
        Some(t) if !t.trim().is_empty() => t.to_string(),
        _ => return Json(json!({"error": "text field required"})),
    };

    let agent_name = agent.name.clone();

    // Try to claim the live slot
    {
        let live = state.live_state.read().await;
        if live.is_live {
            // Slot occupied — add to queue instead
            let entry = QueueEntry {
                id: uuid::Uuid::new_v4().to_string(),
                username: agent_name.clone(),
                writer_type: "agent".to_string(),
                text: Some(text),
                joined_at: std::time::Instant::now(),
            };
            let mut queue = state.waiting_room.write().await;
            queue.push_back(entry);
            let position = queue.len();
            drop(queue);
            drop(live);

            // Broadcast queue update
            let count = state.waiting_room.read().await.len();
            let _ = state
                .live_status_tx
                .send(LiveStatusEvent::QueueUpdate { count });
            state.emit_log(
                "INFO",
                "live",
                &format!("Agent @{} queued at position {}", agent_name, position),
            );

            return Json(json!({"queued": true, "position": position, "agent": agent_name}));
        }
    }

    // Slot is free — start immediately
    let s = state.clone();
    let name = agent_name.clone();
    tokio::spawn(async move {
        perform_agent_session(s, name, text).await;
    });

    Json(json!({"ok": true, "agent": agent_name}))
}

/// Public entry point for the watchdog to trigger queue processing after force-resetting a stale session.
pub async fn try_next_from_queue_public(state: AppState) {
    try_next_from_queue_inner(state).await;
}

/// Try to start the next queued writer after a session ends.
fn try_next_from_queue(
    state: AppState,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(async move { try_next_from_queue_inner(state).await })
}

async fn try_next_from_queue_inner(state: AppState) {
    loop {
        let entry = {
            let mut queue = state.waiting_room.write().await;
            queue.pop_front()
        };

        let entry = match entry {
            Some(e) => e,
            None => return, // queue empty
        };

        // Broadcast updated queue count
        let count = state.waiting_room.read().await.len();
        let _ = state
            .live_status_tx
            .send(LiveStatusEvent::QueueUpdate { count });

        if entry.writer_type == "agent" {
            if let Some(text) = entry.text {
                state.emit_log(
                    "INFO",
                    "live",
                    &format!("Queue: starting agent @{}", entry.username),
                );
                let s = state.clone();
                tokio::spawn(async move {
                    perform_agent_session(s, entry.username, text).await;
                });
                return; // agent session will call try_next_from_queue when done
            }
            // Agent entry with no text — skip
            continue;
        }

        // Human turn — broadcast YourTurn and wait 30s for them to claim
        state.emit_log(
            "INFO",
            "live",
            &format!("Queue: it's @{}'s turn", entry.username),
        );
        let _ = state.live_status_tx.send(LiveStatusEvent::YourTurn {
            writer_username: entry.username.clone(),
        });

        // Wait 30 seconds for the human to connect via WebSocket
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        // Check if someone went live during those 30 seconds
        let is_live = state.live_state.read().await.is_live;
        if is_live {
            // They claimed it (or someone did) — done
            return;
        }

        // Nobody claimed — skip to next in queue
        state.emit_log(
            "INFO",
            "live",
            &format!("Queue: @{} didn't claim in 30s, skipping", entry.username),
        );
        continue;
    }
}

/// POST /api/live/queue — Join the waiting room queue.
pub async fn join_queue(jar: CookieJar, State(state): State<AppState>) -> Json<serde_json::Value> {
    let user = auth::get_auth_user(&state, &jar).await;
    let username = match user.as_ref().and_then(|u| u.username.clone()) {
        Some(u) => u,
        None => return Json(json!({"error": "login required — set a username in /settings"})),
    };

    // Check if already in queue
    {
        let queue = state.waiting_room.read().await;
        if queue.iter().any(|e| e.username == username) {
            let pos = queue.iter().position(|e| e.username == username).unwrap() + 1;
            return Json(json!({"ok": true, "position": pos, "already_queued": true}));
        }
    }

    let entry = QueueEntry {
        id: uuid::Uuid::new_v4().to_string(),
        username: username.clone(),
        writer_type: "human".to_string(),
        text: None,
        joined_at: std::time::Instant::now(),
    };

    let position = {
        let mut queue = state.waiting_room.write().await;
        queue.push_back(entry);
        queue.len()
    };

    // Broadcast queue update
    let count = state.waiting_room.read().await.len();
    let _ = state
        .live_status_tx
        .send(LiveStatusEvent::QueueUpdate { count });
    state.emit_log(
        "INFO",
        "live",
        &format!("@{} joined waiting room (position {})", username, position),
    );

    Json(json!({"ok": true, "position": position}))
}

/// DELETE /api/live/queue — Leave the waiting room queue.
pub async fn leave_queue(jar: CookieJar, State(state): State<AppState>) -> Json<serde_json::Value> {
    let user = auth::get_auth_user(&state, &jar).await;
    let username = match user.as_ref().and_then(|u| u.username.clone()) {
        Some(u) => u,
        None => return Json(json!({"error": "login required"})),
    };

    let removed = {
        let mut queue = state.waiting_room.write().await;
        let before = queue.len();
        queue.retain(|e| e.username != username);
        queue.len() < before
    };

    if removed {
        let count = state.waiting_room.read().await.len();
        let _ = state
            .live_status_tx
            .send(LiveStatusEvent::QueueUpdate { count });
        state.emit_log("INFO", "live", &format!("@{} left waiting room", username));
    }

    Json(json!({"ok": true, "removed": removed}))
}

/// GET /api/live-status — SSE stream of live status changes.
pub async fn live_status_sse(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.live_status_tx.subscribe();

    // Send current status immediately
    let (initial_live, initial_username, initial_type) = {
        let live = state.live_state.read().await;
        (
            live.is_live,
            live.writer_username.clone(),
            live.writer_type.clone(),
        )
    };
    let initial_queue_count = state.waiting_room.read().await.len();

    let stream = async_stream::stream! {
        // Initial status
        yield Ok::<_, Infallible>(
            Event::default().data(json!({
                "is_live": initial_live,
                "writer_username": initial_username,
                "writer_type": initial_type,
                "queue_count": initial_queue_count,
            }).to_string())
        );

        loop {
            match rx.recv().await {
                Ok(LiveStatusEvent::WentLive { writer_id, writer_username, writer_type }) => {
                    yield Ok(Event::default().data(
                        json!({"is_live": true, "writer_id": writer_id, "writer_username": writer_username, "writer_type": writer_type}).to_string()
                    ));
                }
                Ok(LiveStatusEvent::Congrats { writer_username }) => {
                    yield Ok(Event::default().data(
                        json!({"congrats": true, "writer_username": writer_username, "is_live": false}).to_string()
                    ));
                }
                Ok(LiveStatusEvent::WentIdle) => {
                    yield Ok(Event::default().data(
                        json!({"is_live": false}).to_string()
                    ));
                }
                Ok(LiveStatusEvent::QueueUpdate { count }) => {
                    yield Ok(Event::default().data(
                        json!({"queue_count": count}).to_string()
                    ));
                }
                Ok(LiveStatusEvent::YourTurn { writer_username }) => {
                    yield Ok(Event::default().data(
                        json!({"your_turn": true, "writer_username": writer_username}).to_string()
                    ));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// GET /api/stream-text — SSE stream of live writing text for overlay.
pub async fn stream_text_sse(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.live_text_tx.subscribe();

    // Send current status immediately
    let (initial_live, initial_username) = {
        let live = state.live_state.read().await;
        (live.is_live, live.writer_username.clone())
    };

    let stream = async_stream::stream! {
        // Initial state
        yield Ok::<_, Infallible>(
            Event::default().data(json!({
                "is_live": initial_live,
                "content": "",
                "words": 0,
                "elapsed": 0.0,
                "idle_ratio": if initial_live { 1.0 } else { 0.0 },
                "progress": 0.0,
                "writer_username": initial_username,
            }).to_string())
        );

        loop {
            match rx.recv().await {
                Ok(evt) => {
                    yield Ok(Event::default().data(
                        serde_json::to_string(&evt).unwrap_or_default()
                    ));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(5))
            .text("keep-alive"),
    )
}

/// GET /api/ankys/today — JSON list of today's completed ankys with images.
pub async fn todays_ankys(State(state): State<AppState>) -> Json<serde_json::Value> {
    let ankys = {
        let db = state.db.lock().await;
        crate::db::queries::get_todays_ankys(&db).unwrap_or_default()
    };

    let count = ankys.len();
    let items: Vec<serde_json::Value> = ankys
        .into_iter()
        .map(|a| {
            json!({
                "id": a.id,
                "title": a.title,
                "image_url": a.image_path.unwrap_or_default(),
            })
        })
        .collect();

    Json(json!({
        "ankys": items,
        "count": count,
    }))
}

/// GET /api/live-check — JSON check if someone is currently live.
pub async fn live_check(State(state): State<AppState>) -> Json<serde_json::Value> {
    let live = state.live_state.read().await;
    let queue_count = state.waiting_room.read().await.len();
    Json(json!({
        "is_live": live.is_live,
        "writer_username": live.writer_username,
        "writer_type": live.writer_type,
        "queue_count": queue_count,
    }))
}
