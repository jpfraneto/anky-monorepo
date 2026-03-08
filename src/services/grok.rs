use anyhow::{bail, Result};
use serde::Serialize;

const XAI_BASE_URL: &str = "https://api.x.ai/v1";

#[derive(Serialize)]
struct ImageRef {
    url: String,
}

#[derive(Serialize)]
struct VideoRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<ImageRef>,
}

/// Submit a text-to-video generation request.
pub async fn generate_video(api_key: &str, prompt: &str, duration_seconds: u32) -> Result<String> {
    generate_video_with_aspect(api_key, prompt, duration_seconds, None, "9:16").await
}

/// Submit an image-to-video generation request.
/// If image_url is provided, Grok animates that image into a video clip.
pub async fn generate_video_from_image(
    api_key: &str,
    prompt: &str,
    duration_seconds: u32,
    image_url: Option<&str>,
) -> Result<String> {
    generate_video_with_aspect(api_key, prompt, duration_seconds, image_url, "9:16").await
}

/// Submit an image-to-video generation request with a specific aspect ratio.
pub async fn generate_video_from_image_with_aspect(
    api_key: &str,
    prompt: &str,
    duration_seconds: u32,
    image_url: Option<&str>,
    aspect_ratio: &str,
) -> Result<String> {
    generate_video_with_aspect(api_key, prompt, duration_seconds, image_url, aspect_ratio).await
}

async fn generate_video_with_aspect(
    api_key: &str,
    prompt: &str,
    duration_seconds: u32,
    image_url: Option<&str>,
    aspect_ratio: &str,
) -> Result<String> {
    // xAI enforces 1–15 second range
    let duration_seconds = duration_seconds.clamp(1, 15);

    tracing::info!(
        "xAI: submitting video gen, key_len={}, prompt_len={}, duration={}s, aspect_ratio={}, has_image={}",
        api_key.len(),
        prompt.len(),
        duration_seconds,
        aspect_ratio,
        image_url.is_some()
    );

    let client = reqwest::Client::new();
    let req = VideoRequest {
        model: "grok-imagine-video".to_string(),
        prompt: prompt.to_string(),
        duration: Some(duration_seconds),
        aspect_ratio: Some(aspect_ratio.to_string()),
        resolution: Some("720p".to_string()),
        image: image_url.map(|s| ImageRef { url: s.to_string() }),
    };

    let resp = client
        .post(format!("{}/videos/generations", XAI_BASE_URL))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&req)
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;

    tracing::info!(
        "xAI video submit response ({}): {}",
        status,
        &body.to_string()[..200.min(body.to_string().len())]
    );

    if !status.is_success() {
        bail!("xAI video API error ({}): {}", status, body);
    }

    if let Some(id) = body.get("request_id").and_then(|v| v.as_str()) {
        return Ok(id.to_string());
    }

    bail!("Unexpected xAI response (no request_id): {}", body)
}

/// Poll for video generation status. Returns (status, Option<video_url>).
/// Status: "pending", "done", "expired".
pub async fn poll_video(api_key: &str, request_id: &str) -> Result<(String, Option<String>)> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/videos/{}", XAI_BASE_URL, request_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;

    if !status.is_success() {
        bail!("xAI poll error ({}): {}", status, body);
    }

    // xAI returns { "video": { "url": "..." } } when complete (no "status" field),
    // or { "status": "pending" } while still generating.
    let video_url = body
        .get("video")
        .and_then(|v| v.get("url"))
        .and_then(|v| v.as_str())
        .or_else(|| body.get("video_url").and_then(|v| v.as_str()))
        .or_else(|| body.get("url").and_then(|v| v.as_str()))
        .or_else(|| body.get("video").and_then(|v| v.as_str()))
        .map(|s| s.to_string());

    // If we found a video URL, the generation is complete regardless of status field
    let gen_status = if video_url.is_some() {
        "complete".to_string()
    } else {
        body.get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string()
    };

    tracing::info!(
        "xAI poll {}: status={}, has_url={}",
        request_id,
        gen_status,
        video_url.is_some()
    );

    Ok((gen_status, video_url))
}

/// Download a video from URL and save to disk.
pub async fn download_video(url: &str, output_path: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        bail!("Failed to download video: {}", resp.status());
    }
    let bytes = resp.bytes().await?;
    std::fs::create_dir_all(
        std::path::Path::new(output_path)
            .parent()
            .unwrap_or(std::path::Path::new(".")),
    )?;
    std::fs::write(output_path, &bytes)?;
    Ok(())
}
