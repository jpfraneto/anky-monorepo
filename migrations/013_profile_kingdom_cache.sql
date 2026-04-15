-- Caches Claude's classification of each anky into one of the 8 ankyverse
-- kingdoms, used by the /profile-testing prototype.
CREATE TABLE IF NOT EXISTS anky_kingdom_classification (
    anky_id         TEXT PRIMARY KEY REFERENCES ankys(id) ON DELETE CASCADE,
    kingdom_name    TEXT NOT NULL,
    kingdom_id      INTEGER NOT NULL,
    reason          TEXT NOT NULL,
    model           TEXT NOT NULL DEFAULT 'claude-haiku-4-5',
    classified_at   TEXT NOT NULL DEFAULT (to_char(now() AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"'))
);
