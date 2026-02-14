use crate::services::claude;
use crate::state::AppState;
use anyhow::Result;

/// Generate a stream of consciousness for a thinker, then run the image-only pipeline.
/// If `existing_anky_id` is provided, uses that record instead of creating a new one.
pub async fn generate_for_thinker(
    state: &AppState,
    thinker_name: &str,
    moment: &str,
    collection_id: Option<&str>,
    existing_anky_id: Option<&str>,
) -> Result<String> {
    let api_key = &state.config.anthropic_api_key;

    state.emit_log(
        "INFO",
        "stream_gen",
        &format!("Generating stream for {} at '{}'", thinker_name, moment),
    );

    // Generate the stream of consciousness
    let stream_result =
        claude::generate_stream_for_thinker(api_key, thinker_name, moment).await?;

    let stream_text = stream_result.text.clone();

    // Record cost
    let cost = crate::pipeline::cost::estimate_claude_cost(
        stream_result.input_tokens,
        stream_result.output_tokens,
    );
    {
        let db = state.db.lock().await;
        crate::db::queries::insert_cost_record(
            &db,
            "claude",
            "claude-sonnet-4-20250514",
            stream_result.input_tokens,
            stream_result.output_tokens,
            cost,
            collection_id,
        )?;
    }

    state.emit_log(
        "INFO",
        "stream_gen",
        &format!(
            "Stream generated for {} ({} words, ${:.4})",
            thinker_name,
            stream_text.split_whitespace().count(),
            cost
        ),
    );

    // Save stream to disk
    let stream_id = uuid::Uuid::new_v4().to_string();
    let stream_path = format!("data/streams/{}.txt", stream_id);
    std::fs::create_dir_all("data/streams")?;
    std::fs::write(&stream_path, &stream_text)?;

    // Create a writing session record
    let session_id = uuid::Uuid::new_v4().to_string();
    let word_count = stream_text.split_whitespace().count() as i32;
    {
        let db = state.db.lock().await;
        crate::db::queries::ensure_user(&db, "system")?;
        crate::db::queries::insert_writing_session(
            &db,
            &session_id,
            "system",
            &stream_text,
            480.0, // Simulated 8-minute session
            word_count,
            true,
            None,
        )?;
    }

    // Create Anky record (or use existing one)
    let anky_id = if let Some(id) = existing_anky_id {
        // Update the existing record with writing session link
        {
            let db = state.db.lock().await;
            db.execute(
                "UPDATE ankys SET writing_session_id = ?2 WHERE id = ?1",
                rusqlite::params![id, session_id],
            )?;
        }
        id.to_string()
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        {
            let db = state.db.lock().await;
            crate::db::queries::insert_anky(
                &db,
                &id,
                &session_id,
                "system",
                None, None, None, None, None,
                Some(thinker_name),
                Some(moment),
                "generating",
                "generated",
            )?;
        }
        id
    };

    // Run the image-only pipeline (generated ankys skip reflection/title)
    crate::pipeline::image_gen::generate_image_only(
        state,
        &anky_id,
        &stream_text,
        None,
    )
    .await?;

    Ok(anky_id)
}
