ALTER TABLE writing_sessions
    ADD COLUMN IF NOT EXISTS session_hash TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_writing_sessions_user_session_hash_unique
    ON writing_sessions(user_id, session_hash)
    WHERE session_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_writing_sessions_session_hash
    ON writing_sessions(session_hash)
    WHERE session_hash IS NOT NULL;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS session_hash TEXT;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS session_payload TEXT;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS reflection_status TEXT NOT NULL DEFAULT 'pending';

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS image_status TEXT NOT NULL DEFAULT 'pending';

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS solana_status TEXT NOT NULL DEFAULT 'pending';

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS processing_job_state TEXT NOT NULL DEFAULT 'idle';

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS accepted_at TEXT;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS reflection_started_at TEXT;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS reflection_completed_at TEXT;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS image_completed_at TEXT;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS solana_completed_at TEXT;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS done_at TEXT;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS last_error_stage TEXT;

ALTER TABLE ankys
    ADD COLUMN IF NOT EXISTS last_error_message TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_ankys_user_session_hash_unique
    ON ankys(user_id, session_hash)
    WHERE session_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_ankys_session_hash
    ON ankys(session_hash)
    WHERE session_hash IS NOT NULL;

UPDATE ankys
SET reflection_status = 'complete'
WHERE COALESCE(reflection, '') <> ''
  AND reflection_status = 'pending';

UPDATE ankys
SET image_status = 'complete'
WHERE COALESCE(image_path, '') <> ''
  AND image_status = 'pending';

UPDATE ankys
SET solana_status = 'complete'
WHERE COALESCE(solana_mint_tx, '') <> ''
  AND solana_status = 'pending';

UPDATE ankys
SET processing_job_state = 'complete',
    done_at = COALESCE(done_at, created_at)
WHERE status IN ('complete', 'archived')
  AND processing_job_state = 'idle';
