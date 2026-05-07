-- Recovery states for mobile proof jobs.
-- These states mean public chain evidence suggests the proof landed, but the
-- backend/indexer has not finished writing finalized receipt metadata yet.

ALTER TABLE mobile_proof_jobs
    DROP CONSTRAINT IF EXISTS mobile_proof_jobs_status;

ALTER TABLE mobile_proof_jobs
    ADD CONSTRAINT mobile_proof_jobs_status
    CHECK (status IN (
        'queued',
        'proving',
        'syncing',
        'backfill_required',
        'finalized',
        'failed',
        'unavailable'
    ));
