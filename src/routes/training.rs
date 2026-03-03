use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path as AxumPath, State};
use axum::http::HeaderMap;
use axum::response::Html;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::path::Path;

// --- Pages ---

pub async fn training_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    ctx.insert("page", "training");
    let html = state.tera.render("training.html", &ctx)?;
    Ok(Html(html))
}

pub async fn trainings_list(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    ctx.insert("page", "trainings");
    let html = state.tera.render("trainings.html", &ctx)?;
    Ok(Html(html))
}

pub async fn general_instructions(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    ctx.insert("page", "trainings");
    let html = state.tera.render("training_general_instructions.html", &ctx)?;
    Ok(Html(html))
}

pub async fn training_run_detail(
    State(state): State<AppState>,
    AxumPath(date): AxumPath<String>,
) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    ctx.insert("date", &date);
    let html = state.tera.render("training_run.html", &ctx)?;
    Ok(Html(html))
}

// --- API: get next unlabelled image ---

#[derive(Serialize)]
pub struct NextResponse {
    anky: Option<AnkyCard>,
    approved: i64,
    rejected: i64,
    remaining: i64,
}

#[derive(Serialize)]
pub struct AnkyCard {
    id: String,
    title: Option<String>,
    image_prompt: Option<String>,
    image_url: String,
}

pub async fn next_image(State(state): State<AppState>) -> Result<Json<NextResponse>, AppError> {
    let db = state.db.lock().await;

    // Ensure training_labels table exists
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS training_labels (
            anky_id TEXT PRIMARY KEY,
            approved BOOLEAN NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    )?;

    // Count stats
    let approved: i64 = db.query_row(
        "SELECT COUNT(*) FROM training_labels WHERE approved = 1",
        [],
        |row| row.get(0),
    )?;
    let rejected: i64 = db.query_row(
        "SELECT COUNT(*) FROM training_labels WHERE approved = 0",
        [],
        |row| row.get(0),
    )?;

    // Get next unlabelled anky that has an image
    let result = db.query_row(
        "SELECT a.id, a.title, a.image_prompt, a.image_path, a.image_webp
         FROM ankys a
         WHERE a.status = 'complete'
           AND a.image_path IS NOT NULL
           AND a.id NOT IN (SELECT anky_id FROM training_labels)
         ORDER BY a.created_at ASC
         LIMIT 1",
        [],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        },
    );

    let total_unlabelled: i64 = db.query_row(
        "SELECT COUNT(*) FROM ankys
         WHERE status = 'complete'
           AND image_path IS NOT NULL
           AND id NOT IN (SELECT anky_id FROM training_labels)",
        [],
        |row| row.get(0),
    )?;

    let anky = match result {
        Ok((id, title, image_prompt, image_path, image_webp)) => {
            // Prefer webp, fall back to png
            let filename = image_webp.or(image_path).unwrap_or_default();
            let image_url = format!("/data/images/{}", filename);
            Some(AnkyCard {
                id,
                title,
                image_prompt,
                image_url,
            })
        }
        Err(_) => None,
    };

    Ok(Json(NextResponse {
        anky,
        approved,
        rejected,
        remaining: total_unlabelled,
    }))
}

// --- API: submit vote ---

#[derive(Deserialize)]
pub struct VoteRequest {
    anky_id: String,
    approved: bool,
}

pub async fn vote(
    State(state): State<AppState>,
    Json(req): Json<VoteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.lock().await;

    // Ensure table exists
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS training_labels (
            anky_id TEXT PRIMARY KEY,
            approved BOOLEAN NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    )?;

    // Insert or replace the label
    db.execute(
        "INSERT OR REPLACE INTO training_labels (anky_id, approved) VALUES (?1, ?2)",
        rusqlite::params![req.anky_id, req.approved],
    )?;

    // If approved, copy the image to training-images/
    if req.approved {
        // Get the image path from the anky record
        let paths: (Option<String>, Option<String>) = db.query_row(
            "SELECT image_path, image_prompt FROM ankys WHERE id = ?1",
            rusqlite::params![req.anky_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        if let Some(image_filename) = paths.0 {
            let src = format!("data/images/{}", image_filename);
            let dst = format!("data/training-images/{}", image_filename);

            if Path::new(&src).exists() {
                if let Err(e) = std::fs::copy(&src, &dst) {
                    tracing::warn!("Failed to copy training image {}: {}", src, e);
                }
            }

            // Also write the caption file (image_prompt as .txt)
            if let Some(prompt) = paths.1 {
                let txt_name = image_filename.replace(".png", ".txt").replace(".webp", ".txt");
                let txt_path = format!("data/training-images/{}", txt_name);
                let _ = std::fs::write(&txt_path, &prompt);
            }
        }
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
