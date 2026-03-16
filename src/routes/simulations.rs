//! /simulations — 8 parallel inference slots on Yang (RTX 4090).
//! Real-time dashboard showing what's running on each Ollama slot.

use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, Json};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

const NUM_SLOTS: usize = 8;
const OLLAMA_URL: &str = "http://127.0.0.1:11434";

// ── Slot state ──────────────────────────────────────────────

#[derive(Clone, Serialize)]
pub struct Slot {
    pub id: usize,
    pub status: &'static str, // "idle" or "running"
    pub task: Option<String>,
    pub prompt_preview: Option<String>,
    pub started_at: Option<f64>,
    pub tokens: u64,
    pub elapsed_s: f64,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            id: 0,
            status: "idle",
            task: None,
            prompt_preview: None,
            started_at: None,
            tokens: 0,
            elapsed_s: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct SlotTracker {
    slots: Arc<RwLock<Vec<Slot>>>,
}

impl SlotTracker {
    pub fn new() -> Self {
        let slots: Vec<Slot> = (0..NUM_SLOTS)
            .map(|i| Slot {
                id: i,
                ..Default::default()
            })
            .collect();
        Self {
            slots: Arc::new(RwLock::new(slots)),
        }
    }

    pub async fn allocate(&self, task: &str, prompt_preview: Option<&str>) -> Option<usize> {
        let mut slots = self.slots.write().await;
        for s in slots.iter_mut() {
            if s.status == "idle" {
                s.status = "running";
                s.task = Some(task.to_string());
                s.prompt_preview = prompt_preview.map(|p| p.chars().take(120).collect());
                s.started_at = Some(now_secs());
                s.tokens = 0;
                return Some(s.id);
            }
        }
        None
    }

    pub async fn release(&self, slot_id: usize) {
        let mut slots = self.slots.write().await;
        if let Some(s) = slots.get_mut(slot_id) {
            s.status = "idle";
            s.task = None;
            s.prompt_preview = None;
            s.started_at = None;
            s.tokens = 0;
            s.elapsed_s = 0.0;
        }
    }

    pub async fn update_tokens(&self, slot_id: usize, tokens: u64) {
        let mut slots = self.slots.write().await;
        if let Some(s) = slots.get_mut(slot_id) {
            s.tokens = tokens;
        }
    }

    pub async fn snapshot(&self) -> Vec<Slot> {
        let slots = self.slots.read().await;
        let now = now_secs();
        slots
            .iter()
            .map(|s| {
                let mut c = s.clone();
                c.elapsed_s = s.started_at.map(|t| (now - t).max(0.0)).unwrap_or(0.0);
                c.elapsed_s = (c.elapsed_s * 10.0).round() / 10.0;
                c
            })
            .collect()
    }
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

// ── Ollama info ─────────────────────────────────────────────

#[derive(Deserialize)]
struct OllamaPs {
    models: Option<Vec<OllamaModel>>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: Option<String>,
    size_vram: Option<u64>,
}

async fn ollama_info() -> (String, f64) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    match client.get(format!("{}/api/ps", OLLAMA_URL)).send().await {
        Ok(resp) => match resp.json::<OllamaPs>().await {
            Ok(ps) => {
                let models = ps.models.unwrap_or_default();
                let name = models
                    .first()
                    .and_then(|m| m.name.clone())
                    .unwrap_or_else(|| "none".into());
                let vram = models.first().and_then(|m| m.size_vram).unwrap_or(0) as f64 / 1e9;
                (name, (vram * 10.0).round() / 10.0)
            }
            Err(_) => ("error".into(), 0.0),
        },
        Err(_) => ("offline".into(), 0.0),
    }
}

// ── Route handlers ──────────────────────────────────────────

pub async fn simulations_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("simulations.html", &ctx)?;
    Ok(Html(html))
}

#[derive(Serialize)]
pub struct SlotsResponse {
    slots: Vec<Slot>,
    model: String,
    model_vram_gb: f64,
    gpu: &'static str,
    parallel_capacity: usize,
    active: usize,
    idle: usize,
}

pub async fn slots_status(State(state): State<AppState>) -> Json<SlotsResponse> {
    let tracker = state.slot_tracker.clone();
    let slots = tracker.snapshot().await;
    let (model, vram) = ollama_info().await;
    let active = slots.iter().filter(|s| s.status == "running").count();

    Json(SlotsResponse {
        slots,
        model,
        model_vram_gb: vram,
        gpu: "Yang (RTX 4090)",
        parallel_capacity: NUM_SLOTS,
        active,
        idle: NUM_SLOTS - active,
    })
}

pub async fn slots_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let tracker = state.slot_tracker.clone();

    let stream = async_stream::stream! {
        // Send initial state
        let (model, _vram) = ollama_info().await;
        let slots = tracker.snapshot().await;
        let init = serde_json::json!({
            "type": "init",
            "slots": slots,
            "model": model,
        });
        yield Ok(Event::default().data(init.to_string()));

        // Tick every second
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let slots = tracker.snapshot().await;
            let tick = serde_json::json!({
                "type": "tick",
                "slots": slots,
            });
            yield Ok(Event::default().data(tick.to_string()));
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    )
}

#[derive(Serialize)]
pub struct DemoResponse {
    slot: usize,
    status: &'static str,
}

pub async fn slots_demo(State(state): State<AppState>) -> Result<Json<DemoResponse>, AppError> {
    let tracker = state.slot_tracker.clone();
    let slot_id = tracker
        .allocate("demo", Some("What is a simulation slot?"))
        .await
        .ok_or_else(|| AppError::BadRequest("All slots busy".into()))?;

    // Spawn inference in background
    let tracker2 = tracker.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        let resp = client
            .post(format!("{}/api/generate", OLLAMA_URL))
            .json(&serde_json::json!({
                "model": "qwen3.5:9b",
                "prompt": "In one sentence, describe what a simulation slot is.",
                "stream": false,
                "options": {"num_predict": 60}
            }))
            .send()
            .await;
        if let Ok(r) = resp {
            if let Ok(data) = r.json::<serde_json::Value>().await {
                let tokens = data["eval_count"].as_u64().unwrap_or(0);
                tracker2.update_tokens(slot_id, tokens).await;
            }
        }
        tracker2.release(slot_id).await;
    });

    Ok(Json(DemoResponse {
        slot: slot_id,
        status: "started",
    }))
}
