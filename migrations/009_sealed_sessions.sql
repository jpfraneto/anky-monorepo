-- Sealed writing sessions: encrypted envelopes from the iOS app.
-- The backend stores these as opaque blobs. It never decrypts.

CREATE TABLE IF NOT EXISTS sealed_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    ciphertext BYTEA NOT NULL,
    nonce BYTEA NOT NULL,
    tag BYTEA NOT NULL,
    user_encrypted_key BYTEA NOT NULL,
    anky_encrypted_key BYTEA NOT NULL,
    session_hash TEXT NOT NULL,
    metadata_json TEXT,
    solana_tx_signature TEXT,
    sealed_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sealed_sessions_user ON sealed_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sealed_sessions_hash ON sealed_sessions(session_hash);
CREATE INDEX IF NOT EXISTS idx_sealed_sessions_session ON sealed_sessions(session_id);
