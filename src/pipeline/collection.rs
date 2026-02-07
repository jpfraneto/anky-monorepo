use crate::db::queries;
use crate::pipeline::stream_gen;
use crate::state::AppState;
use anyhow::Result;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Being {
    pub name: String,
    pub moment: String,
}

/// Parse the mega-prompt into 88 beings using Claude.
pub async fn expand_beings(state: &AppState, mega_prompt: &str) -> Result<Vec<Being>> {
    let api_key = &state.config.anthropic_api_key;
    let system = r#"You are parsing a mega-prompt that describes 88 beings (thinkers, creators, visionaries) at specific moments in their lives. Extract each being as a JSON array.

Each entry should have:
- "name": The person's full name
- "moment": A brief description of the specific moment

If the prompt describes fewer than 88, extrapolate similar beings to reach 88 total. If it describes more, take the first 88.

OUTPUT: A JSON array only. No markdown, no explanation."#;

    let result = crate::services::claude::generate_prompt(api_key, &format!("SYSTEM: {}\n\nMEGA-PROMPT:\n{}", system, mega_prompt)).await?;

    let beings: Vec<Being> = serde_json::from_str(&result.text)
        .unwrap_or_else(|_| {
            // Fallback: try to extract JSON from response
            if let Some(start) = result.text.find('[') {
                if let Some(end) = result.text.rfind(']') {
                    serde_json::from_str(&result.text[start..=end]).unwrap_or_default()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        });

    Ok(beings)
}

/// Generate a full collection of 88 Anky images sequentially.
pub async fn generate_collection(
    state: &AppState,
    collection_id: &str,
    beings: &[Being],
) -> Result<()> {
    state.emit_log(
        "INFO",
        "collection",
        &format!("Starting collection {} with {} beings", &collection_id[..8], beings.len()),
    );

    {
        let db = state.db.lock().await;
        queries::update_collection_status(&db, collection_id, "generating")?;
    }

    for (i, being) in beings.iter().enumerate() {
        state.emit_log(
            "INFO",
            "collection",
            &format!("[{}/{}] Generating: {} — {}", i + 1, beings.len(), being.name, being.moment),
        );

        match stream_gen::generate_for_thinker(
            state,
            &being.name,
            &being.moment,
            Some(collection_id),
        )
        .await
        {
            Ok(anky_id) => {
                state.emit_log(
                    "INFO",
                    "collection",
                    &format!("[{}/{}] Complete: {} (anky {})", i + 1, beings.len(), being.name, &anky_id[..8]),
                );
            }
            Err(e) => {
                state.emit_log(
                    "ERROR",
                    "collection",
                    &format!("[{}/{}] Failed: {} — {}", i + 1, beings.len(), being.name, e),
                );
            }
        }

        // Update progress
        {
            let db = state.db.lock().await;
            queries::update_collection_progress(&db, collection_id, (i + 1) as i32)?;
        }
    }

    {
        let db = state.db.lock().await;
        queries::update_collection_status(&db, collection_id, "complete")?;
    }

    state.emit_log(
        "INFO",
        "collection",
        &format!("Collection {} complete!", &collection_id[..8]),
    );

    Ok(())
}
