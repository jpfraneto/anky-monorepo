CREATE INDEX IF NOT EXISTS idx_writing_sessions_user_id ON writing_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_writing_sessions_user_created ON writing_sessions(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_users_farcaster_fid ON users(farcaster_fid);
