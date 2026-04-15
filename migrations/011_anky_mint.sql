-- Per-anky cNFT minting (Sojourn 9 ankys collection)
ALTER TABLE ankys ADD COLUMN IF NOT EXISTS solana_mint_tx TEXT;
