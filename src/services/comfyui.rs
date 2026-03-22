use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

const COMFYUI_URL: &str = "http://127.0.0.1:8188";
// Flux.1-dev needs separate UNet, VAE, and text encoder files
const FLUX_UNET: &str = "flux1-dev.safetensors";
const FLUX_VAE: &str = "ae.safetensors";
const FLUX_CLIP_L: &str = "clip_l.safetensors";
const FLUX_T5: &str = "t5xxl_fp8_e4m3fn.safetensors";
const COMFY_LORAS_DIR: &str = "/home/kithkui/ComfyUI/models/loras";
const DEFAULT_LORA_MODEL: &str = "anky_flux_lora_v2.safetensors";
const FALLBACK_LORA_MODEL: &str = "anky_flux_lora.safetensors";
const LORA_STRENGTH: f64 = 0.85;
const STEPS: u32 = 20;
const GUIDANCE: f64 = 3.5;

/// Build the ComfyUI workflow for Flux.1-dev + anky LoRA.
/// Uses separate UNETLoader + DualCLIPLoader + VAELoader (correct setup for flux1-dev.safetensors).
fn build_workflow(prompt: &str, client_id: &str) -> Value {
    let lora_name = resolve_lora_model_name();
    let prompt = ensure_trigger_word(prompt);
    json!({
        "client_id": client_id,
        "prompt": {
            // 1: Load UNet
            "1": {
                "class_type": "UNETLoader",
                "inputs": {
                    "unet_name": FLUX_UNET,
                    "weight_dtype": "fp8_e4m3fn"
                }
            },
            // 2: Load VAE
            "2": {
                "class_type": "VAELoader",
                "inputs": { "vae_name": FLUX_VAE }
            },
            // 3: Load CLIP (dual: clip_l + t5)
            "3": {
                "class_type": "DualCLIPLoader",
                "inputs": {
                    "clip_name1": FLUX_CLIP_L,
                    "clip_name2": FLUX_T5,
                    "type": "flux"
                }
            },
            // 4: Apply LoRA to UNet + CLIP
            "4": {
                "class_type": "LoraLoader",
                "inputs": {
                    "model": ["1", 0],
                    "clip": ["3", 0],
                    "lora_name": lora_name,
                    "strength_model": LORA_STRENGTH,
                    "strength_clip": LORA_STRENGTH
                }
            },
            // 5: Encode positive prompt
            "5": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "clip": ["4", 1],
                    "text": prompt
                }
            },
            // 6: Empty latent
            "6": {
                "class_type": "EmptyLatentImage",
                "inputs": {
                    "width": 1024,
                    "height": 1024,
                    "batch_size": 1
                }
            },
            // 7: Sample
            "7": {
                "class_type": "KSampler",
                "inputs": {
                    "model": ["4", 0],
                    "positive": ["5", 0],
                    "negative": ["5", 0],
                    "latent_image": ["6", 0],
                    "seed": rand_seed(),
                    "steps": STEPS,
                    "cfg": GUIDANCE,
                    "sampler_name": "euler",
                    "scheduler": "simple",
                    "denoise": 1.0
                }
            },
            // 8: Decode
            "8": {
                "class_type": "VAEDecode",
                "inputs": {
                    "samples": ["7", 0],
                    "vae": ["2", 0]
                }
            },
            // 9: Save
            "9": {
                "class_type": "SaveImage",
                "inputs": {
                    "images": ["8", 0],
                    "filename_prefix": "anky"
                }
            }
        }
    })
}

