use crate::db::Connection;
use anyhow::Result;

// --- Users ---
pub fn ensure_user(conn: &Connection, user_id: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO users (id) VALUES (?1)",
        crate::params![user_id],
    )?;
    Ok(())
}

fn normalize_wallet_address(wallet_address: &str) -> String {
    let trimmed = wallet_address.trim();
    if trimmed.starts_with("0x") && trimmed[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        trimmed.to_lowercase()
    } else {
        trimmed.to_string()
    }
}

// --- Wallet address ---
pub fn get_user_by_wallet(conn: &Connection, wallet_address: &str) -> Result<Option<String>> {
    let addr_lower = normalize_wallet_address(wallet_address);
    let mut stmt = conn.prepare("SELECT id FROM users WHERE wallet_address = ?1")?;
    let mut rows = stmt.query_map(crate::params![addr_lower], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn set_wallet_address(conn: &Connection, user_id: &str, wallet_address: &str) -> Result<()> {
    let addr_lower = normalize_wallet_address(wallet_address);
    conn.execute(
        "UPDATE users SET wallet_address = ?2 WHERE id = ?1",
        crate::params![user_id, addr_lower],
    )?;
    Ok(())
}

pub fn create_user_with_wallet(
    conn: &Connection,
    user_id: &str,
    wallet_address: &str,
) -> Result<()> {
    let addr_lower = normalize_wallet_address(wallet_address);
    conn.execute(
        "INSERT OR IGNORE INTO users (id, wallet_address) VALUES (?1, ?2)",
        crate::params![user_id, addr_lower],
    )?;
    Ok(())
}

pub fn get_user_wallet(conn: &Connection, user_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT wallet_address FROM users WHERE id = ?1")?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        row.get::<_, Option<String>>(0)
    })?;
    Ok(rows.next().and_then(|r| r.ok()).flatten())
}

// --- Privy DID ---
pub fn get_user_by_privy_did(conn: &Connection, privy_did: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT id FROM users WHERE privy_did = ?1")?;
    let mut rows = stmt.query_map(crate::params![privy_did], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn set_privy_did(conn: &Connection, user_id: &str, privy_did: &str) -> Result<()> {
    conn.execute(
        "UPDATE users SET privy_did = ?2 WHERE id = ?1",
        crate::params![user_id, privy_did],
    )?;
    Ok(())
}

pub fn set_generated_wallet(
    conn: &Connection,
    user_id: &str,
    wallet_address: &str,
    generated_wallet_secret: &str,
) -> Result<()> {
    let normalized_wallet = normalize_wallet_address(wallet_address);
    conn.execute(
        "UPDATE users
         SET wallet_address = ?2,
             generated_wallet_secret = ?3,
             wallet_generated_at = COALESCE(wallet_generated_at, datetime('now'))
         WHERE id = ?1 AND COALESCE(wallet_address, '') = ''",
        crate::params![user_id, normalized_wallet, generated_wallet_secret],
    )?;
    Ok(())
}

// --- Child Profiles ---

pub struct ChildProfileRecord {
    pub id: String,
    pub parent_wallet_address: String,
    pub derived_wallet_address: String,
    pub name: String,
    pub birthdate: String,
    pub emoji_pattern: String,
    pub created_at: String,
}

const CHILD_PROFILE_COLS: &str = "\
    id,
    parent_wallet_address,
    derived_wallet_address,
    name,
    birthdate,
    emoji_pattern,
    created_at";

fn row_to_child_profile(row: &crate::db::Row) -> anyhow::Result<ChildProfileRecord> {
    Ok(ChildProfileRecord {
        id: row.get(0)?,
        parent_wallet_address: row.get(1)?,
        derived_wallet_address: row.get(2)?,
        name: row.get(3)?,
        birthdate: row.get(4)?,
        emoji_pattern: row.get(5)?,
        created_at: row.get(6)?,
    })
}

pub fn create_child_profile(
    conn: &Connection,
    id: &str,
    parent_wallet_address: &str,
    derived_wallet_address: &str,
    name: &str,
    birthdate: &str,
    emoji_pattern: &str,
) -> Result<()> {
    let parent_wallet_address = normalize_wallet_address(parent_wallet_address);
    let derived_wallet_address = normalize_wallet_address(derived_wallet_address);
    conn.execute(
        "INSERT INTO child_profiles (
            id,
            parent_wallet_address,
            derived_wallet_address,
            name,
            birthdate,
            emoji_pattern
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        crate::params![
            id,
            parent_wallet_address,
            derived_wallet_address,
            name,
            birthdate,
            emoji_pattern
        ],
    )?;
    Ok(())
}

pub fn get_child_profiles_by_parent_wallet(
    conn: &Connection,
    parent_wallet_address: &str,
) -> Result<Vec<ChildProfileRecord>> {
    let parent_wallet_address = normalize_wallet_address(parent_wallet_address);
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM child_profiles
         WHERE parent_wallet_address = ?1
         ORDER BY created_at ASC",
        CHILD_PROFILE_COLS
    ))?;
    let rows = stmt.query_map(crate::params![parent_wallet_address], row_to_child_profile)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_child_profile_by_id(conn: &Connection, id: &str) -> Result<Option<ChildProfileRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM child_profiles
         WHERE id = ?1
         LIMIT 1",
        CHILD_PROFILE_COLS
    ))?;
    let mut rows = stmt.query_map(crate::params![id], row_to_child_profile)?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn get_child_profile_by_id_and_parent_wallet(
    conn: &Connection,
    id: &str,
    parent_wallet_address: &str,
) -> Result<Option<ChildProfileRecord>> {
    let parent_wallet_address = normalize_wallet_address(parent_wallet_address);
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM child_profiles
         WHERE id = ?1 AND parent_wallet_address = ?2
         LIMIT 1",
        CHILD_PROFILE_COLS
    ))?;
    let mut rows = stmt.query_map(
        crate::params![id, parent_wallet_address],
        row_to_child_profile,
    )?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn get_child_profile_by_derived_wallet_and_parent_wallet(
    conn: &Connection,
    derived_wallet_address: &str,
    parent_wallet_address: &str,
) -> Result<Option<ChildProfileRecord>> {
    let derived_wallet_address = normalize_wallet_address(derived_wallet_address);
    let parent_wallet_address = normalize_wallet_address(parent_wallet_address);
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM child_profiles
         WHERE derived_wallet_address = ?1
           AND parent_wallet_address = ?2
         LIMIT 1",
        CHILD_PROFILE_COLS
    ))?;
    let mut rows = stmt.query_map(
        crate::params![derived_wallet_address, parent_wallet_address],
        row_to_child_profile,
    )?;
    Ok(rows.next().and_then(|r| r.ok()))
}

// --- Email ---
pub fn set_email(conn: &Connection, user_id: &str, email: &str) -> Result<()> {
    let email_lower = email.to_lowercase();
    conn.execute(
        "UPDATE users SET email = ?2 WHERE id = ?1",
        crate::params![user_id, email_lower],
    )?;
    Ok(())
}

pub fn get_user_email(conn: &Connection, user_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT email FROM users WHERE id = ?1")?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        row.get::<_, Option<String>>(0)
    })?;
    Ok(rows.next().and_then(|r| r.ok()).flatten())
}

// --- Farcaster ---
pub fn get_user_by_fid(conn: &Connection, fid: i64) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT id FROM users WHERE farcaster_fid = ?1")?;
    let mut rows = stmt.query_map(crate::params![fid], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn create_user_with_farcaster(
    conn: &Connection,
    user_id: &str,
    fid: i64,
    username: &str,
    pfp_url: Option<&str>,
    wallet_address: Option<&str>,
) -> Result<()> {
    let addr_lower = wallet_address.map(normalize_wallet_address);
    conn.execute(
        "INSERT OR IGNORE INTO users (id, farcaster_fid, farcaster_username, farcaster_pfp_url, wallet_address) VALUES (?1, ?2, ?3, ?4, ?5)",
        crate::params![user_id, fid, username, pfp_url, addr_lower],
    )?;
    Ok(())
}

pub fn set_farcaster_info(
    conn: &Connection,
    user_id: &str,
    fid: u64,
    username: &str,
    pfp_url: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE users SET farcaster_fid = ?2, farcaster_username = ?3, farcaster_pfp_url = ?4 WHERE id = ?1",
        crate::params![user_id, fid as i64, username, pfp_url],
    )?;
    Ok(())
}

// --- Usernames ---
pub fn set_username(conn: &Connection, user_id: &str, username: &str) -> Result<()> {
    conn.execute(
        "UPDATE users SET username = ?2 WHERE id = ?1",
        crate::params![user_id, username],
    )?;
    Ok(())
}

pub fn get_user_by_username(conn: &Connection, username: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT id FROM users WHERE username = ?1")?;
    let mut rows = stmt.query_map(crate::params![username], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn check_username_available(
    conn: &Connection,
    username: &str,
    exclude_user_id: &str,
) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE username = ?1 AND id != ?2",
        crate::params![username, exclude_user_id],
        |row| row.get(0),
    )?;
    Ok(count == 0)
}

pub fn get_user_username(conn: &Connection, user_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT username FROM users WHERE id = ?1")?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        row.get::<_, Option<String>>(0)
    })?;
    Ok(rows.next().and_then(|r| r.ok()).flatten())
}

/// Returns display username: users.username → x_users.username → "someone"
pub fn get_display_username(conn: &Connection, user_id: &str) -> Result<String> {
    // Check users.username first
    if let Some(name) = get_user_username(conn, user_id)? {
        return Ok(name);
    }
    // Fall back to x_users.username
    let mut stmt = conn.prepare("SELECT username FROM x_users WHERE user_id = ?1 LIMIT 1")?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| row.get::<_, String>(0))?;
    if let Some(Ok(name)) = rows.next() {
        return Ok(name);
    }
    Ok("someone".to_string())
}

// --- User Settings ---
pub struct UserSettings {
    pub font_family: String,
    pub font_size: i32,
    pub theme: String,
    pub idle_timeout: i32,
    pub keyboard_layout: String,
    pub preferred_language: String,
    pub preferred_model: String,
}

pub fn get_user_settings(conn: &Connection, user_id: &str) -> Result<UserSettings> {
    let mut stmt = conn.prepare(
        "SELECT font_family, font_size, theme, idle_timeout, keyboard_layout, preferred_language, preferred_model FROM user_settings WHERE user_id = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        Ok(UserSettings {
            font_family: row.get(0)?,
            font_size: row.get(1)?,
            theme: row.get(2)?,
            idle_timeout: row.get(3)?,
            keyboard_layout: row
                .get::<_, String>(4)
                .unwrap_or_else(|_| "qwerty".to_string()),
            preferred_language: row.get::<_, String>(5).unwrap_or_else(|_| "en".to_string()),
            preferred_model: row
                .get::<_, String>(6)
                .unwrap_or_else(|_| "default".to_string()),
        })
    })?;
    match rows.next() {
        Some(Ok(s)) => Ok(s),
        _ => Ok(UserSettings {
            font_family: "monospace".to_string(),
            font_size: 18,
            theme: "dark".to_string(),
            idle_timeout: 8,
            keyboard_layout: "qwerty".to_string(),
            preferred_language: "en".to_string(),
            preferred_model: "default".to_string(),
        }),
    }
}

pub fn upsert_user_settings(
    conn: &Connection,
    user_id: &str,
    font_family: &str,
    font_size: i32,
    theme: &str,
    idle_timeout: i32,
    keyboard_layout: &str,
    preferred_language: &str,
    preferred_model: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO user_settings (user_id, font_family, font_size, theme, idle_timeout, keyboard_layout, preferred_language, preferred_model)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(user_id) DO UPDATE SET
            font_family = excluded.font_family,
            font_size = excluded.font_size,
            theme = excluded.theme,
            idle_timeout = excluded.idle_timeout,
            keyboard_layout = excluded.keyboard_layout,
            preferred_language = excluded.preferred_language,
            preferred_model = excluded.preferred_model",
        crate::params![user_id, font_family, font_size, theme, idle_timeout, keyboard_layout, preferred_language, preferred_model],
    )?;
    Ok(())
}

// --- Writing Sessions ---
pub fn insert_writing_session(
    conn: &Connection,
    id: &str,
    user_id: &str,
    content: &str,
    duration: f64,
    word_count: i32,
    is_anky: bool,
    response: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO writing_sessions (id, user_id, content, duration_seconds, word_count, is_anky, response) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        crate::params![id, user_id, content, duration, word_count, is_anky, response],
    )?;
    Ok(())
}

pub fn insert_writing_session_with_flow(
    conn: &Connection,
    id: &str,
    user_id: &str,
    content: &str,
    duration: f64,
    word_count: i32,
    is_anky: bool,
    response: Option<&str>,
    keystroke_deltas: Option<&str>,
    flow_score: Option<f64>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO writing_sessions (id, user_id, content, duration_seconds, word_count, is_anky, response, keystroke_deltas, flow_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![id, user_id, content, duration, word_count, is_anky, response, keystroke_deltas, flow_score],
    )?;
    Ok(())
}

pub fn upsert_active_writing_session(
    conn: &Connection,
    id: &str,
    user_id: &str,
    content: &str,
    duration: f64,
    word_count: i32,
    status: &str,
    pause_used: bool,
    session_token: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO writing_sessions (
            id, user_id, content, duration_seconds, word_count, is_anky, response,
            status, pause_used, paused_at, resumed_at, session_token
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, 0, NULL,
            ?6, ?7,
            CASE WHEN ?6 = 'paused' THEN datetime('now') ELSE NULL END,
            CASE WHEN ?6 = 'resumed' THEN datetime('now') ELSE NULL END,
            ?8
         )
         ON CONFLICT(id) DO UPDATE SET
            content = excluded.content,
            duration_seconds = excluded.duration_seconds,
            word_count = excluded.word_count,
            status = excluded.status,
            pause_used = excluded.pause_used,
            paused_at = CASE
                WHEN excluded.status = 'paused' THEN datetime('now')
                ELSE writing_sessions.paused_at
            END,
            resumed_at = CASE
                WHEN excluded.status = 'resumed' THEN datetime('now')
                ELSE writing_sessions.resumed_at
            END,
            session_token = COALESCE(excluded.session_token, writing_sessions.session_token)",
        crate::params![
            id,
            user_id,
            content,
            duration,
            word_count,
            status,
            pause_used,
            session_token
        ],
    )?;
    Ok(())
}

/// Ensure a writing_session row exists for this checkpoint session with the real user_id.
/// If the row doesn't exist, create it with status='active' so recovery never has to guess.
/// If it exists with a placeholder user_id (system, recovered-unknown), update it.
pub fn ensure_checkpoint_session_owner(
    conn: &Connection,
    id: &str,
    user_id: &str,
    content: &str,
    duration: f64,
    word_count: i32,
    session_token: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO writing_sessions (id, user_id, content, duration_seconds, word_count, is_anky, status, session_token)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 'active', ?6)
         ON CONFLICT(id) DO UPDATE SET
            user_id = CASE
                WHEN writing_sessions.user_id IN ('system', 'recovered-unknown') THEN excluded.user_id
                ELSE writing_sessions.user_id
            END,
            content = excluded.content,
            duration_seconds = excluded.duration_seconds,
            word_count = excluded.word_count,
            session_token = COALESCE(excluded.session_token, writing_sessions.session_token)",
        crate::params![id, user_id, content, duration, word_count, session_token],
    )?;
    Ok(())
}

pub fn update_checkpoint_backed_writing_session(
    conn: &Connection,
    id: &str,
    content: &str,
    duration: f64,
    word_count: i32,
    session_token: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE writing_sessions
         SET content = ?2,
             duration_seconds = ?3,
             word_count = ?4,
             session_token = COALESCE(?5, session_token)
         WHERE id = ?1
           AND status IN ('active', 'paused', 'resumed')",
        crate::params![id, content, duration, word_count, session_token],
    )?;
    Ok(())
}

pub fn upsert_completed_writing_session_with_flow(
    conn: &Connection,
    id: &str,
    user_id: &str,
    content: &str,
    duration: f64,
    word_count: i32,
    is_anky: bool,
    response: Option<&str>,
    keystroke_deltas: Option<&str>,
    flow_score: Option<f64>,
    session_token: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO writing_sessions (
            id, user_id, content, duration_seconds, word_count, is_anky, response,
            keystroke_deltas, flow_score, status, pause_used, session_token
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7,
            ?8, ?9::double precision, 'completed', 0, ?10
         )
         ON CONFLICT(id) DO UPDATE SET
            user_id = excluded.user_id,
            content = excluded.content,
            duration_seconds = excluded.duration_seconds,
            word_count = excluded.word_count,
            is_anky = excluded.is_anky,
            response = excluded.response,
            keystroke_deltas = excluded.keystroke_deltas,
            flow_score = excluded.flow_score,
            status = 'completed',
            session_token = COALESCE(excluded.session_token, writing_sessions.session_token)",
        crate::params![
            id,
            user_id,
            content,
            duration,
            word_count,
            is_anky,
            response,
            keystroke_deltas,
            flow_score,
            session_token
        ],
    )?;
    Ok(())
}

