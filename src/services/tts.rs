/// Anky TTS — F5-TTS HTTP client for local text-to-speech generation.
use anyhow::Result;

/// Synthesize text to audio via the local F5-TTS service.
/// Returns raw WAV bytes and duration in seconds.
pub async fn synthesize(
    tts_base_url: &str,
    text: &str,
    language: &str,
    timeout_secs: u64,
) -> Result<(Vec<u8>, f64)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let resp = client
        .post(format!("{}/synthesize", tts_base_url))
        .json(&serde_json::json!({
            "text": text,
            "language": language,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("TTS service returned {}: {}", status, body);
    }

    let duration: f64 = resp
        .headers()
        .get("x-audio-duration")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);

    let bytes = resp.bytes().await?.to_vec();
    Ok((bytes, duration))
}

/// Check if the TTS service is reachable and healthy.
pub async fn is_healthy(tts_base_url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build();
    match client {
        Ok(c) => c
            .get(format!("{}/health", tts_base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false),
        Err(_) => false,
    }
}
