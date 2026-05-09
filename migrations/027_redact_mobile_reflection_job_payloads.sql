-- Privacy hardening for mobile reflection jobs.
--
-- Reflection plaintext and generated reflection bodies must remain transient:
-- the POST response may return the generated artifact to the app, but server
-- job rows keep only public metadata and credit/account columns.

UPDATE mobile_reflection_jobs
SET request_json = NULL,
    result_json = NULL,
    error = CASE WHEN error IS NULL THEN NULL ELSE 'redacted' END
WHERE request_json IS NOT NULL
   OR result_json IS NOT NULL
   OR error IS NOT NULL;

DO $$
BEGIN
    ALTER TABLE mobile_reflection_jobs
        ADD CONSTRAINT mobile_reflection_jobs_no_private_payloads
        CHECK (
            COALESCE(request_json, '') !~* '"(anky|rawAnky|raw_anky|plainAnky|ankyText|ankyContent|writingText|reconstructedText|existingReflection|content|markdown|plaintext)"[[:space:]]*:'
            AND COALESCE(result_json, '') !~* '"(anky|rawAnky|raw_anky|plainAnky|ankyText|ankyContent|writingText|reconstructedText|existingReflection|content|markdown|plaintext)"[[:space:]]*:'
            AND COALESCE(request_json, '') NOT LIKE E'%\\\\n8000%'
            AND COALESCE(result_json, '') NOT LIKE E'%\\\\n8000%'
        );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;