/// Build the ComfyUI workflow for story phase images.
/// Matches the default Flux workflow except for a vertical phone frame
/// and a distinct ComfyUI filename prefix.
fn build_story_workflow(prompt: &str, client_id: &str) -> Value {
    let lora_name = resolve_lora_model_name();
    let prompt = ensure_trigger_word(prompt);
    json!({
        "client_id": client_id,
        "prompt": {
            "1": {
                "class_type": "UNETLoader",
                "inputs": {
                    "unet_name": FLUX_UNET,
                    "weight_dtype": "fp8_e4m3fn"
                }
            },
            "2": {
                "class_type": "VAELoader",
                "inputs": { "vae_name": FLUX_VAE }
            },
            "3": {
                "class_type": "DualCLIPLoader",
                "inputs": {
                    "clip_name1": FLUX_CLIP_L,
                    "clip_name2": FLUX_T5,
                    "type": "flux"
                }
            },
            "4": {
                "class_type": "LoraLoader",
                "inputs": {
                    "model": ["1", 0],
                    "clip": ["3", 0],
                    "lora_name": lora_name,
                    "strength_model": LORA_STRENGTH,
                    "strength_clip": LORA_STRENGTH
                }
            },
            "5": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "clip": ["4", 1],
                    "text": prompt
                }
            },
            "6": {
                "class_type": "EmptyLatentImage",
                "inputs": {
                    "width": 768,
                    "height": 1344,
                    "batch_size": 1
                }
            },
            "7": {
                "class_type": "KSampler",
                "inputs": {
                    "model": ["4", 0],
                    "positive": ["5", 0],
                    "negative": ["5", 0],
                    "latent_image": ["6", 0],
                    "seed": rand_seed(),
                    "steps": STEPS,
                    "cfg": GUIDANCE,
                    "sampler_name": "euler",
                    "scheduler": "simple",
                    "denoise": 1.0
                }
            },
            "8": {
                "class_type": "VAEDecode",
                "inputs": {
                    "samples": ["7", 0],
                    "vae": ["2", 0]
                }
            },
            "9": {
                "class_type": "SaveImage",
                "inputs": {
                    "images": ["8", 0],
                    "filename_prefix": "anky_story"
                }
            }
        }
    })
}

/// Pick LoRA filename with this priority:
/// 1) COMFYUI_LORA_MODEL env override
/// 2) anky_flux_lora_v2.safetensors if present
/// 3) legacy anky_flux_lora.safetensors
fn resolve_lora_model_name() -> String {
    if let Ok(override_name) = std::env::var("COMFYUI_LORA_MODEL") {
        let trimmed = override_name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let preferred = Path::new(COMFY_LORAS_DIR).join(DEFAULT_LORA_MODEL);
    if preferred.exists() {
        return DEFAULT_LORA_MODEL.to_string();
    }

    FALLBACK_LORA_MODEL.to_string()
}

fn rand_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ (d.as_secs() * 6364136223846793005))
        .unwrap_or(42)
}

/// Check if ComfyUI is reachable.
pub async fn is_available() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    client
        .get(format!("{}/system_stats", COMFYUI_URL))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Generate an image using Flux.1-dev + anky LoRA via ComfyUI.
/// Returns PNG bytes.
pub async fn generate_image(prompt: &str) -> Result<Vec<u8>> {
    generate_image_at_url(prompt, COMFYUI_URL).await
}

/// Generate a vertical story image using the existing Flux + LoRA workflow.
pub async fn generate_story_image(prompt: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let client_id = Uuid::new_v4().to_string();
    let workflow = build_story_workflow(prompt, &client_id);

    let resp = client
        .post(format!("{}/prompt", COMFYUI_URL))
        .json(&workflow)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("ComfyUI queue failed: {}", body));
    }

    let queue_resp: Value = resp.json().await?;
    let prompt_id = queue_resp["prompt_id"]
        .as_str()
        .ok_or_else(|| anyhow!("No prompt_id in ComfyUI response"))?
        .to_string();

    for _ in 0..120 {
        sleep(Duration::from_secs(2)).await;

        let history_resp = client
            .get(format!("{}/history/{}", COMFYUI_URL, prompt_id))
            .send()
            .await?;

        if !history_resp.status().is_success() {
            continue;
        }

        let history: Value = history_resp.json().await?;
        let entry = &history[&prompt_id];
        if entry.is_null() {
            continue;
        }

        if let Some(status) = entry["status"].as_object() {
            if let Some(msgs) = status.get("messages").and_then(|m| m.as_array()) {
                for msg in msgs {
                    if msg[0].as_str() == Some("execution_error") {
                        let err = msg[1]["exception_message"]
                            .as_str()
                            .unwrap_or("unknown error");
                        return Err(anyhow!("ComfyUI execution error: {}", err));
                    }
                }
            }
        }

        let outputs = &entry["outputs"];
        let mut image_filename = None;
        if let Some(obj) = outputs.as_object() {
            for (_node_id, output) in obj {
                if let Some(images) = output["images"].as_array() {
                    if let Some(img) = images.first() {
                        image_filename = img["filename"].as_str().map(|s| s.to_string());
                        break;
                    }
                }
            }
        }

        let filename = match image_filename {
            Some(f) => f,
            None => continue,
        };

        let img_resp = client
            .get(format!(
                "{}/view?filename={}&type=output",
                COMFYUI_URL, filename
            ))
            .send()
            .await?;

        if img_resp.status().is_success() {
            return Ok(img_resp.bytes().await?.to_vec());
        }
    }

    Err(anyhow!("ComfyUI generation timed out after 240s"))
}

