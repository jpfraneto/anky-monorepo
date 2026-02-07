use crate::db::queries;
use crate::services::{claude, gemini};
use crate::state::AppState;
use anyhow::Result;

/// Full pipeline for generating an Anky from a writing session.
/// 1. Claude: writing -> image prompt
/// 2. Claude: writing -> reflection
/// 3. Claude: writing + prompt + reflection -> title
/// 4. Gemini: prompt + references -> image
/// 5. Save everything to DB
pub async fn generate_anky_from_writing(
    state: &AppState,
    anky_id: &str,
    writing_session_id: &str,
    user_id: &str,
    writing_text: &str,
) -> Result<()> {
    let api_key = &state.config.anthropic_api_key;
    let gemini_key = &state.config.gemini_api_key;

    if api_key.is_empty() || gemini_key.is_empty() {
        state.emit_log("WARN", "image_gen", "API keys not configured, skipping Anky generation");
        return Ok(());
    }

    state.emit_log("INFO", "image_gen", &format!("Starting Anky generation for session {}", &writing_session_id[..8]));

    // Step 1: Generate image prompt
    state.emit_log("INFO", "claude", "Generating image prompt from writing...");
    let prompt_result = claude::generate_prompt(api_key, writing_text).await?;
    let image_prompt = prompt_result.text.clone();

    // Record cost
    let prompt_cost = crate::pipeline::cost::estimate_claude_cost(
        prompt_result.input_tokens,
        prompt_result.output_tokens,
    );
    {
        let db = state.db.lock().await;
        queries::insert_cost_record(&db, "claude", "claude-sonnet-4-20250514", prompt_result.input_tokens, prompt_result.output_tokens, prompt_cost, Some(anky_id))?;
    }
    state.emit_log("INFO", "claude", &format!("Image prompt generated (${:.4})", prompt_cost));

    // Step 2: Generate reflection
    state.emit_log("INFO", "claude", "Generating reflection...");
    let reflection_result = claude::generate_reflection(api_key, writing_text).await?;
    let reflection = reflection_result.text.clone();

    let refl_cost = crate::pipeline::cost::estimate_claude_cost(
        reflection_result.input_tokens,
        reflection_result.output_tokens,
    );
    {
        let db = state.db.lock().await;
        queries::insert_cost_record(&db, "claude", "claude-sonnet-4-20250514", reflection_result.input_tokens, reflection_result.output_tokens, refl_cost, Some(anky_id))?;
    }
    state.emit_log("INFO", "claude", &format!("Reflection generated (${:.4})", refl_cost));

    // Step 3: Generate title
    state.emit_log("INFO", "claude", "Generating title...");
    let title_result = claude::generate_title(api_key, writing_text, &image_prompt, &reflection).await?;
    let title = title_result.text.trim().to_lowercase().replace(['\'', '"'], "");

    let title_cost = crate::pipeline::cost::estimate_claude_cost(
        title_result.input_tokens,
        title_result.output_tokens,
    );
    {
        let db = state.db.lock().await;
        queries::insert_cost_record(&db, "claude", "claude-sonnet-4-20250514", title_result.input_tokens, title_result.output_tokens, title_cost, Some(anky_id))?;
    }
    state.emit_log("INFO", "claude", &format!("Title: '{}' (${:.4})", title, title_cost));

    // Step 4: Generate image with Gemini
    state.emit_log("INFO", "gemini", "Generating Anky image...");
    let references = gemini::load_references(std::path::Path::new("static/references"));
    let image_result = gemini::generate_image(gemini_key, &image_prompt, &references).await?;

    // Save image to disk
    let image_path = gemini::save_image(&image_result.base64, anky_id)?;

    {
        let db = state.db.lock().await;
        queries::insert_cost_record(&db, "gemini", "gemini-2.5-flash-image", 0, 0, 0.04, Some(anky_id))?;
    }
    state.emit_log("INFO", "gemini", &format!("Image saved: {}", image_path));

    // Step 5: Update Anky record
    let caption = format!("{} — {}", title, image_prompt);
    {
        let db = state.db.lock().await;
        queries::update_anky_fields(&db, anky_id, &image_prompt, &reflection, &title, &image_path, &caption)?;
    }

    let total_cost = prompt_cost + refl_cost + title_cost + 0.04;
    state.emit_log("INFO", "image_gen", &format!("Anky '{}' complete! Total cost: ${:.4}", title, total_cost));

    Ok(())
}
