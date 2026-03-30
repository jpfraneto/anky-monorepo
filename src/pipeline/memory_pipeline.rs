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
    _ollama_base_url: &str,
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

    // Step 1: Embedding dropped (local embeddings removed)

    // Step 2: Extract structured memories
    let extracted =
        match crate::memory::extraction::extract_memories(anthropic_key, writing_text).await {
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
        let Some(db) = crate::db::get_conn_logged(&state.db) else {
            return;
        };
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

        if crate::services::honcho::is_configured(&state.config) {
            // Use Honcho's peer model to populate all four profile fields
            state.emit_log("INFO", "memory", "Using Honcho for profile generation");
            let api_key = &state.config.honcho_api_key;
            let workspace_id = &state.config.honcho_workspace_id;

            let prompts = [
                ("psychological_profile", "Write a psychological profile of this person in under 400 words. Cover their core themes, emotional signature, communication style. Use their actual words and patterns."),
                ("core_tensions", "What are this person's core tensions and internal contradictions? 3-5 tensions, each one sentence."),
                ("growth_edges", "Where is this person evolving or struggling to evolve? 3-5 growth edges, each one sentence."),
                ("emotional_signature", "Describe this person's emotional signature in 2-3 sentences. The dominant emotional texture of their writing."),
            ];

            let mut profile_text = String::new();
            let mut core_tensions = String::new();
            let mut growth_edges = String::new();
            let mut emotional_signature = String::new();

            for (field, query) in &prompts {
                match crate::services::honcho::chat_about_peer(
                    api_key,
                    workspace_id,
                    user_id,
                    query,
                )
                .await
                {
                    Ok(response) => {
                        let response = response.trim().to_string();
                        if !response.is_empty() {
                            match *field {
                                "psychological_profile" => profile_text = response,
                                "core_tensions" => core_tensions = response,
                                "growth_edges" => growth_edges = response,
                                "emotional_signature" => emotional_signature = response,
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Honcho chat for {} failed: {}", field, e);
                    }
                }
            }

            // Write results to DB (only overwrite non-empty responses)
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let Some(db) = crate::db::get_conn_logged(&state.db) else {
                return;
            };
            if !profile_text.is_empty() {
                let _ = db.execute(
                    "INSERT INTO user_profiles (user_id, psychological_profile, last_profile_update, updated_at)
                     VALUES (?1, ?2, ?3, ?3)
                     ON CONFLICT(user_id) DO UPDATE SET
                        psychological_profile = excluded.psychological_profile,
                        last_profile_update = excluded.last_profile_update,
                        updated_at = excluded.updated_at",
                    crate::params![user_id, profile_text, now],
                );
            }
            if !core_tensions.is_empty() {
                let _ = db.execute(
                    "UPDATE user_profiles SET core_tensions = ?1, updated_at = ?2 WHERE user_id = ?3",
                    crate::params![core_tensions, now, user_id],
                );
            }
            if !growth_edges.is_empty() {
                let _ = db.execute(
                    "UPDATE user_profiles SET growth_edges = ?1, updated_at = ?2 WHERE user_id = ?3",
                    crate::params![growth_edges, now, user_id],
                );
            }
            if !emotional_signature.is_empty() {
                let _ = db.execute(
                    "UPDATE user_profiles SET emotional_signature = ?1, updated_at = ?2 WHERE user_id = ?3",
                    crate::params![emotional_signature, now, user_id],
                );
            }
            state.emit_log("INFO", "memory", "Honcho-powered profile updated");
        } else {
            // Fallback: existing Ollama profile update
            if let Err(e) =
                crate::memory::profile::update_profile(&state.db, anthropic_key, user_id).await
            {
                state.emit_log("WARN", "memory", &format!("Profile update failed: {}", e));
            } else {
                state.emit_log("INFO", "memory", "Psychological profile updated");
            }
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
        let Some(db) = crate::db::get_conn_logged(&state.db) else {
            return (0, 0);
        };
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
            .query_map(crate::params![], |row| {
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
