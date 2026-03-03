use anyhow::Result;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GeminiPart {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },
}

#[derive(Serialize)]
struct InlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseModalities")]
    response_modalities: Vec<String>,
    #[serde(rename = "imageConfig", skip_serializing_if = "Option::is_none")]
    image_config: Option<ImageConfig>,
}

#[derive(Serialize)]
struct ImageConfig {
    #[serde(rename = "aspectRatio")]
    aspect_ratio: String,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize, Debug)]
struct CandidateContent {
    parts: Option<Vec<ResponsePart>>,
}

#[derive(Deserialize, Debug)]
struct ResponsePart {
    text: Option<String>,
    #[serde(rename = "inlineData")]
    inline_data: Option<ResponseInlineData>,
}

#[derive(Deserialize, Debug)]
struct ResponseInlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

/// Load reference images from disk and return as base64 strings.
pub fn load_references(references_dir: &Path) -> Vec<String> {
    let files = ["anky-1.png", "anky-2.png", "anky-3.png"];
    let mut refs = Vec::new();

    for file in &files {
        let path = references_dir.join(file);
        if let Ok(data) = std::fs::read(&path) {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            tracing::info!("Loaded reference: {} ({}KB)", file, data.len() / 1024);
            refs.push(b64);
        } else {
            tracing::warn!("Reference not found: {}", path.display());
        }
    }

    tracing::info!("Total references loaded: {}", refs.len());
    refs
}

/// Generate an Anky image using Gemini with reference images (default 1:1 aspect ratio).
pub async fn generate_image(
    api_key: &str,
    prompt: &str,
    references: &[String],
) -> Result<ImageResult> {
    generate_image_with_aspect(api_key, prompt, references, "1:1").await
}

/// Generate an image with the exact prompt text provided by the caller.
/// No prompt templating or rewriting is applied.
pub async fn generate_image_exact(
    api_key: &str,
    prompt: &str,
    references: &[String],
) -> Result<ImageResult> {
    generate_image_exact_with_aspect(api_key, prompt, references, "1:1").await
}

