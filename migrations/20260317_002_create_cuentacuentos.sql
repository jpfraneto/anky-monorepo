BEGIN;

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS cuentacuentos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    writing_id UUID NOT NULL REFERENCES writing_sessions(id),
    parent_wallet_address TEXT NOT NULL,
    child_wallet_address TEXT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    guidance_phases JSONB NOT NULL,
    played BOOLEAN NOT NULL DEFAULT false,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT cuentacuentos_guidance_phases_is_array CHECK (
        jsonb_typeof(guidance_phases) = 'array'
    )
);

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_parent_ready
    ON cuentacuentos(parent_wallet_address, played, generated_at);

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_parent_child_ready
    ON cuentacuentos(parent_wallet_address, child_wallet_address, played, generated_at);

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_writing_id
    ON cuentacuentos(writing_id);

COMMIT;
