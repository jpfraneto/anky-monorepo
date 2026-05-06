-- Dedupe Helius webhook retries by public Solana transaction signature.
-- Payloads without a valid signature still fall back to migration 021's
-- network/payload_hash uniqueness.

CREATE UNIQUE INDEX IF NOT EXISTS idx_mobile_helius_webhook_events_network_signature_unique
    ON mobile_helius_webhook_events(network, signature)
    WHERE signature IS NOT NULL;
