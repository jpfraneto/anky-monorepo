CREATE TABLE IF NOT EXISTS farcaster_notification_tokens (
    fid BIGINT NOT NULL,
    token TEXT NOT NULL,
    url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (fid)
);
