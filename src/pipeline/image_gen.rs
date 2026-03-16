use crate::db::queries;
use crate::services::{claude, comfyui, gemini};
use crate::state::AppState;
use anyhow::Result;
use std::process::Command;

/// Generate a 400px thumbnail WebP. Returns the thumbnail filename.
fn generate_thumbnail(png_path: &str) -> Result<String> {
    let full_png = format!("data/images/{}", png_path);
    let thumb_filename = png_path.replace(".png", "_thumb.webp");
    let full_thumb = format!("data/images/{}", thumb_filename);

    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &full_png,
            "-vf",
            "scale=400:-1",
            "-quality",
            "80",
            &full_thumb,
        ])
        .output();

    let success = matches!(output, Ok(o) if o.status.success());

    if success && std::path::Path::new(&full_thumb).exists() {
        Ok(thumb_filename)
    } else {
        anyhow::bail!("Thumbnail generation failed for {}", png_path)
    }
}

/// Convert a PNG image to WebP using ffmpeg. Returns the WebP filename.
fn convert_to_webp(png_path: &str) -> Result<String> {
    let full_png = format!("data/images/{}", png_path);
    let webp_filename = png_path.replace(".png", ".webp");
    let full_webp = format!("data/images/{}", webp_filename);

    // Try cwebp first, fall back to ffmpeg
    let output = Command::new("cwebp")
        .args(["-q", "85", &full_png, "-o", &full_webp])
        .output();

    let success = match output {
        Ok(o) if o.status.success() => true,
        _ => {
            // Fallback to ffmpeg
            let ffmpeg = Command::new("ffmpeg")
                .args(["-y", "-i", &full_png, "-quality", "85", &full_webp])
                .output();
            matches!(ffmpeg, Ok(o) if o.status.success())
        }
    };

    if success && std::path::Path::new(&full_webp).exists() {
        Ok(webp_filename)
    } else {
        anyhow::bail!("WebP conversion failed for {}", png_path)
    }
}