pub struct WritingSessionState {
    pub user_id: String,
    pub status: String,
    pub pause_used: bool,
    pub session_token: Option<String>,
}

pub fn get_writing_session_state(
    conn: &Connection,
    id: &str,
) -> Result<Option<WritingSessionState>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, COALESCE(status, 'completed'), COALESCE(pause_used, 0), session_token
         FROM writing_sessions
         WHERE id = ?1
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![id], |row| {
        Ok(WritingSessionState {
            user_id: row.get(0)?,
            status: row.get(1)?,
            pause_used: row.get(2)?,
            session_token: row.get(3)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub struct ResumableWritingSession {
    pub id: String,
    pub content: String,
    pub duration_seconds: f64,
    pub word_count: i32,
    pub pause_used: bool,
    pub status: String,
    pub paused_at: Option<String>,
    pub resumed_at: Option<String>,
    pub session_token: Option<String>,
}

pub fn get_resumable_writing_session(
    conn: &Connection,
    user_id: &str,
) -> Result<Option<ResumableWritingSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, duration_seconds, word_count,
                COALESCE(pause_used, 0),
                COALESCE(status, 'completed'),
                paused_at,
                resumed_at,
                session_token
         FROM writing_sessions
         WHERE user_id = ?1
           AND status IN ('paused', 'resumed')
         ORDER BY COALESCE(resumed_at, paused_at, created_at) DESC
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        Ok(ResumableWritingSession {
            id: row.get(0)?,
            content: row.get(1)?,
            duration_seconds: row.get(2)?,
            word_count: row.get(3)?,
            pause_used: row.get(4)?,
            status: row.get(5)?,
            paused_at: row.get(6)?,
            resumed_at: row.get(7)?,
            session_token: row.get(8)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn discard_resumable_writing_session(
    conn: &Connection,
    user_id: &str,
    session_id: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE writing_sessions
         SET status = 'discarded'
         WHERE id = ?1
           AND user_id = ?2
           AND status IN ('paused', 'resumed')",
        crate::params![session_id, user_id],
    )?;
    Ok(())
}

/// Calculate flow score from keystroke deltas (0-100).
/// Measures rhythm consistency, velocity, and sustained attention.
pub fn calculate_flow_score(deltas: &[f64], duration: f64, word_count: i32) -> f64 {
    if deltas.len() < 10 || duration < 30.0 {
        return 0.0;
    }

    // 1. Rhythm consistency (0-30 pts): low std dev of inter-keystroke intervals = flow
    let mean_delta: f64 = deltas.iter().sum::<f64>() / deltas.len() as f64;
    let variance: f64 =
        deltas.iter().map(|d| (d - mean_delta).powi(2)).sum::<f64>() / deltas.len() as f64;
    let std_dev = variance.sqrt();
    // Ideal: std_dev around 50-100ms. Penalize both too erratic (>500ms) and too robotic (<20ms)
    let rhythm_score = if std_dev < 20.0 {
        15.0 // suspiciously robotic
    } else if std_dev < 150.0 {
        30.0 // excellent flow
    } else if std_dev < 300.0 {
        30.0 * (1.0 - (std_dev - 150.0) / 150.0).max(0.0)
    } else {
        5.0 // very erratic
    };

    // 2. Velocity (0-25 pts): words per minute
    let wpm = (word_count as f64 / duration) * 60.0;
    let velocity_score = if wpm < 10.0 {
        5.0
    } else if wpm < 30.0 {
        5.0 + 20.0 * ((wpm - 10.0) / 20.0)
    } else if wpm <= 80.0 {
        25.0 // sweet spot
    } else {
        25.0 * (1.0 - ((wpm - 80.0) / 40.0).min(1.0)).max(0.5)
    };

    // 3. Sustained attention (0-25 pts): few long pauses
    let long_pauses = deltas.iter().filter(|&&d| d > 2000.0).count();
    let pause_ratio = long_pauses as f64 / deltas.len() as f64;
    let attention_score = 25.0 * (1.0 - (pause_ratio * 10.0).min(1.0));

    // 4. Duration bonus (0-20 pts): longer = more flow (up to 8 min)
    let duration_score = 20.0 * (duration / 480.0).min(1.0);

    let total = rhythm_score + velocity_score + attention_score + duration_score;
    total.round().min(100.0).max(0.0)
}

/// Update user profile streak and flow scores after a writing session.
pub fn update_user_flow_stats(
    conn: &Connection,
    user_id: &str,
    flow_score: f64,
    is_anky: bool,
) -> Result<()> {
    // Ensure user_profiles row exists
    conn.execute(
        "INSERT OR IGNORE INTO user_profiles (user_id) VALUES (?1)",
        crate::params![user_id],
    )?;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    if is_anky {
        // Get current streak info
        let (last_date, current_streak): (Option<String>, i32) = conn.query_row(
            "SELECT last_anky_date, COALESCE(current_streak, 0) FROM user_profiles WHERE user_id = ?1",
            crate::params![user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let new_streak = if let Some(ref ld) = last_date {
            if ld == &today {
                current_streak // same day, no change
            } else if let Ok(last) = chrono::NaiveDate::parse_from_str(ld, "%Y-%m-%d") {
                let today_date =
                    chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap_or(last);
                let diff = (today_date - last).num_days();
                if diff == 1 {
                    current_streak + 1
                } else {
                    1
                }
            } else {
                1
            }
        } else {
            1
        };

        conn.execute(
            "UPDATE user_profiles SET
                total_anky_sessions = COALESCE(total_anky_sessions, 0) + 1,
                current_streak = ?2,
                longest_streak = MAX(COALESCE(longest_streak, 0), ?2),
                best_flow_score = MAX(COALESCE(best_flow_score, 0), ?3),
                last_anky_date = ?4,
                updated_at = datetime('now')
            WHERE user_id = ?1",
            crate::params![user_id, new_streak, flow_score, today],
        )?;
    }

    // Update session count and avg flow
    conn.execute(
        "UPDATE user_profiles SET
            total_sessions = COALESCE(total_sessions, 0) + 1,
            avg_flow_score = (
                SELECT COALESCE(AVG(flow_score), 0) FROM writing_sessions
                WHERE user_id = ?1 AND flow_score IS NOT NULL AND COALESCE(status, 'completed') = 'completed'
            ),
            total_words_written = (
                SELECT COALESCE(SUM(word_count), 0) FROM writing_sessions
                WHERE user_id = ?1 AND COALESCE(status, 'completed') = 'completed'
            ),
            updated_at = datetime('now')
        WHERE user_id = ?1",
        crate::params![user_id],
    )?;

    Ok(())
}

// --- Leaderboard ---
pub struct LeaderboardEntry {
    pub rank: i32,
    pub username: String,
    pub best_flow_score: f64,
    pub avg_flow_score: f64,
    pub total_ankys: i32,
    pub total_words: i32,
    pub current_streak: i32,
    pub longest_streak: i32,
}

pub fn get_leaderboard(
    conn: &Connection,
    sort_by: &str,
    limit: i32,
) -> Result<Vec<LeaderboardEntry>> {
    let order = match sort_by {
        "streak" => "up.current_streak DESC, up.best_flow_score DESC",
        "ankys" => "up.total_anky_sessions DESC, up.best_flow_score DESC",
        "words" => "up.total_words_written DESC, up.best_flow_score DESC",
        _ => "up.best_flow_score DESC, up.avg_flow_score DESC", // default: flow
    };
    let sql = format!(
        "SELECT
            COALESCE(u.username, u.farcaster_username, (SELECT xu.username FROM x_users xu WHERE xu.user_id = u.id LIMIT 1), 'anon-' || substr(u.id, 1, 6)) as display_name,
            COALESCE(up.best_flow_score, 0),
            COALESCE(up.avg_flow_score, 0),
            COALESCE(up.total_anky_sessions, 0),
            COALESCE(up.total_words_written, 0),
            COALESCE(up.current_streak, 0),
            COALESCE(up.longest_streak, 0)
        FROM user_profiles up
        JOIN users u ON u.id = up.user_id
        WHERE up.total_anky_sessions > 0
        ORDER BY {}
        LIMIT ?1",
        order
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(crate::params![limit], |row| {
        Ok(LeaderboardEntry {
            rank: 0, // filled in after
            username: row.get(0)?,
            best_flow_score: row.get(1)?,
            avg_flow_score: row.get(2)?,
            total_ankys: row.get(3)?,
            total_words: row.get(4)?,
            current_streak: row.get(5)?,
            longest_streak: row.get(6)?,
        })
    })?;
    let mut entries: Vec<LeaderboardEntry> = rows.filter_map(|r| r.ok()).collect();
    for (i, entry) in entries.iter_mut().enumerate() {
        entry.rank = (i + 1) as i32;
    }
    Ok(entries)
}

pub struct WritingSession {
    pub id: String,
    pub content: String,
    pub duration_seconds: f64,
    pub word_count: i32,
    pub is_anky: bool,
    pub response: Option<String>,
    pub created_at: String,
}

pub fn get_user_writings(conn: &Connection, user_id: &str) -> Result<Vec<WritingSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, duration_seconds, word_count, is_anky, response, created_at
         FROM writing_sessions
         WHERE user_id = ?1
           AND COALESCE(status, 'completed') = 'completed'
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![user_id], |row| {
        Ok(WritingSession {
            id: row.get(0)?,
            content: row.get(1)?,
            duration_seconds: row.get(2)?,
            word_count: row.get(3)?,
            is_anky: row.get(4)?,
            response: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub struct WritingWithAnky {
    pub id: String,
    pub content: String,
    pub duration_seconds: f64,
    pub word_count: i32,
    pub is_anky: bool,
    pub response: Option<String>,
    pub created_at: String,
    pub anky_id: Option<String>,
    pub anky_title: Option<String>,
    pub anky_image_path: Option<String>,
    pub anky_reflection: Option<String>,
    pub conversation_json: Option<String>,
}

pub struct WritingArchiveRecord {
    pub id: String,
    pub wallet_address: String,
    pub content: String,
    pub created_at: String,
}

pub fn get_writings_for_file_archive(conn: &Connection) -> Result<Vec<WritingArchiveRecord>> {
    let mut stmt = conn.prepare(
        "SELECT ws.id, u.wallet_address, ws.content, ws.created_at
         FROM writing_sessions ws
         JOIN users u ON u.id = ws.user_id
         WHERE COALESCE(ws.status, 'completed') = 'completed'
           AND ws.is_anky = 1
           AND COALESCE(u.wallet_address, '') != ''
         ORDER BY ws.created_at ASC",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok(WritingArchiveRecord {
            id: row.get(0)?,
            wallet_address: row.get(1)?,
            content: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_user_writings_with_ankys(
    conn: &Connection,
    user_id: &str,
) -> Result<Vec<WritingWithAnky>> {
    let mut stmt = conn.prepare(
        "SELECT ws.id, ws.content, ws.duration_seconds, ws.word_count, ws.is_anky, ws.response, ws.created_at,
                a.id, a.title, a.image_path, a.reflection, a.conversation_json
         FROM writing_sessions ws
         LEFT JOIN ankys a ON a.writing_session_id = ws.id AND a.status = 'complete'
         WHERE ws.user_id = ?1
           AND COALESCE(ws.status, 'completed') = 'completed'
         ORDER BY ws.created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![user_id], |row| {
        Ok(WritingWithAnky {
            id: row.get(0)?,
            content: row.get(1)?,
            duration_seconds: row.get(2)?,
            word_count: row.get(3)?,
            is_anky: row.get(4)?,
            response: row.get(5)?,
            created_at: row.get(6)?,
            anky_id: row.get(7)?,
            anky_title: row.get(8)?,
            anky_image_path: row.get(9)?,
            anky_reflection: row.get(10)?,
            conversation_json: row.get(11)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_writing_session(conn: &Connection, id: &str) -> Result<Option<WritingSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, duration_seconds, word_count, is_anky, response, created_at FROM writing_sessions WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![id], |row| {
        Ok(WritingSession {
            id: row.get(0)?,
            content: row.get(1)?,
            duration_seconds: row.get(2)?,
            word_count: row.get(3)?,
            is_anky: row.get(4)?,
            response: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

// --- Ankys ---
pub fn insert_anky(
    conn: &Connection,
    id: &str,
    writing_session_id: &str,
    user_id: &str,
    image_prompt: Option<&str>,
    reflection: Option<&str>,
    title: Option<&str>,
    image_path: Option<&str>,
    caption: Option<&str>,
    thinker_name: Option<&str>,
    thinker_moment: Option<&str>,
    status: &str,
    origin: &str,
    prompt_id: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO ankys (id, writing_session_id, user_id, image_prompt, reflection, title, image_path, caption, thinker_name, thinker_moment, status, origin, prompt_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        crate::params![id, writing_session_id, user_id, image_prompt, reflection, title, image_path, caption, thinker_name, thinker_moment, status, origin, prompt_id],
    )?;
    Ok(())
}

pub fn get_anky_by_writing_session_id(
    conn: &Connection,
    writing_session_id: &str,
) -> Result<
    Option<(
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )>,
> {
    // (id, status, image_path, title, reflection)
    let mut stmt = conn.prepare(
        "SELECT id, status, image_path, title, reflection FROM ankys
         WHERE writing_session_id = ?1
         ORDER BY rowid DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![writing_session_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn set_anky_image_model(conn: &Connection, id: &str, image_model: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET image_model = ?2 WHERE id = ?1",
        crate::params![id, image_model],
    )?;
    Ok(())
}

pub fn update_anky_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET status = ?2 WHERE id = ?1",
        crate::params![id, status],
    )?;
    Ok(())
}

pub fn update_anky_fields(
    conn: &Connection,
    id: &str,
    image_prompt: &str,
    reflection: &str,
    title: &str,
    image_path: &str,
    caption: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET image_prompt = ?2, reflection = ?3, title = ?4, image_path = ?5, caption = ?6, status = 'complete' WHERE id = ?1",
        crate::params![id, image_prompt, reflection, title, image_path, caption],
    )?;
    Ok(())
}

pub fn update_anky_title_reflection(
    conn: &Connection,
    id: &str,
    title: &str,
    reflection: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ankys
         SET title = ?2,
             reflection = ?3,
             reflection_status = CASE
                 WHEN session_hash IS NOT NULL THEN 'complete'
                 ELSE COALESCE(reflection_status, 'pending')
             END,
             reflection_completed_at = CASE
                 WHEN session_hash IS NOT NULL THEN COALESCE(reflection_completed_at, datetime('now'))
                 ELSE reflection_completed_at
             END
         WHERE id = ?1",
        crate::params![id, title, reflection],
    )?;
    Ok(())
}

pub fn update_anky_conversation(
    conn: &Connection,
    id: &str,
    conversation_json: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET conversation_json = ?2 WHERE id = ?1",
        crate::params![id, conversation_json],
    )?;
    Ok(())
}

pub fn get_anky_conversation(conn: &Connection, id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT conversation_json FROM ankys WHERE id = ?1")?;
    let result: Option<Option<String>> = stmt
        .query_map(crate::params![id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .next();
    Ok(result.flatten())
}

pub fn update_anky_image_complete(
    conn: &Connection,
    id: &str,
    image_prompt: &str,
    image_path: &str,
    caption: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET image_prompt = ?2, image_path = ?3, caption = ?4, status = 'complete' WHERE id = ?1",
        crate::params![id, image_prompt, image_path, caption],
    )?;
    Ok(())
}

pub fn update_anky_image_only(
    conn: &Connection,
    id: &str,
    image_prompt: &str,
    image_path: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET image_prompt = ?2, image_path = ?3, status = 'complete' WHERE id = ?1",
        crate::params![id, image_prompt, image_path],
    )?;
    Ok(())
}

pub fn update_anky_webp(conn: &Connection, id: &str, image_webp: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET image_webp = ?2 WHERE id = ?1",
        crate::params![id, image_webp],
    )?;
    Ok(())
}

pub struct AnkyRecord {
    pub id: String,
    pub title: Option<String>,
    pub image_path: Option<String>,
    pub image_webp: Option<String>,
    pub reflection: Option<String>,
    pub image_prompt: Option<String>,
    pub thinker_name: Option<String>,
    pub status: String,
    pub created_at: String,
    pub origin: String,
    pub image_model: String,
}

pub fn get_all_ankys(conn: &Connection) -> Result<Vec<AnkyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, image_path, image_webp, reflection, image_prompt, thinker_name, status, created_at, origin, COALESCE(image_model, 'gemini') FROM ankys ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok(AnkyRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            image_webp: row.get(3)?,
            reflection: row.get(4)?,
            image_prompt: row.get(5)?,
            thinker_name: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            origin: row.get(9)?,
            image_model: row.get(10).unwrap_or_else(|_| "gemini".to_string()),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_all_complete_ankys(conn: &Connection) -> Result<Vec<AnkyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, image_path, image_webp, reflection, image_prompt, thinker_name, status, created_at, origin, COALESCE(image_model, 'gemini') FROM ankys WHERE status = 'complete' ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok(AnkyRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            image_webp: row.get(3)?,
            reflection: row.get(4)?,
            image_prompt: row.get(5)?,
            thinker_name: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            origin: row.get(9)?,
            image_model: row.get(10).unwrap_or_else(|_| "gemini".to_string()),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_user_ankys(conn: &Connection, user_id: &str) -> Result<Vec<AnkyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, image_path, image_webp, reflection, image_prompt, thinker_name, status, created_at, origin, COALESCE(image_model, 'gemini') FROM ankys WHERE user_id = ?1 AND status = 'complete' ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![user_id], |row| {
        Ok(AnkyRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            image_webp: row.get(3)?,
            reflection: row.get(4)?,
            image_prompt: row.get(5)?,
            thinker_name: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            origin: row.get(9)?,
            image_model: row.get(10).unwrap_or_else(|_| "gemini".to_string()),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_user_viewed_ankys(conn: &Connection, user_id: &str) -> Result<Vec<AnkyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.title, a.image_path, a.image_webp, a.reflection, a.image_prompt, a.thinker_name, a.status, a.created_at, a.origin, COALESCE(a.image_model, 'gemini')
         FROM user_collections uc
         JOIN ankys a ON a.id = uc.anky_id
         WHERE uc.user_id = ?1 AND a.status = 'complete'
         ORDER BY uc.collected_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![user_id], |row| {
        Ok(AnkyRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            image_webp: row.get(3)?,
            reflection: row.get(4)?,
            image_prompt: row.get(5)?,
            thinker_name: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            origin: row.get(9)?,
            image_model: row.get(10).unwrap_or_else(|_| "gemini".to_string()),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_generated_ankys(conn: &Connection) -> Result<Vec<AnkyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, image_path, image_webp, reflection, image_prompt, thinker_name, status, created_at, origin, COALESCE(image_model, 'gemini') FROM ankys WHERE origin = 'generated' AND status = 'complete' ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok(AnkyRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            image_webp: row.get(3)?,
            reflection: row.get(4)?,
            image_prompt: row.get(5)?,
            thinker_name: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            origin: row.get(9)?,
            image_model: row.get(10).unwrap_or_else(|_| "gemini".to_string()),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub struct AnkyDetail {
    pub id: String,
    pub writing_session_id: Option<String>,
    pub title: Option<String>,
    pub image_path: Option<String>,
    pub image_webp: Option<String>,
    pub reflection: Option<String>,
    pub image_prompt: Option<String>,
    pub caption: Option<String>,
    pub thinker_name: Option<String>,
    pub thinker_moment: Option<String>,
    pub status: String,
    pub writing_text: Option<String>,
    pub created_at: String,
    pub origin: String,
    pub image_model: String,
    pub conversation_json: Option<String>,
    pub prompt_id: Option<String>,
    pub prompt_text: Option<String>,
    pub formatted_writing: Option<String>,
    pub anky_story: Option<String>,
    pub kingdom_id: Option<i32>,
    pub kingdom_name: Option<String>,
    pub kingdom_chakra: Option<String>,
}

pub fn get_anky_by_id(conn: &Connection, id: &str) -> Result<Option<AnkyDetail>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.title, a.image_path, a.image_webp, a.reflection, a.image_prompt, a.caption, a.thinker_name, a.thinker_moment, a.status, w.content, a.created_at, a.origin, COALESCE(a.image_model, 'gemini'), a.conversation_json, a.prompt_id, p.prompt_text, a.formatted_writing, a.writing_session_id, a.anky_story, a.kingdom_id, a.kingdom_name, a.kingdom_chakra
         FROM ankys a
         LEFT JOIN writing_sessions w ON w.id = a.writing_session_id
         LEFT JOIN prompts p ON p.id = a.prompt_id
         WHERE a.id = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![id], |row| {
        Ok(AnkyDetail {
            id: row.get(0)?,
            writing_session_id: row.get(18)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            image_webp: row.get(3)?,
            reflection: row.get(4)?,
            image_prompt: row.get(5)?,
            caption: row.get(6)?,
            thinker_name: row.get(7)?,
            thinker_moment: row.get(8)?,
            status: row.get(9)?,
            writing_text: row.get(10)?,
            created_at: row.get(11)?,
            origin: row.get(12)?,
            image_model: row.get(13)?,
            conversation_json: row.get(14)?,
            prompt_id: row.get(15)?,
            prompt_text: row.get(16)?,
            formatted_writing: row.get(17)?,
            anky_story: row.get(19)?,
            kingdom_id: row.get(20)?,
            kingdom_name: row.get(21)?,
            kingdom_chakra: row.get(22)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

// --- Collections ---
pub fn insert_collection(
    conn: &Connection,
    id: &str,
    user_id: &str,
    mega_prompt: &str,
    cost_estimate: f64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO collections (id, user_id, mega_prompt, cost_estimate_usd) VALUES (?1, ?2, ?3, ?4)",
        crate::params![id, user_id, mega_prompt, cost_estimate],
    )?;
    Ok(())
}

pub fn update_collection_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE collections SET status = ?2 WHERE id = ?1",
        crate::params![id, status],
    )?;
    Ok(())
}

pub fn update_collection_progress(conn: &Connection, id: &str, progress: i32) -> Result<()> {
    conn.execute(
        "UPDATE collections SET progress = ?2 WHERE id = ?1",
        crate::params![id, progress],
    )?;
    Ok(())
}

pub fn update_collection_payment(conn: &Connection, id: &str, tx_hash: &str) -> Result<()> {
    conn.execute(
        "UPDATE collections SET payment_tx_hash = ?2, status = 'paid' WHERE id = ?1",
        crate::params![id, tx_hash],
    )?;
    Ok(())
}

pub struct CollectionRecord {
    pub id: String,
    pub mega_prompt: String,
    pub beings_json: Option<String>,
    pub status: String,
    pub progress: i32,
    pub total: i32,
    pub cost_estimate_usd: Option<f64>,
    pub created_at: String,
}

pub fn get_collection(conn: &Connection, id: &str) -> Result<Option<CollectionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, mega_prompt, beings_json, status, progress, total, cost_estimate_usd, created_at FROM collections WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![id], |row| {
        Ok(CollectionRecord {
            id: row.get(0)?,
            mega_prompt: row.get(1)?,
            beings_json: row.get(2)?,
            status: row.get(3)?,
            progress: row.get(4)?,
            total: row.get(5)?,
            cost_estimate_usd: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

// --- Cost Records ---
pub fn insert_cost_record(
    conn: &Connection,
    service: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f64,
    related_id: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO cost_records (service, model, input_tokens, output_tokens, cost_usd, related_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        crate::params![service, model, input_tokens, output_tokens, cost_usd, related_id],
    )?;
    Ok(())
}

pub fn get_total_cost(conn: &Connection) -> Result<f64> {
    let cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM(cost_usd), 0) FROM cost_records",
        crate::params![],
        |row| row.get(0),
    )?;
    Ok(cost)
}

pub struct ServiceSpend {
    pub service: String,
    pub model: String,
    pub calls: i32,
    pub total_cost_usd: f64,
}

pub fn get_video_service_spend(
    conn: &Connection,
    user_id: &str,
    since_days: Option<i32>,
) -> Result<Vec<ServiceSpend>> {
    let mut out = Vec::new();
    if let Some(days) = since_days {
        let modifier = format!("-{} days", days.max(1));
        let mut stmt = conn.prepare(
            "SELECT c.service, c.model, COUNT(*), COALESCE(SUM(c.cost_usd), 0)
             FROM cost_records c
             JOIN video_projects v ON v.id = c.related_id
             WHERE v.user_id = ?1 AND c.created_at >= datetime('now', ?2)
             GROUP BY c.service, c.model
             ORDER BY SUM(c.cost_usd) DESC",
        )?;
        let rows = stmt.query_map(crate::params![user_id, modifier], |row| {
            Ok(ServiceSpend {
                service: row.get(0)?,
                model: row.get(1)?,
                calls: row.get(2)?,
                total_cost_usd: row.get(3)?,
            })
        })?;
        for row in rows {
            if let Ok(item) = row {
                out.push(item);
            }
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT c.service, c.model, COUNT(*), COALESCE(SUM(c.cost_usd), 0)
             FROM cost_records c
             JOIN video_projects v ON v.id = c.related_id
             WHERE v.user_id = ?1
             GROUP BY c.service, c.model
             ORDER BY SUM(c.cost_usd) DESC",
        )?;
        let rows = stmt.query_map(crate::params![user_id], |row| {
            Ok(ServiceSpend {
                service: row.get(0)?,
                model: row.get(1)?,
                calls: row.get(2)?,
                total_cost_usd: row.get(3)?,
            })
        })?;
        for row in rows {
            if let Ok(item) = row {
                out.push(item);
            }
        }
    }
    Ok(out)
}

pub struct VideoProjectSpend {
    pub id: String,
    pub status: String,
    pub created_at: String,
    pub total_cost_usd: f64,
}

pub fn get_recent_video_project_spend(
    conn: &Connection,
    user_id: &str,
    limit: i32,
) -> Result<Vec<VideoProjectSpend>> {
    let mut stmt = conn.prepare(
        "SELECT v.id, v.status, v.created_at, COALESCE(SUM(c.cost_usd), 0)
         FROM video_projects v
         LEFT JOIN cost_records c ON c.related_id = v.id
         WHERE v.user_id = ?1
         GROUP BY v.id, v.status, v.created_at
         ORDER BY v.created_at DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(crate::params![user_id, limit.max(1)], |row| {
        Ok(VideoProjectSpend {
            id: row.get(0)?,
            status: row.get(1)?,
            created_at: row.get(2)?,
            total_cost_usd: row.get(3)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_pipeline_prompt(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM pipeline_prompts WHERE key = ?1")?;
    let mut rows = stmt.query_map(crate::params![key], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn upsert_pipeline_prompt(
    conn: &Connection,
    key: &str,
    value: &str,
    updated_by: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO pipeline_prompts (key, value, updated_by, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_by = excluded.updated_by,
            updated_at = datetime('now')",
        crate::params![key, value, updated_by],
    )?;
    Ok(())
}

// --- Training Runs ---
pub fn insert_training_run(
    conn: &Connection,
    id: &str,
    base_model: &str,
    dataset_size: i32,
    steps: i32,
) -> Result<()> {
    conn.execute(
        "INSERT INTO training_runs (id, base_model, dataset_size, steps, status, started_at) VALUES (?1, ?2, ?3, ?4, 'running', datetime('now'))",
        crate::params![id, base_model, dataset_size, steps],
    )?;
    Ok(())
}

pub fn update_training_progress(
    conn: &Connection,
    id: &str,
    current_step: i32,
    loss: f64,
) -> Result<()> {
    conn.execute(
        "UPDATE training_runs SET current_step = ?2, loss = ?3 WHERE id = ?1",
        crate::params![id, current_step, loss],
    )?;
    Ok(())
}

pub fn complete_training_run(conn: &Connection, id: &str, lora_path: &str) -> Result<()> {
    conn.execute(
        "UPDATE training_runs SET status = 'complete', lora_weights_path = ?2, completed_at = datetime('now') WHERE id = ?1",
        crate::params![id, lora_path],
    )?;
    Ok(())
}

// --- Notification Signups ---
pub fn insert_notification_signup(
    conn: &Connection,
    email: Option<&str>,
    telegram_chat_id: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO notification_signups (email, telegram_chat_id) VALUES (?1, ?2)",
        crate::params![email, telegram_chat_id],
    )?;
    Ok(())
}

pub fn get_notification_signups(
    conn: &Connection,
) -> Result<Vec<(Option<String>, Option<String>)>> {
    let mut stmt = conn.prepare("SELECT email, telegram_chat_id FROM notification_signups")?;
    let rows = stmt.query_map(crate::params![], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// --- API Keys ---
pub struct ApiKeyRecord {
    pub key: String,
    pub label: Option<String>,
    pub balance_usd: f64,
    pub total_spent_usd: f64,
    pub total_transforms: i32,
    pub is_active: bool,
    pub created_at: String,
}

pub fn create_api_key(conn: &Connection, key: &str, label: Option<&str>) -> Result<()> {
    conn.execute(
        "INSERT INTO api_keys (key, label) VALUES (?1, ?2)",
        crate::params![key, label],
    )?;
    Ok(())
}

pub fn get_api_key(conn: &Connection, key: &str) -> Result<Option<ApiKeyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT key, label, balance_usd, total_spent_usd, total_transforms, is_active, created_at FROM api_keys WHERE key = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![key], |row| {
        Ok(ApiKeyRecord {
            key: row.get(0)?,
            label: row.get(1)?,
            balance_usd: row.get(2)?,
            total_spent_usd: row.get(3)?,
            total_transforms: row.get(4)?,
            is_active: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn deactivate_api_key(conn: &Connection, key: &str) -> Result<()> {
    conn.execute(
        "UPDATE api_keys SET is_active = 0 WHERE key = ?1",
        crate::params![key],
    )?;
    Ok(())
}

pub fn insert_transformation(
    conn: &Connection,
    id: &str,
    api_key: &str,
    input_text: &str,
    prompt: Option<&str>,
    output_text: &str,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO transformations (id, api_key, input_text, prompt, output_text, input_tokens, output_tokens, cost_usd) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        crate::params![id, api_key, input_text, prompt, output_text, input_tokens, output_tokens, cost_usd],
    )?;
    Ok(())
}

pub struct TransformationRecord {
    pub id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
    pub created_at: String,
}

pub fn get_recent_transformations(
    conn: &Connection,
    api_key: &str,
    limit: i32,
) -> Result<Vec<TransformationRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, input_tokens, output_tokens, cost_usd, created_at FROM transformations WHERE api_key = ?1 ORDER BY created_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(crate::params![api_key, limit], |row| {
        Ok(TransformationRecord {
            id: row.get(0)?,
            input_tokens: row.get(1)?,
            output_tokens: row.get(2)?,
            cost_usd: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// --- Agents ---
pub struct AgentRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub api_key: String,
    pub free_generations_remaining: i32,
    pub total_sessions: i32,
    pub created_at: String,
}

pub fn insert_agent(
    conn: &Connection,
    id: &str,
    name: &str,
    description: Option<&str>,
    model: Option<&str>,
    api_key: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO agents (id, name, description, model, api_key) VALUES (?1, ?2, ?3, ?4, ?5)",
        crate::params![id, name, description, model, api_key],
    )?;
    Ok(())
}

pub fn get_agent_by_key(conn: &Connection, api_key: &str) -> Result<Option<AgentRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, model, api_key, free_sessions_remaining, total_sessions, created_at FROM agents WHERE api_key = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![api_key], |row| {
        Ok(AgentRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            model: row.get(3)?,
            api_key: row.get(4)?,
            free_generations_remaining: row.get(5)?,
            total_sessions: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn decrement_free_session(conn: &Connection, agent_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE agents SET free_sessions_remaining = free_sessions_remaining - 1, total_sessions = total_sessions + 1 WHERE id = ?1 AND free_sessions_remaining > 0",
        crate::params![agent_id],
    )?;
    Ok(())
}

pub fn increment_agent_sessions(conn: &Connection, agent_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE agents SET total_sessions = total_sessions + 1 WHERE id = ?1",
        crate::params![agent_id],
    )?;
    Ok(())
}

// --- Agent Session Events ---
pub struct AgentSessionOwner {
    pub user_id: String,
    pub agent_id: String,
    pub agent_name: String,
}

pub struct AgentSessionEventRecord {
    pub id: i64,
    pub session_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub event_type: String,
    pub chunk_index: Option<i32>,
    pub elapsed_seconds: f64,
    pub words_total: i32,
    pub chunk_text: Option<String>,
    pub chunk_word_count: Option<i32>,
    pub detail_json: Option<String>,
    pub created_at: String,
}

#[allow(clippy::too_many_arguments)]
pub fn insert_agent_session_event(
    conn: &Connection,
    session_id: &str,
    user_id: &str,
    agent_id: &str,
    agent_name: &str,
    event_type: &str,
    chunk_index: Option<i32>,
    elapsed_seconds: f64,
    words_total: i32,
    chunk_text: Option<&str>,
    chunk_word_count: Option<i32>,
    detail_json: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO agent_session_events (
            session_id,
            user_id,
            agent_id,
            agent_name,
            event_type,
            chunk_index,
            elapsed_seconds,
            words_total,
            chunk_text,
            chunk_word_count,
            detail_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        crate::params![
            session_id,
            user_id,
            agent_id,
            agent_name,
            event_type,
            chunk_index,
            elapsed_seconds,
            words_total,
            chunk_text,
            chunk_word_count,
            detail_json
        ],
    )?;
    Ok(())
}

pub fn get_agent_session_owner(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<AgentSessionOwner>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, agent_id, agent_name
         FROM agent_session_events
         WHERE session_id = ?1
         ORDER BY id ASC
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![session_id], |row| {
        Ok(AgentSessionOwner {
            user_id: row.get(0)?,
            agent_id: row.get(1)?,
            agent_name: row.get(2)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn list_agent_session_events(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<AgentSessionEventRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
            id,
            session_id,
            user_id,
            agent_id,
            agent_name,
            event_type,
            chunk_index,
            elapsed_seconds,
            words_total,
            chunk_text,
            chunk_word_count,
            detail_json,
            created_at
         FROM agent_session_events
         WHERE session_id = ?1
         ORDER BY id ASC",
    )?;

    let rows = stmt.query_map(crate::params![session_id], |row| {
        Ok(AgentSessionEventRecord {
            id: row.get(0)?,
            session_id: row.get(1)?,
            user_id: row.get(2)?,
            agent_id: row.get(3)?,
            agent_name: row.get(4)?,
            event_type: row.get(5)?,
            chunk_index: row.get(6)?,
            elapsed_seconds: row.get(7)?,
            words_total: row.get(8)?,
            chunk_text: row.get(9)?,
            chunk_word_count: row.get(10)?,
            detail_json: row.get(11)?,
            created_at: row.get(12)?,
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

// --- Writing Checkpoints ---
pub fn insert_checkpoint(
    conn: &Connection,
    session_id: &str,
    content: &str,
    elapsed: f64,
    word_count: i32,
    session_token: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO writing_checkpoints (session_id, content, elapsed_seconds, word_count, session_token) VALUES (?1, ?2, ?3, ?4, ?5)",
        crate::params![session_id, content, elapsed, word_count, session_token],
    )?;
    Ok(())
}

pub struct CheckpointRecord {
    pub elapsed_seconds: f64,
    pub session_token: Option<String>,
    pub created_at: String,
}

pub fn get_latest_checkpoint(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<CheckpointRecord>> {
    let mut stmt = conn.prepare(
        "SELECT elapsed_seconds, session_token, created_at FROM writing_checkpoints WHERE session_id = ?1 ORDER BY id DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![session_id], |row| {
        Ok(CheckpointRecord {
            elapsed_seconds: row.get(0)?,
            session_token: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Recover orphaned checkpoints: sessions that have checkpoints but no writing_session.
/// Only recovers sessions older than 10 minutes (to avoid grabbing active sessions).
/// Uses the checkpoint session_id as the writing_session id to prevent duplicate recovery.
pub fn recover_orphaned_checkpoints(conn: &Connection) -> Result<i32> {
    // Find checkpoint session_ids that have no matching writing_session (by id),
    // where the latest checkpoint is older than 10 minutes
    let mut stmt = conn.prepare(
        "SELECT c.session_id, MAX(c.elapsed_seconds) as elapsed, MAX(c.word_count) as words
         FROM writing_checkpoints c
         WHERE NOT EXISTS (
             SELECT 1 FROM writing_sessions ws WHERE ws.id = c.session_id
         )
         AND c.created_at < datetime('now', '-10 minutes')
         GROUP BY c.session_id
         HAVING MAX(c.elapsed_seconds) >= 60
         ORDER BY MAX(c.created_at) DESC
         LIMIT 20",
    )?;

    let orphans: Vec<(String, f64, i32)> = stmt
        .query_map(crate::params![], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut recovered = 0;
    for (session_id, elapsed, word_count) in &orphans {
        // Get the content from the latest checkpoint for this session
        let latest_content: String = conn.query_row(
            "SELECT content FROM writing_checkpoints WHERE session_id = ?1 ORDER BY elapsed_seconds DESC LIMIT 1",
            crate::params![session_id],
            |row| row.get(0),
        )?;

        let is_anky = *elapsed >= 480.0 && *word_count >= 300;

        // Get the created_at from the first checkpoint of this session
        let created_at: String = conn.query_row(
            "SELECT created_at FROM writing_checkpoints WHERE session_id = ?1 ORDER BY elapsed_seconds ASC LIMIT 1",
            crate::params![session_id],
            |row| row.get(0),
        )?;

        // Try to find the real user from the checkpoint's session_token → auth_sessions,
        // then fall back to the most recent writing_sessions row before this checkpoint.
        let session_token: Option<String> = conn.query_row(
            "SELECT session_token FROM writing_checkpoints WHERE session_id = ?1 AND session_token IS NOT NULL ORDER BY elapsed_seconds DESC LIMIT 1",
            crate::params![session_id],
            |row| row.get(0),
        ).ok();

        let user_id: String = if let Some(ref token) = session_token {
            // Look up user from auth_sessions via session_token
            conn.query_row(
                "SELECT user_id FROM auth_sessions WHERE session_token = ?1 LIMIT 1",
                crate::params![token],
                |row| row.get(0),
            ).unwrap_or_else(|_| {
                // Token didn't match auth_sessions — try writing_sessions with same token
                conn.query_row(
                    "SELECT user_id FROM writing_sessions WHERE session_token = ?1 AND user_id NOT IN ('system', 'recovered-unknown') LIMIT 1",
                    crate::params![token],
                    |row| row.get(0),
                ).unwrap_or_else(|_| "recovered-unknown".to_string())
            })
        } else {
            conn.query_row(
                "SELECT COALESCE(
                    (SELECT user_id FROM writing_sessions WHERE created_at < ?1 AND user_id NOT IN ('system', 'recovered-unknown') ORDER BY created_at DESC LIMIT 1),
                    'recovered-unknown'
                )",
                crate::params![&created_at],
                |row| row.get(0),
            ).unwrap_or_else(|_| "recovered-unknown".to_string())
        };

        // Use the checkpoint session_id as the writing_session id — this prevents
        // duplicate recovery since the NOT EXISTS check uses ws.id = c.session_id
        conn.execute(
            "INSERT INTO writing_sessions (id, user_id, content, duration_seconds, word_count, is_anky, response, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'recovered from checkpoints', ?7)",
            crate::params![session_id, &user_id, &latest_content, elapsed, word_count, is_anky, &created_at],
        )?;
        recovered += 1;
    }

    Ok(recovered)
}

// --- Cost Estimates ---
/// Average total cost per anky from cost_records (grouped by related_id).
pub fn get_average_anky_cost(conn: &Connection) -> Result<f64> {
    let avg: f64 = conn.query_row(
        "SELECT COALESCE(AVG(total_cost), 0) FROM (
            SELECT SUM(cost_usd) as total_cost
            FROM cost_records
            WHERE related_id IS NOT NULL
            GROUP BY related_id
        )",
        crate::params![],
        |row| row.get(0),
    )?;
    Ok(avg)
}

/// Get failed ankys (status = 'pending' or 'failed' with a writing session).
pub fn get_failed_ankys(conn: &Connection) -> Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.writing_session_id, w.content
         FROM ankys a
         JOIN writing_sessions w ON w.id = a.writing_session_id
         WHERE a.status IN ('pending', 'failed', 'generating')
         AND a.created_at < datetime('now', '-2 minutes')
         ORDER BY a.created_at ASC
         LIMIT 10",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Get ankys that are 'complete' but missing a reflection (title or reflection is NULL/empty).
/// These slipped through the streaming pipeline and need recovery.
pub fn get_ankys_missing_reflection(
    conn: &Connection,
) -> Result<Vec<(String, String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.writing_session_id, a.user_id, w.content
         FROM ankys a
         JOIN writing_sessions w ON w.id = a.writing_session_id
         WHERE a.status = 'complete'
         AND (a.reflection IS NULL OR a.reflection = '')
         AND w.content IS NOT NULL AND w.content != ''
         AND a.created_at > datetime('now', '-24 hours')
         ORDER BY a.created_at DESC
         LIMIT 5",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

pub fn mark_anky_failed(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET status = 'failed' WHERE id = ?1 AND status IN ('pending', 'generating', 'failed')",
        crate::params![id],
    )?;
    Ok(())
}

pub fn set_anky_kingdom(
    conn: &Connection,
    anky_id: &str,
    kingdom_id: u8,
    kingdom_name: &str,
    kingdom_chakra: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET kingdom_id = ?2, kingdom_name = ?3, kingdom_chakra = ?4 WHERE id = ?1",
        crate::params![anky_id, kingdom_id as i32, kingdom_name, kingdom_chakra],
    )?;
    Ok(())
}

pub fn get_user_farcaster_fid(conn: &Connection, user_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT farcaster_fid FROM users WHERE id = ?1")?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        row.get::<_, Option<String>>(0)
    })?;
    Ok(rows.next().and_then(|r| r.ok()).flatten())
}

pub fn increment_anky_retry(conn: &Connection, anky_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET retry_count = COALESCE(retry_count, 0) + 1, last_retry_at = datetime('now') WHERE id = ?1",
        crate::params![anky_id],
    )?;
    Ok(())
}

pub fn get_anky_retry_info(conn: &Connection, anky_id: &str) -> Result<(u32, Option<String>)> {
    let mut stmt =
        conn.prepare("SELECT COALESCE(retry_count, 0), last_retry_at FROM ankys WHERE id = ?1")?;
    Ok(stmt.query_row(crate::params![anky_id], |row| {
        Ok((row.get::<_, u32>(0)?, row.get::<_, Option<String>>(1)?))
    })?)
}

pub fn mark_anky_abandoned(conn: &Connection, anky_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET status = 'abandoned' WHERE id = ?1",
        crate::params![anky_id],
    )?;
    Ok(())
}

// --- Feedback ---
pub struct FeedbackRecord {
    pub id: String,
    pub source: String,
    pub author: Option<String>,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

pub fn insert_feedback(
    conn: &Connection,
    id: &str,
    source: &str,
    author: Option<&str>,
    content: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO feedback (id, source, author, content) VALUES (?1, ?2, ?3, ?4)",
        crate::params![id, source, author, content],
    )?;
    Ok(())
}

pub fn get_all_feedback(conn: &Connection) -> Result<Vec<FeedbackRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, source, author, content, status, created_at FROM feedback ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok(FeedbackRecord {
            id: row.get(0)?,
            source: row.get(1)?,
            author: row.get(2)?,
            content: row.get(3)?,
            status: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// --- Generation Records ---
pub fn insert_generation_record(
    conn: &Connection,
    id: &str,
    anky_id: &str,
    api_key: Option<&str>,
    agent_id: Option<&str>,
    payment_method: &str,
    amount_usd: f64,
    tx_hash: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO generation_records (id, anky_id, api_key, agent_id, payment_method, amount_usd, tx_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        crate::params![id, anky_id, api_key, agent_id, payment_method, amount_usd, tx_hash],
    )?;
    Ok(())
}

// --- User Collections (privacy) ---
pub fn collect_anky(conn: &Connection, user_id: &str, anky_id: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO user_collections (user_id, anky_id) VALUES (?1, ?2)",
        crate::params![user_id, anky_id],
    )?;
    Ok(())
}

pub fn has_collected(conn: &Connection, user_id: &str, anky_id: &str) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM user_collections WHERE user_id = ?1 AND anky_id = ?2",
        crate::params![user_id, anky_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn get_anky_owner(conn: &Connection, anky_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT user_id FROM ankys WHERE id = ?1")?;
    let mut rows = stmt.query_map(crate::params![anky_id], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

// ===== Prompts =====

pub struct PromptRecord {
    pub id: String,
    pub creator_user_id: String,
    pub prompt_text: String,
    pub image_path: Option<String>,
    pub status: String,
    pub payment_tx_hash: Option<String>,
    pub created_at: String,
    pub created_by: Option<String>,
}

pub fn insert_prompt(
    conn: &Connection,
    id: &str,
    creator_user_id: &str,
    prompt_text: &str,
    payment_tx_hash: Option<&str>,
    created_by: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO prompts (id, creator_user_id, prompt_text, payment_tx_hash, created_by) VALUES (?1, ?2, ?3, ?4, ?5)",
        crate::params![id, creator_user_id, prompt_text, payment_tx_hash, created_by],
    )?;
    Ok(())
}

pub fn get_prompt_by_id(conn: &Connection, id: &str) -> Result<Option<PromptRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, creator_user_id, prompt_text, image_path, status, payment_tx_hash, created_at, created_by FROM prompts WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![id], |row| {
        Ok(PromptRecord {
            id: row.get(0)?,
            creator_user_id: row.get(1)?,
            prompt_text: row.get(2)?,
            image_path: row.get(3)?,
            status: row.get(4)?,
            payment_tx_hash: row.get(5)?,
            created_at: row.get(6)?,
            created_by: row.get(7)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn update_prompt_image(conn: &Connection, id: &str, image_path: &str) -> Result<()> {
    conn.execute(
        "UPDATE prompts SET image_path = ?2, status = 'complete' WHERE id = ?1",
        crate::params![id, image_path],
    )?;
    Ok(())
}

pub fn update_prompt_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE prompts SET status = ?2 WHERE id = ?1",
        crate::params![id, status],
    )?;
    Ok(())
}

pub fn get_user_prompts(conn: &Connection, user_id: &str) -> Result<Vec<PromptRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, creator_user_id, prompt_text, image_path, status, payment_tx_hash, created_at, created_by FROM prompts WHERE creator_user_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![user_id], |row| {
        Ok(PromptRecord {
            id: row.get(0)?,
            creator_user_id: row.get(1)?,
            prompt_text: row.get(2)?,
            image_path: row.get(3)?,
            status: row.get(4)?,
            payment_tx_hash: row.get(5)?,
            created_at: row.get(6)?,
            created_by: row.get(7)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub struct PromptListItem {
    pub id: String,
    pub prompt_text: String,
    pub image_path: Option<String>,
    pub creator_username: String,
    pub sessions_count: i32,
    pub created_at: String,
    pub created_by: Option<String>,
}

pub fn get_prompts_paginated(
    conn: &Connection,
    page: i32,
    limit: i32,
    sort: &str,
) -> Result<(Vec<PromptListItem>, i32)> {
    let offset = (page - 1) * limit;
    let total: i32 = conn.query_row(
        "SELECT COUNT(*) FROM prompts WHERE status = 'complete'",
        crate::params![],
        |row| row.get(0),
    )?;
    let order_clause = if sort == "popular" {
        "ORDER BY sessions_count DESC, p.created_at DESC"
    } else {
        "ORDER BY p.created_at DESC"
    };
    let sql = format!(
        "SELECT p.id, p.prompt_text, p.image_path,
                COALESCE(u.username, (SELECT xu.username FROM x_users xu WHERE xu.user_id = p.creator_user_id LIMIT 1), 'someone') as creator_username,
                (SELECT COUNT(*) FROM prompt_sessions ps WHERE ps.prompt_id = p.id) as sessions_count,
                p.created_at,
                p.created_by
         FROM prompts p
         LEFT JOIN users u ON u.id = p.creator_user_id
         WHERE p.status = 'complete'
         {}
         LIMIT ?1 OFFSET ?2",
        order_clause
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(crate::params![limit, offset], |row| {
        Ok(PromptListItem {
            id: row.get(0)?,
            prompt_text: row.get(1)?,
            image_path: row.get(2)?,
            creator_username: row.get(3)?,
            sessions_count: row.get(4)?,
            created_at: row.get(5)?,
            created_by: row.get(6)?,
        })
    })?;
    Ok((rows.filter_map(|r| r.ok()).collect(), total))
}

pub fn get_random_prompt(conn: &Connection) -> Result<Option<PromptListItem>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.prompt_text, p.image_path,
                COALESCE(u.username, (SELECT xu.username FROM x_users xu WHERE xu.user_id = p.creator_user_id LIMIT 1), 'someone') as creator_username,
                (SELECT COUNT(*) FROM prompt_sessions ps WHERE ps.prompt_id = p.id) as sessions_count,
                p.created_at,
                p.created_by
         FROM prompts p
         LEFT JOIN users u ON u.id = p.creator_user_id
         WHERE p.status = 'complete'
         ORDER BY RANDOM()
         LIMIT 1"
    )?;
    let mut rows = stmt.query_map(crate::params![], |row| {
        Ok(PromptListItem {
            id: row.get(0)?,
            prompt_text: row.get(1)?,
            image_path: row.get(2)?,
            creator_username: row.get(3)?,
            sessions_count: row.get(4)?,
            created_at: row.get(5)?,
            created_by: row.get(6)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn get_prompt_session_count(conn: &Connection, prompt_id: &str) -> Result<i32> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM prompt_sessions WHERE prompt_id = ?1",
        crate::params![prompt_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub fn get_failed_prompts(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, prompt_text FROM prompts WHERE status IN ('failed', 'pending') AND image_path IS NULL",
    )?;
    let rows = stmt.query_map(crate::params![], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ===== Prompt Sessions =====

pub struct PromptSessionRecord {
    pub id: String,
    pub prompt_id: String,
    pub user_id: Option<String>,
    pub content: Option<String>,
    pub keystroke_deltas: Option<String>,
    pub duration_seconds: Option<f64>,
    pub word_count: i32,
    pub completed: bool,
    pub created_at: String,
}

pub fn insert_prompt_session(
    conn: &Connection,
    id: &str,
    prompt_id: &str,
    user_id: Option<&str>,
    content: &str,
    keystroke_deltas: &str,
    page_opened_at: &str,
    first_keystroke_at: Option<&str>,
    duration_seconds: f64,
    word_count: i32,
) -> Result<()> {
    conn.execute(
        "INSERT INTO prompt_sessions (id, prompt_id, user_id, content, keystroke_deltas, page_opened_at, first_keystroke_at, duration_seconds, word_count, completed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
        crate::params![id, prompt_id, user_id, content, keystroke_deltas, page_opened_at, first_keystroke_at, duration_seconds, word_count],
    )?;
    Ok(())
}

pub fn get_prompt_sessions_for_prompt(
    conn: &Connection,
    prompt_id: &str,
) -> Result<Vec<PromptSessionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, prompt_id, user_id, content, keystroke_deltas, duration_seconds, word_count, completed, created_at FROM prompt_sessions WHERE prompt_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![prompt_id], |row| {
        Ok(PromptSessionRecord {
            id: row.get(0)?,
            prompt_id: row.get(1)?,
            user_id: row.get(2)?,
            content: row.get(3)?,
            keystroke_deltas: row.get(4)?,
            duration_seconds: row.get(5)?,
            word_count: row.get(6)?,
            completed: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ===== X Users / Auth =====

pub struct XUserRecord {
    pub x_user_id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub profile_image_url: Option<String>,
}

pub fn upsert_x_user(
    conn: &Connection,
    x_user_id: &str,
    user_id: &str,
    username: &str,
    display_name: Option<&str>,
    profile_image_url: Option<&str>,
    access_token: &str,
    refresh_token: Option<&str>,
    token_expires_at: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO x_users (x_user_id, user_id, username, display_name, profile_image_url, access_token, refresh_token, token_expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(x_user_id) DO UPDATE SET
            username = excluded.username,
            display_name = excluded.display_name,
            profile_image_url = excluded.profile_image_url,
            access_token = excluded.access_token,
            refresh_token = COALESCE(excluded.refresh_token, x_users.refresh_token),
            token_expires_at = excluded.token_expires_at,
            updated_at = datetime('now')",
        crate::params![x_user_id, user_id, username, display_name, profile_image_url, access_token, refresh_token, token_expires_at],
    )?;
    Ok(())
}

pub fn get_x_user_by_x_id(conn: &Connection, x_user_id: &str) -> Result<Option<XUserRecord>> {
    let mut stmt = conn.prepare(
        "SELECT x_user_id, user_id, username, display_name, profile_image_url FROM x_users WHERE x_user_id = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![x_user_id], |row| {
        Ok(XUserRecord {
            x_user_id: row.get(0)?,
            user_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            profile_image_url: row.get(4)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn create_auth_session(
    conn: &Connection,
    token: &str,
    user_id: &str,
    x_user_id: Option<&str>,
    expires_at: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO auth_sessions (token, user_id, x_user_id, expires_at) VALUES (?1, ?2, ?3, ?4)",
        crate::params![token, user_id, x_user_id, expires_at],
    )?;
    Ok(())
}

pub fn get_auth_session(
    conn: &Connection,
    token: &str,
) -> Result<Option<(String, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, x_user_id
         FROM auth_sessions
         WHERE token = ?1 AND datetime(expires_at) > datetime('now')",
    )?;
    let mut rows = stmt.query_map(crate::params![token], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn delete_auth_session(conn: &Connection, token: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM auth_sessions WHERE token = ?1",
        crate::params![token],
    )?;
    Ok(())
}

pub struct AuthChallengeRecord {
    pub id: String,
    pub wallet_address: String,
    pub challenge_text: String,
    pub expires_at: String,
}

pub fn create_auth_challenge(
    conn: &Connection,
    id: &str,
    wallet_address: &str,
    challenge_text: &str,
    expires_at: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO auth_challenges (id, wallet_address, challenge_text, expires_at)
         VALUES (?1, ?2, ?3, ?4)",
        crate::params![id, wallet_address.trim(), challenge_text, expires_at],
    )?;
    Ok(())
}

pub fn get_active_auth_challenge(
    conn: &Connection,
    id: &str,
) -> Result<Option<AuthChallengeRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, wallet_address, challenge_text, expires_at
         FROM auth_challenges
         WHERE id = ?1
           AND consumed_at IS NULL
           AND datetime(expires_at) > datetime('now')
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![id], |row| {
        Ok(AuthChallengeRecord {
            id: row.get(0)?,
            wallet_address: row.get(1)?,
            challenge_text: row.get(2)?,
            expires_at: row.get(3)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn consume_auth_challenge(conn: &Connection, id: &str) -> Result<bool> {
    let updated = conn.execute(
        "UPDATE auth_challenges
         SET consumed_at = datetime('now')
         WHERE id = ?1 AND consumed_at IS NULL",
        crate::params![id],
    )?;
    Ok(updated > 0)
}

pub fn save_oauth_state(
    conn: &Connection,
    state: &str,
    code_verifier: &str,
    redirect_to: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO oauth_states (state, code_verifier, redirect_to) VALUES (?1, ?2, ?3)",
        crate::params![state, code_verifier, redirect_to],
    )?;
    Ok(())
}

pub fn get_and_delete_oauth_state(
    conn: &Connection,
    state: &str,
) -> Result<Option<(String, Option<String>)>> {
    let mut stmt =
        conn.prepare("SELECT code_verifier, redirect_to FROM oauth_states WHERE state = ?1")?;
    let mut rows = stmt.query_map(crate::params![state], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    let result = rows.next().and_then(|r| r.ok());
    if result.is_some() {
        conn.execute(
            "DELETE FROM oauth_states WHERE state = ?1",
            crate::params![state],
        )?;
    }
    Ok(result)
}

// ===== X Interactions (Bot) =====

pub fn insert_x_interaction(
    conn: &Connection,
    id: &str,
    tweet_id: &str,
    x_user_id: Option<&str>,
    x_username: Option<&str>,
    tweet_text: Option<&str>,
    status: &str,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO x_interactions (id, tweet_id, x_user_id, x_username, tweet_text, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        crate::params![id, tweet_id, x_user_id, x_username, tweet_text, status],
    )?;
    Ok(())
}

pub fn update_x_interaction_status(
    conn: &Connection,
    id: &str,
    status: &str,
    classification: Option<&str>,
    prompt_id: Option<&str>,
    reply_tweet_id: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE x_interactions SET status = ?2, classification = ?3, prompt_id = ?4, reply_tweet_id = ?5 WHERE id = ?1",
        crate::params![id, status, classification, prompt_id, reply_tweet_id],
    )?;
    Ok(())
}

pub fn interaction_exists(conn: &Connection, tweet_id: &str) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM x_interactions WHERE tweet_id = ?1",
        crate::params![tweet_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn get_latest_interaction_tweet_id(conn: &Connection) -> Result<Option<String>> {
    let mut stmt =
        conn.prepare("SELECT tweet_id FROM x_interactions ORDER BY created_at DESC LIMIT 1")?;
    let mut rows = stmt.query_map(crate::params![], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn count_user_interactions_today(conn: &Connection, x_user_id: &str) -> Result<i32> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM x_interactions WHERE x_user_id = ?1 AND created_at > datetime('now', '-1 day')",
        crate::params![x_user_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

// ===== Video Recordings =====

pub fn insert_video_recording(
    conn: &Connection,
    id: &str,
    user_id: Option<&str>,
    title: Option<&str>,
    file_path: &str,
    duration_seconds: f64,
    scene_data: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO video_recordings (id, user_id, title, file_path, duration_seconds, status, scene_data) VALUES (?1, ?2, ?3, ?4, ?5, 'uploaded', ?6)",
        crate::params![id, user_id, title, file_path, duration_seconds, scene_data],
    )?;
    Ok(())
}

pub fn update_video_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE video_recordings SET status = ?2 WHERE id = ?1",
        crate::params![id, status],
    )?;
    Ok(())
}

// ===== Stream Overlay =====

/// Get the latest completed anky with user info (for Farcaster OG embed).
pub struct LatestAnkyEmbed {
    pub title: Option<String>,
    pub image_path: String,
    pub display_username: String,
    pub pfp_url: Option<String>,
}

pub fn get_latest_anky_for_embed(conn: &Connection) -> Result<Option<LatestAnkyEmbed>> {
    let mut stmt = conn.prepare(
        "SELECT a.title, a.image_path,
                COALESCE(u.username, u.farcaster_username, (SELECT xu.username FROM x_users xu WHERE xu.user_id = a.user_id LIMIT 1), 'someone') as display_username,
                COALESCE(u.farcaster_pfp_url, (SELECT xu.profile_image_url FROM x_users xu WHERE xu.user_id = a.user_id LIMIT 1)) as pfp_url
         FROM ankys a
         JOIN users u ON u.id = a.user_id
         WHERE a.status = 'complete' AND a.image_path IS NOT NULL
         ORDER BY a.created_at DESC
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![], |row| {
        Ok(LatestAnkyEmbed {
            title: row.get(0)?,
            image_path: row.get(1)?,
            display_username: row.get(2)?,
            pfp_url: row.get(3)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Get today's completed ankys with images (for stream overlay stickers).
pub fn get_todays_ankys(conn: &Connection) -> Result<Vec<AnkyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, image_path, image_webp, reflection, image_prompt, thinker_name, status, created_at, origin, COALESCE(image_model, 'gemini')
         FROM ankys
         WHERE status = 'complete' AND image_path IS NOT NULL AND date(created_at) = date('now')
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok(AnkyRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            image_webp: row.get(3)?,
            reflection: row.get(4)?,
            image_prompt: row.get(5)?,
            thinker_name: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            origin: row.get(9)?,
            image_model: row.get(10).unwrap_or_else(|_| "gemini".to_string()),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ===== Video Projects =====

pub fn insert_video_project(
    conn: &Connection,
    id: &str,
    user_id: &str,
    anky_id: Option<&str>,
    writing_session_id: Option<&str>,
    script_json: &str,
    total_scenes: i32,
    payment_tx_hash: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO video_projects (id, user_id, anky_id, writing_session_id, script_json, total_scenes, status, payment_tx_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'generating', ?7)",
        crate::params![id, user_id, anky_id, writing_session_id, script_json, total_scenes, payment_tx_hash],
    )?;
    Ok(())
}

/// Insert a video project immediately with 'pending' status (before script generation).
pub fn insert_video_project_pending(
    conn: &Connection,
    id: &str,
    user_id: &str,
    anky_id: &str,
    payment_tx_hash: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO video_projects (id, user_id, anky_id, script_json, total_scenes, status, current_step, payment_tx_hash) VALUES (?1, ?2, ?3, '', 0, 'pending', 'script', ?4)",
        crate::params![id, user_id, anky_id, payment_tx_hash],
    )?;
    Ok(())
}

/// Update a pending video project with generated script data and set status to 'generating'.
pub fn update_video_project_script(
    conn: &Connection,
    id: &str,
    script_json: &str,
    total_scenes: i32,
) -> Result<()> {
    conn.execute(
        "UPDATE video_projects SET script_json = ?2, total_scenes = ?3, status = 'generating', current_step = 'generating' WHERE id = ?1",
        crate::params![id, script_json, total_scenes],
    )?;
    Ok(())
}

pub fn update_video_project_progress(conn: &Connection, id: &str, completed: i32) -> Result<()> {
    conn.execute(
        "UPDATE video_projects SET completed_scenes = ?2 WHERE id = ?1",
        crate::params![id, completed],
    )?;
    Ok(())
}

pub fn update_video_project_complete(
    conn: &Connection,
    id: &str,
    video_path: &str,
    script_json: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE video_projects SET status = 'complete', video_path = ?2, script_json = ?3 WHERE id = ?1",
        crate::params![id, video_path, script_json],
    )?;
    Ok(())
}

pub fn update_video_project_step(conn: &Connection, id: &str, step: &str) -> Result<()> {
    conn.execute(
        "UPDATE video_projects SET current_step = ?2 WHERE id = ?1",
        crate::params![id, step],
    )?;
    Ok(())
}

pub fn update_video_project_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE video_projects SET status = ?2 WHERE id = ?1",
        crate::params![id, status],
    )?;
    Ok(())
}

pub struct VideoProjectRecord {
    pub id: String,
    pub user_id: String,
    pub anky_id: Option<String>,
    pub script_json: Option<String>,
    pub status: String,
    pub video_path: Option<String>,
    pub video_path_720p: Option<String>,
    pub video_path_360p: Option<String>,
    pub story_spine: Option<String>,
    pub total_scenes: i32,
    pub completed_scenes: i32,
    pub current_step: Option<String>,
    pub created_at: String,
}

pub fn get_video_project(conn: &Connection, id: &str) -> Result<Option<VideoProjectRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, anky_id, script_json, status, video_path, total_scenes, completed_scenes, current_step, created_at, video_path_720p, video_path_360p, story_spine FROM video_projects WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![id], |row| {
        Ok(VideoProjectRecord {
            id: row.get(0)?,
            user_id: row.get(1)?,
            anky_id: row.get(2)?,
            script_json: row.get(3)?,
            status: row.get(4)?,
            video_path: row.get(5)?,
            total_scenes: row.get(6)?,
            completed_scenes: row.get(7)?,
            current_step: row.get(8)?,
            created_at: row.get(9)?,
            video_path_720p: row.get(10)?,
            video_path_360p: row.get(11)?,
            story_spine: row.get(12)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn get_user_video_projects(
    conn: &Connection,
    user_id: &str,
) -> Result<Vec<VideoProjectRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, anky_id, script_json, status, video_path, total_scenes, completed_scenes, current_step, created_at, video_path_720p, video_path_360p, story_spine FROM video_projects WHERE user_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![user_id], |row| {
        Ok(VideoProjectRecord {
            id: row.get(0)?,
            user_id: row.get(1)?,
            anky_id: row.get(2)?,
            script_json: row.get(3)?,
            status: row.get(4)?,
            video_path: row.get(5)?,
            total_scenes: row.get(6)?,
            completed_scenes: row.get(7)?,
            current_step: row.get(8)?,
            created_at: row.get(9)?,
            video_path_720p: row.get(10)?,
            video_path_360p: row.get(11)?,
            story_spine: row.get(12)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Find an active (pending/generating) video project for a given anky.
pub fn find_active_video_project_for_anky(
    conn: &Connection,
    anky_id: &str,
) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM video_projects WHERE anky_id = ?1 AND status IN ('pending', 'generating') ORDER BY created_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![anky_id], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn update_video_project_paths(
    conn: &Connection,
    id: &str,
    path_720p: &str,
    path_360p: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE video_projects SET video_path_720p = ?2, video_path_360p = ?3 WHERE id = ?1",
        crate::params![id, path_720p, path_360p],
    )?;
    Ok(())
}

pub fn update_video_project_story_spine(
    conn: &Connection,
    id: &str,
    story_spine: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE video_projects SET story_spine = ?2 WHERE id = ?1",
        crate::params![id, story_spine],
    )?;
    Ok(())
}

/// Get the latest complete anky with its writing text for a user.
pub fn get_latest_user_anky_with_writing(
    conn: &Connection,
    user_id: &str,
) -> Result<Option<(String, String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, COALESCE(a.title, ''), ws.content, COALESCE(a.image_path, '')
         FROM ankys a
         JOIN writing_sessions ws ON ws.id = a.writing_session_id
         WHERE a.user_id = ?1 AND ws.is_anky = 1
         ORDER BY a.created_at DESC
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Get ALL user writing sessions that are ankys (for the video page selector).
pub fn get_user_anky_writings(
    conn: &Connection,
    user_id: &str,
) -> Result<Vec<(String, String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, COALESCE(a.title, 'untitled'), SUBSTR(ws.content, 1, 120), COALESCE(a.image_path, '')
         FROM ankys a
         JOIN writing_sessions ws ON ws.id = a.writing_session_id
         WHERE a.user_id = ?1 AND ws.is_anky = 1
         ORDER BY a.created_at DESC
         LIMIT 20"
    )?;
    let rows = stmt.query_map(crate::params![user_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ===== Feed =====

pub struct FeedItem {
    pub id: String,
    pub title: Option<String>,
    pub image_webp: Option<String>,
    pub image_path: Option<String>,
    pub thinker_name: Option<String>,
    pub created_at: String,
    pub like_count: i32,
    pub user_liked: bool,
}

pub fn get_feed(
    conn: &Connection,
    viewer_user_id: Option<&str>,
    page: i32,
    per_page: i32,
) -> Result<Vec<FeedItem>> {
    let offset = (page - 1) * per_page;
    let viewer = viewer_user_id.unwrap_or("");
    let mut stmt = conn.prepare(
        "SELECT a.id, a.title, a.image_webp, a.image_path,
                COALESCE(u.username, u.farcaster_username, (SELECT xu.username FROM x_users xu WHERE xu.user_id = a.user_id LIMIT 1), 'someone') as thinker_name,
                a.created_at,
                (SELECT COUNT(*) FROM anky_likes al WHERE al.anky_id = a.id) as like_count,
                (SELECT COUNT(*) FROM anky_likes al WHERE al.anky_id = a.id AND al.user_id = ?3) as user_liked
         FROM ankys a
         JOIN users u ON u.id = a.user_id
         WHERE a.status = 'complete' AND a.image_path IS NOT NULL
         ORDER BY a.created_at DESC
         LIMIT ?1 OFFSET ?2"
    )?;
    let rows = stmt.query_map(crate::params![per_page, offset, viewer], |row| {
        Ok(FeedItem {
            id: row.get(0)?,
            title: row.get(1)?,
            image_webp: row.get(2)?,
            image_path: row.get(3)?,
            thinker_name: row.get(4)?,
            created_at: row.get(5)?,
            like_count: row.get(6)?,
            user_liked: row.get::<_, i32>(7)? > 0,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ===== Likes =====

pub fn toggle_like(conn: &Connection, user_id: &str, anky_id: &str) -> Result<bool> {
    let exists: i32 = conn.query_row(
        "SELECT COUNT(*) FROM anky_likes WHERE user_id = ?1 AND anky_id = ?2",
        crate::params![user_id, anky_id],
        |row| row.get(0),
    )?;
    if exists > 0 {
        conn.execute(
            "DELETE FROM anky_likes WHERE user_id = ?1 AND anky_id = ?2",
            crate::params![user_id, anky_id],
        )?;
        Ok(false)
    } else {
        conn.execute(
            "INSERT INTO anky_likes (user_id, anky_id) VALUES (?1, ?2)",
            crate::params![user_id, anky_id],
        )?;
        Ok(true)
    }
}

pub fn get_like_count(conn: &Connection, anky_id: &str) -> Result<i32> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM anky_likes WHERE anky_id = ?1",
        crate::params![anky_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

// ===== Thumbnail =====

pub fn update_anky_thumb(conn: &Connection, id: &str, image_thumb: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET image_thumb = ?2 WHERE id = ?1",
        crate::params![id, image_thumb],
    )?;
    Ok(())
}

// ===== Feed Stats =====

pub struct FeedStats {
    pub total_sessions_24h: i32,
    pub total_ankys_24h: i32,
    pub unique_writers_24h: i32,
    pub total_minutes_24h: f64,
    pub total_words_24h: i32,
}

pub fn get_feed_stats_24h(conn: &Connection) -> Result<FeedStats> {
    let row = conn.query_row(
        "SELECT
            COUNT(*) as total_sessions,
            SUM(CASE WHEN is_anky = 1 THEN 1 ELSE 0 END) as total_ankys,
            COUNT(DISTINCT user_id) as unique_writers,
            COALESCE(SUM(duration_seconds), 0) as total_seconds,
            COALESCE(SUM(word_count), 0) as total_words
         FROM writing_sessions
         WHERE created_at > datetime('now', '-24 hours')
           AND COALESCE(status, 'completed') = 'completed'",
        crate::params![],
        |row| {
            Ok(FeedStats {
                total_sessions_24h: row.get(0)?,
                total_ankys_24h: row.get(1)?,
                unique_writers_24h: row.get(2)?,
                total_minutes_24h: row.get::<_, f64>(3)? / 60.0,
                total_words_24h: row.get(4)?,
            })
        },
    )?;
    Ok(row)
}

pub struct FeedAnky {
    pub id: String,
    pub title: Option<String>,
    pub image_path: Option<String>,
    pub image_webp: Option<String>,
    pub thinker_name: Option<String>,
    pub origin: String,
    pub created_at: String,
}

pub fn get_feed_ankys(conn: &Connection, limit: i32, offset: i32) -> Result<Vec<FeedAnky>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.title, a.image_path, a.image_webp,
                COALESCE(u.username, u.farcaster_username, (SELECT xu.username FROM x_users xu WHERE xu.user_id = a.user_id LIMIT 1), 'someone') as thinker_name,
                a.origin, a.created_at
         FROM ankys a
         LEFT JOIN users u ON u.id = a.user_id
         WHERE a.status = 'complete' AND a.image_path IS NOT NULL
         ORDER BY a.created_at DESC
         LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(crate::params![limit, offset], |row| {
        Ok(FeedAnky {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            image_webp: row.get(3)?,
            thinker_name: row.get(4)?,
            origin: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ===== Slideshow =====

pub struct SlideshowAnky {
    pub id: String,
    pub title: Option<String>,
    pub image_path: String,
    pub origin: String,
    pub display_username: String,
    pub created_at: String,
}

pub struct SlideshowVideo {
    pub id: String,
    pub video_path: String,
    pub created_at: String,
}

pub fn get_slideshow_videos(conn: &Connection) -> Result<Vec<SlideshowVideo>> {
    let mut stmt = conn.prepare(
        "SELECT id, video_path, created_at FROM video_projects WHERE status = 'complete' AND video_path IS NOT NULL",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok(SlideshowVideo {
            id: row.get(0)?,
            video_path: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_slideshow_ankys(conn: &Connection) -> Result<Vec<SlideshowAnky>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.title, a.image_path, a.origin,
                COALESCE(u.username, u.farcaster_username, (SELECT xu.username FROM x_users xu WHERE xu.user_id = a.user_id LIMIT 1), 'someone') as display_username,
                a.created_at
         FROM ankys a
         LEFT JOIN users u ON u.id = a.user_id
         WHERE a.status = 'complete' AND a.image_path IS NOT NULL
         ORDER BY a.created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok(SlideshowAnky {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            origin: row.get(3)?,
            display_username: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// --- Inquiry System ---

/// Get the current unanswered inquiry for a user.
pub fn get_current_inquiry(conn: &Connection, user_id: &str) -> Result<Option<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, question FROM user_inquiries
         WHERE user_id = ?1 AND response_text IS NULL AND skipped = 0
         ORDER BY created_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Create a new inquiry for a user.
pub fn create_inquiry(
    conn: &Connection,
    user_id: &str,
    question: &str,
    language: &str,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO user_inquiries (id, user_id, question, language) VALUES (?1, ?2, ?3, ?4)",
        crate::params![id, user_id, question, language],
    )?;
    Ok(id)
}

/// Mark an inquiry as answered with the user's writing text and session id.
pub fn mark_inquiry_answered(
    conn: &Connection,
    inquiry_id: &str,
    text: &str,
    session_id: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE user_inquiries SET response_text = ?2, response_session_id = ?3, answered_at = datetime('now') WHERE id = ?1",
        crate::params![inquiry_id, text, session_id],
    )?;
    Ok(())
}

/// Mark an inquiry as skipped.
pub fn mark_inquiry_skipped(conn: &Connection, inquiry_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE user_inquiries SET skipped = 1 WHERE id = ?1",
        crate::params![inquiry_id],
    )?;
    Ok(())
}

/// Get inquiry history for a user (for Claude context).
pub fn get_inquiry_history(
    conn: &Connection,
    user_id: &str,
    limit: i32,
) -> Result<Vec<(String, Option<String>, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT question, response_text, answered_at FROM user_inquiries
         WHERE user_id = ?1
         ORDER BY created_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(crate::params![user_id, limit], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Get the language stored on the user's most recent inquiry.
pub fn get_inquiry_language(conn: &Connection, user_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT language FROM user_inquiries WHERE user_id = ?1 ORDER BY created_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

// --- Interview System ---

use crate::models::{InterviewSummary, UserInterviewContext};

pub fn create_interview(
    conn: &Connection,
    id: &str,
    user_id: Option<&str>,
    guest_name: &str,
    is_anonymous: bool,
) -> Result<()> {
    conn.execute(
        "INSERT INTO interviews (id, user_id, guest_name, is_anonymous) VALUES (?1, ?2, ?3, ?4)",
        crate::params![id, user_id, guest_name, is_anonymous],
    )?;
    Ok(())
}

pub fn save_interview_message(
    conn: &Connection,
    interview_id: &str,
    role: &str,
    content: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO interview_messages (interview_id, role, content) VALUES (?1, ?2, ?3)",
        crate::params![interview_id, role, content],
    )?;
    // Update message count
    conn.execute(
        "UPDATE interviews SET message_count = (SELECT COUNT(*) FROM interview_messages WHERE interview_id = ?1) WHERE id = ?1",
        crate::params![interview_id],
    )?;
    Ok(())
}

pub fn end_interview(
    conn: &Connection,
    interview_id: &str,
    summary: Option<&str>,
    duration_seconds: Option<f64>,
    message_count: Option<i32>,
) -> Result<()> {
    conn.execute(
        "UPDATE interviews SET ended_at = datetime('now'), summary = COALESCE(?2, summary), duration_seconds = COALESCE(?3, duration_seconds), message_count = COALESCE(?4, message_count) WHERE id = ?1",
        crate::params![interview_id, summary, duration_seconds, message_count],
    )?;
    Ok(())
}

pub fn get_user_interview_history(
    conn: &Connection,
    user_id: &str,
    limit: i64,
) -> Result<Vec<InterviewSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, guest_name, started_at, summary, duration_seconds FROM interviews WHERE user_id = ?1 ORDER BY started_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(crate::params![user_id, limit], |row| {
        Ok(InterviewSummary {
            id: row.get(0)?,
            guest_name: row.get(1)?,
            started_at: row.get(2)?,
            summary: row.get(3)?,
            duration_seconds: row.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_user_context_for_interview(
    conn: &Connection,
    user_id: &str,
) -> Result<Option<UserInterviewContext>> {
    // Get username
    let username: Option<String> = conn
        .prepare("SELECT username FROM users WHERE id = ?1")?
        .query_map(crate::params![user_id], |row| {
            row.get::<_, Option<String>>(0)
        })?
        .next()
        .and_then(|r| r.ok())
        .flatten();

    // Get profile data
    let (psychological_profile, core_tensions, growth_edges) = conn
        .prepare("SELECT psychological_profile, core_tensions, growth_edges FROM user_profiles WHERE user_id = ?1")?
        .query_map(crate::params![user_id], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?
        .next()
        .and_then(|r| r.ok())
        .unwrap_or((None, None, None));

    // Get last 5 writing session summaries (response = Ollama/Claude feedback)
    let mut ws_stmt = conn.prepare(
        "SELECT response FROM writing_sessions
         WHERE user_id = ?1
           AND response IS NOT NULL
           AND COALESCE(status, 'completed') = 'completed'
         ORDER BY created_at DESC
         LIMIT 5",
    )?;
    let recent_writings: Vec<String> = ws_stmt
        .query_map(crate::params![user_id], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    // Get past interview summaries
    let past_interviews = get_user_interview_history(conn, user_id, 5)?;

    Ok(Some(UserInterviewContext {
        username,
        psychological_profile,
        core_tensions,
        growth_edges,
        recent_writings,
        past_interviews,
    }))
}

// --- Video Gallery ---

pub struct VideoGalleryItem {
    pub project_id: String,
    pub video_path: String,
    pub created_at: String,
    pub anky_title: Option<String>,
    pub image_path: Option<String>,
    pub image_webp: Option<String>,
    pub image_thumb: Option<String>,
}

pub fn get_all_complete_video_projects(conn: &Connection) -> Result<Vec<VideoGalleryItem>> {
    let mut stmt = conn.prepare(
        "SELECT vp.id, vp.video_path, vp.created_at,
                a.title, a.image_path, a.image_webp, a.image_thumb
         FROM video_projects vp
         LEFT JOIN ankys a ON a.id = vp.anky_id
         WHERE vp.status = 'complete' AND vp.video_path IS NOT NULL
         ORDER BY vp.created_at DESC",
    )?;
    let rows = stmt
        .query_map(crate::params![], |row| {
            Ok(VideoGalleryItem {
                project_id: row.get(0)?,
                video_path: row.get(1)?,
                created_at: row.get(2)?,
                anky_title: row.get(3)?,
                image_path: row.get(4)?,
                image_webp: row.get(5)?,
                image_thumb: row.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

// ===== Premium =====

pub fn is_user_premium(conn: &Connection, user_id: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT is_premium FROM users WHERE id = ?1")?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| row.get::<_, bool>(0))?;
    Ok(rows.next().and_then(|r| r.ok()).unwrap_or(false))
}

pub fn is_user_pro(conn: &Connection, user_id: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT is_pro FROM users WHERE id = ?1")?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| row.get::<_, bool>(0))?;
    Ok(rows.next().and_then(|r| r.ok()).unwrap_or(false))
}

pub fn set_user_premium(conn: &Connection, user_id: &str, is_premium: bool) -> Result<()> {
    if is_premium {
        conn.execute(
            "UPDATE users SET is_premium = 1, premium_since = datetime('now') WHERE id = ?1",
            crate::params![user_id],
        )?;
    } else {
        conn.execute(
            "UPDATE users SET is_premium = 0 WHERE id = ?1",
            crate::params![user_id],
        )?;
    }
    Ok(())
}
pub fn get_writing_content(conn: &Connection, session_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT content FROM writing_sessions WHERE id = ?1")?;
    let mut rows = stmt.query_map(crate::params![session_id], |row| row.get::<_, String>(0))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Nullify the raw writing content after the ritual lifecycle is complete.
/// The writing has been consumed: training pair logged, story generated.
/// Sets content to NULL and records when deletion happened.
pub fn nullify_writing_content(conn: &Connection, session_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE writing_sessions SET content = NULL, content_deleted_at = datetime('now') WHERE id = ?1",
        crate::params![session_id],
    )?;
    Ok(())
}

/// Mark a training pair as exported (ready for LoRA fine-tuning).
pub fn mark_training_pair_exported(conn: &Connection, writing_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE story_training_pairs SET exported_at = datetime('now') WHERE writing_id = ?1",
        crate::params![writing_id],
    )?;
    Ok(())
}

// ===== Cuentacuentos =====

pub struct CuentacuentosRecord {
    pub id: String,
    pub writing_id: String,
    pub parent_wallet_address: String,
    pub child_wallet_address: Option<String>,
    pub title: String,
    pub content: String,
    pub guidance_phases: String,
    pub played: bool,
    pub generated_at: String,
    pub chakra: Option<i32>,
    pub kingdom: Option<String>,
    pub city: Option<String>,
    pub content_es: Option<String>,
    pub content_zh: Option<String>,
    pub content_hi: Option<String>,
    pub content_ar: Option<String>,
}

const CUENTACUENTOS_COLS: &str = "\
    id,
    writing_id,
    parent_wallet_address,
    child_wallet_address,
    title,
    content,
    guidance_phases,
    played,
    generated_at,
    chakra,
    kingdom,
    city,
    content_es,
    content_zh,
    content_hi,
    content_ar";

fn row_to_cuentacuentos(row: &crate::db::Row) -> anyhow::Result<CuentacuentosRecord> {
    Ok(CuentacuentosRecord {
        id: row.get(0)?,
        writing_id: row.get(1)?,
        parent_wallet_address: row.get(2)?,
        child_wallet_address: row.get(3)?,
        title: row.get(4)?,
        content: row.get(5)?,
        guidance_phases: row.get(6)?,
        played: row.get(7)?,
        generated_at: row.get(8)?,
        chakra: row.get(9)?,
        kingdom: row.get(10)?,
        city: row.get(11)?,
        content_es: row.get(12)?,
        content_zh: row.get(13)?,
        content_hi: row.get(14)?,
        content_ar: row.get(15)?,
    })
}

pub struct CreateCuentacuentosParams<'a> {
    pub id: &'a str,
    pub writing_id: &'a str,
    pub parent_wallet_address: &'a str,
    pub child_wallet_address: Option<&'a str>,
    pub title: &'a str,
    pub content: &'a str,
    pub guidance_phases: &'a str,
    pub chakra: Option<i32>,
    pub kingdom: Option<&'a str>,
    pub city: Option<&'a str>,
}

pub fn create_cuentacuentos(conn: &Connection, p: &CreateCuentacuentosParams) -> Result<()> {
    let parent_wallet_address = normalize_wallet_address(p.parent_wallet_address);
    let child_wallet_address = p.child_wallet_address.map(normalize_wallet_address);
    conn.execute(
        "INSERT INTO cuentacuentos (
            id,
            writing_id,
            parent_wallet_address,
            child_wallet_address,
            title,
            content,
            guidance_phases,
            chakra,
            kingdom,
            city
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        crate::params![
            p.id,
            p.writing_id,
            parent_wallet_address,
            child_wallet_address,
            p.title,
            p.content,
            p.guidance_phases,
            p.chakra,
            p.kingdom,
            p.city,
        ],
    )?;
    Ok(())
}

/// Update translated content columns after async Ollama translation.
pub fn update_cuentacuentos_translations(
    conn: &Connection,
    id: &str,
    content_es: Option<&str>,
    content_zh: Option<&str>,
    content_hi: Option<&str>,
    content_ar: Option<&str>,
    translated_guidance_phases: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE cuentacuentos
         SET content_es = ?2,
             content_zh = ?3,
             content_hi = ?4,
             content_ar = ?5,
             guidance_phases = COALESCE(?6, guidance_phases)
         WHERE id = ?1",
        crate::params![
            id,
            content_es,
            content_zh,
            content_hi,
            content_ar,
            translated_guidance_phases
        ],
    )?;
    Ok(())
}

pub fn get_ready_cuentacuentos(
    conn: &Connection,
    parent_wallet_address: &str,
    child_wallet_address: Option<&str>,
) -> Result<Option<CuentacuentosRecord>> {
    let parent_wallet_address = normalize_wallet_address(parent_wallet_address);
    if let Some(child_wallet_address) = child_wallet_address {
        let child_wallet_address = normalize_wallet_address(child_wallet_address);
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM cuentacuentos
             WHERE parent_wallet_address = ?1
               AND child_wallet_address = ?2
               AND played = 0
             ORDER BY generated_at ASC
             LIMIT 1",
            CUENTACUENTOS_COLS
        ))?;
        let mut rows = stmt.query_map(
            crate::params![parent_wallet_address, child_wallet_address],
            row_to_cuentacuentos,
        )?;
        Ok(rows.next().and_then(|r| r.ok()))
    } else {
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM cuentacuentos
             WHERE parent_wallet_address = ?1
               AND child_wallet_address IS NULL
               AND played = 0
             ORDER BY generated_at ASC
             LIMIT 1",
            CUENTACUENTOS_COLS
        ))?;
        let mut rows =
            stmt.query_map(crate::params![parent_wallet_address], row_to_cuentacuentos)?;
        Ok(rows.next().and_then(|r| r.ok()))
    }
}

pub fn get_cuentacuentos_history(
    conn: &Connection,
    parent_wallet_address: &str,
    child_wallet_address: Option<&str>,
) -> Result<Vec<CuentacuentosRecord>> {
    let parent_wallet_address = normalize_wallet_address(parent_wallet_address);
    if let Some(child_wallet_address) = child_wallet_address {
        let child_wallet_address = normalize_wallet_address(child_wallet_address);
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM cuentacuentos
             WHERE parent_wallet_address = ?1
               AND child_wallet_address = ?2
             ORDER BY generated_at DESC",
            CUENTACUENTOS_COLS
        ))?;
        let rows = stmt.query_map(
            crate::params![parent_wallet_address, child_wallet_address],
            row_to_cuentacuentos,
        )?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    } else {
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM cuentacuentos
             WHERE parent_wallet_address = ?1
               AND child_wallet_address IS NULL
             ORDER BY generated_at DESC",
            CUENTACUENTOS_COLS
        ))?;
        let rows = stmt.query_map(crate::params![parent_wallet_address], row_to_cuentacuentos)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

pub fn get_cuentacuentos_by_id_and_parent_wallet(
    conn: &Connection,
    id: &str,
    parent_wallet_address: &str,
) -> Result<Option<CuentacuentosRecord>> {
    let parent_wallet_address = normalize_wallet_address(parent_wallet_address);
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM cuentacuentos
         WHERE id = ?1
           AND parent_wallet_address = ?2
         LIMIT 1",
        CUENTACUENTOS_COLS
    ))?;
    let mut rows = stmt.query_map(
        crate::params![id, parent_wallet_address],
        row_to_cuentacuentos,
    )?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn get_cuentacuentos_by_writing_id(
    conn: &Connection,
    writing_id: &str,
) -> Result<Option<CuentacuentosRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM cuentacuentos
         WHERE writing_id = ?1
         ORDER BY generated_at DESC LIMIT 1",
        CUENTACUENTOS_COLS
    ))?;
    let mut rows = stmt.query_map(crate::params![writing_id], row_to_cuentacuentos)?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Get all stories, newest first. Public — no wallet filter.
pub fn get_all_cuentacuentos(conn: &Connection, limit: i32) -> Result<Vec<CuentacuentosRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM cuentacuentos
         ORDER BY generated_at DESC
         LIMIT ?1",
        CUENTACUENTOS_COLS
    ))?;
    let rows = stmt.query_map(crate::params![limit], row_to_cuentacuentos)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Get a single story by ID (public, no wallet check).
pub fn get_cuentacuentos_by_id(conn: &Connection, id: &str) -> Result<Option<CuentacuentosRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM cuentacuentos WHERE id = ?1 LIMIT 1",
        CUENTACUENTOS_COLS
    ))?;
    let mut rows = stmt.query_map(crate::params![id], row_to_cuentacuentos)?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn mark_cuentacuentos_played(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE cuentacuentos SET played = 1 WHERE id = ?1",
        crate::params![id],
    )?;
    Ok(())
}

pub fn assign_cuentacuentos_to_child(
    conn: &Connection,
    id: &str,
    parent_wallet_address: &str,
    child_wallet_address: &str,
) -> Result<bool> {
    let parent_wallet_address = normalize_wallet_address(parent_wallet_address);
    let child_wallet_address = normalize_wallet_address(child_wallet_address);
    let updated = conn.execute(
        "UPDATE cuentacuentos
         SET child_wallet_address = ?3
         WHERE id = ?1
           AND parent_wallet_address = ?2",
        crate::params![id, parent_wallet_address, child_wallet_address],
    )?;
    Ok(updated > 0)
}

pub struct CuentacuentosImageRecord {
    pub id: String,
    pub cuentacuentos_id: String,
    pub phase_index: i32,
    pub image_prompt: String,
    pub image_url: Option<String>,
    pub status: String,
    pub attempts: i32,
    pub generated_at: Option<String>,
    pub created_at: String,
}

const CUENTACUENTOS_IMAGE_COLS: &str = "\
    id,
    cuentacuentos_id,
    phase_index,
    image_prompt,
    image_url,
    status,
    attempts,
    generated_at,
    created_at";

fn row_to_cuentacuentos_image(row: &crate::db::Row) -> anyhow::Result<CuentacuentosImageRecord> {
    Ok(CuentacuentosImageRecord {
        id: row.get(0)?,
        cuentacuentos_id: row.get(1)?,
        phase_index: row.get(2)?,
        image_prompt: row.get(3)?,
        image_url: row.get(4)?,
        status: row.get(5)?,
        attempts: row.get(6)?,
        generated_at: row.get(7)?,
        created_at: row.get(8)?,
    })
}

pub fn create_cuentacuentos_image(
    conn: &Connection,
    id: &str,
    cuentacuentos_id: &str,
    phase_index: i32,
    image_prompt: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO cuentacuentos_images (
            id,
            cuentacuentos_id,
            phase_index,
            image_prompt
         ) VALUES (?1, ?2, ?3, ?4)",
        crate::params![id, cuentacuentos_id, phase_index, image_prompt],
    )?;
    Ok(())
}

pub fn get_pending_cuentacuentos_images(
    conn: &Connection,
    cuentacuentos_id: &str,
) -> Result<Vec<CuentacuentosImageRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM cuentacuentos_images
         WHERE cuentacuentos_id = ?1
           AND status = 'pending'
         ORDER BY phase_index ASC",
        CUENTACUENTOS_IMAGE_COLS
    ))?;
    let rows = stmt.query_map(crate::params![cuentacuentos_id], row_to_cuentacuentos_image)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_cuentacuentos_images(
    conn: &Connection,
    cuentacuentos_id: &str,
) -> Result<Vec<CuentacuentosImageRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM cuentacuentos_images
         WHERE cuentacuentos_id = ?1
         ORDER BY phase_index ASC",
        CUENTACUENTOS_IMAGE_COLS
    ))?;
    let rows = stmt.query_map(crate::params![cuentacuentos_id], row_to_cuentacuentos_image)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_retryable_failed_cuentacuentos_ids(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT cuentacuentos_id
         FROM cuentacuentos_images
         WHERE status = 'failed'
           AND attempts < 3
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(crate::params![], |row| row.get::<_, String>(0))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn requeue_retryable_cuentacuentos_images(
    conn: &Connection,
    cuentacuentos_id: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE cuentacuentos_images
         SET status = 'pending'
         WHERE cuentacuentos_id = ?1
           AND status = 'failed'
           AND attempts < 3",
        crate::params![cuentacuentos_id],
    )?;
    Ok(())
}

pub fn mark_cuentacuentos_image_generating(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE cuentacuentos_images
         SET status = 'generating',
             attempts = attempts + 1
         WHERE id = ?1",
        crate::params![id],
    )?;
    Ok(())
}

pub fn set_cuentacuentos_image_generated(
    conn: &Connection,
    id: &str,
    image_url: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE cuentacuentos_images
         SET image_url = ?2,
             status = 'generated',
             generated_at = datetime('now')
         WHERE id = ?1",
        crate::params![id, image_url],
    )?;
    Ok(())
}

pub fn set_cuentacuentos_image_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE cuentacuentos_images
         SET status = ?2
         WHERE id = ?1",
        crate::params![id, status],
    )?;
    Ok(())
}

// --- Next Prompt ---

pub fn upsert_next_prompt(
    conn: &Connection,
    user_id: &str,
    prompt_text: &str,
    from_session: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO next_prompts (user_id, prompt_text, generated_from_session, created_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(user_id) DO UPDATE SET
           prompt_text = excluded.prompt_text,
           generated_from_session = excluded.generated_from_session,
           created_at = excluded.created_at",
        crate::params![user_id, prompt_text, from_session],
    )?;
    Ok(())
}

pub fn get_next_prompt(
    conn: &Connection,
    user_id: &str,
) -> Result<Option<(String, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT prompt_text, generated_from_session FROM next_prompts WHERE user_id = ?1",
    )?;
    let result = stmt
        .query_map(crate::params![user_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?
        .next()
        .and_then(|r| r.ok());
    Ok(result)
}

// --- Device Tokens ---

pub fn upsert_device_token(
    conn: &Connection,
    user_id: &str,
    device_token: &str,
    platform: &str,
) -> Result<()> {
    let id = format!("dt-{}", uuid::Uuid::new_v4());
    conn.execute(
        "INSERT INTO device_tokens (id, user_id, device_token, platform, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
         ON CONFLICT(user_id, platform) DO UPDATE SET
           device_token = excluded.device_token,
           updated_at = datetime('now')",
        crate::params![id, user_id, device_token, platform],
    )?;
    Ok(())
}

pub fn delete_device_token(conn: &Connection, user_id: &str, platform: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM device_tokens WHERE user_id = ?1 AND platform = ?2",
        crate::params![user_id, platform],
    )?;
    Ok(())
}

pub fn get_device_tokens_for_user(conn: &Connection, user_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT device_token FROM device_tokens WHERE user_id = ?1")?;
    let tokens = stmt
        .query_map(crate::params![user_id], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tokens)
}

/// Returns (user_id, device_token, psychological_profile) for all users with tokens + writings.
pub fn get_notification_targets(
    conn: &Connection,
) -> Result<Vec<(String, String, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT dt.user_id, dt.device_token, up.psychological_profile
         FROM device_tokens dt
         JOIN user_profiles up ON up.user_id = dt.user_id
         WHERE up.total_sessions > 0",
    )?;
    let rows = stmt
        .query_map(crate::params![], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

// --- User Profile (You tab) ---

pub struct UserProfileResponse {
    pub total_sessions: i32,
    pub total_anky_sessions: i32,
    pub total_words_written: i32,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub best_flow_score: f64,
    pub avg_flow_score: f64,
    pub psychological_profile: Option<String>,
    pub emotional_signature: Option<String>,
    pub core_tensions: Option<String>,
    pub growth_edges: Option<String>,
    pub last_profile_update: Option<String>,
}

pub fn get_user_profile_full(
    conn: &Connection,
    user_id: &str,
) -> Result<Option<UserProfileResponse>> {
    let mut stmt = conn.prepare(
        "SELECT total_sessions, total_anky_sessions, total_words_written,
                COALESCE(current_streak, 0), COALESCE(longest_streak, 0),
                COALESCE(best_flow_score, 0), COALESCE(avg_flow_score, 0),
                psychological_profile, emotional_signature, core_tensions,
                growth_edges, last_profile_update
         FROM user_profiles WHERE user_id = ?1",
    )?;
    let result = stmt
        .query_map(crate::params![user_id], |row| {
            Ok(UserProfileResponse {
                total_sessions: row.get(0)?,
                total_anky_sessions: row.get(1)?,
                total_words_written: row.get(2)?,
                current_streak: row.get(3)?,
                longest_streak: row.get(4)?,
                best_flow_score: row.get(5)?,
                avg_flow_score: row.get(6)?,
                psychological_profile: row.get(7)?,
                emotional_signature: row.get(8)?,
                core_tensions: row.get(9)?,
                growth_edges: row.get(10)?,
                last_profile_update: row.get(11)?,
            })
        })?
        .next()
        .and_then(|r| r.ok());
    Ok(result)
}

// --- Cuentacuentos TTS Audio ---

pub struct CuentacuentosAudioRecord {
    pub id: String,
    pub cuentacuentos_id: String,
    pub language: String,
    pub status: String,
    pub audio_url: Option<String>,
    pub duration_seconds: Option<f64>,
}

pub fn create_cuentacuentos_audio(
    conn: &Connection,
    id: &str,
    cuentacuentos_id: &str,
    language: &str,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO cuentacuentos_audio (id, cuentacuentos_id, language)
         VALUES (?1, ?2, ?3)",
        crate::params![id, cuentacuentos_id, language],
    )?;
    Ok(())
}

pub fn update_cuentacuentos_audio_generating(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE cuentacuentos_audio SET status = 'generating', attempts = attempts + 1 WHERE id = ?1",
        crate::params![id],
    )?;
    Ok(())
}

pub fn update_cuentacuentos_audio_complete(
    conn: &Connection,
    id: &str,
    r2_key: &str,
    audio_url: &str,
    duration_seconds: f64,
) -> Result<()> {
    conn.execute(
        "UPDATE cuentacuentos_audio
         SET status = 'complete', r2_key = ?2, audio_url = ?3,
             duration_seconds = ?4, generated_at = datetime('now')
         WHERE id = ?1",
        crate::params![id, r2_key, audio_url, duration_seconds],
    )?;
    Ok(())
}

pub fn update_cuentacuentos_audio_failed(conn: &Connection, id: &str, error: &str) -> Result<()> {
    conn.execute(
        "UPDATE cuentacuentos_audio SET status = 'failed', error_message = ?2 WHERE id = ?1",
        crate::params![id, error],
    )?;
    Ok(())
}

pub fn get_cuentacuentos_audio(
    conn: &Connection,
    cuentacuentos_id: &str,
) -> Result<Vec<CuentacuentosAudioRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, cuentacuentos_id, language, status, audio_url, duration_seconds
         FROM cuentacuentos_audio
         WHERE cuentacuentos_id = ?1 AND status = 'complete'
         ORDER BY language ASC",
    )?;
    let rows = stmt.query_map(crate::params![cuentacuentos_id], |row| {
        Ok(CuentacuentosAudioRecord {
            id: row.get(0)?,
            cuentacuentos_id: row.get(1)?,
            language: row.get(2)?,
            status: row.get(3)?,
            audio_url: row.get(4)?,
            duration_seconds: row.get(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ===== Minting =====

/// Check if an anky is eligible for minting: complete, is_anky, not minted, not gas-funded, belongs to user.
/// Returns (anky_id, writing_content, user_wallet_address) if eligible.
pub fn check_mint_eligibility(
    conn: &Connection,
    session_id: &str,
    user_id: &str,
) -> Result<Option<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, ws.content, u.wallet_address
         FROM ankys a
         JOIN writing_sessions ws ON ws.id = a.writing_session_id
         JOIN users u ON u.id = a.user_id
         WHERE a.writing_session_id = ?1
           AND a.user_id = ?2
           AND a.status = 'complete'
           AND ws.is_anky = 1
           AND a.is_minted = 0
           AND a.gas_funded_at IS NULL
         ORDER BY a.rowid DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![session_id, user_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Check if a wallet address has been gas-funded in the last hour.
pub fn check_mint_rate_limit(conn: &Connection, wallet_address: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ankys a
         JOIN users u ON u.id = a.user_id
         WHERE u.wallet_address = ?1
           AND a.gas_funded_at IS NOT NULL
           AND a.gas_funded_at > datetime('now', '-1 hour')",
        crate::params![wallet_address],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Mark an anky as gas-funded with session CID and metadata URI.
pub fn set_anky_gas_funded(
    conn: &Connection,
    anky_id: &str,
    session_cid: &str,
    metadata_uri: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET gas_funded_at = datetime('now'), session_cid = ?2, metadata_uri = ?3
         WHERE id = ?1",
        crate::params![anky_id, session_cid, metadata_uri],
    )?;
    Ok(())
}

/// Mark an anky as minted with tx hash and optional token ID.
pub fn set_anky_minted(
    conn: &Connection,
    anky_id: &str,
    tx_hash: &str,
    token_id: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET is_minted = 1, mint_tx_hash = ?2, token_id = ?3 WHERE id = ?1",
        crate::params![anky_id, tx_hash, token_id],
    )?;
    Ok(())
}

/// Get anky for mint confirmation: gas-funded but not yet minted.
pub fn get_anky_for_mint_confirm(
    conn: &Connection,
    session_id: &str,
    user_id: &str,
) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT a.id FROM ankys a
         WHERE a.writing_session_id = ?1
           AND a.user_id = ?2
           AND a.gas_funded_at IS NOT NULL
           AND a.is_minted = 0
         ORDER BY a.rowid DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![session_id, user_id], |row| {
        row.get::<_, String>(0)
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn get_pending_cuentacuentos_audio(conn: &Connection) -> Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, cuentacuentos_id, language
         FROM cuentacuentos_audio
         WHERE status IN ('pending', 'failed') AND attempts < 3
         ORDER BY created_at ASC
         LIMIT 20",
    )?;
    let rows = stmt.query_map(crate::params![], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// --- Mirrors ---

/// Get the most recent mirror for a given FID (cache lookup).
pub fn get_mirror_by_fid(
    conn: &Connection,
    fid: u64,
) -> Result<
    Option<(
        String,
        i64,
        String,
        String,
        Option<String>,
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
    )>,
> {
    let mut stmt = conn.prepare(
        "SELECT id, fid, username, display_name, avatar_url, follower_count, bio, public_mirror, gap, flux_descriptors_json, image_path, created_at
         FROM mirrors WHERE fid = ?1 ORDER BY created_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![fid as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, String>(11)?,
        ))
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn insert_mirror(
    conn: &Connection,
    id: &str,
    fid: u64,
    username: &str,
    display_name: &str,
    avatar_url: Option<&str>,
    follower_count: u64,
    bio: &str,
    public_mirror: &str,
    gap: &str,
    flux_descriptors_json: &str,
    image_path: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO mirrors (id, fid, username, display_name, avatar_url, follower_count, bio, public_mirror, gap, flux_descriptors_json, image_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        crate::params![
            id,
            fid as i64,
            username,
            display_name,
            avatar_url,
            follower_count as i64,
            bio,
            public_mirror,
            gap,
            flux_descriptors_json,
            image_path,
        ],
    )?;
    Ok(())
}

pub fn list_mirrors(
    conn: &Connection,
    limit: u32,
    offset: u32,
) -> Result<
    Vec<(
        String,
        i64,
        String,
        String,
        Option<String>,
        i64,
        String,
        String,
        String,
        Option<String>,
        String,
    )>,
> {
    let mut stmt = conn.prepare(
        "SELECT id, fid, username, display_name, avatar_url, follower_count, public_mirror, gap, flux_descriptors_json, image_path, created_at
         FROM mirrors ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(crate::params![limit, offset], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, String>(10)?,
        ))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// --- Solana Mirror Minting (Sojourn 9) ---

/// Check if a Farcaster FID has already minted a mirror cNFT.
pub fn has_fid_minted(conn: &Connection, fid: u64) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM mirrors WHERE fid = ?1 AND solana_mint_tx IS NOT NULL",
        crate::params![fid as i64],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Check if a user_id (iOS path) has already minted a mirror cNFT.
pub fn has_user_minted(conn: &Connection, user_id: &str) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM mirrors WHERE user_id = ?1 AND solana_mint_tx IS NOT NULL",
        crate::params![user_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Check if a Solana address has already received a mirror cNFT.
pub fn has_solana_address_minted(conn: &Connection, address: &str) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM mirrors WHERE solana_recipient = ?1 AND solana_mint_tx IS NOT NULL",
        crate::params![address],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Get existing mint info for a user (for "already minted" responses).
pub fn get_user_existing_mint(
    conn: &Connection,
    user_id: &str,
) -> Result<Option<(String, String, Option<i32>, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT solana_mint_tx, id, kingdom, kingdom_name FROM mirrors WHERE user_id = ?1 AND solana_mint_tx IS NOT NULL LIMIT 1",
    )?;
    match stmt.query_row(crate::params![user_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i32>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    }) {
        Ok(row) => Ok(Some(row)),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("no rows") || msg.contains("RowNotFound") {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

/// Count total minted mirrors across both surfaces.
pub fn count_minted_mirrors(conn: &Connection) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM mirrors WHERE solana_mint_tx IS NOT NULL",
        crate::params![],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Mark a mirror as minted with Solana transaction details.
pub fn set_mirror_minted(
    conn: &Connection,
    mirror_id: &str,
    tx: &str,
    recipient: &str,
    asset_id: Option<&str>,
    kingdom: i32,
    kingdom_name: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE mirrors SET solana_mint_tx = ?1, solana_recipient = ?2, solana_asset_id = ?3, solana_minted_at = NOW(), kingdom = ?4, kingdom_name = ?5 WHERE id = ?6",
        crate::params![tx, recipient, asset_id, kingdom, kingdom_name, mirror_id],
    )?;
    Ok(())
}

/// Insert a raw mirror (iOS path — no Farcaster data).
pub fn insert_raw_mirror(
    conn: &Connection,
    id: &str,
    user_id: &str,
    solana_address: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO mirrors (id, fid, username, display_name, bio, public_mirror, flux_descriptors_json, gap, mirror_type, user_id, solana_recipient)
         VALUES (?1, 0, '', '', '', 'this mirror has not yet reflected', '{}', '', 'raw', ?2, ?3)",
        crate::params![id, user_id, solana_address],
    )?;
    Ok(())
}

/// Insert a raw mirror linked to a specific writing session (per-anky minting).
pub fn insert_raw_mirror_for_session(
    conn: &Connection,
    id: &str,
    user_id: &str,
    solana_address: &str,
    writing_session_id: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO mirrors (id, fid, username, display_name, bio, public_mirror, flux_descriptors_json, gap, mirror_type, user_id, solana_recipient, writing_session_id)
         VALUES (?1, 0, '', '', '', 'this mirror has not yet reflected', '{}', '', 'raw', ?2, ?3, ?4)",
        crate::params![id, user_id, solana_address, writing_session_id],
    )?;
    Ok(())
}

/// Store items JSON on a mirror.
pub fn set_mirror_items(conn: &Connection, mirror_id: &str, items_json: &str) -> Result<()> {
    conn.execute(
        "UPDATE mirrors SET items_json = ?1 WHERE id = ?2",
        crate::params![items_json, mirror_id],
    )?;
    Ok(())
}

/// Get items JSON for a mirror.
pub fn get_mirror_items(conn: &Connection, mirror_id: &str) -> Result<Option<String>> {
    let result: Option<String> = conn
        .query_row(
            "SELECT items_json FROM mirrors WHERE id = ?1",
            crate::params![mirror_id],
            |row| row.get(0),
        )
        .ok();
    Ok(result)
}

/// Get a user's mirror items (for the living items endpoint).
pub fn get_user_mirror_items(conn: &Connection, user_id: &str) -> Result<Option<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, items_json FROM mirrors WHERE user_id = ?1 AND items_json IS NOT NULL ORDER BY created_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(crate::params![user_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Get full mirror data including Solana fields (for metadata endpoint).
pub fn get_mirror_full(
    conn: &Connection,
    id: &str,
) -> Result<
    Option<(
        String,         // id
        i64,            // fid
        String,         // username
        String,         // display_name
        Option<String>, // avatar_url
        i64,            // follower_count
        String,         // bio
        String,         // public_mirror
        String,         // gap
        String,         // flux_descriptors_json
        Option<String>, // image_path
        String,         // created_at
        Option<String>, // solana_mint_tx
        Option<String>, // solana_recipient
        Option<i32>,    // kingdom
        Option<String>, // kingdom_name
        String,         // mirror_type
        Option<String>, // user_id
    )>,
> {
    let mut stmt = conn.prepare(
        "SELECT id, fid, username, display_name, avatar_url, follower_count, bio, public_mirror, gap, flux_descriptors_json, image_path, created_at,
                solana_mint_tx, solana_recipient, kingdom, kingdom_name, mirror_type, user_id
         FROM mirrors WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, Option<String>>(12)?,
            row.get::<_, Option<String>>(13)?,
            row.get::<_, Option<i32>>(14)?,
            row.get::<_, Option<String>>(15)?,
            row.get::<_, String>(16)?,
            row.get::<_, Option<String>>(17)?,
        ))
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

// --- AnkyStory persistence ---

pub fn save_anky_story(conn: &Connection, anky_id: &str, story: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET anky_story = ?1 WHERE id = ?2",
        crate::params![story, anky_id],
    )?;
    Ok(())
}

pub fn get_anky_story(conn: &Connection, anky_id: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT anky_story FROM ankys WHERE id = ?1")?;
    let mut rows = stmt.query_map(crate::params![anky_id], |row| {
        row.get::<_, Option<String>>(0)
    })?;
    Ok(rows.next().and_then(|r| r.ok()).flatten())
}

// --- Programming classes ---

pub fn insert_programming_class(
    conn: &Connection,
    class_number: i64,
    title: &str,
    description: &str,
    concept: &str,
    slides_json: &str,
    changelog_slug: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO programming_classes (id, class_number, title, description, concept, slides_json, changelog_slug)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(class_number) DO UPDATE SET
            title = excluded.title,
            description = excluded.description,
            concept = excluded.concept,
            slides_json = excluded.slides_json,
            changelog_slug = excluded.changelog_slug",
        crate::params![
            class_number,
            class_number,
            title,
            description,
            concept,
            slides_json,
            changelog_slug,
        ],
    )?;
    Ok(())
}

pub fn next_class_number(conn: &Connection) -> Result<i64> {
    let n: i64 = conn.query_row(
        "SELECT COALESCE(MAX(class_number), 0) + 1 FROM programming_classes",
        crate::params![],
        |row| row.get(0),
    )?;
    Ok(n)
}

// ── Anky Now ────────────────────────────────────────────────────────────

pub struct NowRecord {
    pub id: String,
    pub slug: String,
    pub prompt: String,
    pub prompt_image_path: Option<String>,
    pub prompt_image_status: String,
    pub creator_id: Option<String>,
    pub mode: String,
    pub duration_seconds: i32,
    pub starts_at: Option<String>,
    pub started: bool,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub created_at: String,
}

pub fn insert_now(
    conn: &Connection,
    id: &str,
    slug: &str,
    prompt: &str,
    creator_id: Option<&str>,
    mode: &str,
    duration_seconds: i32,
    starts_at: Option<&str>,
    latitude: Option<f64>,
    longitude: Option<f64>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO nows (id, slug, prompt, creator_id, mode, duration_seconds, starts_at, latitude, longitude)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![id, slug, prompt, creator_id, mode, duration_seconds, starts_at, latitude, longitude],
    )?;
    Ok(())
}

pub fn get_now_by_slug(conn: &Connection, slug: &str) -> Result<Option<NowRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, slug, prompt, prompt_image_path, prompt_image_status, creator_id,
                mode, duration_seconds, starts_at, started, latitude, longitude, created_at
         FROM nows WHERE slug = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![slug], |row| {
        Ok(NowRecord {
            id: row.get(0)?,
            slug: row.get(1)?,
            prompt: row.get(2)?,
            prompt_image_path: row.get(3)?,
            prompt_image_status: row.get(4)?,
            creator_id: row.get(5)?,
            mode: row.get(6)?,
            duration_seconds: row.get(7)?,
            starts_at: row.get(8)?,
            started: row.get(9)?,
            latitude: row.get(10)?,
            longitude: row.get(11)?,
            created_at: row.get(12)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn get_now_by_id(conn: &Connection, id: &str) -> Result<Option<NowRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, slug, prompt, prompt_image_path, prompt_image_status, creator_id,
                mode, duration_seconds, starts_at, started, latitude, longitude, created_at
         FROM nows WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(crate::params![id], |row| {
        Ok(NowRecord {
            id: row.get(0)?,
            slug: row.get(1)?,
            prompt: row.get(2)?,
            prompt_image_path: row.get(3)?,
            prompt_image_status: row.get(4)?,
            creator_id: row.get(5)?,
            mode: row.get(6)?,
            duration_seconds: row.get(7)?,
            starts_at: row.get(8)?,
            started: row.get(9)?,
            latitude: row.get(10)?,
            longitude: row.get(11)?,
            created_at: row.get(12)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn update_now_image(
    conn: &Connection,
    now_id: &str,
    status: &str,
    image_path: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE nows SET prompt_image_status = ?2, prompt_image_path = COALESCE(?3, prompt_image_path)
         WHERE id = ?1",
        crate::params![now_id, status, image_path],
    )?;
    Ok(())
}

pub fn mark_now_started(conn: &Connection, now_id: &str, starts_at: &str) -> Result<()> {
    conn.execute(
        "UPDATE nows SET started = 1, starts_at = ?2 WHERE id = ?1",
        crate::params![now_id, starts_at],
    )?;
    Ok(())
}

pub fn insert_now_session(
    conn: &Connection,
    now_id: &str,
    writing_session_id: &str,
) -> Result<i32> {
    let seq: i32 = conn.query_row(
        "INSERT INTO now_sessions (now_id, writing_session_id, sequence)
         VALUES (?1, ?2, COALESCE((SELECT MAX(sequence) FROM now_sessions WHERE now_id = ?1), 0) + 1)
         RETURNING sequence",
        crate::params![now_id, writing_session_id],
        |row| row.get(0),
    )?;
    Ok(seq)
}

pub fn get_now_sessions(conn: &Connection, now_id: &str) -> Result<Vec<(String, i32, String)>> {
    let mut stmt = conn.prepare(
        "SELECT writing_session_id, sequence, created_at
         FROM now_sessions WHERE now_id = ?1 ORDER BY sequence ASC",
    )?;
    let rows = stmt.query_map(crate::params![now_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i32>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn upsert_now_presence(
    conn: &Connection,
    now_id: &str,
    user_id: &str,
    display_name: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO now_presence (now_id, user_id, display_name)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(now_id, user_id) DO UPDATE SET
            display_name = excluded.display_name,
            last_seen_at = anky_now()",
        crate::params![now_id, user_id, display_name],
    )?;
    Ok(())
}

pub fn heartbeat_now_presence(conn: &Connection, now_id: &str, user_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE now_presence SET last_seen_at = anky_now() WHERE now_id = ?1 AND user_id = ?2",
        crate::params![now_id, user_id],
    )?;
    Ok(())
}

pub fn get_now_active_presence(
    conn: &Connection,
    now_id: &str,
    cutoff: &str,
) -> Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, display_name, joined_at
         FROM now_presence WHERE now_id = ?1 AND last_seen_at >= ?2
         ORDER BY joined_at ASC",
    )?;
    let rows = stmt.query_map(crate::params![now_id, cutoff], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
