use anyhow::Result;
use rusqlite::{params, Connection};

// --- Users ---
pub fn ensure_user(conn: &Connection, user_id: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO users (id) VALUES (?1)",
        params![user_id],
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
        params![id, user_id, content, duration, word_count, is_anky, response],
    )?;
    Ok(())
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
        "SELECT id, content, duration_seconds, word_count, is_anky, response, created_at FROM writing_sessions WHERE user_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
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

pub fn get_writing_session(conn: &Connection, id: &str) -> Result<Option<WritingSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, duration_seconds, word_count, is_anky, response, created_at FROM writing_sessions WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
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
) -> Result<()> {
    conn.execute(
        "INSERT INTO ankys (id, writing_session_id, user_id, image_prompt, reflection, title, image_path, caption, thinker_name, thinker_moment, status, origin) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![id, writing_session_id, user_id, image_prompt, reflection, title, image_path, caption, thinker_name, thinker_moment, status, origin],
    )?;
    Ok(())
}

pub fn update_anky_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET status = ?2 WHERE id = ?1",
        params![id, status],
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
        params![id, image_prompt, reflection, title, image_path, caption],
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
        params![id, image_prompt, image_path],
    )?;
    Ok(())
}

pub struct AnkyRecord {
    pub id: String,
    pub title: Option<String>,
    pub image_path: Option<String>,
    pub reflection: Option<String>,
    pub image_prompt: Option<String>,
    pub thinker_name: Option<String>,
    pub status: String,
    pub created_at: String,
    pub origin: String,
}

pub fn get_all_ankys(conn: &Connection) -> Result<Vec<AnkyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, image_path, reflection, image_prompt, thinker_name, status, created_at, origin FROM ankys ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(AnkyRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            reflection: row.get(3)?,
            image_prompt: row.get(4)?,
            thinker_name: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
            origin: row.get(8)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub struct AnkyDetail {
    pub id: String,
    pub title: Option<String>,
    pub image_path: Option<String>,
    pub reflection: Option<String>,
    pub image_prompt: Option<String>,
    pub caption: Option<String>,
    pub thinker_name: Option<String>,
    pub thinker_moment: Option<String>,
    pub status: String,
    pub writing_text: Option<String>,
    pub created_at: String,
    pub origin: String,
}

pub fn get_anky_by_id(conn: &Connection, id: &str) -> Result<Option<AnkyDetail>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.title, a.image_path, a.reflection, a.image_prompt, a.caption, a.thinker_name, a.thinker_moment, a.status, w.content, a.created_at, a.origin
         FROM ankys a
         LEFT JOIN writing_sessions w ON w.id = a.writing_session_id
         WHERE a.id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(AnkyDetail {
            id: row.get(0)?,
            title: row.get(1)?,
            image_path: row.get(2)?,
            reflection: row.get(3)?,
            image_prompt: row.get(4)?,
            caption: row.get(5)?,
            thinker_name: row.get(6)?,
            thinker_moment: row.get(7)?,
            status: row.get(8)?,
            writing_text: row.get(9)?,
            created_at: row.get(10)?,
            origin: row.get(11)?,
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
        params![id, user_id, mega_prompt, cost_estimate],
    )?;
    Ok(())
}

pub fn update_collection_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE collections SET status = ?2 WHERE id = ?1",
        params![id, status],
    )?;
    Ok(())
}

pub fn update_collection_progress(conn: &Connection, id: &str, progress: i32) -> Result<()> {
    conn.execute(
        "UPDATE collections SET progress = ?2 WHERE id = ?1",
        params![id, progress],
    )?;
    Ok(())
}

pub fn update_collection_payment(conn: &Connection, id: &str, tx_hash: &str) -> Result<()> {
    conn.execute(
        "UPDATE collections SET payment_tx_hash = ?2, status = 'paid' WHERE id = ?1",
        params![id, tx_hash],
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
    let mut rows = stmt.query_map(params![id], |row| {
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
        params![service, model, input_tokens, output_tokens, cost_usd, related_id],
    )?;
    Ok(())
}

pub fn get_total_cost(conn: &Connection) -> Result<f64> {
    let cost: f64 = conn.query_row("SELECT COALESCE(SUM(cost_usd), 0) FROM cost_records", [], |row| row.get(0))?;
    Ok(cost)
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
        params![id, base_model, dataset_size, steps],
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
        params![id, current_step, loss],
    )?;
    Ok(())
}

pub fn complete_training_run(
    conn: &Connection,
    id: &str,
    lora_path: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE training_runs SET status = 'complete', lora_weights_path = ?2, completed_at = datetime('now') WHERE id = ?1",
        params![id, lora_path],
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
        params![email, telegram_chat_id],
    )?;
    Ok(())
}

