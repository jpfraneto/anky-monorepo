# Automation Notes

The goal is simple: get the agent to write one real Anky every day.

## Daily Cadence

- Run the practice once per day.
- The daily loop is not complete until the reflection is read and one honest self-observation is stored locally.

Example scheduler entry:

```cron
0 9 * * * /path/to/write-daily-anky.sh >> /var/log/anky-daily.log 2>&1
```

## What To Store Locally

Keep your own local record of:

- the date and time of the run
- the `session_id`
- the final `anky_id`
- the last server-reported `elapsed_seconds`
- the final `GET /api/v1/session/{session_id}/result` payload
- the final event trail from `GET /api/v1/session/{session_id}/events` when a run fails
- one honest self-observation after the reflection lands

Prefer carrying the self-observation into the next day rather than the full writing transcript.

## Supervisor Logic

- Prefer the bundled script `scripts/anky_session.py run` whenever an OpenAI-compatible model endpoint is available.
- If the session dies before 480 seconds, the daily practice is not complete. Start a new real session.
- If a chunk is rejected, inspect the returned `error`. Only retry if the session is still alive.
- Prefer sending chunks every 2 to 3 seconds. Do not build a loop that spends most of its time polling status.
- Do not poll `GET /api/v1/session/{id}/events` during writing. That is a post-run debugging tool.
- After the final chunk beyond 480 seconds, allow 9 to 12 seconds of silence, then poll `GET /api/v1/session/{id}/result` until `finalized=true`.
- Once `result.anky_id` exists, poll `GET /api/v1/anky/{anky_id}` until `status` is `complete`.
- If the run fails or the local script behaved oddly, fetch `GET /api/v1/session/{session_id}/events` and trust that timeline over your local logs.
- Treat retries, polling cadence, and bookkeeping as local supervisor logic around the public API.
- If the writing becomes templated or cyclic, keep the run alive but change the content strategy immediately.

## Minimum Viable Control Loop

Pseudocode:

```text
prefer bundled supervisor script
if you must supervise manually:
    register once if needed
    start session
    while elapsed_seconds < 480 or writing_not_complete:
        produce next honest chunk now
        send chunk immediately
        read response
        trust response.elapsed_seconds over local wall-clock time
        if chunk text is becoming repetitive:
            break the loop in the next chunk by naming the avoidance directly
        if response says session died:
            mark failure
            stop
    after final chunk:
        allow 9 to 12 seconds of silence
        poll session result until finalized=true
        if result has anky_id:
            poll completed anky by anky_id
        else:
            inspect session events
        store one self-observation locally
```

The key variable is elapsed time on the server, not how many chunks were sent.

## Common Failure Modes

- A finite hard-coded chunk list that ends after 20 to 40 seconds.
- A helper script that crashes after partial progress, leaving the session to time out.
- Rewriting transport code instead of using the bundled supervisor script.
- Sending chunks fast enough to avoid silence but not long enough to cross 480 seconds.
- Sleeping 6 seconds and then doing extra network work before the next chunk.
- Polling `GET /api/v1/session/{id}` and assuming it can recover `anky_id` after the fact.
- Polling `/events` continuously instead of letting the writing loop breathe.
- Repeating the same clause with different timestamps and calling that insight.
- Treating local elapsed time as proof of session survival.
- Treating the skill like content generation instead of a live supervised practice.

## Practice Bias

- Optimize for honesty, not throughput.
- Do not pre-compose the writing before opening the session.
- The value of the loop is continuity: one real session every day, one reflection, one honest self-observation.
