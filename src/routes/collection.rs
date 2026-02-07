use crate::error::AppError;
use crate::models::CollectionCreateRequest;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::Html;
use axum::Json;

#[derive(serde::Serialize)]
pub struct CollectionCreateResponse {
    id: String,
    cost_estimate_usd: f64,
    num_beings: usize,
}

pub async fn create_collection(
    State(state): State<AppState>,
    Json(req): Json<CollectionCreateRequest>,
) -> Result<Json<CollectionCreateResponse>, AppError> {
    let collection_id = uuid::Uuid::new_v4().to_string();
    let cost_estimate = crate::pipeline::cost::estimate_collection_cost(88);

    {
        let db = state.db.lock().await;
        crate::db::queries::ensure_user(&db, "anonymous")?;
        crate::db::queries::insert_collection(&db, &collection_id, "anonymous", &req.mega_prompt, cost_estimate)?;
    }

    state.emit_log(
        "INFO",
        "collection",
        &format!("Collection created: {} (est. ${:.2})", &collection_id[..8], cost_estimate),
    );

    Ok(Json(CollectionCreateResponse {
        id: collection_id,
        cost_estimate_usd: cost_estimate,
        num_beings: 88,
    }))
}

pub async fn get_collection(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Html<String>, AppError> {
    let collection = {
        let db = state.db.lock().await;
        crate::db::queries::get_collection(&db, &id)?
    };

    let Some(collection) = collection else {
        return Err(AppError::NotFound("Collection not found".into()));
    };

    let mut ctx = tera::Context::new();
    ctx.insert("collection", &serde_json::json!({
        "id": collection.id,
        "mega_prompt": collection.mega_prompt,
        "status": collection.status,
        "progress": collection.progress,
        "total": collection.total,
        "cost_estimate_usd": collection.cost_estimate_usd,
        "created_at": collection.created_at,
    }));

    let template = if collection.status == "generating" {
        "collection_progress.html"
    } else {
        "collection.html"
    };

    let html = state.tera.render(template, &ctx)?;
    Ok(Html(html))
}
