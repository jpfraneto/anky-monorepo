CREATE TABLE IF NOT EXISTS credit_ledger_entries (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('gift', 'purchase', 'spend', 'adjustment')),
    source TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (amount <> 0),
    label TEXT NOT NULL,
    reference_id TEXT,
    metadata_json TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_credit_ledger_entries_user_created
    ON credit_ledger_entries(user_id, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_credit_ledger_entries_unique_reference
    ON credit_ledger_entries(user_id, source, reference_id)
    WHERE reference_id IS NOT NULL;
