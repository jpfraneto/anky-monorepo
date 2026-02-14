use crate::services::stream;
use crate::state::{AppState, LiveStatusEvent};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use std::convert::Infallible;

/// GET /ws/live — WebSocket handler for GO LIVE.
/// Only one writer at a time. Sends text updates, server writes to file for ffmpeg.
pub async fn ws_live(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_live_socket(socket, state))
}

async fn handle_live_socket(mut socket: WebSocket, state: AppState) {
    let writer_id = uuid::Uuid::new_v4().to_string();

    // Try to claim the live slot
    {
        let mut live = state.live_state.write().await;
        if live.is_live {
            // Slot occupied — reject
            let _ = socket
                .send(Message::Text(
                    json!({"type":"error","message":"slot occupied"}).to_string().into(),
                ))
                .await;
            drop(socket);
            return;
        }
        live.is_live = true;
        live.writer_id = Some(writer_id.clone());
    }

    // Broadcast that we went live
    let _ = state.live_status_tx.send(LiveStatusEvent::WentLive {
        writer_id: writer_id.clone(),
    });
    state.emit_log("INFO", "live", "Writer went live on pump.fun stream");

    // Clear text for new session
    stream::write_live_text("live now\n\nwriting in progress...");

    // Confirm to client
    let _ = socket
        .send(Message::Text(
            json!({"type":"live","writer_id":writer_id}).to_string().into(),
        ))
        .await;

    // Read messages from client
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    if parsed.get("type").and_then(|t| t.as_str()) == Some("text") {
                        if let Some(content) = parsed.get("content").and_then(|c| c.as_str()) {
                            let words = parsed.get("words").and_then(|v| v.as_i64()).unwrap_or(0);
                            let elapsed = parsed.get("elapsed").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let idle_ratio = parsed.get("idle_ratio").and_then(|v| v.as_f64()).unwrap_or(1.0);
                            let progress = parsed.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            stream::write_live_frame(content, words, elapsed, idle_ratio, progress);
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    // Writer disconnected — release the slot
    {
        let mut live = state.live_state.write().await;
        if live.writer_id.as_deref() == Some(&writer_id) {
            live.is_live = false;
            live.writer_id = None;
        }
    }

    let _ = state.live_status_tx.send(LiveStatusEvent::WentIdle);
    state.emit_log("INFO", "live", "Writer ended live session");
    stream::write_idle_text();
}

/// GET /api/live-status — SSE stream of live status changes.
pub async fn live_status_sse(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.live_status_tx.subscribe();

    // Send current status immediately
    let initial_live = {
        let live = state.live_state.read().await;
        live.is_live
    };

    let stream = async_stream::stream! {
        // Initial status
        yield Ok::<_, Infallible>(
            Event::default().data(json!({"is_live": initial_live}).to_string())
        );

        loop {
            match rx.recv().await {
                Ok(LiveStatusEvent::WentLive { writer_id }) => {
                    yield Ok(Event::default().data(
                        json!({"is_live": true, "writer_id": writer_id}).to_string()
                    ));
                }
                Ok(LiveStatusEvent::WentIdle) => {
                    yield Ok(Event::default().data(
                        json!({"is_live": false}).to_string()
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

/// GET /api/live-check — JSON check if someone is currently live.
pub async fn live_check(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let live = state.live_state.read().await;
    Json(json!({
        "is_live": live.is_live,
    }))
}
