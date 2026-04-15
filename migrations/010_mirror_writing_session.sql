-- Link mirrors to writing sessions for per-anky minting
ALTER TABLE mirrors ADD COLUMN IF NOT EXISTS writing_session_id TEXT;

CREATE INDEX IF NOT EXISTS idx_mirrors_writing_session
    ON mirrors(writing_session_id) WHERE writing_session_id IS NOT NULL;
