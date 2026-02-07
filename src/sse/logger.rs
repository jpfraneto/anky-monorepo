use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub module: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl LogEntry {
    pub fn info(module: &str, message: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            level: "INFO".into(),
            module: module.into(),
            message: message.into(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn to_sse_data(&self) -> String {
        let ts = self.timestamp.format("%H:%M:%S");
        let meta = self
            .metadata
            .as_ref()
            .map(|m| format!(" {}", m))
            .unwrap_or_default();
        format!(
            "[{}] [{}] [{}] {}{}",
            ts, self.level, self.module, self.message, meta
        )
    }
}
