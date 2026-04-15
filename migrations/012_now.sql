-- Anky Now: collaborative writing anchored to QR codes

CREATE TABLE IF NOT EXISTS nows (
    id TEXT PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL,
    prompt TEXT NOT NULL,
    prompt_image_path TEXT,
    prompt_image_status TEXT NOT NULL DEFAULT 'queued',
    creator_id TEXT,
    mode TEXT NOT NULL DEFAULT 'sticker',
    duration_seconds INTEGER NOT NULL DEFAULT 480,
    starts_at TEXT,
    started INTEGER NOT NULL DEFAULT 0,
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    created_at TEXT NOT NULL DEFAULT anky_now()
);

CREATE INDEX IF NOT EXISTS idx_nows_slug ON nows(slug);

CREATE TABLE IF NOT EXISTS now_sessions (
    now_id TEXT NOT NULL REFERENCES nows(id),
    writing_session_id TEXT NOT NULL REFERENCES writing_sessions(id),
    sequence INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT anky_now(),
    PRIMARY KEY (now_id, writing_session_id)
);

CREATE INDEX IF NOT EXISTS idx_now_sessions_seq ON now_sessions(now_id, sequence);

CREATE TABLE IF NOT EXISTS now_presence (
    now_id TEXT NOT NULL REFERENCES nows(id),
    user_id TEXT NOT NULL,
    display_name TEXT NOT NULL DEFAULT '',
    joined_at TEXT NOT NULL DEFAULT anky_now(),
    last_seen_at TEXT NOT NULL DEFAULT anky_now(),
    PRIMARY KEY (now_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_now_presence_active ON now_presence(now_id, last_seen_at);
