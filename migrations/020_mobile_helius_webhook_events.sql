-- Public Helius webhook delivery receipts for Sojourn 9 seal indexing.
-- This stores public enhanced webhook payloads only. It must never store .anky
-- plaintext, SP1 witness bytes, or private proof inputs.

CREATE TABLE IF NOT EXISTS mobile_helius_webhook_events (
    id TEXT PRIMARY KEY,
    network TEXT NOT NULL DEFAULT 'devnet',
    source TEXT NOT NULL DEFAULT 'helius_enhanced_webhook',
    payload_hash TEXT NOT NULL,
    signature TEXT,
    event_count INTEGER NOT NULL DEFAULT 1,
    payload_json TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (network, payload_hash),
    CONSTRAINT mobile_helius_webhook_events_payload_hash_hex
        CHECK (payload_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT mobile_helius_webhook_events_event_count_positive
        CHECK (event_count > 0),
    CONSTRAINT mobile_helius_webhook_events_source
        CHECK (source IN ('helius_enhanced_webhook'))
);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'mobile_helius_webhook_events_payload_hash_hex'
          AND conrelid = 'mobile_helius_webhook_events'::regclass
    ) THEN
        ALTER TABLE mobile_helius_webhook_events
            ADD CONSTRAINT mobile_helius_webhook_events_payload_hash_hex
            CHECK (payload_hash ~ '^[0-9a-f]{64}$');
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'mobile_helius_webhook_events_event_count_positive'
          AND conrelid = 'mobile_helius_webhook_events'::regclass
    ) THEN
        ALTER TABLE mobile_helius_webhook_events
            ADD CONSTRAINT mobile_helius_webhook_events_event_count_positive
            CHECK (event_count > 0);
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'mobile_helius_webhook_events_source'
          AND conrelid = 'mobile_helius_webhook_events'::regclass
    ) THEN
        ALTER TABLE mobile_helius_webhook_events
            ADD CONSTRAINT mobile_helius_webhook_events_source
            CHECK (source IN ('helius_enhanced_webhook'));
    END IF;
END
$$;

CREATE UNIQUE INDEX IF NOT EXISTS idx_mobile_helius_webhook_events_network_payload_hash_unique
    ON mobile_helius_webhook_events(network, payload_hash);

CREATE INDEX IF NOT EXISTS idx_mobile_helius_webhook_events_signature_created
    ON mobile_helius_webhook_events(signature, created_at DESC);
