-- Sponsored Solana transaction audit trail for mobile Loom minting and sealing.
-- This table stores only public wallet/action/hash metadata. It must never
-- contain .anky plaintext, private keys, keypair paths, API keys, or raw proof
-- witnesses.

CREATE TABLE IF NOT EXISTS mobile_sponsorship_events (
    id TEXT PRIMARY KEY,
    network TEXT NOT NULL DEFAULT 'devnet',
    wallet TEXT NOT NULL,
    action TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    utc_day BIGINT,
    session_hash TEXT,
    loom_asset TEXT,
    sponsor_payer TEXT NOT NULL,
    estimated_lamports BIGINT NOT NULL DEFAULT 0 CHECK (estimated_lamports >= 0),
    signature TEXT,
    status TEXT NOT NULL DEFAULT 'prepared',
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT mobile_sponsorship_events_action
        CHECK (action IN ('mint_loom', 'seal', 'proof')),
    CONSTRAINT mobile_sponsorship_events_status
        CHECK (status IN ('prepared', 'submitted', 'confirmed', 'finalized', 'failed', 'expired')),
    CONSTRAINT mobile_sponsorship_events_session_hash_hex
        CHECK (session_hash IS NULL OR session_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT mobile_sponsorship_events_utc_day_nonnegative
        CHECK (utc_day IS NULL OR utc_day >= 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_mobile_sponsorship_events_idempotency
    ON mobile_sponsorship_events(network, action, idempotency_key);

CREATE INDEX IF NOT EXISTS idx_mobile_sponsorship_events_wallet_created
    ON mobile_sponsorship_events(wallet, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_mobile_sponsorship_events_budget
    ON mobile_sponsorship_events(network, created_at DESC, status);

CREATE UNIQUE INDEX IF NOT EXISTS idx_mobile_sponsored_daily_seal_once
    ON mobile_sponsorship_events(network, wallet, utc_day)
    WHERE action = 'seal'
      AND utc_day IS NOT NULL
      AND status IN ('prepared', 'submitted', 'confirmed', 'finalized');
