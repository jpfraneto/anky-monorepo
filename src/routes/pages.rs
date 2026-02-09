use crate::error::AppError;
use crate::state::{AppState, GpuStatus};
use axum::extract::{Path, State};
use axum::response::Html;

pub async fn home(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    // If training, redirect to sleeping page
    {
        let gpu = state.gpu_status.read().await;
        if matches!(*gpu, GpuStatus::Training { .. }) {
            let mut ctx = tera::Context::new();
            ctx.insert("gpu_status", &gpu.to_string());
            let html = state.tera.render("sleeping.html", &ctx)?;
            return Ok(Html(html));
        }
    }

    let ctx = tera::Context::new();
    let html = state.tera.render("home.html", &ctx)?;
    Ok(Html(html))
}

pub async fn gallery(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ankys = {
        let db = state.db.lock().await;
        crate::db::queries::get_all_ankys(&db)?
    };

    let mut ctx = tera::Context::new();
    ctx.insert("ankys", &serde_json::to_value(
        ankys.iter().map(|a| {
            serde_json::json!({
                "id": a.id,
                "title": a.title.as_deref().unwrap_or("untitled"),
                "image_path": a.image_path.as_deref().unwrap_or(""),
                "thinker_name": a.thinker_name.as_deref().unwrap_or(""),
                "status": a.status,
                "created_at": a.created_at,
            })
        }).collect::<Vec<_>>()
    ).unwrap_or_default());

    let html = state.tera.render("gallery.html", &ctx)?;
    Ok(Html(html))
}

pub async fn help(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("help.html", &ctx)?;
    Ok(Html(html))
}

pub async fn generate(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("generate.html", &ctx)?;
    Ok(Html(html))
}

pub async fn sleeping(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let gpu = state.gpu_status.read().await;
    let mut ctx = tera::Context::new();
    ctx.insert("gpu_status", &gpu.to_string());
    let html = state.tera.render("sleeping.html", &ctx)?;
    Ok(Html(html))
}

pub async fn feedback(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let entries = {
        let db = state.db.lock().await;
        crate::db::queries::get_all_feedback(&db)?
    };

    let mut ctx = tera::Context::new();
    ctx.insert("entries", &serde_json::to_value(
        entries.iter().map(|f| {
            serde_json::json!({
                "id": f.id,
                "source": f.source,
                "author": f.author.as_deref().unwrap_or("anonymous"),
                "content": f.content,
                "status": f.status,
                "created_at": f.created_at,
            })
        }).collect::<Vec<_>>()
    ).unwrap_or_default());

    let html = state.tera.render("feedback.html", &ctx)?;
    Ok(Html(html))
}

pub async fn anky_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Html<String>, AppError> {
    let anky = {
        let db = state.db.lock().await;
        crate::db::queries::get_anky_by_id(&db, &id)?
    };

    let anky = anky.ok_or_else(|| AppError::NotFound("anky not found".into()))?;

    let mut ctx = tera::Context::new();
    ctx.insert("id", &anky.id);
    ctx.insert("title", &anky.title.as_deref().unwrap_or("untitled"));
    ctx.insert("image_path", &anky.image_path.as_deref().unwrap_or(""));
    ctx.insert("reflection", &anky.reflection.as_deref().unwrap_or(""));
    ctx.insert("image_prompt", &anky.image_prompt.as_deref().unwrap_or(""));
    ctx.insert("thinker_name", &anky.thinker_name.as_deref().unwrap_or(""));
    ctx.insert("thinker_moment", &anky.thinker_moment.as_deref().unwrap_or(""));
    ctx.insert("status", &anky.status);
    ctx.insert("writing", &anky.writing_text.as_deref().unwrap_or(""));
    ctx.insert("created_at", &anky.created_at);

    let html = state.tera.render("anky.html", &ctx)?;
    Ok(Html(html))
}
