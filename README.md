# anky

Anky is a ritual writing system for unedited thought.

Write for 8 minutes without stopping. No backtracking. No polishing. Anky reflects what emerges and remembers your pattern over time.

**Live at [anky.app](https://anky.app)**

## Core idea

Anky is not just "AI writing tooling". It is a space where humans (and agents) cannot hide behind edits.

- 8-minute stream-of-consciousness sessions
- hard anti-edit constraints during session
- periodic checkpoints so writing is never lost
- AI reflection + image generation after completion
- memory-aware trajectory across sessions

## What happens in a full session

1. You write continuously for at least 480 seconds.
2. The client enforces anti-edit flow constraints.
3. Checkpoints are saved every 30 seconds.
4. On completion, pipeline generates reflection/title/image.
5. Session history feeds memory-aware transformation over time.

## Stack

- **Rust / Axum / Tokio** - backend server
- **SQLite (rusqlite)** - persistent storage
- **Tera + vanilla JS + HTMX/SSE** - web UI
- **Claude** - analysis/reflection/prompt shaping
- **Gemini** - image generation
- **Grok path + media pipeline** - video generation flow
- **USDC on Base + x402 support** - paid API/generation flows

## Project map (high level)

```text
src/
  main.rs                 # boot, scheduler/watchdogs, route mounting
  config.rs               # env/config
  state.rs                # shared state
  error.rs                # app error mapping
  db/
    migrations.rs         # schema
    queries.rs            # SQL query layer
  routes/
    pages.rs              # SSR pages
    writing.rs            # /write flow
    api.rs                # JSON API endpoints
    auth.rs               # auth/session handling
    live.rs               # live writing websocket + queue
    prompt.rs             # prompt routes + paid flows
    payment.rs            # payment endpoints
    ...
  pipeline/
    image_gen.rs
    stream_gen.rs
    video_gen.rs
    memory_pipeline.rs
    collection.rs
    cost.rs
  services/
    claude.rs
    gemini.rs
    grok.rs
    ollama.rs
    payment.rs
    stream.rs
  middleware/
    api_auth.rs
    x402.rs
    security_headers.rs
    honeypot.rs

templates/                # server-rendered UI
static/                   # css/js/assets
scripts/                  # local utilities (including GIF generation)
skills.md                 # agent docs (served at /skills)
```

## Key routes

- `/` - landing + writing entry
- `/help` - human/agent/code docs
- `/gallery` - generated ankys
- `/generate` - prompt/thinker generation
- `/generate/video` - video studio flow
- `/video-dashboard` - media dashboard
- `/poiesis` - live system console
- `/dashboard` - log streaming UI
- `/skills` - agent protocol docs
- `/skill.md` - redirect to `/skills`

## Key API endpoints

```text
POST /write
POST /api/checkpoint
GET  /api/v1/anky/{id}
POST /api/v1/generate
POST /api/v1/register
GET  /api/cost-estimate
GET  /api/treasury
GET  /health
```

See full agent/API docs in [`skills.md`](skills.md) or live at `/skills`.

## Running locally

```bash
git clone https://github.com/jpfraneto/anky-monorepo.git
cd anky-monorepo

cp .env.example .env
# fill required vars

cargo run
# http://localhost:8889
```

## Required environment (minimum)

- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`
- `TREASURY_ADDRESS`

Useful optional variables:

- `PORT` (default `8889`)
- `BASE_RPC_URL`
- `X402_FACILITATOR_URL`
- `OLLAMA_BASE_URL`
- `OLLAMA_MODEL`
- `PUMPFUN_RTMP_URL` / `PUMPFUN_STREAM_KEY` (if live relay path is used)

## Landing media utility

A local static ffmpeg build is used in this repo to generate compressed landing GIFs from scene clips.

```bash
./scripts/generate_landing_gifs.sh 48
```

- Output: `data/images/landing_gifs`
- Current settings: max 3s clips, aggressively compressed for fast page load

## For agents

- Canonical operating docs: `/skill.md` and `/skills`
- API docs: `skills.md`
- System intent: optimize for human transformation and continuity, not just throughput

## License

MIT
