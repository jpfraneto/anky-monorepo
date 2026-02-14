use crate::config::Config;
use crate::sse::logger::LogEntry;
use rusqlite::Connection;
use std::sync::Arc;
use tera::Tera;
use tokio::sync::{broadcast, Mutex, RwLock};

#[derive(Debug, Clone, PartialEq)]
pub enum GpuStatus {
    Idle,
    Generating,
    Training { step: u32, total: u32 },
}

impl std::fmt::Display for GpuStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuStatus::Idle => write!(f, "idle"),
            GpuStatus::Generating => write!(f, "generating"),
            GpuStatus::Training { step, total } => write!(f, "training ({}/{})", step, total),
        }
    }
}

/// Live streaming state — tracks who's currently broadcasting
#[derive(Debug, Clone)]
pub struct LiveState {
    pub is_live: bool,
    pub writer_id: Option<String>,
}

impl Default for LiveState {
    fn default() -> Self {
        Self { is_live: false, writer_id: None }
    }
}

/// Events broadcast to all clients about live status changes
#[derive(Debug, Clone)]
pub enum LiveStatusEvent {
    WentLive { writer_id: String },
    WentIdle,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub tera: Arc<Tera>,
    pub config: Arc<Config>,
    pub gpu_status: Arc<RwLock<GpuStatus>>,
    pub log_tx: broadcast::Sender<LogEntry>,
    pub live_state: Arc<RwLock<LiveState>>,
    pub live_status_tx: broadcast::Sender<LiveStatusEvent>,
}

impl AppState {
    pub fn emit_log(&self, level: &str, module: &str, message: &str) {
        let entry = LogEntry {
            timestamp: chrono::Utc::now(),
            level: level.to_string(),
            module: module.to_string(),
            message: message.to_string(),
            metadata: None,
        };
        let _ = self.log_tx.send(entry);
    }
}
