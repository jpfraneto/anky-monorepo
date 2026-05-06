-- Legacy direct native mobile credit purchases.
--
-- The current Anky mobile credits flow uses RevenueCat CREDITS instead of this
-- direct verifier. Keep this ledger only for deployments that may have already
-- applied the migration while the direct-IAP implementation existed.

CREATE TABLE IF NOT EXISTS mobile_credit_purchases (
    id TEXT PRIMARY KEY,
    identity_id TEXT NOT NULL REFERENCES mobile_credit_accounts(identity_id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    app_product_id TEXT NOT NULL,
    package_id TEXT NOT NULL,
    purchase_key TEXT NOT NULL,
    credits_granted INTEGER NOT NULL CHECK (credits_granted > 0),
    verification_status TEXT NOT NULL DEFAULT 'verified',
    raw_receipt_json TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(platform, purchase_key)
);

CREATE INDEX IF NOT EXISTS idx_mobile_credit_purchases_identity_created
    ON mobile_credit_purchases(identity_id, created_at DESC);