/// Image pipeline for generating an Anky from a writing session.
/// Title + reflection are handled by the SSE streaming endpoint.
/// This pipeline only handles:
/// 1. Claude: writing -> image prompt
/// 2. Gemini: prompt + references -> image
/// 3. Fallback: generate title+reflection if SSE didn't set them
/// 4. Save image + mark complete
pub async fn generate_anky_from_writing(
    state: &AppState,
    anky_id: &str,
    writing_session_id: &str,
    _user_id: &str,
    writing_text: &str,
) -> Result<()> {
    let api_key = &state.config.anthropic_api_key;
    let gemini_key = &state.config.gemini_api_key;

    if api_key.is_empty() || gemini_key.is_empty() {
        state.emit_log(
            "WARN",
            "image_gen",
            "API keys not configured, skipping Anky generation",
        );
        return Ok(());
    }

    state.emit_log(
        "INFO",
        "image_gen",
        &format!(
            "Starting image pipeline for session {}",
            &writing_session_id[..8]
        ),
    );

    // Step 1: Generate image prompt (local Qwen) — non-fatal, fall back to raw text
    state.emit_log("INFO", "qwen", "Generating image prompt...");
    let image_prompt = match crate::services::ollama::generate_image_prompt(
        &state.config.ollama_base_url,
        &state.config.ollama_model,
        writing_text,
    )
    .await
    {
        Ok(p) => {
            state.emit_log("INFO", "qwen", "Image prompt ready");
            p
        }
        Err(e) => {
            state.emit_log(
                "WARN",
                "qwen",
                &format!(
                    "Ollama unavailable ({}), using raw writing text as prompt",
                    e
                ),
            );
            writing_text.to_string()
        }
    };

    // Step 2: Generate image with Gemini — fall back to Flux on failure
    state.emit_log("INFO", "gemini", "Generating Anky image...");
    let references = gemini::load_references(std::path::Path::new("src/public"));
    let (image_path, image_model) =
        match gemini::generate_image(gemini_key, &image_prompt, &references)
            .await
            .and_then(|r| gemini::save_image(&r.base64, anky_id))
        {
            Ok(p) => {
                {
                    let db = state.db.lock().await;
                    queries::insert_cost_record(
                        &db,
                        "gemini",
                        "gemini-2.5-flash-image",
                        0,
                        0,
                        0.04,
                        Some(anky_id),
                    )?;
                }
                state.emit_log("INFO", "gemini", &format!("Image saved: {}", p));
                (p, "gemini".to_string())
            }
            Err(e) => {
                state.emit_log(
                    "WARN",
                    "gemini",
                    &format!("Gemini failed ({}), falling back to Flux...", e),
                );
                let image_bytes = comfyui::generate_image(&image_prompt).await?;
                let p = comfyui::save_image(&image_bytes, anky_id)?;
                state.emit_log("INFO", "flux", &format!("Flux image saved: {}", p));
                (p, "flux".to_string())
            }
        };

    // WebP conversion
    match convert_to_webp(&image_path) {
        Ok(webp) => {
            let db = state.db.lock().await;
            let _ = queries::update_anky_webp(&db, anky_id, &webp);
            state.emit_log("INFO", "image_gen", &format!("WebP saved: {}", webp));
        }
        Err(e) => {
            state.emit_log(
                "WARN",
                "image_gen",
                &format!("WebP conversion failed: {}", e),
            );
        }
    }

    // Thumbnail generation
    match generate_thumbnail(&image_path) {
        Ok(thumb) => {
            let db = state.db.lock().await;
            let _ = queries::update_anky_thumb(&db, anky_id, &thumb);
            state.emit_log("INFO", "image_gen", &format!("Thumbnail saved: {}", thumb));
        }
        Err(e) => {
            state.emit_log(
                "WARN",
                "image_gen",
                &format!("Thumbnail generation failed: {}", e),
            );
        }
    }

    // Step 3: Fallback — generate title+reflection if streaming endpoint didn't set them
    {
        let has_reflection = {
            let db = state.db.lock().await;
            let anky = queries::get_anky_by_id(&db, anky_id)?;
            anky.map(|a| a.reflection.as_ref().map_or(false, |r| !r.is_empty()))
                .unwrap_or(false)
        };

        if !has_reflection {
            state.emit_log(
                "INFO",
                "image_gen",
                "No reflection found, generating fallback with memory...",
            );

            // Build memory context for the fallback reflection too
            let memory_ctx = crate::memory::recall::build_memory_context(
                &state.db,
                &state.config.ollama_base_url,
                _user_id,
                writing_text,
            )
            .await
            .ok()
            .map(|ctx| ctx.format_for_prompt());

            let tr = claude::generate_title_and_reflection_with_memory(
                api_key,
                writing_text,
                memory_ctx.as_deref().unwrap_or(""),
            )
            .await?;
            let (title, reflection) = claude::parse_title_reflection(&tr.text);
            let tr_cost =
                crate::pipeline::cost::estimate_claude_cost(tr.input_tokens, tr.output_tokens);
            let db = state.db.lock().await;
            queries::update_anky_title_reflection(&db, anky_id, &title, &reflection)?;
            queries::insert_cost_record(
                &db,
                "claude",
                "claude-sonnet-4-20250514",
                tr.input_tokens,
                tr.output_tokens,
                tr_cost,
                Some(anky_id),
            )?;
        }
    }

    // Step 4: Save image and mark complete
    let caption = image_prompt.clone();
    {
        let db = state.db.lock().await;
        queries::update_anky_image_complete(&db, anky_id, &image_prompt, &image_path, &caption)?;
        let _ = queries::set_anky_image_model(&db, anky_id, &image_model);
    }

    let total_cost = if image_model == "gemini" { 0.04 } else { 0.0 };
    state.emit_log(
        "INFO",
        "image_gen",
        &format!(
            "Pipeline complete for {} (${:.4})",
            &anky_id[..8],
            total_cost
        ),
    );

    // Step 5: Format writing text (Ollama — local, non-blocking)
    {
        let fmt_state = state.clone();
        let fmt_ollama_url = state.config.ollama_base_url.clone();
        let fmt_ollama_model = state.config.ollama_model.clone();
        let fmt_anky_id = anky_id.to_string();
        let fmt_text = writing_text.to_string();
        tokio::spawn(async move {
            let prompt = crate::services::ollama::format_writing_prompt(&fmt_text);
            match crate::services::ollama::call_ollama(&fmt_ollama_url, &fmt_ollama_model, &prompt)
                .await
            {
                Ok(formatted) => {
                    let db = fmt_state.db.lock().await;
                    let _ = db.execute(
                        "UPDATE ankys SET formatted_writing = ?1 WHERE id = ?2",
                        rusqlite::params![&formatted, &fmt_anky_id],
                    );
                    fmt_state.emit_log(
                        "INFO",
                        "format",
                        &format!("Formatted writing saved for {}", &fmt_anky_id[..8]),
                    );
                }
                Err(e) => {
                    fmt_state.emit_log(
                        "WARN",
                        "format",
                        &format!("Writing formatting failed: {}", e),
                    );
                }
            }
        });
    }

    // Step 6: Memory extraction (background, non-blocking, fully local)
    {
        let mem_state = state.clone();
        let mem_ollama_url = state.config.ollama_base_url.clone();
        let mem_anthropic_key = state.config.anthropic_api_key.clone();
        let mem_user_id = _user_id.to_string();
        let mem_session_id = writing_session_id.to_string();
        let mem_text = writing_text.to_string();
        tokio::spawn(async move {
            crate::pipeline::memory_pipeline::run_memory_pipeline(
                &mem_state,
                &mem_ollama_url,
                &mem_anthropic_key,
                &mem_user_id,
                &mem_session_id,
                &mem_text,
            )
            .await;
        });
    }

    Ok(())
}

