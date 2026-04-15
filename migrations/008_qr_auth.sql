CREATE TABLE IF NOT EXISTS qr_auth_challenges (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
    token TEXT NOT NULL UNIQUE,
    solana_address TEXT,
    sealed BOOLEAN NOT NULL DEFAULT FALSE,
    session_token TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT NOW()::text
);

CREATE INDEX idx_qr_auth_token ON qr_auth_challenges(token);
CREATE INDEX idx_qr_auth_expires ON qr_auth_challenges(expires_at);
