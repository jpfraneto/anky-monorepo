use crate::state::AppState;

/// Run the full memory pipeline after an anky is generated.
/// This is non-blocking — failures are logged but don't affect the user.
///
/// Steps:
/// 1. Embed the writing session for future similarity search
/// 2. Extract structured memories (themes, emotions, patterns, etc.)
/// 3. Store memories with semantic deduplication
/// 4. Update psychological profile every 5th session
pub async fn run_memory_pipeline(
    state: &AppState,
    ollama_base_url: &str,
    anthropic_key: &str,
    user_id: &str,
    writing_session_id: &str,
    writing_text: &str,
) {
    state.emit_log(
        "INFO",
        "memory",
        &format!(
            "Starting memory pipeline for session {}",
            &writing_session_id[..8.min(writing_session_id.len())]
        ),
    );

    // Step 1: Embed the writing session
    match crate::memory::embeddings::embed_text(ollama_base_url, writing_text).await {
        Ok(embedding) => {
            let embed_id = format!("ws-{}", writing_session_id);
            let content_preview: String = writing_text.chars().take(500).collect();
            let db = state.db.lock().await;
            if let Err(e) = crate::memory::embeddings::store_embedding(
                &db,
                &embed_id,
                user_id,
                Some(writing_session_id),
                "writing",
                &content_preview,
                &embedding,
            ) {
                state.emit_log(
                    "WARN",
                    "memory",
                    &format!("Failed to store embedding: {}", e),
                );
            } else {
                state.emit_log("INFO", "memory", "Writing session embedded");
            }
            drop(db);
        }
        Err(e) => {
            state.emit_log("WARN", "memory", &format!("Embedding failed: {}", e));
        }
    }

    // Step 2: Extract structured memories
    let extracted =
        match crate::memory::extraction::extract_memories(
            ollama_base_url,
            &state.config.ollama_model,
            writing_text,
        )
        .await
        {
            Ok(m) => {
                let total = m.themes.len()
                    + m.emotions.len()
                    + m.entities.len()
                    + m.patterns.len()
                    + m.breakthroughs.len()
                    + m.avoidances.len();
                state.emit_log(
                    "INFO",
                    "memory",
                    &format!(
                "Extracted {} memories ({} themes, {} emotions, {} patterns, {} breakthroughs)",
                total, m.themes.len(), m.emotions.len(), m.patterns.len(), m.breakthroughs.len()
            ),
                );
                m
            }
            Err(e) => {
                state.emit_log(
                    "WARN",
                    "memory",
                    &format!("Memory extraction failed: {}", e),
                );
                return;
            }
        };

    // Step 3: Store with dedup (uses Arc<Mutex<Connection>> internally)
    match crate::memory::extraction::store_memories(
        &state.db,
        ollama_base_url,
        user_id,
        writing_session_id,
        &extracted,
    )
    .await
    {
        Ok(stored) => {
            state.emit_log(
                "INFO",
                "memory",
                &format!("{} new memories stored (deduped)", stored),
            );
        }
        Err(e) => {
            state.emit_log(
                "WARN",
                "memory",
                &format!("Failed to store memories: {}", e),
            );
        }
    }

    // Step 4: Update profile every 5th anky session
    let session_count = {
        let db = state.db.lock().await;
        crate::memory::extraction::get_user_session_count(&db, user_id).unwrap_or(0)
    };

    if session_count > 0 && (session_count == 1 || session_count % 5 == 0) {
        state.emit_log(
            "INFO",
            "memory",
            &format!(
                "Updating psychological profile (session #{})",
                session_count
            ),
        );
        if let Err(e) = crate::memory::profile::update_profile(
            &state.db,
            ollama_base_url,
            &state.config.ollama_model,
            user_id,
        )
        .await
        {
            state.emit_log("WARN", "memory", &format!("Profile update failed: {}", e));
        } else {
            state.emit_log("INFO", "memory", "Psychological profile updated");
        }
    }

    state.emit_log("INFO", "memory", "Memory pipeline complete");
}

/// Backfill memory for all existing writing sessions.
pub async fn backfill_memories(
    state: &AppState,
    ollama_base_url: &str,
    anthropic_key: &str,
) -> (usize, usize) {
    let sessions: Vec<(String, String, String)> = {
        let db = state.db.lock().await;
        let mut stmt = match db.prepare(
            "SELECT ws.id, ws.user_id, ws.content
             FROM writing_sessions ws
             WHERE ws.is_anky = 1
             AND ws.id NOT IN (SELECT DISTINCT writing_session_id FROM memory_embeddings WHERE writing_session_id IS NOT NULL)
             ORDER BY ws.created_at ASC",
        ) {
            Ok(s) => s,
            Err(e) => {
                state.emit_log("ERROR", "memory", &format!("Backfill query failed: {}", e));
                return (0, 0);
            }
        };
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap_or_else(|_| panic!("backfill query failed"));
        rows.filter_map(|r| r.ok()).collect()
    };

    let total = sessions.len();
    state.emit_log(
        "INFO",
        "memory",
        &format!("Backfilling memory for {} sessions", total),
    );

    let mut processed = 0;
    for (session_id, user_id, content) in &sessions {
        run_memory_pipeline(
            state,
            ollama_base_url,
            anthropic_key,
            user_id,
            session_id,
            content,
        )
        .await;
        processed += 1;

        if processed % 10 == 0 {
            state.emit_log(
                "INFO",
                "memory",
                &format!("Backfill progress: {}/{}", processed, total),
            );
        }

        // Small delay to avoid rate limits
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    (processed, total)
}
