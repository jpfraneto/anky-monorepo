use crate::error::AppError;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
struct MindChoice {
    message: MindResponseMessage,
}

#[derive(Deserialize)]
struct MindResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct MindCompletion {
    choices: Vec<MindChoice>,
}

#[derive(Deserialize)]
pub struct SlotStatus {
    pub id: u8,
    pub is_processing: bool,
}

fn strip_think(text: &str) -> String {
    if let Some(end) = text.find("</think>") {
        text[end + 8..].trim().to_string()
    } else {
        text.trim().to_string()
    }
}

pub async fn call(
    mind_url: &str,
    system: &str,
    user: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<String, AppError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {}", e)))?;

    let payload = serde_json::json!({
        "model": "anky",
        "messages": [
            { "role": "system", "content": system },
            { "role": "user",   "content": user   },
        ],
        "max_tokens": max_tokens,
        "temperature": temperature,
        "stream": false,
        "chat_template_kwargs": { "enable_thinking": false },
    });

    let resp = client
        .post(&format!("{}/v1/chat/completions", mind_url))
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Mind unavailable: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Mind error {}: {}",
            status,
            &body[..body.len().min(200)]
        )));
    }

    let completion: MindCompletion = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Mind parse error: {}", e)))?;

    let raw = completion
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| AppError::Internal("Mind returned empty choices".into()))?;

    Ok(strip_think(&raw))
}

/// Multi-turn chat via Mind (OpenAI-compatible API).
/// Messages use the same format as OllamaChatMessage: role + content.
pub async fn chat(
    mind_url: &str,
    messages: &[(String, String)], // (role, content) pairs including system
    max_tokens: u32,
) -> Result<String, AppError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {}", e)))?;

    let msgs: Vec<serde_json::Value> = messages
        .iter()
        .map(|(role, content)| serde_json::json!({"role": role, "content": content}))
        .collect();

    let payload = serde_json::json!({
        "model": "anky",
        "messages": msgs,
        "max_tokens": max_tokens,
        "temperature": 0.7,
        "stream": false,
        "chat_template_kwargs": { "enable_thinking": false },
    });

    let resp = client
        .post(&format!("{}/v1/chat/completions", mind_url))
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Mind unavailable: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Mind error {}: {}",
            status,
            &body[..body.len().min(200)]
        )));
    }

    let completion: MindCompletion = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Mind parse error: {}", e)))?;

    let raw = completion
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| AppError::Internal("Mind returned empty choices".into()))?;

    Ok(strip_think(&raw))
}

pub async fn get_slots(mind_url: &str) -> Result<Vec<SlotStatus>, AppError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let resp = client
        .get(&format!("{}/slots", mind_url))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Mind /slots failed: {}", e)))?;

    resp.json::<Vec<SlotStatus>>()
        .await
        .map_err(|e| AppError::Internal(format!("Mind /slots parse: {}", e)))
}

#[allow(dead_code)]
pub async fn is_available(mind_url: &str) -> bool {
    get_slots(mind_url).await.is_ok()
}