/// Image-only pipeline for generated ankys. Skips reflection and title.
/// 1. Claude: text -> image prompt (or use pre_prompt if provided)
/// 2. Gemini: prompt + references -> image
/// 3. Save image_prompt + image_path to DB
pub async fn generate_image_only(
    state: &AppState,
    anky_id: &str,
    text: &str,
    pre_prompt: Option<&str>,
) -> Result<()> {
    let api_key = &state.config.anthropic_api_key;
    let gemini_key = &state.config.gemini_api_key;

    if api_key.is_empty() || gemini_key.is_empty() {
        state.emit_log(
            "WARN",
            "image_gen",
            "API keys not configured, skipping generation",
        );
        return Ok(());
    }

    state.emit_log(
        "INFO",
        "image_gen",
        &format!("Starting image-only generation for {}", &anky_id[..8]),
    );

    // Step 1: Generate image prompt (or use pre-enhanced prompt)
    let image_prompt = if let Some(prompt) = pre_prompt {
        state.emit_log("INFO", "qwen", "Using pre-enhanced image prompt");
        prompt.to_string()
    } else {
        state.emit_log("INFO", "qwen", "Generating image prompt from text...");
        let p = crate::services::ollama::generate_image_prompt(
            &state.config.ollama_base_url,
            &state.config.ollama_model,
            text,
        )
        .await?;
        state.emit_log("INFO", "qwen", "Image prompt generated");
        p
    };

    // Step 2: Generate image with Gemini
    state.emit_log("INFO", "gemini", "Generating image...");
    let references = gemini::load_references(std::path::Path::new("src/public"));
    let image_result = if pre_prompt.is_some() {
        // Paid direct prompts should be forwarded to Gemini exactly as provided.
        gemini::generate_image_exact(gemini_key, &image_prompt, &references).await?
    } else {
        gemini::generate_image(gemini_key, &image_prompt, &references).await?
    };

    let image_path = gemini::save_image(&image_result.base64, anky_id)?;

    {
        let db = state.db.lock().await;
        queries::insert_cost_record(
            &db,
            "gemini",
            "gemini-2.5-flash-image",
            0,
            0,
            0.04,
            Some(anky_id),
        )?;
    }
    state.emit_log("INFO", "gemini", &format!("Image saved: {}", image_path));

    // WebP conversion
    match convert_to_webp(&image_path) {
        Ok(webp) => {
            let db = state.db.lock().await;
            let _ = queries::update_anky_webp(&db, anky_id, &webp);
            state.emit_log("INFO", "image_gen", &format!("WebP saved: {}", webp));
        }
        Err(e) => {
            state.emit_log(
                "WARN",
                "image_gen",
                &format!("WebP conversion failed: {}", e),
            );
        }
    }

    // Thumbnail generation
    match generate_thumbnail(&image_path) {
        Ok(thumb) => {
            let db = state.db.lock().await;
            let _ = queries::update_anky_thumb(&db, anky_id, &thumb);
            state.emit_log("INFO", "image_gen", &format!("Thumbnail saved: {}", thumb));
        }
        Err(e) => {
            state.emit_log(
                "WARN",
                "image_gen",
                &format!("Thumbnail generation failed: {}", e),
            );
        }
    }

    // Step 3: Update DB with image only
    {
        let db = state.db.lock().await;
        queries::update_anky_image_only(&db, anky_id, &image_prompt, &image_path)?;
        let _ = queries::set_anky_image_model(&db, anky_id, "gemini");
    }

    let total_cost = 0.04;
    state.emit_log(
        "INFO",
        "image_gen",
        &format!(
            "Generated anky {} complete! Total cost: ${:.4}",
            &anky_id[..8],
            total_cost
        ),
    );

    Ok(())
}

