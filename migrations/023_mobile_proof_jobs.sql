-- Mobile SP1 proof job metadata for the explicit "prove rite" action.
-- This table stores only public job/receipt metadata. Raw .anky bytes are
-- transient temp-file input outside the repo and must never be stored here.

CREATE TABLE IF NOT EXISTS mobile_proof_jobs (
    id TEXT PRIMARY KEY,
    network TEXT NOT NULL DEFAULT 'devnet',
    wallet TEXT NOT NULL,
    session_hash TEXT NOT NULL,
    seal_signature TEXT NOT NULL,
    loom_asset TEXT,
    core_collection TEXT,
    utc_day BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued',
    proof_hash TEXT,
    proof_signature TEXT,
    redacted_error TEXT,
    protocol_version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT mobile_proof_jobs_session_hash_hex
        CHECK (session_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT mobile_proof_jobs_proof_hash_hex
        CHECK (proof_hash IS NULL OR proof_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT mobile_proof_jobs_protocol_version
        CHECK (protocol_version = 1),
    CONSTRAINT mobile_proof_jobs_utc_day_nonnegative
        CHECK (utc_day >= 0),
    CONSTRAINT mobile_proof_jobs_status
        CHECK (status IN ('queued', 'proving', 'finalized', 'failed', 'unavailable'))
);

CREATE INDEX IF NOT EXISTS idx_mobile_proof_jobs_wallet_created
    ON mobile_proof_jobs(wallet, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_mobile_proof_jobs_hash_created
    ON mobile_proof_jobs(session_hash, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_mobile_proof_jobs_status_created
    ON mobile_proof_jobs(status, created_at DESC);
