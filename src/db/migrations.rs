use anyhow::Result;
use rusqlite::Connection;

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS writing_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            content TEXT NOT NULL,
            duration_seconds REAL NOT NULL,
            word_count INTEGER NOT NULL,
            is_anky BOOLEAN NOT NULL DEFAULT 0,
            response TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS ankys (
            id TEXT PRIMARY KEY,
            writing_session_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            image_prompt TEXT,
            reflection TEXT,
            title TEXT,
            image_path TEXT,
            caption TEXT,
            thinker_name TEXT,
            thinker_moment TEXT,
            is_minted BOOLEAN NOT NULL DEFAULT 0,
            mint_tx_hash TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (writing_session_id) REFERENCES writing_sessions(id),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            mega_prompt TEXT NOT NULL,
            beings_json TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            payment_tx_hash TEXT,
            cost_estimate_usd REAL,
            actual_cost_usd REAL DEFAULT 0,
            progress INTEGER NOT NULL DEFAULT 0,
            total INTEGER NOT NULL DEFAULT 88,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS cost_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            service TEXT NOT NULL,
            model TEXT NOT NULL,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            cost_usd REAL NOT NULL DEFAULT 0,
            related_id TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS training_runs (
            id TEXT PRIMARY KEY,
            base_model TEXT NOT NULL,
            dataset_size INTEGER NOT NULL,
            steps INTEGER NOT NULL,
            current_step INTEGER NOT NULL DEFAULT 0,
            loss REAL,
            status TEXT NOT NULL DEFAULT 'pending',
            lora_weights_path TEXT,
            started_at TEXT,
            completed_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS notification_signups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT,
            telegram_chat_id TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS api_keys (
            key TEXT PRIMARY KEY,
            label TEXT,
            balance_usd REAL NOT NULL DEFAULT 0,
            total_spent_usd REAL NOT NULL DEFAULT 0,
            total_transforms INTEGER NOT NULL DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS transformations (
            id TEXT PRIMARY KEY,
            api_key TEXT NOT NULL,
            input_text TEXT NOT NULL,
            prompt TEXT,
            output_text TEXT NOT NULL,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            cost_usd REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (api_key) REFERENCES api_keys(key)
        );

        CREATE TABLE IF NOT EXISTS credit_purchases (
            id TEXT PRIMARY KEY,
            api_key TEXT NOT NULL,
            tx_hash TEXT NOT NULL,
            amount_usdc REAL NOT NULL,
            amount_credited_usd REAL NOT NULL,
            verified BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (api_key) REFERENCES api_keys(key)
        );

        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            model TEXT,
            api_key TEXT NOT NULL,
            free_sessions_remaining INTEGER NOT NULL DEFAULT 4,
            total_sessions INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (api_key) REFERENCES api_keys(key)
        );

        CREATE TABLE IF NOT EXISTS generation_records (
            id TEXT PRIMARY KEY,
            anky_id TEXT NOT NULL,
            api_key TEXT,
            agent_id TEXT,
            payment_method TEXT NOT NULL,
            amount_usd REAL NOT NULL DEFAULT 0,
            tx_hash TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS writing_checkpoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            content TEXT NOT NULL,
            elapsed_seconds REAL NOT NULL,
            word_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS feedback (
            id TEXT PRIMARY KEY,
            source TEXT NOT NULL,
            author TEXT,
            content TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;
    Ok(())
}