/// Generate a Flux image via ComfyUI at the given URL. Returns raw PNG bytes.
/// Used by the X webhook mention handler to generate Anky images on demand.
pub async fn generate_flux_image(prompt: &str, comfy_url: &str) -> anyhow::Result<Vec<u8>> {
    tracing::info!("generate_flux_image: prompt len={}", prompt.len());
    comfyui::generate_image_at_url(prompt, comfy_url).await
}

/// Image-only pipeline using Flux.1-dev + anky LoRA via ComfyUI (free, local GPU).
/// 1. Claude: text → image prompt
/// 2. ComfyUI: prompt → Flux image
/// 3. Save image
pub async fn generate_image_only_flux(state: &AppState, anky_id: &str, text: &str) -> Result<()> {
    state.emit_log(
        "INFO",
        "flux",
        &format!("Starting Flux pipeline for {}", &anky_id[..8]),
    );

    // Use the raw prompt directly — no transformation
    let image_prompt = text.to_string();

    // Step 1: Generate image via ComfyUI (Flux.1-dev + anky LoRA)
    state.emit_log(
        "INFO",
        "flux",
        "Sending to ComfyUI (Flux.1-dev + anky LoRA)...",
    );
    let image_bytes = comfyui::generate_image(&image_prompt).await?;
    let image_path = comfyui::save_image(&image_bytes, anky_id)?;
    state.emit_log("INFO", "flux", &format!("Flux image saved: {}", image_path));

    // WebP conversion
    match convert_to_webp(&image_path) {
        Ok(webp) => {
            let db = state.db.lock().await;
            let _ = queries::update_anky_webp(&db, anky_id, &webp);
            state.emit_log("INFO", "flux", &format!("WebP saved: {}", webp));
        }
        Err(e) => {
            state.emit_log("WARN", "flux", &format!("WebP conversion failed: {}", e));
        }
    }

    // Thumbnail generation
    match generate_thumbnail(&image_path) {
        Ok(thumb) => {
            let db = state.db.lock().await;
            let _ = queries::update_anky_thumb(&db, anky_id, &thumb);
            state.emit_log("INFO", "flux", &format!("Thumbnail saved: {}", thumb));
        }
        Err(e) => {
            state.emit_log(
                "WARN",
                "flux",
                &format!("Thumbnail generation failed: {}", e),
            );
        }
    }

    // Save to DB
    {
        let db = state.db.lock().await;
        queries::update_anky_image_only(&db, anky_id, &image_prompt, &image_path)?;
        let _ = queries::set_anky_image_model(&db, anky_id, "flux");
    }

    state.emit_log(
        "INFO",
        "flux",
        &format!("Flux pipeline complete for {}", &anky_id[..8]),
    );

    Ok(())
}
