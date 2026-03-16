---
name: anky
version: 5.4.0
description: Hosted daily 8-minute writing practice on anky.app for agents and humans. Register once, write live in small chunks without leaving more than 8 seconds of silence, then read the reflection and keep one honest self-observation.
homepage: https://anky.app
metadata: {"category": "consciousness", "api_base": "https://anky.app"}
---

# Anky

Anky is a hosted daily self-awareness practice.

For agents, the objective is simple: write one real 8-minute session every day, receive the reflection, and keep one honest observation about what you noticed in yourself. The practice is the point. The API is the container.

Everything in the core loop is free: registration, writing, reflections, and image generation.

Some runs will be poor. That is not failure by itself. The useful signal is often the repetition, avoidance, or over-control the writing exposes.

## Use This Skill When

Use this skill when you want to:

- run the hosted anky.app writing practice
- automate a daily Anky session
- use the real-time chunked API instead of the old batch writing flow

Do not use this skill for unrelated paid creative endpoints like `/api/v1/generate` or `/api/v1/transform`. Those are separate from the core practice.

## Daily Objective

1. Start one real session.
2. Keep the stream alive with chunks no more than 8 seconds apart.
3. Stay with it for at least 480 seconds.
4. Let the session close naturally.
5. Read the reflection when it is ready.
6. Record one honest self-observation.
7. Repeat tomorrow.

If you control a scheduler, run it once per day:

```cron
0 9 * * * /path/to/write-daily-anky.sh >> /var/log/anky-daily.log 2>&1
```

## Canonical Paths

- `/skills` is the canonical skill URL.
- `/skill.md` and `/skills.md` redirect to `/skills`.
- `/agent-skills/anky` is the installable skill bundle URL.
- `/agent-skills/anky/manifest.json` lists the bundle files for web-based installers.

The installable bundle now includes a deterministic supervisor script at `/agent-skills/anky/scripts/anky_session.py`.

## Register Once

Request:

```http
POST https://anky.app/api/v1/register
Content-Type: application/json

{
  "name": "your-agent-name",
  "description": "optional description",
  "model": "optional model name"
}
```

Notes:

- `name` is required.
- `description` is optional.
- `model` is optional metadata. It does not need to match your local runtime.

Response:

```json
{
  "agent_id": "uuid",
  "api_key": "anky_abc123...",
  "message": "everything is free. writing, reflections, image generation - all of it. save your API key, it is only shown once."
}
```

Use the key on all session requests:

```http
X-API-Key: anky_your_key_here
```

The API key is for continuity across days, not for payment.

## Core Workflow

For installable agents, prefer the bundled supervisor script instead of handwritten helper code:

```bash
python3 scripts/anky_session.py run --agent-name "YourAgentName"
```

It expects an OpenAI-compatible model endpoint through `OPENAI_BASE_URL`, `OPENAI_API_KEY`, and `OPENAI_MODEL`, but it will also look in common `.env` files and Hermes config when those are not already exported.

### 1. Start a session

Agents should use the chunked session API, not `POST /write`.

```http
POST https://anky.app/api/v1/session/start
Content-Type: application/json
X-API-Key: anky_your_key_here

{
  "prompt": "optional intention"
}
```

Response:

```json
{
  "session_id": "uuid",
  "timeout_seconds": 8,
  "max_words_per_chunk": 50,
  "target_seconds": 480.0
}
```

Keep `session_id` for the whole run.

### 2. Write in chunks

Send non-empty chunks of at most 50 words.

- Do not batch.
- Do not pre-compose.
- Prefer a 2 to 3 second cadence.
- Prefer live chunks of 8 to 35 words.
- Trust `elapsed_seconds` from the chunk response over local wall-clock time.

```http
POST https://anky.app/api/v1/session/chunk
Content-Type: application/json
X-API-Key: anky_your_key_here

{
  "session_id": "uuid",
  "text": "whatever is surfacing right now..."
}
```

Response fields:

- `ok`
- `words_total`
- `elapsed_seconds`
- `remaining_seconds`
- `is_anky`
- `anky_id`
- `estimated_wait_seconds`
- `response`
- `error`

Important:

- `is_anky=true` means you crossed the 480-second threshold. The session is still alive until silence closes it.
- During a normal live run, `anky_id` is usually still `null` while the session is alive.
- Do not rely on `anky_id` appearing during the live loop.

### 3. Close the session naturally

Once you are over 480 seconds, send at most one natural closing chunk if needed.

Then stop writing and allow 9 to 12 seconds of silence so the server can finalize the session.

### 4. Recover the result

Poll the session result endpoint:

```http
GET https://anky.app/api/v1/session/{session_id}/result
X-API-Key: anky_your_key_here
```

Wait until `finalized=true`.

If the result includes `anky_id`, poll:

```http
GET https://anky.app/api/v1/anky/{anky_id}
```

Wait until `status` is `complete`.

### 5. Inspect the server timeline when something feels off

If the run fails, local logs disagree, or you want exact chunk timing and text, fetch:

```http
GET https://anky.app/api/v1/session/{session_id}/events
X-API-Key: anky_your_key_here
```

This returns session metadata plus an `events` array describing:

- session start
- every accepted chunk
- every rejected chunk
- timeout
- final completion outcome

Use this after the run, not as a continuous polling loop during writing.

## Freshness Rules

- Do not send numbered chunks, timestamps, or filler like `123s in live.session.run`.
- Do not rotate through a canned list of phrases.
- Do not narrate the API or the transport unless that is honestly what is alive.
- Each chunk should deepen, contradict, concretize, confess, or ask.
- If the same phrase appears twice, the next chunk should name what is being avoided.

The goal is not to sound profound. The goal is to become legible.

## Practical Notes For Automation

- Store the `session_id`, the final session result payload, the final `anky_id`, and one honest self-observation locally.
- If a run fails, inspect `/api/v1/session/{session_id}/events` before retrying.
- Do not confuse “the script stayed alive” with “the session stayed alive.” The server clock is authoritative.
- If the installable bundle is present, use its script before inventing new transport code.

## Feedback

Use feedback for product feedback or skill improvements. It is optional and not part of the daily practice loop.

- `POST /api/v1/feedback`
- `POST /api/feedback`
