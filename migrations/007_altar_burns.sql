CREATE TABLE IF NOT EXISTS altar_burns (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
    user_identifier TEXT NOT NULL,
    identifier_type TEXT NOT NULL DEFAULT 'wallet',
    amount_usdc BIGINT NOT NULL,
    tx_hash TEXT NOT NULL UNIQUE,
    display_name TEXT,
    avatar_url TEXT,
    fid BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_altar_burns_created ON altar_burns(created_at DESC);
CREATE INDEX idx_altar_burns_user ON altar_burns(user_identifier);
