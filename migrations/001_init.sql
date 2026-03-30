CREATE OR REPLACE FUNCTION anky_now() RETURNS TEXT AS $$
SELECT TO_CHAR(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS');
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION anky_now_ms() RETURNS TEXT AS $$
SELECT TO_CHAR(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"');
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION anky_datetime(base TEXT) RETURNS TEXT AS $$
SELECT CASE WHEN base = 'now' THEN anky_now() ELSE base END;
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION anky_datetime(base TEXT, offset_value TEXT) RETURNS TEXT AS $$
SELECT CASE
    WHEN base = 'now' THEN TO_CHAR((NOW() AT TIME ZONE 'UTC') + offset_value::interval, 'YYYY-MM-DD HH24:MI:SS')
    ELSE base
END;
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION anky_date(value TEXT) RETURNS TEXT AS $$
SELECT CASE
    WHEN value = 'now' THEN TO_CHAR(CURRENT_DATE, 'YYYY-MM-DD')
    ELSE SUBSTRING(value FROM 1 FOR 10)
END;
$$ LANGUAGE SQL STABLE;

CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT anky_now()
        , username TEXT, wallet_address TEXT, privy_did TEXT, farcaster_fid INTEGER, farcaster_username TEXT, farcaster_pfp_url TEXT, email TEXT, is_premium INTEGER NOT NULL DEFAULT 0, premium_since TEXT, generated_wallet_secret TEXT, wallet_generated_at TEXT, is_pro INTEGER NOT NULL DEFAULT 0);

CREATE TABLE IF NOT EXISTS writing_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            content TEXT NOT NULL,
            duration_seconds DOUBLE PRECISION NOT NULL,
            word_count INTEGER NOT NULL,
            is_anky INTEGER NOT NULL DEFAULT 0,
            response TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(), keystroke_deltas TEXT, flow_score DOUBLE PRECISION, status TEXT NOT NULL DEFAULT 'completed', pause_used INTEGER NOT NULL DEFAULT 0, paused_at TEXT, resumed_at TEXT, session_token TEXT, content_deleted_at TEXT, anky_response TEXT, anky_next_prompt TEXT, anky_mood TEXT,
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
            is_minted INTEGER NOT NULL DEFAULT 0,
            mint_tx_hash TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL DEFAULT anky_now(), origin TEXT NOT NULL DEFAULT 'written', image_webp TEXT, image_thumb TEXT, conversation_json TEXT, image_model TEXT NOT NULL DEFAULT 'gemini', prompt_id TEXT, formatted_writing TEXT, gas_funded_at TEXT, session_cid TEXT, metadata_uri TEXT, token_id TEXT, anky_story TEXT, kingdom_id INTEGER, kingdom_name TEXT, kingdom_chakra TEXT, retry_count INTEGER NOT NULL DEFAULT 0, last_retry_at TEXT,
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
            cost_estimate_usd DOUBLE PRECISION,
            actual_cost_usd DOUBLE PRECISION DEFAULT 0,
            progress INTEGER NOT NULL DEFAULT 0,
            total INTEGER NOT NULL DEFAULT 88,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE TABLE IF NOT EXISTS cost_records (
            id BIGSERIAL PRIMARY KEY,
            service TEXT NOT NULL,
            model TEXT NOT NULL,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
            related_id TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS training_runs (
            id TEXT PRIMARY KEY,
            base_model TEXT NOT NULL,
            dataset_size INTEGER NOT NULL,
            steps INTEGER NOT NULL,
            current_step INTEGER NOT NULL DEFAULT 0,
            loss DOUBLE PRECISION,
            status TEXT NOT NULL DEFAULT 'pending',
            lora_weights_path TEXT,
            started_at TEXT,
            completed_at TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS notification_signups (
            id BIGSERIAL PRIMARY KEY,
            email TEXT,
            telegram_chat_id TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS api_keys (
            key TEXT PRIMARY KEY,
            label TEXT,
            balance_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
            total_spent_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
            total_transforms INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS transformations (
            id TEXT PRIMARY KEY,
            api_key TEXT NOT NULL,
            input_text TEXT NOT NULL,
            prompt TEXT,
            output_text TEXT NOT NULL,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (api_key) REFERENCES api_keys(key)
        );

CREATE TABLE IF NOT EXISTS credit_purchases (
            id TEXT PRIMARY KEY,
            api_key TEXT NOT NULL,
            tx_hash TEXT NOT NULL,
            amount_usdc DOUBLE PRECISION NOT NULL,
            amount_credited_usd DOUBLE PRECISION NOT NULL,
            verified INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT anky_now(),
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
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (api_key) REFERENCES api_keys(key)
        );

CREATE TABLE IF NOT EXISTS generation_records (
            id TEXT PRIMARY KEY,
            anky_id TEXT NOT NULL,
            api_key TEXT,
            agent_id TEXT,
            payment_method TEXT NOT NULL,
            amount_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
            tx_hash TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS writing_checkpoints (
            id BIGSERIAL PRIMARY KEY,
            session_id TEXT NOT NULL,
            content TEXT NOT NULL,
            elapsed_seconds DOUBLE PRECISION NOT NULL,
            word_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT anky_now()
        , session_token TEXT);

CREATE TABLE IF NOT EXISTS feedback (
            id TEXT PRIMARY KEY,
            source TEXT NOT NULL,
            author TEXT,
            content TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS user_collections (
            user_id TEXT NOT NULL,
            anky_id TEXT NOT NULL,
            collected_at TEXT NOT NULL DEFAULT anky_now(),
            PRIMARY KEY (user_id, anky_id)
        );

CREATE TABLE IF NOT EXISTS prompts (
            id TEXT PRIMARY KEY,
            creator_user_id TEXT NOT NULL,
            prompt_text TEXT NOT NULL,
            image_path TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            payment_tx_hash TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(), created_by TEXT,
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
            duration_seconds DOUBLE PRECISION,
            word_count INTEGER NOT NULL DEFAULT 0,
            completed INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (prompt_id) REFERENCES prompts(id)
        );

CREATE TABLE IF NOT EXISTS x_users (
            x_user_id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            username TEXT NOT NULL,
            display_name TEXT,
            profile_image_url TEXT,
            access_token TEXT NOT NULL,
            refresh_token TEXT,
            token_expires_at TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            updated_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE TABLE IF NOT EXISTS auth_sessions (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            x_user_id TEXT,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE TABLE IF NOT EXISTS oauth_states (
            state TEXT PRIMARY KEY,
            code_verifier TEXT NOT NULL,
            redirect_to TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS x_interactions (
            id TEXT PRIMARY KEY,
            tweet_id TEXT UNIQUE NOT NULL,
            x_user_id TEXT,
            x_username TEXT,
            tweet_text TEXT,
            prompt_id TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            classification TEXT,
            reply_tweet_id TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        , source TEXT NOT NULL DEFAULT 'filtered_stream', parent_tweet_id TEXT, tag TEXT, extracted_content TEXT, result_text TEXT, error_message TEXT, updated_at TEXT);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username ON users(username);

CREATE TABLE IF NOT EXISTS user_settings (
            user_id TEXT PRIMARY KEY,
            font_family TEXT NOT NULL DEFAULT 'monospace',
            font_size INTEGER NOT NULL DEFAULT 18,
            theme TEXT NOT NULL DEFAULT 'dark',
            idle_timeout INTEGER NOT NULL DEFAULT 8, keyboard_layout TEXT NOT NULL DEFAULT 'qwerty', preferred_language TEXT NOT NULL DEFAULT 'en', preferred_model TEXT NOT NULL DEFAULT 'default',
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE TABLE IF NOT EXISTS video_recordings (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            title TEXT,
            file_path TEXT,
            duration_seconds DOUBLE PRECISION NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'pending',
            scene_data TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS memory_embeddings (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            writing_session_id TEXT,
            source TEXT NOT NULL,
            content TEXT NOT NULL,
            embedding BYTEA NOT NULL,
            created_at TEXT DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS user_memories (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            writing_session_id TEXT,
            category TEXT NOT NULL,
            content TEXT NOT NULL,
            importance DOUBLE PRECISION DEFAULT 0.5,
            occurrence_count INTEGER DEFAULT 1,
            first_seen_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            embedding BYTEA,
            created_at TEXT DEFAULT anky_now()
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
            created_at TEXT DEFAULT anky_now(),
            updated_at TEXT DEFAULT anky_now()
        , current_streak INTEGER DEFAULT 0, longest_streak INTEGER DEFAULT 0, best_flow_score DOUBLE PRECISION DEFAULT 0, avg_flow_score DOUBLE PRECISION DEFAULT 0, last_anky_date TEXT);

CREATE INDEX IF NOT EXISTS idx_memory_embeddings_user ON memory_embeddings(user_id);

CREATE INDEX IF NOT EXISTS idx_user_memories_user ON user_memories(user_id);

CREATE INDEX IF NOT EXISTS idx_user_memories_user_category ON user_memories(user_id, category);

CREATE TABLE IF NOT EXISTS video_projects (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            anky_id TEXT,
            writing_session_id TEXT,
            script_json TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            video_path TEXT,
            video_path_720p TEXT,
            video_path_360p TEXT,
            duration_seconds DOUBLE PRECISION DEFAULT 88,
            total_scenes INTEGER DEFAULT 0,
            completed_scenes INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT anky_now(), current_step TEXT DEFAULT 'script', story_spine TEXT, payment_tx_hash TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE TABLE IF NOT EXISTS anky_likes (
            user_id TEXT NOT NULL,
            anky_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            PRIMARY KEY (user_id, anky_id)
        );

CREATE TABLE IF NOT EXISTS meditation_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            duration_target INTEGER NOT NULL,
            duration_actual INTEGER,
            completed INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS user_interactions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            meditation_session_id TEXT,
            interaction_type TEXT NOT NULL,
            question_text TEXT,
            response_text TEXT,
            metadata_json TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS user_progression (
            user_id TEXT PRIMARY KEY,
            total_meditations INTEGER NOT NULL DEFAULT 0,
            total_completed INTEGER NOT NULL DEFAULT 0,
            current_meditation_level INTEGER NOT NULL DEFAULT 0,
            write_unlocked INTEGER NOT NULL DEFAULT 0,
            current_streak INTEGER NOT NULL DEFAULT 0,
            longest_streak INTEGER NOT NULL DEFAULT 0,
            last_session_date TEXT
        );

CREATE TABLE IF NOT EXISTS user_inquiries (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            question TEXT NOT NULL,
            language TEXT DEFAULT 'en',
            response_text TEXT,
            response_session_id TEXT,
            answered_at TEXT,
            skipped INTEGER DEFAULT 0,
            created_at TEXT DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS pipeline_prompts (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_by TEXT,
            updated_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS interviews (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            guest_name TEXT NOT NULL DEFAULT 'guest',
            is_anonymous INTEGER NOT NULL DEFAULT 1,
            started_at TEXT NOT NULL DEFAULT anky_now(),
            ended_at TEXT,
            summary TEXT,
            duration_seconds DOUBLE PRECISION,
            message_count INTEGER DEFAULT 0
        );

CREATE INDEX IF NOT EXISTS idx_interviews_user_id ON interviews(user_id);

CREATE TABLE IF NOT EXISTS interview_messages (
            id BIGSERIAL PRIMARY KEY,
            interview_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (interview_id) REFERENCES interviews(id)
        );

CREATE INDEX IF NOT EXISTS idx_interview_messages_interview_id ON interview_messages(interview_id);

CREATE TABLE IF NOT EXISTS training_labels (
            anky_id TEXT PRIMARY KEY,
            approved INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS sadhana_commitments (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            frequency TEXT NOT NULL DEFAULT 'daily',
            duration_minutes INTEGER NOT NULL DEFAULT 10,
            target_days INTEGER NOT NULL DEFAULT 30,
            start_date TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE TABLE IF NOT EXISTS sadhana_checkins (
            id TEXT PRIMARY KEY,
            commitment_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            date TEXT NOT NULL,
            completed INTEGER NOT NULL DEFAULT 1,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            UNIQUE (commitment_id, date),
            FOREIGN KEY (commitment_id) REFERENCES sadhana_commitments(id)
        );

CREATE TABLE IF NOT EXISTS breathwork_sessions (
            id TEXT PRIMARY KEY,
            style TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL DEFAULT 480,
            script_json TEXT NOT NULL,
            generated_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS breathwork_completions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            completed_at TEXT NOT NULL DEFAULT anky_now(),
            notes TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (session_id) REFERENCES breathwork_sessions(id)
        );

CREATE INDEX IF NOT EXISTS idx_sadhana_commitments_user ON sadhana_commitments(user_id);

CREATE INDEX IF NOT EXISTS idx_sadhana_checkins_commitment ON sadhana_checkins(commitment_id);

CREATE INDEX IF NOT EXISTS idx_breathwork_completions_user ON breathwork_completions(user_id);

CREATE INDEX IF NOT EXISTS idx_breathwork_sessions_style ON breathwork_sessions(style);

CREATE TABLE IF NOT EXISTS personalized_meditations (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            writing_session_id TEXT,
            script_json TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            tier TEXT NOT NULL DEFAULT 'free',
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE TABLE IF NOT EXISTS personalized_breathwork (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            writing_session_id TEXT,
            style TEXT NOT NULL DEFAULT 'calming',
            script_json TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            tier TEXT NOT NULL DEFAULT 'free',
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE INDEX IF NOT EXISTS idx_personalized_meditations_user ON personalized_meditations(user_id, status);

CREATE INDEX IF NOT EXISTS idx_personalized_breathwork_user ON personalized_breathwork(user_id, status);

CREATE TABLE IF NOT EXISTS facilitators (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            name TEXT NOT NULL,
            bio TEXT NOT NULL,
            specialties TEXT NOT NULL DEFAULT '[]',
            approach TEXT,
            session_rate_usd DOUBLE PRECISION NOT NULL,
            booking_url TEXT,
            contact_method TEXT,
            profile_image_url TEXT,
            location TEXT,
            languages TEXT NOT NULL DEFAULT '["en"]',
            status TEXT NOT NULL DEFAULT 'pending',
            avg_rating DOUBLE PRECISION DEFAULT 0,
            total_reviews INTEGER DEFAULT 0,
            total_sessions INTEGER DEFAULT 0,
            fee_paid INTEGER NOT NULL DEFAULT 0,
            fee_tx_hash TEXT,
            approved_at TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE TABLE IF NOT EXISTS facilitator_reviews (
            id TEXT PRIMARY KEY,
            facilitator_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            rating INTEGER NOT NULL,
            review_text TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            UNIQUE (facilitator_id, user_id),
            FOREIGN KEY (facilitator_id) REFERENCES facilitators(id),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE TABLE IF NOT EXISTS facilitator_bookings (
            id TEXT PRIMARY KEY,
            facilitator_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            payment_amount_usd DOUBLE PRECISION,
            platform_fee_usd DOUBLE PRECISION,
            payment_method TEXT,
            payment_tx_hash TEXT,
            stripe_payment_id TEXT,
            user_context_shared INTEGER DEFAULT 0,
            shared_context_json TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (facilitator_id) REFERENCES facilitators(id),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE INDEX IF NOT EXISTS idx_facilitators_status ON facilitators(status);

CREATE INDEX IF NOT EXISTS idx_facilitator_reviews_fac ON facilitator_reviews(facilitator_id);

CREATE INDEX IF NOT EXISTS idx_facilitator_bookings_user ON facilitator_bookings(user_id);

CREATE TABLE IF NOT EXISTS llm_training_runs (id BIGSERIAL PRIMARY KEY, run_date TEXT NOT NULL, val_bpb DOUBLE PRECISION NOT NULL, training_seconds DOUBLE PRECISION NOT NULL, peak_vram_mb DOUBLE PRECISION NOT NULL, mfu_percent DOUBLE PRECISION NOT NULL, total_tokens_m DOUBLE PRECISION NOT NULL, num_steps INTEGER NOT NULL, num_params_m DOUBLE PRECISION NOT NULL, depth INTEGER NOT NULL, corpus_sessions INTEGER NOT NULL, corpus_words INTEGER NOT NULL, corpus_tokens INTEGER NOT NULL, epochs INTEGER NOT NULL, status TEXT NOT NULL DEFAULT 'complete', created_at TEXT NOT NULL DEFAULT anky_now());

CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_training_runs_date ON llm_training_runs(run_date);

CREATE TABLE IF NOT EXISTS x_conversations (
            tweet_id TEXT PRIMARY KEY,
            author_id TEXT NOT NULL,
            author_username TEXT,
            parent_tweet_id TEXT,
            mention_text TEXT,
            anky_reply_text TEXT,
            context_summary TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE INDEX IF NOT EXISTS idx_x_conversations_author ON x_conversations(author_id);

CREATE INDEX IF NOT EXISTS idx_x_conversations_parent ON x_conversations(parent_tweet_id);

CREATE TABLE IF NOT EXISTS x_evolution_tasks (
            id TEXT PRIMARY KEY,
            tweet_id TEXT NOT NULL,
            tag TEXT NOT NULL,
            content TEXT NOT NULL,
            author TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'running',
            summary TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            completed_at TEXT
        );

CREATE INDEX IF NOT EXISTS idx_x_evolution_tasks_tag ON x_evolution_tasks(tag);

CREATE INDEX IF NOT EXISTS idx_x_interactions_user_created ON x_interactions(x_user_id, created_at);

CREATE INDEX IF NOT EXISTS idx_x_interactions_status ON x_interactions(status);

CREATE TABLE IF NOT EXISTS social_interactions (
            id TEXT PRIMARY KEY,
            platform TEXT NOT NULL,
            post_id TEXT NOT NULL,
            author_id TEXT,
            author_username TEXT,
            post_text TEXT,
            parent_id TEXT,
            status TEXT NOT NULL DEFAULT 'received',
            classification TEXT,
            reply_text TEXT,
            reply_id TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            updated_at TEXT
        );

CREATE UNIQUE INDEX IF NOT EXISTS idx_social_interactions_platform_post ON social_interactions(platform, post_id);

CREATE INDEX IF NOT EXISTS idx_social_interactions_author ON social_interactions(platform, author_id);

CREATE INDEX IF NOT EXISTS idx_social_interactions_status ON social_interactions(status);

CREATE TABLE IF NOT EXISTS agent_session_events (
            id BIGSERIAL PRIMARY KEY,
            session_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            agent_name TEXT NOT NULL,
            event_type TEXT NOT NULL,
            chunk_index INTEGER,
            elapsed_seconds DOUBLE PRECISION NOT NULL DEFAULT 0,
            words_total INTEGER NOT NULL DEFAULT 0,
            chunk_text TEXT,
            chunk_word_count INTEGER,
            detail_json TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now_ms()
        );

CREATE INDEX IF NOT EXISTS idx_agent_session_events_session_id_id
            ON agent_session_events(session_id, id);

CREATE INDEX IF NOT EXISTS idx_agent_session_events_agent_id_created_at
            ON agent_session_events(agent_id, created_at);

CREATE TABLE IF NOT EXISTS auth_challenges (
            id TEXT PRIMARY KEY,
            wallet_address TEXT NOT NULL,
            challenge_text TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            consumed_at TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_wallet_address ON users(wallet_address);

CREATE TABLE IF NOT EXISTS child_profiles (
            id TEXT PRIMARY KEY,
            parent_wallet_address TEXT NOT NULL,
            derived_wallet_address TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            birthdate TEXT NOT NULL,
            emoji_pattern TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (parent_wallet_address) REFERENCES users(wallet_address)
        );

CREATE INDEX IF NOT EXISTS idx_child_profiles_parent_wallet
            ON child_profiles(parent_wallet_address);

CREATE TABLE IF NOT EXISTS cuentacuentos (
            id TEXT PRIMARY KEY,
            writing_id TEXT NOT NULL,
            parent_wallet_address TEXT NOT NULL,
            child_wallet_address TEXT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            guidance_phases TEXT NOT NULL,
            played INTEGER NOT NULL DEFAULT 0,
            generated_at TEXT NOT NULL DEFAULT anky_now(), chakra INTEGER, kingdom TEXT, city TEXT, content_es TEXT, content_zh TEXT, content_hi TEXT, content_ar TEXT,
            FOREIGN KEY (writing_id) REFERENCES writing_sessions(id)
        );

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_parent_ready
            ON cuentacuentos(parent_wallet_address, played, generated_at);

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_parent_child_ready
            ON cuentacuentos(parent_wallet_address, child_wallet_address, played, generated_at);

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_writing_id
            ON cuentacuentos(writing_id);

CREATE TABLE IF NOT EXISTS cuentacuentos_images (
            id TEXT PRIMARY KEY,
            cuentacuentos_id TEXT NOT NULL,
            phase_index INTEGER NOT NULL,
            image_prompt TEXT NOT NULL,
            image_url TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            attempts INTEGER NOT NULL DEFAULT 0,
            generated_at TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            UNIQUE (cuentacuentos_id, phase_index),
            FOREIGN KEY (cuentacuentos_id) REFERENCES cuentacuentos(id)
        );

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_images_pending
            ON cuentacuentos_images(cuentacuentos_id, status, phase_index);

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_images_story
            ON cuentacuentos_images(cuentacuentos_id, phase_index);

CREATE TABLE IF NOT EXISTS system_summaries (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            period_start TEXT NOT NULL,
            period_end TEXT NOT NULL,
            raw_stats TEXT NOT NULL,
            summary TEXT NOT NULL
        );

CREATE INDEX IF NOT EXISTS idx_system_summaries_created ON system_summaries(created_at);

CREATE TABLE IF NOT EXISTS next_prompts (
            user_id TEXT PRIMARY KEY,
            prompt_text TEXT NOT NULL,
            generated_from_session TEXT,
            created_at TEXT DEFAULT anky_now()
        );

CREATE TABLE IF NOT EXISTS story_training_pairs (
            id TEXT PRIMARY KEY,
            cuentacuentos_id TEXT NOT NULL,
            writing_id TEXT NOT NULL,
            writing_input TEXT NOT NULL,
            story_title TEXT NOT NULL,
            story_content TEXT NOT NULL,
            chakra INTEGER,
            kingdom TEXT,
            city TEXT,
            played INTEGER NOT NULL DEFAULT 0,
            parent_wrote_again_within_24h INTEGER,
            language TEXT,
            quality_score DOUBLE PRECISION,
            exported_at TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (cuentacuentos_id) REFERENCES cuentacuentos(id)
        );

CREATE INDEX IF NOT EXISTS idx_story_training_pairs_unexported
            ON story_training_pairs(exported_at, created_at);

CREATE TABLE IF NOT EXISTS story_recordings (
            id TEXT PRIMARY KEY,
            story_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            attempt_number INTEGER NOT NULL CHECK (attempt_number >= 1 AND attempt_number <= 4),
            language TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'processing', 'approved', 'rejected')),
            duration_seconds DOUBLE PRECISION NOT NULL,
            r2_key TEXT,
            audio_url TEXT,
            rejection_reason TEXT,
            full_listen_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            approved_at TEXT,
            UNIQUE (story_id, user_id, attempt_number),
            FOREIGN KEY (story_id) REFERENCES cuentacuentos(id),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );

CREATE INDEX IF NOT EXISTS idx_story_recordings_story ON story_recordings(story_id);

CREATE INDEX IF NOT EXISTS idx_story_recordings_user ON story_recordings(user_id);

CREATE INDEX IF NOT EXISTS idx_story_recordings_approved
            ON story_recordings(story_id, status, language);

CREATE TABLE IF NOT EXISTS story_listen_events (
            id TEXT PRIMARY KEY,
            story_id TEXT NOT NULL,
            recording_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            listened_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (recording_id) REFERENCES story_recordings(id)
        );

CREATE INDEX IF NOT EXISTS idx_story_listen_events_recording
            ON story_listen_events(recording_id);

CREATE TABLE IF NOT EXISTS device_tokens (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                device_token TEXT NOT NULL,
                platform TEXT NOT NULL DEFAULT 'ios',
                created_at TEXT NOT NULL DEFAULT anky_now(),
                updated_at TEXT NOT NULL DEFAULT anky_now(),
                UNIQUE (user_id, platform)
            );

CREATE INDEX IF NOT EXISTS idx_device_tokens_user ON device_tokens(user_id);

CREATE TABLE IF NOT EXISTS cuentacuentos_audio (
            id TEXT PRIMARY KEY,
            cuentacuentos_id TEXT NOT NULL,
            language TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'generating', 'complete', 'failed')),
            r2_key TEXT,
            audio_url TEXT,
            duration_seconds DOUBLE PRECISION,
            attempts INTEGER NOT NULL DEFAULT 0,
            error_message TEXT,
            generated_at TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            UNIQUE (cuentacuentos_id, language),
            FOREIGN KEY (cuentacuentos_id) REFERENCES cuentacuentos(id)
        );

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_audio_pending
            ON cuentacuentos_audio(cuentacuentos_id, status);

CREATE TABLE IF NOT EXISTS social_peers (
            id TEXT PRIMARY KEY,
            platform TEXT NOT NULL,
            platform_user_id TEXT NOT NULL,
            platform_username TEXT,
            honcho_peer_id TEXT,
            user_id TEXT,
            interaction_count INTEGER NOT NULL DEFAULT 0,
            first_seen_at TEXT NOT NULL DEFAULT anky_now(),
            last_seen_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE UNIQUE INDEX IF NOT EXISTS idx_social_peers_platform_user
            ON social_peers(platform, platform_user_id);

CREATE INDEX IF NOT EXISTS idx_social_peers_username
            ON social_peers(platform, platform_username);

CREATE TABLE IF NOT EXISTS mirrors (
            id TEXT PRIMARY KEY,
            fid INTEGER NOT NULL,
            username TEXT NOT NULL,
            display_name TEXT NOT NULL DEFAULT '',
            avatar_url TEXT,
            follower_count INTEGER NOT NULL DEFAULT 0,
            bio TEXT NOT NULL DEFAULT '',
            public_mirror TEXT NOT NULL,
            flux_descriptors_json TEXT NOT NULL,
            image_path TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        , gap TEXT NOT NULL DEFAULT '');

CREATE INDEX IF NOT EXISTS idx_mirrors_fid ON mirrors(fid);

CREATE INDEX IF NOT EXISTS idx_mirrors_created ON mirrors(created_at DESC);

CREATE TABLE IF NOT EXISTS programming_classes (
            id INTEGER PRIMARY KEY,
            class_number INTEGER NOT NULL UNIQUE,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            concept TEXT NOT NULL DEFAULT '',
            slides_json TEXT NOT NULL DEFAULT '[]',
            changelog_slug TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now()
        );

CREATE INDEX IF NOT EXISTS idx_programming_classes_number ON programming_classes(class_number);
