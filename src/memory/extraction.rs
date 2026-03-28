use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

const EXTRACTION_SYSTEM: &str = r#"You are a psychological pattern extractor for a consciousness journaling app called Anky. You analyze raw stream-of-consciousness writing sessions and extract structured psychological memories.

Your job is to identify:
- THEMES: recurring topics, subjects, areas of focus (e.g., "control", "fatherhood", "creative ambition")
- EMOTIONS: the emotional undercurrents present in the writing (e.g., "quiet anxiety", "suppressed anger", "tentative hope")
- ENTITIES: specific people, places, things that matter to the writer (e.g., "Sarah", "the old apartment", "Tuesday meetings")
- PATTERNS: behavioral or psychological patterns visible in how they write (e.g., "intellectualizes grief", "uses humor to deflect from vulnerability")
- BREAKTHROUGHS: moments of genuine insight or first-time honesty (e.g., "first time naming anger directly")
- AVOIDANCES: things the writing circles around but never addresses head-on (e.g., "never names the loss explicitly", "always deflects from discussing mother")

OUTPUT FORMAT — raw JSON only, no markdown, no explanation:
{
  "themes": ["theme1", "theme2"],
  "emotions": ["emotion1", "emotion2"],
  "entities": ["entity1", "entity2"],
  "patterns": ["pattern1", "pattern2"],
  "breakthroughs": ["breakthrough1"],
  "avoidances": ["avoidance1"]
}

Each item should be a concise phrase (3-10 words). Extract what's genuinely there — don't invent patterns that aren't present. It's fine to return empty arrays for categories with nothing noteworthy."#;

#[derive(Debug, Deserialize, Default)]
pub struct ExtractedMemories {
    #[serde(default)]
    pub themes: Vec<String>,
    #[serde(default)]
    pub emotions: Vec<String>,
    #[serde(default)]
    pub entities: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub breakthroughs: Vec<String>,
    #[serde(default)]
    pub avoidances: Vec<String>,
}

/// Extract structured memories from a writing session using Claude Haiku.
pub async fn extract_memories(api_key: &str, writing: &str) -> Result<ExtractedMemories> {
    let text = crate::services::claude::call_haiku_with_system(api_key, EXTRACTION_SYSTEM, writing)
        .await?;
    let mut trimmed = text.trim();
    if trimmed.starts_with("```") {
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                trimmed = &trimmed[start..=end];
            }
        }
    }

    let memories: ExtractedMemories = serde_json::from_str(trimmed).unwrap_or_default();
    Ok(memories)
}

/// Store extracted memories in the database, deduplicating by exact text match.
/// Takes Arc<Mutex<Connection>> to safely lock/unlock across async boundaries.
pub async fn store_memories(
    db: &Arc<Mutex<Connection>>,
    user_id: &str,
    writing_session_id: &str,
    memories: &ExtractedMemories,
) -> Result<usize> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut stored = 0;

    let items: Vec<(&str, &Vec<String>)> = vec![
        ("theme", &memories.themes),
        ("emotion", &memories.emotions),
        ("entity", &memories.entities),
        ("pattern", &memories.patterns),
        ("breakthrough", &memories.breakthroughs),
        ("avoidance", &memories.avoidances),
    ];

    let conn = db.lock().await;

    for (category, entries) in items {
        for entry in entries {
            if entry.trim().is_empty() {
                continue;
            }

            // Check for exact text match dedup
            let existing: Option<(String, i32)> = conn
                .query_row(
                    "SELECT id, occurrence_count FROM user_memories WHERE user_id = ?1 AND category = ?2 AND content = ?3",
                    params![user_id, category, entry],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)),
                )
                .ok();

            if let Some((existing_id, existing_count)) = existing {
                conn.execute(
                    "UPDATE user_memories SET occurrence_count = ?2, last_seen_at = ?3, importance = MIN(1.0, importance + 0.05)
                     WHERE id = ?1",
                    params![existing_id, existing_count + 1, now],
                )?;
            } else {
                let id = uuid::Uuid::new_v4().to_string();
                let importance = match category {
                    "breakthrough" => 0.8,
                    "pattern" | "avoidance" => 0.6,
                    "theme" | "emotion" => 0.5,
                    "entity" => 0.4,
                    _ => 0.5,
                };

                conn.execute(
                    "INSERT INTO user_memories (id, user_id, writing_session_id, category, content, importance, occurrence_count, first_seen_at, last_seen_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)",
                    params![id, user_id, writing_session_id, category, entry, importance, now],
                )?;
                stored += 1;
            }
        }
    }

    Ok(stored)
}

/// Get the count of anky writing sessions for a user.
pub fn get_user_session_count(conn: &Connection, user_id: &str) -> Result<i32> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM writing_sessions WHERE user_id = ?1 AND is_anky = 1",
        params![user_id],
        |row| row.get(0),
    )?;
    Ok(count)
}
