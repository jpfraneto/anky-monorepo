-- Hosted Solana wallets for Farcaster miniapp users
CREATE TABLE IF NOT EXISTS farcaster_wallets (
    fid BIGINT PRIMARY KEY,
    solana_address TEXT NOT NULL UNIQUE,
    encrypted_keypair BYTEA NOT NULL,
    kingdom_id INTEGER,
    kingdom_name TEXT,
    onboarded BOOLEAN NOT NULL DEFAULT FALSE,
    mint_tx TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    onboarded_at TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_farcaster_wallets_address ON farcaster_wallets(solana_address);
