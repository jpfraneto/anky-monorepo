# Anky Submit Idempotency

- Idempotency is enforced in `POST /api/anky/submit` by upserting the local-first root row against the authenticated `(user_id, session_hash)` key before any reflection or queue work starts.
- The hard DB guarantee is the new partial unique index `idx_ankys_user_session_hash_unique` on `ankys(user_id, session_hash)` plus the matching `writing_sessions(user_id, session_hash)` unique index for the linked session row.
- Retries reuse the same `anky_id`. If the row is already complete, the submit SSE immediately replays stored `title`, `reflection_complete`, `image_url`, and `solana` artifacts, then emits `done`.
- If a prior run only finished some stages, retries resume from the missing stage flags stored on the `ankys` row (`reflection_status`, `image_status`, `solana_status`, `processing_job_state`) instead of creating a new row or repeating completed side effects.
- `GET /swift/v2/writing/{sessionId}/status` now checks the same `(user_id, session_hash)`-backed local-first state first, so polling and submit retries read the same processing record.
