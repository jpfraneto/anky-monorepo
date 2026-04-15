-- Caches the 3-4 sentence written portrait synthesized across ALL of a user's
-- sessions, used by the /profile-testing prototype's top layer. Invalidated
-- whenever the anky_count changes (i.e. the user wrote a new one).
CREATE TABLE IF NOT EXISTS anky_user_portrait (
    user_id       TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    portrait      TEXT NOT NULL,
    anky_count    INTEGER NOT NULL,
    model         TEXT NOT NULL DEFAULT 'claude-sonnet-4',
    generated_at  TEXT NOT NULL DEFAULT (to_char(now() AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"'))
);
