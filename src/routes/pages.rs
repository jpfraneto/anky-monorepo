use crate::error::AppError;
use crate::state::{AppState, GpuStatus};
use axum::extract::{Path, Query, State};
use axum::response::Html;
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct GalleryQuery {
    pub tab: Option<String>,
}

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

pub async fn gallery(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<GalleryQuery>,
) -> Result<Html<String>, AppError> {
    let user_id = jar.get("anky_user_id").map(|c| c.value().to_string());
    let has_user = user_id.is_some();

    let tab = match query.tab.as_deref() {
        Some("mine") if has_user => "mine",
        Some("viewed") if has_user => "viewed",
        _ => "all",
    };

    let ankys = {
        let db = state.db.lock().await;
        match tab {
            "mine" => crate::db::queries::get_user_ankys(&db, user_id.as_deref().unwrap())?,
            "viewed" => crate::db::queries::get_user_viewed_ankys(&db, user_id.as_deref().unwrap())?,
            _ => {
                // "all" tab: only show non-written ankys (writing sessions are private)
                let all = crate::db::queries::get_all_complete_ankys(&db)?;
                all.into_iter().filter(|a| a.origin != "written").collect()
            }
        }
    };

    let mut ctx = tera::Context::new();
    ctx.insert("active_tab", tab);
    ctx.insert("has_user", &has_user);
    ctx.insert("ankys", &serde_json::to_value(
        ankys.iter().map(|a| {
            serde_json::json!({
                "id": a.id,
                "title": a.title.as_deref().unwrap_or("untitled"),
                "image_path": a.image_path.as_deref().unwrap_or(""),
                "image_webp": a.image_webp.as_deref().unwrap_or(""),
                "image_prompt": a.image_prompt.as_deref().unwrap_or(""),
                "reflection": a.reflection.as_deref().unwrap_or(""),
                "thinker_name": a.thinker_name.as_deref().unwrap_or(""),
                "status": a.status,
                "created_at": a.created_at,
                "origin": a.origin,
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

pub async fn ankycoin_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("ankycoin.html", &ctx)?;
    Ok(Html(html))
}

pub async fn login_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("login.html", &ctx)?;
    Ok(Html(html))
}

pub async fn generate_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
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

pub async fn video_dashboard(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("video.html", &ctx)?;
    Ok(Html(html))
}

pub async fn changelog(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("changelog.html", &ctx)?;
    Ok(Html(html))
}

pub async fn anky_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<Html<String>, AppError> {
    let viewer_id = jar.get("anky_user_id").map(|c| c.value().to_string());

    let anky = {
        let db = state.db.lock().await;
        crate::db::queries::get_anky_by_id(&db, &id)?
    };

    let anky = anky.ok_or_else(|| AppError::NotFound("anky not found".into()))?;

    // Always collect when a logged-in user views any anky (tracks views)
    if let Some(ref vid) = viewer_id {
        let db = state.db.lock().await;
        let _ = crate::db::queries::collect_anky(&db, vid, &id);
    }

    // Determine if the viewer can see the writing text
    // Having the link means the writer shared it — show everything
    let show_writing = anky.origin != "generated";

    let mut ctx = tera::Context::new();
    ctx.insert("id", &anky.id);
    ctx.insert("title", &anky.title.as_deref().unwrap_or("untitled"));
    ctx.insert("image_path", &anky.image_path.as_deref().unwrap_or(""));
    ctx.insert("image_webp", &anky.image_webp.as_deref().unwrap_or(""));
    ctx.insert("reflection", &anky.reflection.as_deref().unwrap_or(""));
    ctx.insert("image_prompt", &anky.image_prompt.as_deref().unwrap_or(""));
    ctx.insert("thinker_name", &anky.thinker_name.as_deref().unwrap_or(""));
    ctx.insert("thinker_moment", &anky.thinker_moment.as_deref().unwrap_or(""));
    ctx.insert("status", &anky.status);
    ctx.insert("created_at", &anky.created_at);
    ctx.insert("origin", &anky.origin);

    if show_writing {
        ctx.insert("writing", &anky.writing_text.as_deref().unwrap_or(""));
    } else {
        ctx.insert("writing", &"");
    }

    let html = state.tera.render("anky.html", &ctx)?;
    Ok(Html(html))
}
