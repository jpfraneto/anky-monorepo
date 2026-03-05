use anyhow::Result;
use rusqlite::{params, Connection};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::memory::embeddings;

/// A recalled memory for context injection.
#[derive(Debug)]
pub struct RecalledMemory {
    pub content: String,
    pub source: String,
    pub score: f32,
}

/// A structured memory pattern from user_memories.
#[derive(Debug)]
pub struct MemoryPattern {
    pub category: String,
    pub content: String,
    pub occurrence_count: i32,
    pub importance: f64,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

/// Full memory context assembled for injection into Claude's prompt.
#[derive(Debug)]
pub struct MemoryContext {
    pub profile: Option<String>,
    pub patterns: Vec<MemoryPattern>,
    pub similar_moments: Vec<RecalledMemory>,
    pub session_count: i32,
}

impl MemoryContext {
    /// Format the memory context as a string for injection into a system prompt.
    pub fn format_for_prompt(&self) -> String {
        if self.session_count == 0 {
            return String::new();
        }

        let mut parts = Vec::new();

        parts.push(format!(
            "This person has written {} stream-of-consciousness sessions with you.",
            self.session_count
        ));

        if let Some(ref profile) = self.profile {
            if !profile.is_empty() {
                parts.push(format!("\n## What you know about this person\n{}", profile));
            }
        }

        let significant_patterns: Vec<&MemoryPattern> = self
            .patterns
            .iter()
            .filter(|p| p.occurrence_count >= 2 || p.importance >= 0.7)
            .collect();

        if !significant_patterns.is_empty() {
            let mut pattern_section = "\n## Recurring patterns across their writing\n".to_string();
            for p in significant_patterns.iter().take(8) {
                let freq = if p.occurrence_count >= 5 {
                    "deeply recurring"
                } else if p.occurrence_count >= 3 {
                    "recurring"
                } else {
                    "emerging"
                };
                pattern_section.push_str(&format!(
                    "- [{}] {} ({}×, {})\n",
                    p.category, p.content, p.occurrence_count, freq
                ));
            }
            parts.push(pattern_section);
        }

        if !self.similar_moments.is_empty() {
            let mut moments_section = "\n## Relevant past writing moments\n".to_string();
            for m in self.similar_moments.iter().take(3) {
                let snippet: String = m.content.chars().take(200).collect();
                let snippet = if m.content.len() > 200 {
                    format!("{}...", snippet)
                } else {
                    snippet
                };
                moments_section.push_str(&format!("- (score {:.2}): \"{}\"\n", m.score, snippet));
            }
            parts.push(moments_section);
        }

        if !parts.is_empty() {
            parts.push(
                "\n## How to use this context\nUse this context to make your reflection DEEPLY personal. Reference their journey. Name patterns you see evolving. If something appears for the first time, note it. If something keeps recurring, name it directly. The person should feel KNOWN — like you remember them.".to_string()
            );
        }

        parts.join("\n")
    }
}

/// Build a complete memory context for a user, given their new writing.
/// Takes Arc<Mutex<Connection>> to safely lock/unlock across async boundaries.
pub async fn build_memory_context(
    db: &Arc<Mutex<Connection>>,
    ollama_base_url: &str,
    user_id: &str,
    new_writing: &str,
) -> Result<MemoryContext> {
    // 1. Sync DB reads — lock, read, release
    let (session_count, profile, patterns) = {
        let conn = db.lock().await;

        let session_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM writing_sessions WHERE user_id = ?1 AND is_anky = 1",
            params![user_id],
            |row| row.get(0),
        )?;

        if session_count == 0 {
            return Ok(MemoryContext {
                profile: None,
                patterns: Vec::new(),
                similar_moments: Vec::new(),
                session_count: 0,
            });
        }

        let profile = get_user_profile(&conn, user_id)?;
        let patterns = get_significant_patterns(&conn, user_id, 12)?;

        (session_count, profile, patterns)
    }; // conn dropped here

    // 2. Async embedding call (no conn held)
    let similar_moments = {
        match embeddings::embed_text(ollama_base_url, new_writing).await {
            Ok(query_embedding) => {
                // 3. Lock again for vector search
                let conn = db.lock().await;
                let results = embeddings::search_similar(&conn, user_id, &query_embedding, 5, 0.3)?;
                results
                    .into_iter()
                    .map(|(_, _, source, content, score)| RecalledMemory {
                        content,
                        source,
                        score,
                    })
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    };

    Ok(MemoryContext {
        profile,
        patterns,
        similar_moments,
        session_count,
    })
}

fn get_user_profile(conn: &Connection, user_id: &str) -> Result<Option<String>> {
    let mut stmt =
        conn.prepare("SELECT psychological_profile FROM user_profiles WHERE user_id = ?1")?;
    let mut rows = stmt.query_map(params![user_id], |row| row.get::<_, Option<String>>(0))?;
    Ok(rows.next().and_then(|r| r.ok()).flatten())
}

fn get_significant_patterns(
    conn: &Connection,
    user_id: &str,
    limit: usize,
) -> Result<Vec<MemoryPattern>> {
    let mut stmt = conn.prepare(
        "SELECT category, content, occurrence_count, importance, first_seen_at, last_seen_at
         FROM user_memories
         WHERE user_id = ?1
         ORDER BY (importance * occurrence_count) DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![user_id, limit as i32], |row| {
        Ok(MemoryPattern {
            category: row.get(0)?,
            content: row.get(1)?,
            occurrence_count: row.get(2)?,
            importance: row.get(3)?,
            first_seen_at: row.get(4)?,
            last_seen_at: row.get(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