/// Generate an Anky image with a specific aspect ratio (e.g. "1:1", "2:1", "16:9").
pub async fn generate_image_with_aspect(
    api_key: &str,
    prompt: &str,
    references: &[String],
    aspect_ratio: &str,
) -> Result<ImageResult> {
    let full_prompt = format!(
        r#"{}

CHARACTER — ANKY (when present in the scene, follow exactly):
- Blue-skinned creature with large expressive pointed ears
- Purple swirling hair with golden spiral accents
- Golden/amber glowing eyes
- Golden jewelry and decorative accents on body
- Compact body, ancient yet childlike quality — a messenger from beyond

STYLE:
- Painterly, atmospheric, cinematic depth
- Consistent visual coherence — this is ONE scene from a continuous story
- Follow the color direction specified in the scene description
- Vertical 9:16 composition with layered depth
- Highly detailed, emotionally evocative
- If continuity reference frames are provided, maintain the same environment, lighting, and visual world — only transform what the scene description asks to change"#,
        prompt
    );

    let mut parts: Vec<GeminiPart> = Vec::new();

    // Add reference images
    for (i, ref_b64) in references.iter().take(6).enumerate() {
        tracing::debug!(
            "Adding reference image {} ({}KB b64)",
            i + 1,
            ref_b64.len() / 1024
        );
        parts.push(GeminiPart::InlineData {
            inline_data: InlineData {
                mime_type: "image/png".into(),
                data: ref_b64.clone(),
            },
        });
    }

    if !references.is_empty() {
        parts.push(GeminiPart::Text {
            text: "Reference images above show: (1) Anky's character design — match exactly when Anky appears, (2) Continuity frames from the PREVIOUS scene — you MUST maintain the same visual world, environment, and style. This scene is the NEXT moment in a continuous story. The world should feel like the same place, just progressed forward in the narrative. Match the lighting, environment, and atmosphere, then apply only the changes described in the scene:".into(),
        });
    }

    parts.push(GeminiPart::Text { text: full_prompt });

    let request = GeminiRequest {
        contents: vec![GeminiContent { parts }],
        generation_config: GenerationConfig {
            response_modalities: vec!["TEXT".into(), "IMAGE".into()],
            image_config: Some(ImageConfig {
                aspect_ratio: aspect_ratio.into(),
            }),
        },
    };

    let model = "gemini-2.5-flash-image";
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let resp = client.post(&url).json(&request).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Gemini API error {}: {}", status, body);
    }

    let data: GeminiResponse = resp.json().await?;

    let candidates = data.candidates.unwrap_or_default();
    let parts = candidates
        .first()
        .and_then(|c| c.content.as_ref())
        .and_then(|c| c.parts.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No candidates in Gemini response"))?;

    // Find image part
    let image_part = parts
        .iter()
        .find(|p| {
            p.inline_data
                .as_ref()
                .is_some_and(|d| d.mime_type.starts_with("image/"))
        })
        .and_then(|p| p.inline_data.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No image in Gemini response"))?;

    Ok(ImageResult {
        base64: image_part.data.clone(),
        mime_type: image_part.mime_type.clone(),
    })
}

/// Generate an image with a specific aspect ratio and exact prompt text.
/// The prompt is passed to Gemini verbatim.
pub async fn generate_image_exact_with_aspect(
    api_key: &str,
    prompt: &str,
    references: &[String],
    aspect_ratio: &str,
) -> Result<ImageResult> {
    let mut parts: Vec<GeminiPart> = Vec::new();

    // Add reference images
    for (i, ref_b64) in references.iter().take(6).enumerate() {
        tracing::debug!(
            "Adding reference image {} ({}KB b64)",
            i + 1,
            ref_b64.len() / 1024
        );
        parts.push(GeminiPart::InlineData {
            inline_data: InlineData {
                mime_type: "image/png".into(),
                data: ref_b64.clone(),
            },
        });
    }

    if !references.is_empty() {
        parts.push(GeminiPart::Text {
            text: "Reference images above provide visual context.".into(),
        });
    }

    parts.push(GeminiPart::Text {
        text: prompt.to_string(),
    });

    let request = GeminiRequest {
        contents: vec![GeminiContent { parts }],
        generation_config: GenerationConfig {
            response_modalities: vec!["TEXT".into(), "IMAGE".into()],
            image_config: Some(ImageConfig {
                aspect_ratio: aspect_ratio.into(),
            }),
        },
    };

    let model = "gemini-2.5-flash-image";
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let resp = client.post(&url).json(&request).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Gemini API error {}: {}", status, body);
    }

    let data: GeminiResponse = resp.json().await?;

    let candidates = data.candidates.unwrap_or_default();
    let parts = candidates
        .first()
        .and_then(|c| c.content.as_ref())
        .and_then(|c| c.parts.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No candidates in Gemini response"))?;

    // Find image part
    let image_part = parts
        .iter()
        .find(|p| {
            p.inline_data
                .as_ref()
                .is_some_and(|d| d.mime_type.starts_with("image/"))
        })
        .and_then(|p| p.inline_data.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No image in Gemini response"))?;

    Ok(ImageResult {
        base64: image_part.data.clone(),
        mime_type: image_part.mime_type.clone(),
    })
}

pub struct ImageResult {
    pub base64: String,
    pub mime_type: String,
}

/// Save base64 image to disk, return the file path relative to data/images/.
pub fn save_image(base64_data: &str, image_id: &str) -> Result<String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64_data)?;
    let filename = format!("{}.png", image_id);
    let path = Path::new("data/images").join(&filename);
    std::fs::create_dir_all("data/images")?;
    std::fs::write(&path, &bytes)?;
    tracing::info!("Saved image: {} ({}KB)", path.display(), bytes.len() / 1024);
    Ok(filename)
}

/// Save base64 image to disk as a compressed JPEG (smaller, better for xAI video API).
/// Returns the JPEG filename relative to data/images/.
pub fn save_image_jpeg(base64_data: &str, image_id: &str) -> Result<String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64_data)?;
    let img = image::load_from_memory(&bytes)?;
    // Resize to max 1024px wide to reduce payload size while keeping quality
    let img = img.resize(1024, 1024, image::imageops::FilterType::Lanczos3);
    let filename = format!("{}.jpg", image_id);
    let path = Path::new("data/images").join(&filename);
    std::fs::create_dir_all("data/images")?;
    img.save_with_format(&path, image::ImageFormat::Jpeg)?;
    let sz = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    tracing::info!("Saved JPEG: {} ({}KB)", path.display(), sz / 1024);
    Ok(filename)
}
