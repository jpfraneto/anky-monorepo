use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::response::Html;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use std::convert::Infallible;

pub async fn poiesis_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let gpu_status = state.gpu_status.read().await;
    let total_cost = {
        let db = state.db.lock().await;
        crate::db::queries::get_total_cost(&db).unwrap_or(0.0)
    };

    let mut ctx = tera::Context::new();
    ctx.insert("gpu_status", &gpu_status.to_string());
    ctx.insert("total_cost", &format!("{:.4}", total_cost));

    let html = state.tera.render("poiesis.html", &ctx)?;
    Ok(Html(html))
}

pub async fn poiesis_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.log_tx.subscribe();

    let stream = async_stream::stream! {
        // Send initial connection event
        yield Ok(Event::default().data("[connected to poiesis stream]"));

        loop {
            match rx.recv().await {
                Ok(entry) => {
                    let data = entry.to_sse_data();
                    yield Ok(Event::default().event("log").data(data));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    yield Ok(Event::default().data(format!("[skipped {} messages]", n)));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
