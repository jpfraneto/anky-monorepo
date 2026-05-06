-- SP1 / VerifiedSeal receipt metadata for Sojourn 9.
-- This table stores only public receipt values and on-chain transaction metadata.
-- It must never store .anky plaintext, prover witness bytes, or private proof inputs.

CREATE UNIQUE INDEX IF NOT EXISTS idx_mobile_seal_receipts_network_wallet_hash_unique
    ON mobile_seal_receipts(network, wallet, session_hash);

ALTER TABLE mobile_seal_receipts
    ADD COLUMN IF NOT EXISTS utc_day BIGINT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'mobile_seal_receipts_utc_day_nonnegative'
          AND conrelid = 'mobile_seal_receipts'::regclass
    ) THEN
        ALTER TABLE mobile_seal_receipts
            ADD CONSTRAINT mobile_seal_receipts_utc_day_nonnegative
            CHECK (utc_day IS NULL OR utc_day >= 0);
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS mobile_verified_seal_receipts (
    id TEXT PRIMARY KEY,
    network TEXT NOT NULL DEFAULT 'devnet',
    wallet TEXT NOT NULL,
    session_hash TEXT NOT NULL,
    proof_hash TEXT NOT NULL,
    verifier TEXT NOT NULL,
    protocol_version INTEGER NOT NULL,
    utc_day BIGINT,
    signature TEXT NOT NULL UNIQUE,
    slot BIGINT,
    block_time BIGINT,
    status TEXT NOT NULL DEFAULT 'confirmed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (network, wallet, session_hash),
    CONSTRAINT mobile_verified_seal_receipts_session_hash_hex
        CHECK (session_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT mobile_verified_seal_receipts_proof_hash_hex
        CHECK (proof_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT mobile_verified_seal_receipts_protocol_version
        CHECK (protocol_version = 1),
    CONSTRAINT mobile_verified_seal_receipts_utc_day_nonnegative
        CHECK (utc_day IS NULL OR utc_day >= 0),
    CONSTRAINT mobile_verified_seal_receipts_status
        CHECK (status IN ('confirmed', 'finalized')),
    CONSTRAINT mobile_verified_seal_receipts_matching_seal
        FOREIGN KEY (network, wallet, session_hash)
        REFERENCES mobile_seal_receipts(network, wallet, session_hash)
        ON DELETE CASCADE
);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'mobile_verified_seal_receipts_matching_seal'
          AND conrelid = 'mobile_verified_seal_receipts'::regclass
    ) THEN
        ALTER TABLE mobile_verified_seal_receipts
            ADD CONSTRAINT mobile_verified_seal_receipts_matching_seal
            FOREIGN KEY (network, wallet, session_hash)
            REFERENCES mobile_seal_receipts(network, wallet, session_hash)
            ON DELETE CASCADE;
    END IF;
END
$$;

ALTER TABLE mobile_verified_seal_receipts
    ADD COLUMN IF NOT EXISTS utc_day BIGINT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'mobile_verified_seal_receipts_session_hash_hex'
          AND conrelid = 'mobile_verified_seal_receipts'::regclass
    ) THEN
        ALTER TABLE mobile_verified_seal_receipts
            ADD CONSTRAINT mobile_verified_seal_receipts_session_hash_hex
            CHECK (session_hash ~ '^[0-9a-f]{64}$');
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'mobile_verified_seal_receipts_proof_hash_hex'
          AND conrelid = 'mobile_verified_seal_receipts'::regclass
    ) THEN
        ALTER TABLE mobile_verified_seal_receipts
            ADD CONSTRAINT mobile_verified_seal_receipts_proof_hash_hex
            CHECK (proof_hash ~ '^[0-9a-f]{64}$');
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'mobile_verified_seal_receipts_protocol_version'
          AND conrelid = 'mobile_verified_seal_receipts'::regclass
    ) THEN
        ALTER TABLE mobile_verified_seal_receipts
            ADD CONSTRAINT mobile_verified_seal_receipts_protocol_version
            CHECK (protocol_version = 1);
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'mobile_verified_seal_receipts_utc_day_nonnegative'
          AND conrelid = 'mobile_verified_seal_receipts'::regclass
    ) THEN
        ALTER TABLE mobile_verified_seal_receipts
            ADD CONSTRAINT mobile_verified_seal_receipts_utc_day_nonnegative
            CHECK (utc_day IS NULL OR utc_day >= 0);
    END IF;
END
$$;

ALTER TABLE mobile_verified_seal_receipts
    DROP CONSTRAINT IF EXISTS mobile_verified_seal_receipts_status;

ALTER TABLE mobile_verified_seal_receipts
    ADD CONSTRAINT mobile_verified_seal_receipts_status
    CHECK (status IN ('confirmed', 'finalized'));

CREATE INDEX IF NOT EXISTS idx_mobile_verified_seal_receipts_wallet_created
    ON mobile_verified_seal_receipts(wallet, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_mobile_verified_seal_receipts_hash_created
    ON mobile_verified_seal_receipts(session_hash, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_mobile_verified_seal_receipts_network_wallet_hash_unique
    ON mobile_verified_seal_receipts(network, wallet, session_hash);

CREATE UNIQUE INDEX IF NOT EXISTS idx_mobile_verified_seal_receipts_signature_unique
    ON mobile_verified_seal_receipts(signature);
