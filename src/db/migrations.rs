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

    // Add origin column to ankys (safe for existing data — defaults to 'written')
    let has_origin: bool = conn.prepare("SELECT origin FROM ankys LIMIT 0").is_ok();
    if !has_origin {
        conn.execute_batch("ALTER TABLE ankys ADD COLUMN origin TEXT NOT NULL DEFAULT 'written';")?;
    }

    // User collections — tracks who has viewed/collected a written anky via shared link
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS user_collections (
            user_id TEXT NOT NULL,
            anky_id TEXT NOT NULL,
            collected_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (user_id, anky_id)
        );",
    )?;

    // --- Phase 1: Prompts ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS prompts (
            id TEXT PRIMARY KEY,
            creator_user_id TEXT NOT NULL,
            prompt_text TEXT NOT NULL,
            image_path TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            payment_tx_hash TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (creator_user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS prompt_sessions (
            id TEXT PRIMARY KEY,
            prompt_id TEXT NOT NULL,
            user_id TEXT,
            content TEXT,
            keystroke_deltas TEXT,
            page_opened_at TEXT,
            first_keystroke_at TEXT,
            duration_seconds REAL,
            word_count INTEGER NOT NULL DEFAULT 0,
            completed BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (prompt_id) REFERENCES prompts(id)
        );",
    )?;

    // --- Phase 2: X OAuth ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS x_users (
            x_user_id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            username TEXT NOT NULL,
            display_name TEXT,
            profile_image_url TEXT,
            access_token TEXT NOT NULL,
            refresh_token TEXT,
            token_expires_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS auth_sessions (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            x_user_id TEXT,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS oauth_states (
            state TEXT PRIMARY KEY,
            code_verifier TEXT NOT NULL,
            redirect_to TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    // --- Username on users ---
    let has_username: bool = conn.prepare("SELECT username FROM users LIMIT 0").is_ok();
    if !has_username {
        conn.execute_batch(
            "ALTER TABLE users ADD COLUMN username TEXT;
             CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username ON users(username);",
        )?;
    }

    // --- User settings ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS user_settings (
            user_id TEXT PRIMARY KEY,
            font_family TEXT NOT NULL DEFAULT 'monospace',
            font_size INTEGER NOT NULL DEFAULT 18,
            theme TEXT NOT NULL DEFAULT 'dark',
            idle_timeout INTEGER NOT NULL DEFAULT 8,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );",
    )?;

    // --- Phase 3: X Bot ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS x_interactions (
            id TEXT PRIMARY KEY,
            tweet_id TEXT UNIQUE NOT NULL,
            x_user_id TEXT,
            x_username TEXT,
            tweet_text TEXT,
            prompt_id TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            classification TEXT,
            reply_tweet_id TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    // --- wallet_address on users ---
    let has_wallet: bool = conn
        .prepare("SELECT wallet_address FROM users LIMIT 0")
        .is_ok();
    if !has_wallet {
        conn.execute_batch("ALTER TABLE users ADD COLUMN wallet_address TEXT;")?;
    }

    // --- privy_did on users ---
    let has_privy_did: bool = conn.prepare("SELECT privy_did FROM users LIMIT 0").is_ok();
    if !has_privy_did {
        conn.execute_batch("ALTER TABLE users ADD COLUMN privy_did TEXT;")?;
    }

    // --- image_webp on ankys ---
    let has_image_webp: bool = conn.prepare("SELECT image_webp FROM ankys LIMIT 0").is_ok();
    if !has_image_webp {
        conn.execute_batch("ALTER TABLE ankys ADD COLUMN image_webp TEXT;")?;
    }

    // --- session_token on writing_checkpoints ---
    let has_session_token: bool = conn
        .prepare("SELECT session_token FROM writing_checkpoints LIMIT 0")
        .is_ok();
    if !has_session_token {
        conn.execute_batch("ALTER TABLE writing_checkpoints ADD COLUMN session_token TEXT;")?;
    }

    // --- created_by on prompts ---
    let has_created_by: bool = conn
        .prepare("SELECT created_by FROM prompts LIMIT 0")
        .is_ok();
    if !has_created_by {
        conn.execute_batch("ALTER TABLE prompts ADD COLUMN created_by TEXT;")?;
    }

    // --- Video Recordings ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS video_recordings (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            title TEXT,
            file_path TEXT,
            duration_seconds REAL NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'pending',
            scene_data TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    // --- Memory System ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory_embeddings (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            writing_session_id TEXT,
            source TEXT NOT NULL,
            content TEXT NOT NULL,
            embedding BLOB NOT NULL,
            created_at TEXT DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS user_memories (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            writing_session_id TEXT,
            category TEXT NOT NULL,
            content TEXT NOT NULL,
            importance REAL DEFAULT 0.5,
            occurrence_count INTEGER DEFAULT 1,
            first_seen_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            embedding BLOB,
            created_at TEXT DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS user_profiles (
            user_id TEXT PRIMARY KEY,
            total_sessions INTEGER DEFAULT 0,
            total_anky_sessions INTEGER DEFAULT 0,
            total_words_written INTEGER DEFAULT 0,
            psychological_profile TEXT,
            emotional_signature TEXT,
            core_tensions TEXT,
            growth_edges TEXT,
            last_profile_update TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_memory_embeddings_user ON memory_embeddings(user_id);
        CREATE INDEX IF NOT EXISTS idx_user_memories_user ON user_memories(user_id);
        CREATE INDEX IF NOT EXISTS idx_user_memories_user_category ON user_memories(user_id, category);"
    )?;

    // --- Farcaster fields on users ---
    let has_farcaster_fid: bool = conn
        .prepare("SELECT farcaster_fid FROM users LIMIT 0")
        .is_ok();
    if !has_farcaster_fid {
        conn.execute_batch(
            "ALTER TABLE users ADD COLUMN farcaster_fid INTEGER;
             ALTER TABLE users ADD COLUMN farcaster_username TEXT;
             ALTER TABLE users ADD COLUMN farcaster_pfp_url TEXT;",
        )?;
    }

    // --- Flow Score: keystroke_deltas + flow_score on writing_sessions ---
    let has_flow_score: bool = conn
        .prepare("SELECT flow_score FROM writing_sessions LIMIT 0")
        .is_ok();
    if !has_flow_score {
        conn.execute_batch(
            "ALTER TABLE writing_sessions ADD COLUMN keystroke_deltas TEXT;
             ALTER TABLE writing_sessions ADD COLUMN flow_score REAL;",
        )?;
    }

    // --- Video Projects (Grok pipeline) ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS video_projects (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            anky_id TEXT,
            writing_session_id TEXT,
            script_json TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            video_path TEXT,
            video_path_720p TEXT,
            video_path_360p TEXT,
            duration_seconds REAL DEFAULT 88,
            total_scenes INTEGER DEFAULT 0,
            completed_scenes INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );",
    )?;

    // --- Pipeline prompt overrides (editable from UI) ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS pipeline_prompts (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_by TEXT,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    // --- current_step on video_projects ---
    let has_current_step: bool = conn
        .prepare("SELECT current_step FROM video_projects LIMIT 0")
        .is_ok();
    if !has_current_step {
        conn.execute_batch(
            "ALTER TABLE video_projects ADD COLUMN current_step TEXT DEFAULT 'script';",
        )?;
    }

    // --- payment_tx_hash on video_projects ---
    let has_payment_tx: bool = conn
        .prepare("SELECT payment_tx_hash FROM video_projects LIMIT 0")
        .is_ok();
    if !has_payment_tx {
        conn.execute_batch("ALTER TABLE video_projects ADD COLUMN payment_tx_hash TEXT;")?;
    }

    // --- story_spine on video_projects ---
    let has_story_spine: bool = conn
        .prepare("SELECT story_spine FROM video_projects LIMIT 0")
        .is_ok();
    if !has_story_spine {
        conn.execute_batch("ALTER TABLE video_projects ADD COLUMN story_spine TEXT;")?;
    }

    // --- keyboard_layout on user_settings ---
    let has_keyboard_layout: bool = conn
        .prepare("SELECT keyboard_layout FROM user_settings LIMIT 0")
        .is_ok();
    if !has_keyboard_layout {
        conn.execute_batch(
            "ALTER TABLE user_settings ADD COLUMN keyboard_layout TEXT NOT NULL DEFAULT 'qwerty';",
        )?;
    }

    // --- email on users ---
    let has_email: bool = conn.prepare("SELECT email FROM users LIMIT 0").is_ok();
    if !has_email {
        conn.execute_batch("ALTER TABLE users ADD COLUMN email TEXT;")?;
    }

    // --- Anky Likes ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS anky_likes (
            user_id TEXT NOT NULL,
            anky_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (user_id, anky_id)
        );",
    )?;

    // --- image_thumb on ankys ---
    let has_image_thumb: bool = conn
        .prepare("SELECT image_thumb FROM ankys LIMIT 0")
        .is_ok();
    if !has_image_thumb {
        conn.execute_batch("ALTER TABLE ankys ADD COLUMN image_thumb TEXT;")?;
    }

    // --- image_model on ankys: tracks which model generated the image ---
    let has_image_model: bool = conn
        .prepare("SELECT image_model FROM ankys LIMIT 0")
        .is_ok();
    if !has_image_model {
        conn.execute_batch("ALTER TABLE ankys ADD COLUMN image_model TEXT NOT NULL DEFAULT 'gemini';")?;
    }

    // --- Leaderboard: streak + best_flow_score on user_profiles ---
    let has_streak: bool = conn
        .prepare("SELECT current_streak FROM user_profiles LIMIT 0")
        .is_ok();
    if !has_streak {
        conn.execute_batch(
            "ALTER TABLE user_profiles ADD COLUMN current_streak INTEGER DEFAULT 0;
             ALTER TABLE user_profiles ADD COLUMN longest_streak INTEGER DEFAULT 0;
             ALTER TABLE user_profiles ADD COLUMN best_flow_score REAL DEFAULT 0;
             ALTER TABLE user_profiles ADD COLUMN avg_flow_score REAL DEFAULT 0;
             ALTER TABLE user_profiles ADD COLUMN last_anky_date TEXT;",
        )?;
    }

    // --- Inquiry System ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS user_inquiries (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            question TEXT NOT NULL,
            language TEXT DEFAULT 'en',
            response_text TEXT,
            response_session_id TEXT,
            answered_at TEXT,
            skipped INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );",
    )?;

    // --- Meditation System ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meditation_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            duration_target INTEGER NOT NULL,
            duration_actual INTEGER,
            completed BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS user_interactions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            meditation_session_id TEXT,
            interaction_type TEXT NOT NULL,
            question_text TEXT,
            response_text TEXT,
            metadata_json TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS user_progression (
            user_id TEXT PRIMARY KEY,
            total_meditations INTEGER NOT NULL DEFAULT 0,
            total_completed INTEGER NOT NULL DEFAULT 0,
            current_meditation_level INTEGER NOT NULL DEFAULT 0,
            write_unlocked BOOLEAN NOT NULL DEFAULT 0,
            current_streak INTEGER NOT NULL DEFAULT 0,
            longest_streak INTEGER NOT NULL DEFAULT 0,
            last_session_date TEXT
        );",
    )?;

    // --- conversation_json on ankys ---
    let has_conversation_json: bool = conn
        .prepare("SELECT conversation_json FROM ankys LIMIT 0")
        .is_ok();
    if !has_conversation_json {
        conn.execute_batch("ALTER TABLE ankys ADD COLUMN conversation_json TEXT;")?;
    }

    // --- Interview System ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS interviews (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            guest_name TEXT NOT NULL DEFAULT 'guest',
            is_anonymous BOOLEAN NOT NULL DEFAULT 1,
            started_at TEXT NOT NULL DEFAULT (datetime('now')),
            ended_at TEXT,
            summary TEXT,
            duration_seconds REAL,
            message_count INTEGER DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_interviews_user_id ON interviews(user_id);

        CREATE TABLE IF NOT EXISTS interview_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            interview_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (interview_id) REFERENCES interviews(id)
        );
        CREATE INDEX IF NOT EXISTS idx_interview_messages_interview_id ON interview_messages(interview_id);",
    )?;

    Ok(())
}
