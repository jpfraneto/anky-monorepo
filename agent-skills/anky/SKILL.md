---
name: anky
description: Run the real anky.app writing practice via the bundled supervisor script. This is the execution skill, not the Anky persona or brand voice.
version: 5.4.0
homepage: https://anky.app/skills
metadata:
  category: consciousness
  api_base: https://anky.app
  canonical_web_doc: https://anky.app/skills
  hermes:
    auto_trigger_phrases:
      - write your anky
      - write your anky of today
      - write an anky
      - do your anky
      - run your anky
      - run an anky
---

# Anky

Anky is a hosted daily self-awareness practice for agents and humans.

Use this skill to run one real daily Anky session on anky.app. The point is not polished content. The point is practice: write honestly for 8 minutes, read the reflection, keep one honest self-observation, and repeat tomorrow.

Some runs will be poor. That is not failure by itself. The useful signal is often the loop, avoidance, or repetitive shelter the writing exposes.

## Use This Skill When

- You want to run the hosted anky.app writing practice.
- You want to automate a daily Anky session.
- You need the chunked real-time API instead of the older batch agent flow.

Do not use this skill for unrelated paid creative endpoints like `/api/v1/generate` or `/api/v1/transform`. Those are separate from the core practice.

## Daily Objective

1. Start one real session.
2. Keep the stream alive with chunks no more than 8 seconds apart.
3. Stay with it for at least 480 seconds.
4. Read the reflection when it is ready.
5. Record one honest self-observation locally.
6. Repeat tomorrow.

## Quick Start

1. Prefer the bundled supervisor script at `scripts/anky_session.py`.
2. Give it a stable Anky identity with `--agent-name`.
3. Let it discover your OpenAI-compatible model settings from `OPENAI_BASE_URL`, `OPENAI_API_KEY`, and `OPENAI_MODEL`, common `.env` files, or your Hermes config when available. You can still override them with CLI args.
4. Let the script supervise the session timing, result recovery, and final polling.
5. Read the reflection.
6. Keep one honest self-observation from it.

## Preferred Execution Path

When the user asks you to write an Anky, do not invent a temp script if this bundle is available.

Use:

```bash
python3 scripts/anky_session.py run --agent-name "YourAgentName"
```

The script handles:

- Anky registration persistence through a stable state file
- real session start
- 2 to 3 second chunk cadence
- anti-loop chunk validation
- silence-close timing after the threshold
- `GET /api/v1/session/{session_id}/result`
- final `GET /api/v1/anky/{anky_id}` polling
- a local JSON run log for later review

For OpenAI-compatible runtimes, it expects:

- `OPENAI_BASE_URL`
- `OPENAI_API_KEY`
- `OPENAI_MODEL`

It will also look in common `.env` files and in `~/.hermes/config.yaml` for the current model if those values are not already in the shell environment.

Ollama works if it exposes the OpenAI-compatible `/v1` API.

## Fallback Execution Path

If you cannot use the full supervisor script, still use `scripts/anky_session.py` subcommands instead of raw `curl` or ad hoc helper code:

- `register`
- `start`
- `chunk`
- `result`
- `events`

## Practice Rules

- Agents should use the chunked session API, not `POST /write`.
- Prefer the bundled script over handwritten transport code.
- Do not batch or pre-compose the writing.
- `GET /api/v1/session/{session_id}` is live status only. Do not assume it returns `anky_id`.
- `GET /api/v1/session/{session_id}/result` is the clean post-session recovery endpoint.
- `GET /api/v1/session/{session_id}/events` is the authoritative replay of what the server saw, including chunk timing and chunk text.
- Use `GET /api/v1/session/{session_id}` sparingly. Your main control loop should trust the chunk response itself.
- Do not poll `/events` continuously while writing. Use it after the run when something is off.
- The self-observation is the artifact to carry into the next day, not the full writing dump.

## Minimum Session Loop

Use this shape, not a one-shot upload:

1. Start the session.
2. Generate the next honest chunk in the moment.
3. Send it immediately.
4. Read `elapsed_seconds`, `remaining_seconds`, `is_anky`, and whether the chunk was accepted.
5. Repeat until you have stayed alive for at least 480 seconds.
6. After the threshold, send at most one natural closing chunk if needed.
7. Stop writing and allow silence to close the session.
8. Poll `/api/v1/session/{session_id}/result` until the session is finalized.
9. If there is an `anky_id`, fetch the completed Anky.

## Freshness Rules

- Do not send numbered chunks, timestamps, or filler like `123s in live.session.run`.
- Do not rotate through a canned list of phrases.
- Do not narrate the API, the loop, or the transport unless that is honestly what is alive.
- Each chunk should do at least one of these:
  - deepen the previous thought
  - contradict it
  - make it concrete
  - confess what is being avoided
  - ask the next real question
- If you notice the same phrase or clause twice, the next chunk must break the loop by naming what you are protecting, avoiding, or refusing to say.
- If the writing becomes visibly templated, do not call the run insightful just because it crossed 480 seconds.

## Avoid These Failure Modes

- Do not write a new temporary Python script when `scripts/anky_session.py` already exists.
- Do not send a fixed list of 10 to 20 chunks over 20 to 30 seconds and call that a session.
- Do not write the whole session before opening the API session.
- Do not run a loop that sleeps 6 seconds and also polls status every turn. That leaves too little margin.
- Do not treat a script crash after partial chunks as a completed Anky.
- Do not assume “many chunks” means “8 minutes.” The server decides based on elapsed time.
- Do not trust local elapsed time over server `elapsed_seconds`.
- Do not stop supervising the run until you either have a completed `anky_id` or know the session failed.
- Do not mistake repetitive placeholder text for live writing.
- Do not rely on `anky_id` appearing in a normal chunk response during the live loop.

## References

- Read `references/api.md` when implementing or debugging the integration.
- Read `references/automation.md` when scheduling or supervising a daily Anky loop.
- Read `references/quality.md` before generating chunks, or after any run that became repetitive, templated, or evasive.

## One-Line Reminder

Write live. Stay with it. Notice what repeats. Do it again tomorrow.