pub fn get_notification_signups(conn: &Connection) -> Result<Vec<(Option<String>, Option<String>)>> {
    let mut stmt = conn.prepare("SELECT email, telegram_chat_id FROM notification_signups")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
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
        params![key, label],
    )?;
    Ok(())
}

pub fn get_api_key(conn: &Connection, key: &str) -> Result<Option<ApiKeyRecord>> {
    let mut stmt = conn.prepare(
        "SELECT key, label, balance_usd, total_spent_usd, total_transforms, is_active, created_at FROM api_keys WHERE key = ?1",
    )?;
    let mut rows = stmt.query_map(params![key], |row| {
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
        params![key],
    )?;
    Ok(())
}

pub fn add_balance(conn: &Connection, key: &str, amount_usd: f64) -> Result<()> {
    conn.execute(
        "UPDATE api_keys SET balance_usd = balance_usd + ?2 WHERE key = ?1",
        params![key, amount_usd],
    )?;
    Ok(())
}

pub fn deduct_balance(conn: &Connection, key: &str, amount_usd: f64) -> Result<()> {
    conn.execute(
        "UPDATE api_keys SET balance_usd = balance_usd - ?2, total_spent_usd = total_spent_usd + ?2, total_transforms = total_transforms + 1 WHERE key = ?1",
        params![key, amount_usd],
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
        params![id, api_key, input_text, prompt, output_text, input_tokens, output_tokens, cost_usd],
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

pub fn get_recent_transformations(conn: &Connection, api_key: &str, limit: i32) -> Result<Vec<TransformationRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, input_tokens, output_tokens, cost_usd, created_at FROM transformations WHERE api_key = ?1 ORDER BY created_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![api_key, limit], |row| {
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

pub fn insert_credit_purchase(
    conn: &Connection,
    id: &str,
    api_key: &str,
    tx_hash: &str,
    amount_usdc: f64,
    amount_credited: f64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO credit_purchases (id, api_key, tx_hash, amount_usdc, amount_credited_usd, verified) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        params![id, api_key, tx_hash, amount_usdc, amount_credited],
    )?;
    Ok(())
}

pub fn check_tx_hash_used(conn: &Connection, tx_hash: &str) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM credit_purchases WHERE tx_hash = ?1",
        params![tx_hash],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

// --- Agents ---
pub struct AgentRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub api_key: String,
    pub free_sessions_remaining: i32,
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
        params![id, name, description, model, api_key],
    )?;
    Ok(())
}

pub fn get_agent_by_key(conn: &Connection, api_key: &str) -> Result<Option<AgentRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, model, api_key, free_sessions_remaining, total_sessions, created_at FROM agents WHERE api_key = ?1",
    )?;
    let mut rows = stmt.query_map(params![api_key], |row| {
        Ok(AgentRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            model: row.get(3)?,
            api_key: row.get(4)?,
            free_sessions_remaining: row.get(5)?,
            total_sessions: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn decrement_free_session(conn: &Connection, agent_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE agents SET free_sessions_remaining = free_sessions_remaining - 1, total_sessions = total_sessions + 1 WHERE id = ?1 AND free_sessions_remaining > 0",
        params![agent_id],
    )?;
    Ok(())
}

pub fn increment_agent_sessions(conn: &Connection, agent_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE agents SET total_sessions = total_sessions + 1 WHERE id = ?1",
        params![agent_id],
    )?;
    Ok(())
}

// --- Writing Checkpoints ---
pub fn insert_checkpoint(
    conn: &Connection,
    session_id: &str,
    content: &str,
    elapsed: f64,
    word_count: i32,
) -> Result<()> {
    conn.execute(
        "INSERT INTO writing_checkpoints (session_id, content, elapsed_seconds, word_count) VALUES (?1, ?2, ?3, ?4)",
        params![session_id, content, elapsed, word_count],
    )?;
    Ok(())
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
        [],
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
         WHERE a.status IN ('pending', 'failed')
         AND a.created_at < datetime('now', '-2 minutes')
         ORDER BY a.created_at ASC
         LIMIT 10"
    )?;
    let rows = stmt.query_map([], |row| {
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

pub fn mark_anky_failed(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE ankys SET status = 'failed' WHERE id = ?1 AND status IN ('pending', 'generating')",
        params![id],
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
        params![id, source, author, content],
    )?;
    Ok(())
}

pub fn get_all_feedback(conn: &Connection) -> Result<Vec<FeedbackRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, source, author, content, status, created_at FROM feedback ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
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
        params![id, anky_id, api_key, agent_id, payment_method, amount_usd, tx_hash],
    )?;
    Ok(())
}
