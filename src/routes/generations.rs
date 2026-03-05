use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::Html;
use std::path::PathBuf;

/// GET /generations/:id/dashboard — live generation dashboard
pub async fn generation_dashboard(
    State(state): State<AppState>,
    Path(batch_id): Path<String>,
) -> Result<Html<String>, AppError> {
    let prompts_path = PathBuf::from(format!("data/generations/{}/prompts.json", batch_id));
    if !prompts_path.exists() {
        return Err(AppError::NotFound(format!("Batch '{}' not found", batch_id)));
    }
    let mut ctx = tera::Context::new();
    ctx.insert("batch_id", &batch_id);
    let html = state
        .tera
        .render("generations_dashboard.html", &ctx)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Html(html))
}

/// GET /generations/:id/progress — returns progress.json for the batch
pub async fn generation_progress(
    Path(batch_id): Path<String>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let progress_path = PathBuf::from(format!("data/generations/{}/progress.json", batch_id));
    if !progress_path.exists() {
        return Ok(axum::Json(serde_json::json!({})));
    }
    let raw = std::fs::read_to_string(&progress_path)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let val: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(axum::Json(val))
}

/// GET /generations — list all batches
pub async fn list_batches(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let dir = PathBuf::from("data/generations");

    let mut batches: Vec<(String, usize, String)> = Vec::new(); // (id, count, created)

    if dir.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        entries.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        entries.reverse();

        for entry in entries {
            let id = entry.file_name().to_string_lossy().to_string();
            let prompts_path = entry.path().join("prompts.json");
            let count = if prompts_path.exists() {
                let raw = std::fs::read_to_string(&prompts_path).unwrap_or_default();
                serde_json::from_str::<Vec<String>>(&raw)
                    .map(|v| v.len())
                    .unwrap_or(0)
            } else {
                0
            };
            let created = id
                .strip_prefix("batch-")
                .unwrap_or(&id)
                .replace('-', " ")
                .to_string();
            batches.push((id, count, created));
        }
    }

    let rows: String = if batches.is_empty() {
        "<p style='color:#666'>No batches yet.</p>".to_string()
    } else {
        batches
            .iter()
            .map(|(id, count, created)| {
                format!(
                    r#"<a class="batch-row" href="/generations/{id}">
                      <span class="batch-id">{id}</span>
                      <span class="batch-meta">{count} prompts &middot; {created}</span>
                    </a>"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let html = state
        .tera
        .render(
            "generations_list.html",
            &tera::Context::from_value(serde_json::json!({ "rows": rows })).unwrap(),
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Html(html))
}

/// GET /generations/:id — review a prompt batch
pub async fn review_batch(
    State(state): State<AppState>,
    Path(batch_id): Path<String>,
) -> Result<Html<String>, AppError> {
    let prompts_path = PathBuf::from(format!("data/generations/{}/prompts.json", batch_id));

    if !prompts_path.exists() {
        return Err(AppError::NotFound(format!("Batch '{}' not found", batch_id)));
    }

    let raw = std::fs::read_to_string(&prompts_path)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let prompts: Vec<String> =
        serde_json::from_str(&raw).map_err(|e| AppError::Internal(e.to_string()))?;

    // Check if a status file exists (tracks kept/skipped decisions)
    let status_path = PathBuf::from(format!("data/generations/{}/status.json", batch_id));
    let status: std::collections::HashMap<usize, String> = if status_path.exists() {
        let s = std::fs::read_to_string(&status_path).unwrap_or_default();
        serde_json::from_str(&s).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };

    let kept = status.values().filter(|v| v.as_str() == "keep").count();
    let skipped = status.values().filter(|v| v.as_str() == "skip").count();

    // Build prompts JSON for the template
    let prompts_json = serde_json::to_string(&prompts).unwrap_or_default();
    let status_json = serde_json::to_string(&status).unwrap_or_default();

    let mut ctx = tera::Context::new();
    ctx.insert("batch_id", &batch_id);
    ctx.insert("prompts_json", &prompts_json);
    ctx.insert("status_json", &status_json);
    ctx.insert("total", &prompts.len());
    ctx.insert("kept", &kept);
    ctx.insert("skipped", &skipped);

    let html = state
        .tera
        .render("generations_review.html", &ctx)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Html(html))
}

/// GET /generations/:id/tinder — keyboard-driven approve/reject review of generated images
pub async fn review_images(
    State(state): State<AppState>,
    Path(batch_id): Path<String>,
) -> Result<Html<String>, AppError> {
    let progress_path = PathBuf::from(format!("data/generations/{}/progress.json", batch_id));
    if !progress_path.exists() {
        return Err(AppError::NotFound(format!("Batch '{}' not found", batch_id)));
    }

    let raw = std::fs::read_to_string(&progress_path)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let progress: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| AppError::Internal(e.to_string()))?;

    // Build list of done images: [{image_id, prompt, caption}]
    let mut images: Vec<serde_json::Value> = Vec::new();
    if let Some(obj) = progress.as_object() {
        let mut entries: Vec<_> = obj.iter().collect();
        entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(0));
        for (_, v) in entries {
            if v.get("status").and_then(|s| s.as_str()) == Some("done") {
                if let Some(image_id) = v.get("image_id").and_then(|i| i.as_str()) {
                    images.push(serde_json::json!({
                        "image_id": image_id,
                        "prompt": v.get("prompt").and_then(|p| p.as_str()).unwrap_or(""),
                        "caption": v.get("caption").and_then(|c| c.as_str()).unwrap_or(""),
                    }));
                }
            }
        }
    }

    // Load existing review decisions
    let review_path = PathBuf::from(format!("data/generations/{}/review.json", batch_id));
    let review: serde_json::Value = if review_path.exists() {
        let s = std::fs::read_to_string(&review_path).unwrap_or_default();
        serde_json::from_str(&s).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let images_json = serde_json::to_string(&images).unwrap_or_default();
    let review_json = serde_json::to_string(&review).unwrap_or_default();

    let mut ctx = tera::Context::new();
    ctx.insert("batch_id", &batch_id);
    ctx.insert("images_json", &images_json);
    ctx.insert("review_json", &review_json);
    ctx.insert("total", &images.len());

    let html = state
        .tera
        .render("generations_tinder.html", &ctx)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Html(html))
}

