use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::response::Html;
use serde::Serialize;

const JPFRANETO_ID: &str = "1430539235480719367";

#[derive(Serialize)]
struct EvolutionTrace {
    tweet_id: String,
    author: String,
    text: String,
    status: String,
    classification: Option<String>,
    tag: Option<String>,
    extracted_content: Option<String>,
    result_text: Option<String>,
    error_message: Option<String>,
    reply_tweet_id: Option<String>,
    parent_tweet_id: Option<String>,
    source: Option<String>,
    created_at: String,
    updated_at: Option<String>,
    task_id: Option<String>,
    task_status: Option<String>,
    task_summary: Option<String>,
    task_completed_at: Option<String>,
}

pub async fn evolve_dashboard(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let traces: Vec<EvolutionTrace> = {
        let db = state.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT
                xi.tweet_id,
                COALESCE(NULLIF(xi.x_username, ''), xi.x_user_id, 'unknown') AS author,
                COALESCE(xi.tweet_text, ''),
                xi.status,
                xi.classification,
                xi.tag,
                xi.extracted_content,
                xi.result_text,
                xi.error_message,
                xi.reply_tweet_id,
                xi.parent_tweet_id,
                xi.source,
                xi.created_at,
                xi.updated_at,
                xt.id,
                xt.status,
                xt.summary,
                xt.completed_at
             FROM x_interactions xi
             LEFT JOIN x_evolution_tasks xt ON xt.tweet_id = xi.tweet_id
             WHERE xi.x_user_id = ?1 OR lower(COALESCE(xi.x_username, '')) = 'jpfraneto'
             ORDER BY COALESCE(xi.updated_at, xi.created_at) DESC
             LIMIT 100",
        )?;
        let rows = stmt.query_map([JPFRANETO_ID], |row| {
            Ok(EvolutionTrace {
                tweet_id: row.get(0)?,
                author: row.get(1)?,
                text: row.get(2)?,
                status: row.get(3)?,
                classification: row.get(4)?,
                tag: row.get(5)?,
                extracted_content: row.get(6)?,
                result_text: row.get(7)?,
                error_message: row.get(8)?,
                reply_tweet_id: row.get(9)?,
                parent_tweet_id: row.get(10)?,
                source: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
                task_id: row.get(14)?,
                task_status: row.get(15)?,
                task_summary: row.get(16)?,
                task_completed_at: row.get(17)?,
            })
        })?;

        let mut traces = Vec::new();
        for row in rows {
            traces.push(row?);
        }
        traces
    };

    let tagged_count = traces.iter().filter(|t| t.tag.is_some()).count();
    let running_count = traces
        .iter()
        .filter(|t| {
            t.status.contains("running") || t.status == "received" || t.status == "classified"
        })
        .count();
    let error_count = traces
        .iter()
        .filter(|t| t.status.contains("error") || t.error_message.is_some())
        .count();

    let mut ctx = tera::Context::new();
    ctx.insert("traces", &traces);
    ctx.insert("trace_count", &traces.len());
    ctx.insert("tagged_count", &tagged_count);
    ctx.insert("running_count", &running_count);
    ctx.insert("error_count", &error_count);
    let html = state.tera.render("evolve.html", &ctx)?;
    Ok(Html(html))
}
