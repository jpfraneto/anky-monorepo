use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::response::{Html, Json};
use futures::stream::Stream;
use std::convert::Infallible;

pub async fn dashboard(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let gpu = state.gpu_status.read().await;

    // Load recent summaries for initial render
    let summaries: Vec<serde_json::Value> = {
        let db = state.db.lock().await;
        let mut stmt = db
            .prepare(
                "SELECT id, created_at, period_start, period_end, summary
             FROM system_summaries
             ORDER BY created_at DESC
             LIMIT 20",
            )
            .unwrap_or_else(|_| panic!("summaries query"));
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "created_at": row.get::<_, String>(1)?,
                    "period_start": row.get::<_, String>(2)?,
                    "period_end": row.get::<_, String>(3)?,
                    "summary": row.get::<_, String>(4)?,
                }))
            })
            .unwrap_or_else(|_| panic!("summaries query_map"));
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut ctx = tera::Context::new();
    ctx.insert("gpu_status", &gpu.to_string());
    ctx.insert("summaries", &summaries);
    let html = state.tera.render("dashboard.html", &ctx)?;
    Ok(Html(html))
}

pub async fn dashboard_logs(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.log_tx.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(entry) => {
                    let data = entry.to_sse_data();
                    yield Ok(Event::default().data(data));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    let msg = format!("[...skipped {} messages...]", n);
                    yield Ok(Event::default().data(msg));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    )
}

/// GET /dashboard/summaries — return recent system summaries as JSON
pub async fn dashboard_summaries(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let summaries: Vec<serde_json::Value> = {
        let db = state.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, created_at, period_start, period_end, summary
             FROM system_summaries
             ORDER BY created_at DESC
             LIMIT 20",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "created_at": row.get::<_, String>(1)?,
                "period_start": row.get::<_, String>(2)?,
                "period_end": row.get::<_, String>(3)?,
                "summary": row.get::<_, String>(4)?,
            }))
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };
    Ok(Json(summaries))
}

/// Generate a 30-minute system summary. Called by the background worker.
pub async fn generate_system_summary(state: &AppState) {
    let now = chrono::Utc::now();
    let thirty_min_ago = now - chrono::Duration::minutes(30);
    let period_start = thirty_min_ago.format("%Y-%m-%d %H:%M:%S").to_string();
    let period_end = now.format("%Y-%m-%d %H:%M:%S").to_string();

    // 1. Gather DB activity stats for the last 30 minutes
    let stats = {
        let db = state.db.lock().await;

        let writings: i32 = db
            .query_row(
                "SELECT COUNT(*) FROM writing_sessions WHERE created_at >= ?1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let ankys: i32 = db
            .query_row(
                "SELECT COUNT(*) FROM writing_sessions WHERE created_at >= ?1 AND is_anky = 1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let anky_images: i32 = db
            .query_row(
                "SELECT COUNT(*) FROM ankys WHERE created_at >= ?1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let meditations: i32 = db
            .query_row(
                "SELECT COUNT(*) FROM personalized_meditations WHERE created_at >= ?1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let breathwork: i32 = db
            .query_row(
                "SELECT COUNT(*) FROM personalized_breathwork WHERE created_at >= ?1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let cuentacuentos: i32 = db
            .query_row(
                "SELECT COUNT(*) FROM cuentacuentos WHERE created_at >= ?1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let new_users: i32 = db
            .query_row(
                "SELECT COUNT(*) FROM users WHERE created_at >= ?1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let unique_writers: i32 = db
            .query_row(
                "SELECT COUNT(DISTINCT user_id) FROM writing_sessions WHERE created_at >= ?1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let total_words: i64 = db
            .query_row(
                "SELECT COALESCE(SUM(word_count), 0) FROM writing_sessions WHERE created_at >= ?1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let failed_ankys: i32 = db
            .query_row(
                "SELECT COUNT(*) FROM ankys WHERE status = 'failed' AND created_at >= ?1",
                rusqlite::params![period_start],
                |row| row.get(0),
            )
            .unwrap_or(0);

        serde_json::json!({
            "writings": writings,
            "ankys": ankys,
            "anky_images": anky_images,
            "meditations": meditations,
            "breathwork": breathwork,
            "cuentacuentos": cuentacuentos,
            "new_users": new_users,
            "unique_writers": unique_writers,
            "total_words": total_words,
            "failed_ankys": failed_ankys,
        })
    };

    // 2. Drain recent log entries from the ring buffer
    let recent_logs: Vec<String> = {
        let mut history = state.log_history.lock().await;
        let cutoff = thirty_min_ago;
        // Take only entries from the last 30 minutes
        let logs: Vec<String> = history
            .iter()
            .filter(|e| e.timestamp >= cutoff)
            .map(|e| e.to_sse_data())
            .collect();
        // Remove entries older than 30 minutes
        while history.front().map_or(false, |e| e.timestamp < cutoff) {
            history.pop_front();
        }
        logs
    };

    // 3. Build a compact log digest (take first 100 + last 50 if too many)
    let log_digest = if recent_logs.len() <= 150 {
        recent_logs.join("\n")
    } else {
        let first: Vec<&str> = recent_logs.iter().take(100).map(|s| s.as_str()).collect();
        let last: Vec<&str> = recent_logs
            .iter()
            .skip(recent_logs.len() - 50)
            .map(|s| s.as_str())
            .collect();
        format!(
            "{}\n[...{} log lines omitted...]\n{}",
            first.join("\n"),
            recent_logs.len() - 150,
            last.join("\n")
        )
    };

    let raw_stats = serde_json::to_string_pretty(&stats).unwrap_or_default();

    // 4. Build summary with Ollama
    let prompt = format!(
        r#"You are Anky's system observer. Write a concise summary (3-8 sentences) of what happened in the last 30 minutes. Be specific about numbers. Note anything unusual (errors, failures, zero activity). If nothing happened, say so directly.

Activity stats ({} to {}):
{}

Recent log entries ({} total):
{}"#,
        period_start,
        period_end,
        raw_stats,
        recent_logs.len(),
        log_digest
    );

    let summary = match crate::services::ollama::call_ollama(
        &state.config.ollama_base_url,
        &state.config.ollama_model,
        &prompt,
    )
    .await
    {
        Ok(text) => text.trim().to_string(),
        Err(e) => {
            tracing::warn!("System summary generation failed: {}", e);
            // Fall back to a raw stats summary
            format!(
                "Summary generation failed. Raw stats: {} writings, {} ankys, {} words, {} unique writers, {} new users.",
                stats["writings"], stats["ankys"], stats["total_words"],
                stats["unique_writers"], stats["new_users"]
            )
        }
    };

    // 5. Store in DB
    let id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().await;
        if let Err(e) = db.execute(
            "INSERT INTO system_summaries (id, period_start, period_end, raw_stats, summary)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, period_start, period_end, raw_stats, summary],
        ) {
            tracing::error!("Failed to store system summary: {}", e);
            return;
        }
    }

    // 6. Broadcast to dashboard
    state.emit_log(
        "INFO",
        "summary",
        &format!("30-minute summary: {}", &summary[..100.min(summary.len())]),
    );

    tracing::info!(
        "System summary generated for {} to {}",
        period_start,
        period_end
    );
}