/// POST /generations/:id/review — save approve/reject for an image
pub async fn save_review(
    Path(batch_id): Path<String>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let review_path = PathBuf::from(format!("data/generations/{}/review.json", batch_id));
    let tmp_path = PathBuf::from(format!("data/generations/{}/review.tmp.json", batch_id));

    // Load existing
    let mut review: serde_json::Map<String, serde_json::Value> = if review_path.exists() {
        let s = std::fs::read_to_string(&review_path).unwrap_or_default();
        serde_json::from_str(&s).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };

    let image_id = body
        .get("image_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let decision = body
        .get("decision")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !image_id.is_empty() {
        if decision == "undo" {
            review.remove(image_id);
        } else if !decision.is_empty() {
            review.insert(
                image_id.to_string(),
                serde_json::json!({
                    "decision": decision,
                    "prompt": body.get("prompt").and_then(|v| v.as_str()).unwrap_or(""),
                    "caption": body.get("caption").and_then(|v| v.as_str()).unwrap_or(""),
                }),
            );
        }
    }

    let json_str = serde_json::to_string(&review).map_err(|e| AppError::Internal(e.to_string()))?;
    std::fs::write(&tmp_path, &json_str).map_err(|e| AppError::Internal(e.to_string()))?;
    std::fs::rename(&tmp_path, &review_path).map_err(|e| AppError::Internal(e.to_string()))?;

    let approved = review.values().filter(|v| v.get("decision").and_then(|d| d.as_str()) == Some("approved")).count();
    let rejected = review.values().filter(|v| v.get("decision").and_then(|d| d.as_str()) == Some("rejected")).count();

    Ok(axum::Json(serde_json::json!({ "ok": true, "approved": approved, "rejected": rejected })))
}

/// POST /generations/:id/status — save keep/skip decisions
pub async fn save_status(
    Path(batch_id): Path<String>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let status_path = PathBuf::from(format!("data/generations/{}/status.json", batch_id));

    std::fs::create_dir_all(format!("data/generations/{}", batch_id))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    std::fs::write(&status_path, body.to_string())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(axum::Json(serde_json::json!({ "ok": true })))
}
