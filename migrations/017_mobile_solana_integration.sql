-- Expo Sojourn 9 / Solana devnet integration.
-- These tables intentionally separate public witness data from optional
-- plaintext processing requests. Seal and Loom receipt rows never store .anky
-- contents.

CREATE TABLE IF NOT EXISTS mobile_credit_accounts (
    identity_id TEXT PRIMARY KEY,
    credits_remaining INTEGER NOT NULL DEFAULT 8 CHECK (credits_remaining >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS mobile_credit_events (
    id TEXT PRIMARY KEY,
    identity_id TEXT NOT NULL REFERENCES mobile_credit_accounts(identity_id) ON DELETE CASCADE,
    delta INTEGER NOT NULL,
    reason TEXT NOT NULL,
    related_id TEXT,
    metadata_json TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mobile_credit_events_identity_created
    ON mobile_credit_events(identity_id, created_at DESC);

CREATE TABLE IF NOT EXISTS mobile_mint_authorizations (
    id TEXT PRIMARY KEY,
    network TEXT NOT NULL DEFAULT 'devnet',
    wallet TEXT NOT NULL,
    payer TEXT NOT NULL,
    core_collection TEXT NOT NULL,
    loom_index INTEGER NOT NULL,
    mode TEXT NOT NULL,
    invite_code_hash TEXT,
    allowed BOOLEAN NOT NULL,
    sponsor BOOLEAN NOT NULL DEFAULT FALSE,
    sponsor_payer TEXT,
    reason TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mobile_mint_authorizations_wallet_created
    ON mobile_mint_authorizations(wallet, created_at DESC);

CREATE TABLE IF NOT EXISTS mobile_loom_mints (
    id TEXT PRIMARY KEY,
    network TEXT NOT NULL DEFAULT 'devnet',
    wallet TEXT NOT NULL,
    loom_asset TEXT NOT NULL UNIQUE,
    core_collection TEXT NOT NULL,
    signature TEXT NOT NULL UNIQUE,
    loom_index INTEGER,
    mint_mode TEXT,
    metadata_uri TEXT,
    status TEXT NOT NULL DEFAULT 'confirmed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mobile_loom_mints_wallet_created
    ON mobile_loom_mints(wallet, created_at DESC);

CREATE TABLE IF NOT EXISTS mobile_seal_receipts (
    id TEXT PRIMARY KEY,
    network TEXT NOT NULL DEFAULT 'devnet',
    wallet TEXT NOT NULL,
    loom_asset TEXT NOT NULL,
    core_collection TEXT NOT NULL,
    session_hash TEXT NOT NULL,
    signature TEXT NOT NULL UNIQUE,
    slot BIGINT,
    block_time BIGINT,
    status TEXT NOT NULL DEFAULT 'confirmed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mobile_seal_receipts_wallet_created
    ON mobile_seal_receipts(wallet, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_mobile_seal_receipts_loom_created
    ON mobile_seal_receipts(loom_asset, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_mobile_seal_receipts_hash_created
    ON mobile_seal_receipts(session_hash, created_at DESC);

CREATE TABLE IF NOT EXISTS mobile_reflection_jobs (
    id TEXT PRIMARY KEY,
    identity_id TEXT NOT NULL REFERENCES mobile_credit_accounts(identity_id) ON DELETE CASCADE,
    session_hash TEXT NOT NULL,
    processing_type TEXT NOT NULL,
    status TEXT NOT NULL,
    credits_spent INTEGER NOT NULL DEFAULT 0,
    request_json TEXT,
    result_json TEXT,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mobile_reflection_jobs_identity_created
    ON mobile_reflection_jobs(identity_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_mobile_reflection_jobs_hash_created
    ON mobile_reflection_jobs(session_hash, created_at DESC);
