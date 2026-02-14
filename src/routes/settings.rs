use crate::db::queries;
use crate::error::AppError;
use crate::routes::auth;
use crate::state::AppState;
use axum::extract::State;
use axum::response::Html;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde_json::json;

/// GET /settings — settings page (requires auth)
pub async fn settings_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let user = auth::get_auth_user(&state, &jar).await
        .ok_or_else(|| AppError::BadRequest("login required — connect your wallet first".into()))?;

    let (settings, username) = {
        let db = state.db.lock().await;
        let settings = queries::get_user_settings(&db, &user.user_id)?;
        let username = queries::get_user_username(&db, &user.user_id)?;
        (settings, username)
    };

    let display_username = username
        .or(user.username.clone())
        .unwrap_or_default();

    let mut ctx = tera::Context::new();
    ctx.insert("username", &display_username);
    ctx.insert("font_family", &settings.font_family);
    ctx.insert("font_size", &settings.font_size);
    ctx.insert("theme", &settings.theme);
    ctx.insert("idle_timeout", &settings.idle_timeout);
    ctx.insert("logged_in", &true);

    let html = state.tera.render("settings.html", &ctx)?;
    Ok(Html(html))
}

#[derive(serde::Deserialize)]
pub struct SaveSettingsRequest {
    pub username: Option<String>,
    pub font_family: String,
    pub font_size: i32,
    pub theme: String,
    pub idle_timeout: i32,
}

/// POST /api/settings — save user settings
pub async fn save_settings(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<SaveSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = auth::get_auth_user(&state, &jar).await
        .ok_or_else(|| AppError::BadRequest("login required".into()))?;

    // Validate font_family
    let font_family = match req.font_family.as_str() {
        "monospace" | "serif" | "sans-serif" => req.font_family.clone(),
        _ => "monospace".to_string(),
    };

    // Validate font_size (14-28)
    let font_size = req.font_size.clamp(14, 28);

    // Validate theme
    let theme = match req.theme.as_str() {
        "dark" | "light" => req.theme.clone(),
        _ => "dark".to_string(),
    };

    // Validate idle_timeout (5-15)
    let idle_timeout = req.idle_timeout.clamp(5, 15);

    {
        let db = state.db.lock().await;

        // Update username if provided
        if let Some(ref uname) = req.username {
            let uname = uname.trim().to_lowercase();
            if !uname.is_empty() {
                // Validate: alphanumeric + underscore, 3-20 chars
                if uname.len() < 3 || uname.len() > 20 {
                    return Err(AppError::BadRequest("username must be 3-20 characters".into()));
                }
                if !uname.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Err(AppError::BadRequest("username must be alphanumeric (a-z, 0-9, _)".into()));
                }
                if !queries::check_username_available(&db, &uname, &user.user_id)? {
                    return Err(AppError::BadRequest("username already taken".into()));
                }
                queries::set_username(&db, &user.user_id, &uname)?;
            }
        }

        queries::upsert_user_settings(
            &db,
            &user.user_id,
            &font_family,
            font_size,
            &theme,
            idle_timeout,
        )?;
    }

    Ok(Json(json!({ "saved": true })))
}
