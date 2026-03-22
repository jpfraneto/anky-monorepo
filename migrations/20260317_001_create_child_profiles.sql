BEGIN;

CREATE EXTENSION IF NOT EXISTS pgcrypto;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM users
        WHERE wallet_address IS NOT NULL
        GROUP BY wallet_address
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'cannot add unique constraint on users.wallet_address: duplicate wallet addresses exist';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'users_wallet_address_unique'
          AND conrelid = 'users'::regclass
    ) THEN
        ALTER TABLE users
            ADD CONSTRAINT users_wallet_address_unique UNIQUE (wallet_address);
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS child_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_wallet_address TEXT NOT NULL REFERENCES users(wallet_address),
    derived_wallet_address TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    birthdate DATE NOT NULL,
    emoji_pattern JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT child_profiles_emoji_pattern_shape CHECK (
        jsonb_typeof(emoji_pattern) = 'array'
        AND jsonb_array_length(emoji_pattern) = 12
    )
);

CREATE INDEX IF NOT EXISTS idx_child_profiles_parent_wallet_address
    ON child_profiles(parent_wallet_address);

COMMIT;