/// Ensure the LoRA trigger word "anky" is present in the prompt.
/// If missing, prepend it so the LoRA style activates.
fn ensure_trigger_word(prompt: &str) -> String {
    if prompt.to_lowercase().contains("anky") {
        prompt.to_string()
    } else {
        format!("anky, {}", prompt)
    }
}

/// Like `generate_image` but uses a caller-supplied ComfyUI base URL.
pub async fn generate_image_at_url(prompt: &str, comfy_url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let client_id = Uuid::new_v4().to_string();
    let workflow = build_workflow(prompt, &client_id);

    // Queue the prompt
    let resp = client
        .post(format!("{}/prompt", comfy_url))
        .json(&workflow)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("ComfyUI queue failed: {}", body));
    }

    let queue_resp: Value = resp.json().await?;
    let prompt_id = queue_resp["prompt_id"]
        .as_str()
        .ok_or_else(|| anyhow!("No prompt_id in ComfyUI response"))?
        .to_string();

    // Poll for completion
    for _ in 0..120 {
        sleep(Duration::from_secs(2)).await;

        let history_resp = client
            .get(format!("{}/history/{}", comfy_url, prompt_id))
            .send()
            .await?;

        if !history_resp.status().is_success() {
            continue;
        }

        let history: Value = history_resp.json().await?;
        let entry = &history[&prompt_id];
        if entry.is_null() {
            continue;
        }

        // Check for execution errors reported by ComfyUI
        if let Some(status) = entry["status"].as_object() {
            if let Some(msgs) = status.get("messages").and_then(|m| m.as_array()) {
                for msg in msgs {
                    if msg[0].as_str() == Some("execution_error") {
                        let err = msg[1]["exception_message"]
                            .as_str()
                            .unwrap_or("unknown error");
                        return Err(anyhow!("ComfyUI execution error: {}", err));
                    }
                }
            }
        }

        // Find the output image filename
        let outputs = &entry["outputs"];
        let mut image_filename = None;
        if let Some(obj) = outputs.as_object() {
            for (_node_id, output) in obj {
                if let Some(images) = output["images"].as_array() {
                    if let Some(img) = images.first() {
                        image_filename = img["filename"].as_str().map(|s| s.to_string());
                        break;
                    }
                }
            }
        }

        let filename = match image_filename {
            Some(f) => f,
            None => continue,
        };

        // Fetch the image bytes
        let img_resp = client
            .get(format!(
                "{}/view?filename={}&type=output",
                comfy_url, filename
            ))
            .send()
            .await?;

        if img_resp.status().is_success() {
            return Ok(img_resp.bytes().await?.to_vec());
        }
    }

    Err(anyhow!("ComfyUI generation timed out after 240s"))
}

/// Generate image and return base64-encoded PNG.
pub async fn generate_image_b64(prompt: &str) -> Result<String> {
    let bytes = generate_image(prompt).await?;
    Ok(B64.encode(&bytes))
}

/// Save image bytes to data/images/{anky_id}.png and return the filename.
pub fn save_image(bytes: &[u8], anky_id: &str) -> Result<String> {
    let filename = format!("{}.png", anky_id);
    let path = format!("data/images/{}", filename);
    std::fs::create_dir_all("data/images")?;
    std::fs::write(&path, bytes)?;
    Ok(filename)
}

/// Save story image bytes to data/anky-images/{cuentacuentos_id}/{phase_index}.png.
pub fn save_story_image(
    bytes: Vec<u8>,
    cuentacuentos_id: &str,
    phase_index: usize,
) -> Result<String> {
    let dir = Path::new("data/anky-images").join(cuentacuentos_id);
    let path = dir.join(format!("{}.png", phase_index));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&path, bytes)?;
    Ok(format!(
        "https://anky.app/data/anky-images/{}/{}.png",
        cuentacuentos_id, phase_index
    ))
}
