use anyhow::{anyhow, Result};
use futures::StreamExt;
use serde::Serialize;

use crate::services::streaming_text::StreamRenderBuffer;

const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

#[derive(Clone, Debug, Serialize)]
pub struct OpenRouterMessage {
    pub role: String,
    pub content: String,
}

impl OpenRouterMessage {
    pub fn new(role: &str, content: impl Into<String>) -> Self {
        Self {
            role: role.to_string(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OpenRouterResult {
    pub text: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub provider: Option<String>,
}

/// Call OpenRouter's chat completions API.
pub async fn call_openrouter(
    api_key: &str,
    model: &str,
    system: &str,
    user_message: &str,
    max_tokens: u32,
    timeout_secs: u64,
) -> Result<String> {
    let result = call_openrouter_messages(
        api_key,
        model,
        system,
        vec![OpenRouterMessage::new("user", user_message)],
        max_tokens,
        timeout_secs,
    )
    .await?;
    Ok(result.text)
}

pub async fn call_openrouter_messages(
    api_key: &str,
    model: &str,
    system: &str,
    messages: Vec<OpenRouterMessage>,
    max_tokens: u32,
    timeout_secs: u64,
) -> Result<OpenRouterResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let body = serde_json::json!({
        "model": model,
        "messages": build_messages(system, messages),
        "max_tokens": max_tokens
    });

    let resp = client
        .post(OPENROUTER_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://anky.app")
        .header("X-Title", "anky")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenRouter API error {}: {}", status, text);
    }

    let data: serde_json::Value = resp.json().await?;
    let text = extract_content(
        &data["choices"]
            .get(0)
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    );
    if text.is_empty() {
        anyhow::bail!("empty response from OpenRouter");
    }

    let usage = data.get("usage");
    let provider = data
        .get("provider")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    Ok(OpenRouterResult {
        text,
        input_tokens: parse_usage_value(usage, "prompt_tokens", "input_tokens"),
        output_tokens: parse_usage_value(usage, "completion_tokens", "output_tokens"),
        provider,
    })
}

pub async fn stream_openrouter_messages(
    api_key: &str,
    model: &str,
    system: &str,
    messages: Vec<OpenRouterMessage>,
    max_tokens: u32,
    timeout_secs: u64,
    tx: tokio::sync::mpsc::Sender<String>,
) -> Result<OpenRouterResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let body = serde_json::json!({
        "model": model,
        "messages": build_messages(system, messages),
        "max_tokens": max_tokens,
        "stream": true,
        "stream_options": { "include_usage": true }
    });

    let resp = client
        .post(OPENROUTER_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://anky.app")
        .header("X-Title", "anky")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenRouter API error {}: {}", status, text);
    }

    let mut buffer = String::new();
    let mut full_text = String::new();
    let mut input_tokens = 0_i64;
    let mut output_tokens = 0_i64;
    let mut provider = None;
    let mut stream = resp.bytes_stream();
    let mut render_buffer = StreamRenderBuffer::default();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in event.lines() {
                if line.starts_with(':') {
                    continue;
                }
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                if data.trim() == "[DONE]" {
                    continue;
                }

                let value: serde_json::Value = serde_json::from_str(data)
                    .map_err(|err| anyhow!("OpenRouter stream parse error: {}", err))?;

                if let Some(err) = value.get("error") {
                    let message = err
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error");
                    anyhow::bail!("OpenRouter stream error: {}", message);
                }

                if provider.is_none() {
                    provider = value
                        .get("provider")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string());
                }

                if let Some(usage) = value.get("usage") {
                    input_tokens = parse_usage_value(Some(usage), "prompt_tokens", "input_tokens");
                    output_tokens =
                        parse_usage_value(Some(usage), "completion_tokens", "output_tokens");
                }

                if let Some(choice) = value["choices"].get(0) {
                    if let Some(delta) = choice.get("delta") {
                        let text = extract_content(
                            &delta
                                .get("content")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null),
                        );
                        if !text.is_empty() {
                            full_text.push_str(&text);
                            if let Some(stable_text) = render_buffer.push(&text) {
                                let _ = tx.send(stable_text).await;
                            }
                        }
                    }

                    if choice
                        .get("finish_reason")
                        .and_then(|v| v.as_str())
                        .is_some_and(|finish| finish == "error")
                    {
                        anyhow::bail!("OpenRouter stream terminated with an error");
                    }
                }
            }
        }
    }

    if let Some(remaining_text) = render_buffer.finish() {
        let _ = tx.send(remaining_text).await;
    }

    if full_text.is_empty() {
        anyhow::bail!("OpenRouter returned empty streamed response");
    }

    Ok(OpenRouterResult {
        text: full_text,
        input_tokens,
        output_tokens,
        provider,
    })
}

fn build_messages(system: &str, messages: Vec<OpenRouterMessage>) -> Vec<serde_json::Value> {
    let mut out = Vec::with_capacity(messages.len() + usize::from(!system.is_empty()));
    if !system.is_empty() {
        out.push(serde_json::json!({
            "role": "system",
            "content": system
        }));
    }
    for message in messages {
        out.push(serde_json::json!({
            "role": message.role,
            "content": message.content
        }));
    }
    out
}

fn extract_content(value: &serde_json::Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }

    value
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| {
                    part.get("text")
                        .and_then(|text| text.as_str())
                        .or_else(|| part.as_str())
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn parse_usage_value(
    usage: Option<&serde_json::Value>,
    primary_key: &str,
    fallback_key: &str,
) -> i64 {
    usage
        .and_then(|value| value.get(primary_key).or_else(|| value.get(fallback_key)))
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
}
