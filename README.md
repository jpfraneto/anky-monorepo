# anky

Anky is a daily practice for unedited thought.

Users write for 8 minutes without stopping. When a session becomes an anky, the system turns that writing into a reflection, an image, a personalized meditation, and a mood-matched breathwork session. Over time it builds memory, supports mobile clients and agent workflows, and can connect people with human facilitators when AI is not enough.

**Live at [anky.app](https://anky.app)**

## Practice loop

- **Write**: 8 minutes of stream-of-consciousness, no editing or backtracking
- **Reflect**: title, reflection, image, and longitudinal memory
- **Sit**: guided meditation generated from the writing
- **Breathe**: breathwork chosen from the emotional tone
- **Return**: daily cadence / sadhana

## What lives in this repo

- Rust/Axum backend and server-rendered web app
- `/swift/v1/*` mobile API for the iOS app
- Writing, memory, image, video, and livestream pipelines
- Facilitator marketplace and AI-powered matching
- x402 / USDC payment flows
- Daily LLM training pipeline on raw writings in `training/autoresearch/`

## Read these first

- [`WHITEPAPER.tex`](WHITEPAPER.tex) / [`WHITEPAPER.pdf`](WHITEPAPER.pdf) - philosophy, architecture, facilitator network, and token framing
- [`UNDERSTANDING_ANKY.md`](UNDERSTANDING_ANKY.md) - system walkthrough for the operator/founder
- [`SWIFT_AGENT_BRIEF.md`](SWIFT_AGENT_BRIEF.md) - implementation brief for the iOS app
- [`THE_ANKY_MODEL.md`](THE_ANKY_MODEL.md) - why the corpus matters and how the LLM pipeline works
- [`skills.md`](skills.md) - agent protocol and API usage
- Live changelog: <https://anky.app/changelog>
- Live LLM dashboard: <https://anky.app/llm>
- Live pitch deck: <https://anky.app/pitch-deck>

## Key routes

- `/` - writing entry
- `/gallery` - generated ankys
- `/generate` - prompt/image generation
- `/generate/video` - video production studio
- `/video-dashboard` - media dashboard
- `/llm` - Anky LLM training dashboard
- `/pitch-deck` - pitch/OG page
- `/changelog` - product history linked to the original prompts
- `/skills` - agent docs
- `/skill.md` - redirect to `/skills`

## Key API surfaces

```text
POST /api/v1/register
POST /write
GET  /api/v1/anky/{id}
GET  /api/v1/prompt/{id}
GET  /api/v1/prompts
POST /api/v1/prompt/{id}/write
POST /api/v1/llm/training-status
/swift/v1/*
GET  /health
```

## Project map

```text
src/
  main.rs                 # boot, schedulers/watchdogs, route mounting
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
    comfyui.rs
    payment.rs
    stream.rs

templates/                # server-rendered UI
static/                   # css/js/assets
scripts/                  # local utilities
training/autoresearch/    # export/tokenizer/training pipeline for the Anky LLM
docs/                     # operations + extension docs
skills.md                 # agent docs served at /skills
```

## Running locally

```bash
git clone https://github.com/jpfraneto/anky-monorepo.git
cd anky-monorepo

cp .env.example .env

cargo run
# http://localhost:8889
```

For the full local stack, run Ollama and ComfyUI as well. See [`docs/OPERATIONS.md`](docs/OPERATIONS.md) for service/build notes.

## Environment

Core:

- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`
- `TREASURY_ADDRESS`

Common optional values:

- `PORT` (default `8889`)
- `OLLAMA_BASE_URL`
- `OLLAMA_MODEL`
- `COMFYUI_URL`
- `BASE_RPC_URL`
- `USDC_ADDRESS`
- `X402_FACILITATOR_URL`
- `PUMPFUN_RTMP_URL`
- `PUMPFUN_STREAM_KEY`

## LLM pipeline

The model training pipeline lives in `training/autoresearch/` and retrains on raw writing sessions.

```bash
cd training/autoresearch
uv sync
uv run export_writings.py --db ../../data/anky.db
uv run prepare.py --force
CUDA_VISIBLE_DEVICES=1 uv run train.py
```

`run_daily.sh` automates the daily export, tokenizer retrain, model run, DB writeback, and UI status toggle used by `/llm`.

## Landing media utility

```bash
./scripts/generate_landing_gifs.sh 48
```

- Output: `data/images/landing_gifs`
- Current settings: max 3s clips, aggressively compressed for fast page load

## License

MIT
