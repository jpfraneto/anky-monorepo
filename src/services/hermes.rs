//! Client for the Hermes Bridge HTTP API (localhost:8891).
//! Used to dispatch tagged tasks from JP's X mentions to the AI agent.

use anyhow::Result;
use serde::{Deserialize, Serialize};

const BRIDGE_URL: &str = "http://127.0.0.1:8891";

#[derive(Debug, Clone, Serialize)]
pub struct HermesTask {
    pub tag: String,
    pub content: String,
    pub source_tweet_id: String,
    pub author: String,
}

#[derive(Debug, Deserialize)]
pub struct HermesResult {
    pub status: String,
    pub summary: Option<String>,
    pub task_id: Option<String>,
    pub message: Option<String>,
}

/// Dispatch a tagged task to the Hermes agent via the bridge.
/// This is fire-and-forget from Rust's perspective — the bridge handles
/// the agent execution and returns a summary.
pub async fn dispatch_task(task: &HermesTask) -> Result<HermesResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // Agent can take minutes
        .build()?;

    let resp = client
        .post(format!("{}/task", BRIDGE_URL))
        .json(task)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Hermes bridge error {}: {}", status, body);
    }

    let result: HermesResult = resp.json().await?;
    Ok(result)
}

/// Check if the bridge is running.
pub async fn is_available() -> bool {
    reqwest::get(format!("{}/health", BRIDGE_URL))
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Parse [TAG: content] from a tweet text. Returns (tag, content) if found.
/// Supported tags: EVOLVE, FEATURE_IDEA, BUG, CONFIG
pub fn parse_tag(text: &str) -> Option<(String, String)> {
    // Match [TAG: content] or [TAG content] patterns
    let re_patterns = [
        // [EVOLVE: do something]
        (r"\[EVOLVE[:\s]+(.+?)\]", "EVOLVE"),
        // [FEATURE_IDEA: some idea]
        (r"\[FEATURE_IDEA[:\s]+(.+?)\]", "FEATURE_IDEA"),
        // [BUG: something broke]
        (r"\[BUG[:\s]+(.+?)\]", "BUG"),
        // [CONFIG: change something]
        (r"\[CONFIG[:\s]+(.+?)\]", "CONFIG"),
    ];

    let text_upper = text.to_uppercase();
    for (_, tag) in &re_patterns {
        // Simple bracket-based extraction (avoid regex dependency)
        let tag_prefix = format!("[{}", tag);
        if let Some(start) = text_upper.find(&tag_prefix) {
            // Find the content between the tag and the closing bracket
            let after_tag = &text[start + tag_prefix.len()..];
            // Skip optional colon and whitespace
            let content_start = after_tag
                .char_indices()
                .find(|(_, c)| *c != ':' && *c != ' ')
                .map(|(i, _)| i)
                .unwrap_or(0);
            let after_trim = &after_tag[content_start..];
            let content = if let Some(end) = after_trim.find(']') {
                after_trim[..end].trim()
            } else {
                after_trim.trim()
            };
            if !content.is_empty() {
                return Some((tag.to_string(), content.to_string()));
            }
        }
    }

    None
}
