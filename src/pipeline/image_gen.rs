use crate::db::queries;
use crate::services::{claude, gemini};
use crate::state::AppState;
use anyhow::Result;

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
        state.emit_log("WARN", "image_gen", "API keys not configured, skipping Anky generation");
        return Ok(());
    }

    state.emit_log("INFO", "image_gen", &format!("Starting image pipeline for session {}", &writing_session_id[..8]));

    // Step 1: Generate image prompt
    state.emit_log("INFO", "claude", "Generating image prompt...");
    let prompt_result = claude::generate_prompt(api_key, writing_text).await?;
    let image_prompt = prompt_result.text.clone();

    let prompt_cost = crate::pipeline::cost::estimate_claude_cost(
        prompt_result.input_tokens,
        prompt_result.output_tokens,
    );
    {
        let db = state.db.lock().await;
        queries::insert_cost_record(&db, "claude", "claude-sonnet-4-20250514", prompt_result.input_tokens, prompt_result.output_tokens, prompt_cost, Some(anky_id))?;
    }
    state.emit_log("INFO", "claude", &format!("Image prompt ready (${:.4})", prompt_cost));

    // Step 2: Generate image with Gemini
    state.emit_log("INFO", "gemini", "Generating Anky image...");
    let references = gemini::load_references(std::path::Path::new("src/public"));
    let image_result = gemini::generate_image(gemini_key, &image_prompt, &references).await?;
    let image_path = gemini::save_image(&image_result.base64, anky_id)?;

    {
        let db = state.db.lock().await;
        queries::insert_cost_record(&db, "gemini", "gemini-2.5-flash-image", 0, 0, 0.04, Some(anky_id))?;
    }
    state.emit_log("INFO", "gemini", &format!("Image saved: {}", image_path));

    // Step 3: Fallback — generate title+reflection if streaming endpoint didn't set them
    {
        let has_reflection = {
            let db = state.db.lock().await;
            let anky = queries::get_anky_by_id(&db, anky_id)?;
            anky.map(|a| a.reflection.as_ref().map_or(false, |r| !r.is_empty())).unwrap_or(false)
        };

        if !has_reflection {
            state.emit_log("INFO", "image_gen", "No reflection found, generating fallback...");
            let tr = claude::generate_title_and_reflection(api_key, writing_text).await?;
            let (title, reflection) = claude::parse_title_reflection(&tr.text);
            let tr_cost = crate::pipeline::cost::estimate_claude_cost(tr.input_tokens, tr.output_tokens);
            let db = state.db.lock().await;
            queries::update_anky_title_reflection(&db, anky_id, &title, &reflection)?;
            queries::insert_cost_record(&db, "claude", "claude-sonnet-4-20250514", tr.input_tokens, tr.output_tokens, tr_cost, Some(anky_id))?;
        }
    }

    // Step 4: Save image and mark complete
    let caption = image_prompt.clone();
    {
        let db = state.db.lock().await;
        queries::update_anky_image_complete(&db, anky_id, &image_prompt, &image_path, &caption)?;
    }

    let total_cost = prompt_cost + 0.04;
    state.emit_log("INFO", "image_gen", &format!("Pipeline complete for {} (${:.4})", &anky_id[..8], total_cost));

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
        state.emit_log("WARN", "image_gen", "API keys not configured, skipping generation");
        return Ok(());
    }

    state.emit_log("INFO", "image_gen", &format!("Starting image-only generation for {}", &anky_id[..8]));

    // Step 1: Generate image prompt (or use pre-enhanced prompt)
    let (image_prompt, prompt_cost) = if let Some(prompt) = pre_prompt {
        state.emit_log("INFO", "claude", "Using pre-enhanced image prompt");
        (prompt.to_string(), 0.0)
    } else {
        state.emit_log("INFO", "claude", "Generating image prompt from text...");
        let prompt_result = claude::generate_prompt(api_key, text).await?;
        let cost = crate::pipeline::cost::estimate_claude_cost(
            prompt_result.input_tokens,
            prompt_result.output_tokens,
        );
        {
            let db = state.db.lock().await;
            queries::insert_cost_record(&db, "claude", "claude-sonnet-4-20250514", prompt_result.input_tokens, prompt_result.output_tokens, cost, Some(anky_id))?;
        }
        state.emit_log("INFO", "claude", &format!("Image prompt generated (${:.4})", cost));
        (prompt_result.text.clone(), cost)
    };

    // Step 2: Generate image with Gemini
    state.emit_log("INFO", "gemini", "Generating image...");
    let references = gemini::load_references(std::path::Path::new("src/public"));
    let image_result = gemini::generate_image(gemini_key, &image_prompt, &references).await?;

    let image_path = gemini::save_image(&image_result.base64, anky_id)?;

    {
        let db = state.db.lock().await;
        queries::insert_cost_record(&db, "gemini", "gemini-2.5-flash-image", 0, 0, 0.04, Some(anky_id))?;
    }
    state.emit_log("INFO", "gemini", &format!("Image saved: {}", image_path));

    // Step 3: Update DB with image only
    {
        let db = state.db.lock().await;
        queries::update_anky_image_only(&db, anky_id, &image_prompt, &image_path)?;
    }

    let total_cost = prompt_cost + 0.04;
    state.emit_log("INFO", "image_gen", &format!("Generated anky {} complete! Total cost: ${:.4}", &anky_id[..8], total_cost));

    Ok(())
}
