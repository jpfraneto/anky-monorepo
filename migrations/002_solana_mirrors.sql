-- Sojourn 9: Solana Bubblegum cNFT minting columns + sojourn state

-- Add Solana minting columns to mirrors
ALTER TABLE mirrors ADD COLUMN solana_mint_tx TEXT;
ALTER TABLE mirrors ADD COLUMN solana_recipient TEXT;
ALTER TABLE mirrors ADD COLUMN solana_asset_id TEXT;
ALTER TABLE mirrors ADD COLUMN solana_minted_at TEXT;
ALTER TABLE mirrors ADD COLUMN kingdom INTEGER;
ALTER TABLE mirrors ADD COLUMN kingdom_name TEXT;
ALTER TABLE mirrors ADD COLUMN mirror_type TEXT NOT NULL DEFAULT 'public';
ALTER TABLE mirrors ADD COLUMN user_id TEXT;

-- Track sojourn-level state (singleton row)
CREATE TABLE IF NOT EXISTS sojourn_state (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    sojourn_number INTEGER NOT NULL DEFAULT 9,
    max_supply INTEGER NOT NULL DEFAULT 3456,
    merkle_tree TEXT,
    collection_mint TEXT,
    started_at TEXT
);

INSERT INTO sojourn_state (sojourn_number, max_supply)
VALUES (9, 3456)
ON CONFLICT DO NOTHING;

-- Indexes for duplicate checking across both mint surfaces
CREATE INDEX IF NOT EXISTS idx_mirrors_solana_minted
    ON mirrors(fid) WHERE solana_mint_tx IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_mirrors_user_minted
    ON mirrors(user_id) WHERE solana_mint_tx IS NOT NULL AND user_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_mirrors_solana_recipient
    ON mirrors(solana_recipient) WHERE solana_mint_tx IS NOT NULL;
