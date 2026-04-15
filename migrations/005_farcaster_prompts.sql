CREATE TABLE IF NOT EXISTS farcaster_prompts (
    fid BIGINT PRIMARY KEY,
    prompt_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
