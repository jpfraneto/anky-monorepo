use crate::db::Connection;
use anyhow::Result;

use crate::memory::recall::MemoryPattern;

const PROFILE_UPDATE_SYSTEM: &str = r#"You are building an evolving psychological portrait of a person based on their stream-of-consciousness writing sessions with Anky (a consciousness mirror app).

You will receive:
1. Their CURRENT psychological profile (may be empty if this is the first build)
2. Their RECURRING PATTERNS extracted from recent writing sessions
3. Their RECENT WRITING SUMMARIES

Your job is to write an updated psychological profile — a living document that captures WHO this person is based on what they reveal through writing. This is NOT a clinical assessment. It's an intimate portrait.

Include sections for:
- **Core themes**: what they keep coming back to
- **Emotional signature**: their dominant emotional patterns
- **Growth edges**: where they're actively evolving or struggling
- **Core tensions**: the contradictions and conflicts that define their inner landscape
- **Communication style**: how they express themselves (do they intellectualize? use humor as defense? write in fragments?)

Keep it under 400 words. Be specific — use their actual words and patterns, not generic psychology. This profile will be injected into future reflections so Anky can remember them deeply.

OUTPUT: Just the profile text in markdown. No JSON, no preamble."#;

/// Update (or create) a user's psychological profile based on accumulated memories.
pub async fn update_profile(db: &crate::db::DbPool, api_key: &str, user_id: &str) -> Result<()> {
    // 1. Sync DB reads — lock, read, release
    let (current_profile, patterns, recent_writings, total_sessions, total_words) = {
        let conn = crate::db::conn(db)?;

        let current_profile = get_current_profile(&conn, user_id)?;
        let patterns = get_all_patterns(&conn, user_id)?;
        if patterns.is_empty() {
            return Ok(());
        }
        let recent_writings = get_recent_writing_snippets(&conn, user_id, 5)?;

        let total_sessions: i32 = conn.query_row(
            "SELECT COUNT(*) FROM writing_sessions WHERE user_id = ?1 AND is_anky = 1",
            crate::params![user_id],
            |row| row.get(0),
        )?;
        let total_words: i64 = conn.query_row(
            "SELECT COALESCE(SUM(word_count), 0) FROM writing_sessions WHERE user_id = ?1",
            crate::params![user_id],
            |row| row.get(0),
        )?;

        (
            current_profile,
            patterns,
            recent_writings,
            total_sessions,
            total_words,
        )
    }; // conn dropped

    // 2. Build the prompt
    let mut user_msg = String::new();

    if let Some(ref profile) = current_profile {
        user_msg.push_str(&format!("CURRENT PROFILE:\n{}\n\n", profile));
    } else {
        user_msg.push_str("CURRENT PROFILE: (none yet — first build)\n\n");
    }

    user_msg.push_str("RECURRING PATTERNS:\n");
    for p in &patterns {
        user_msg.push_str(&format!(
            "- [{}] {} (seen {}×, importance {:.1})\n",
            p.category, p.content, p.occurrence_count, p.importance
        ));
    }

    user_msg.push_str("\nRECENT WRITING SNIPPETS:\n");
    for (i, (snippet, date)) in recent_writings.iter().enumerate() {
        user_msg.push_str(&format!(
            "Session {} ({}):\n\"{}\"\n\n",
            i + 1,
            date,
            snippet
        ));
    }

    // 3. Async Haiku call (no conn held)
    let profile_text =
        crate::services::claude::call_haiku_with_system(api_key, PROFILE_UPDATE_SYSTEM, &user_msg)
            .await?
            .trim()
            .to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 4. Sync DB write — lock, write, release
    {
        let conn = crate::db::conn(db)?;
        conn.execute(
            "INSERT INTO user_profiles (user_id, total_sessions, total_anky_sessions, total_words_written, psychological_profile, last_profile_update, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(user_id) DO UPDATE SET
                total_sessions = excluded.total_sessions,
                total_anky_sessions = excluded.total_anky_sessions,
                total_words_written = excluded.total_words_written,
                psychological_profile = excluded.psychological_profile,
                last_profile_update = excluded.last_profile_update,
                updated_at = excluded.updated_at",
            crate::params![user_id, total_sessions, total_sessions, total_words as i32, profile_text, now],
        )?;
    }

    Ok(())
}

fn get_current_profile(conn: &Connection, user_id: &str) -> Result<Option<String>> {
    let mut stmt =
        conn.prepare("SELECT psychological_profile FROM user_profiles WHERE user_id = ?1")?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        row.get::<_, Option<String>>(0)
    })?;
    Ok(rows.next().and_then(|r| r.ok()).flatten())
}

fn get_all_patterns(conn: &Connection, user_id: &str) -> Result<Vec<MemoryPattern>> {
    let mut stmt = conn.prepare(
        "SELECT category, content, occurrence_count, importance, first_seen_at, last_seen_at
         FROM user_memories
         WHERE user_id = ?1
         ORDER BY (importance * occurrence_count) DESC
         LIMIT 30",
    )?;
    let rows = stmt.query_map(crate::params![user_id], |row| {
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

fn get_recent_writing_snippets(
    conn: &Connection,
    user_id: &str,
    limit: usize,
) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT content, created_at FROM writing_sessions
         WHERE user_id = ?1 AND is_anky = 1
         ORDER BY created_at DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(crate::params![user_id, limit as i32], |row| {
        let content: String = row.get(0)?;
        let date: String = row.get(1)?;
        let snippet: String = content.chars().take(300).collect();
        let snippet = if content.len() > 300 {
            format!("{}...", snippet)
        } else {
            snippet
        };
        Ok((snippet, date))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
