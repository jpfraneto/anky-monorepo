use crate::error::AppError;
use crate::state::{AppState, GpuStatus};
use axum::extract::State;
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
