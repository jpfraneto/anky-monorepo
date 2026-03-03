use crate::config::Config;
use crate::services::stream::FrameBuffer;
use crate::sse::logger::LogEntry;
use rusqlite::Connection;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tera::Tera;
use tokio::sync::{broadcast, Mutex, RwLock};

/// Simple in-memory rate limiter: tracks request timestamps per key.
#[derive(Clone)]
pub struct RateLimiter {
    /// key -> list of request timestamps
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    /// max requests allowed in the window
    pub max_requests: usize,
    /// time window
    pub window: std::time::Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: std::time::Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    /// Returns Ok(()) if allowed, Err(seconds_until_next_slot) if rate limited.
    pub async fn check(&self, key: &str) -> Result<(), u64> {
        let mut map = self.requests.lock().await;
        let now = Instant::now();
        let entries = map.entry(key.to_string()).or_default();

        // Prune old entries
        entries.retain(|t| now.duration_since(*t) < self.window);

        if entries.len() >= self.max_requests {
            let oldest = entries[0];
            let wait = self.window - now.duration_since(oldest);
            Err(wait.as_secs() + 1)
        } else {
            entries.push(now);
            Ok(())
        }
    }
}

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
    pub showing_congrats: bool,
    pub writer_id: Option<String>,
    pub writer_username: Option<String>,
    pub writer_type: Option<String>,
    /// When the current live session started (for watchdog stale-session detection)
    pub started_at: Option<Instant>,
}

impl Default for LiveState {
    fn default() -> Self {
        Self {
            is_live: false,
            showing_congrats: false,
            writer_id: None,
            writer_username: None,
            writer_type: None,
            started_at: None,
        }
    }
}

/// A writer waiting in the queue
#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: String,
    pub username: String,
    pub writer_type: String,  // "human" or "agent"
    pub text: Option<String>, // pre-loaded for agents
    pub joined_at: Instant,
}

/// Events broadcast to all clients about live status changes
#[derive(Debug, Clone)]
pub enum LiveStatusEvent {
    WentLive {
        writer_id: String,
        writer_username: String,
        writer_type: String,
    },
    Congrats {
        writer_username: String,
    },
    WentIdle,
    QueueUpdate {
        count: usize,
    },
    YourTurn {
        writer_username: String,
    },
}

/// Live text events broadcast to overlay clients via SSE
#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveTextEvent {
    pub content: String,
    pub words: i64,
    pub elapsed: f64,
    pub idle_ratio: f64,
    pub progress: f64,
    pub is_live: bool,
    pub writer_username: Option<String>,
    pub writer_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub congrats: Option<bool>,
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
    pub live_text_tx: broadcast::Sender<LiveTextEvent>,
    pub frame_buffer: FrameBuffer,
    pub write_limiter: RateLimiter,
    pub waiting_room: Arc<RwLock<VecDeque<QueueEntry>>>,
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
