# Anky Backend — Agent Operating Manual

## Shared Memory

Before starting any session, read `CURRENT_STATE.md` in full.
It is the authoritative record of what is working, what is broken, and what is deferred.
Update it at the end of every session that makes meaningful changes.
Do not rely on conversation history.
`CURRENT_STATE.md` is the truth.

## What Anky Is

Anky is a writing-practice backend that is now explicitly pivoting into a parent/child system: a parent writes an honest 8-minute anky, the backend turns that writing into reflection, image, meditation, breathwork, and an asynchronous Spanish cuentacuentos that can be assigned into a child's derived identity. The point is not social posting or content volume. The point is to transmute a parent's inner life into artifacts that help both parent and child feel less alone, while keeping unfinished work private and custody local.

## Architecture

- Backend is Rust with Axum, plus server-rendered HTML templates.
- Database is SQLite at `data/anky.db`, accessed through the existing `rusqlite` + `AppState` pattern.
- Production runs as a single systemd user service (`anky.service`) on the bare-metal machine `poiesis`, listening on port `8889`.
- Public traffic reaches the server through `cloudflared-anky.service`: `anky.app` -> Cloudflare tunnel -> localhost:8889.
- Current mobile split: `/swift/v2/*` is the active Base/EVM seed-identity path for the parent/child system; `/swift/v1/*` remains the legacy/older mobile surface.
- Text inference follows a local-first fallback chain: Mind (llama-server/qwen3.5-27b at `MIND_URL`) → Claude Haiku (cloud) → OpenRouter (last resort). Ollama is gone.
- The Mind is GPU 0: llama-server serving qwen3.5-27b-q4_k_m via OpenAI-compatible API at `http://127.0.0.1:8080`. It's a thinking model — `<think>` blocks are stripped from responses.
- ComfyUI is the local Flux image layer (GPU 1, the Heart). The live implementation is in `src/services/comfyui.rs` at localhost `8188`.
- Redis/Valkey at `REDIS_URL` handles job persistence for crash recovery.
- `poiesis` is the operational center: Rust server, SQLite, llama-server, ComfyUI, Redis, and the surrounding worker loops all assume that machine-local deployment model.

## Product Stance

- The writing session is sacred. Never modify anything that touches the write path without explicit instruction.
- Privacy is structural. The backend never sees seedphrases. It only sees derived wallet addresses and signed challenges.
- The child's world is derived from the parent's identity. `child_profiles.derived_wallet_address` is always computed on-device, never on the server.
- Cuentacuentos are generated async. Never block the write response waiting for story or image generation.

## Engineering Stance

- Follow existing patterns. Before adding a new abstraction, check if the pattern already exists in this repo.
- SQLite not Postgres. All queries use the existing `rusqlite` + `AppState` pattern.
- The live schema source of truth is `src/db/migrations.rs`, not stray SQL files.
- The live ComfyUI integration is `src/services/comfyui.rs`. Do not build against a nonexistent `src/pipeline/comfyui.rs`.
- `cargo fmt` and `cargo check` must pass before any session ends.
- Never hardcode credentials. Read from environment variables or config struct.
- **Documentation is first-class.** Always update `CURRENT_STATE.md`, `CLAUDE.md`, and changelog when making changes. These files are how sessions stay coherent. Do not skip documentation updates.

## Programming Classes Protocol

**MANDATORY**: Every coding session that results in code changes MUST produce a programming class. This is as non-negotiable as the changelog. Do it at the end of every session, right before build + deploy.

Each class teaches ONE core programming concept through actual code from this repo. The audience is learning about AI and how it works, from the inner core to the outer core. Classes live at `https://anky.app/classes/{number}`.

1. **Pick the ONE concept** this session best illustrates. Examples: intent classification, webhook architectures, LLM structured output, database migrations, async task spawning, template rendering, etc.

2. **Build 8 slides** as JSON objects, each with:
   - `heading`: short title for the slide (the concept point)
   - `body`: 1-3 sentences explaining the concept in simple terms
   - `code`: actual code from the anky repo (not pseudocode) — the real thing that was written or modified
   - `file`: the source file path (e.g. `src/services/claude.rs`)
   - `note`: optional teaching note or "try this" prompt

3. **Structure**: Slide 1 = what we're learning and why. Slides 2-7 = the concept broken down through real code. Slide 8 = recap + connection to the bigger AI picture.

4. **Insert via API**: `POST /api/v1/classes/generate` with `{"title":"...","concept":"...","slides":[...]}`.

5. Classes are numbered sequentially. The Ankyverse calendar (96-day sojourns) will overlay class numbers once the mapping is defined.

## Changelog Protocol

Every conversation that results in code changes MUST update the changelog:

1. **Save each user prompt** as a txt file in `static/changelog/` with the naming convention:
   `YYYY-MM-DD-NNN-slug.txt` (e.g. `2026-02-14-001-video-studio.txt`)
   - NNN is a zero-padded sequence number per day (001, 002, 003...)
   - slug is a short kebab-case description
   - File contains the user's raw prompt text, exactly as they wrote it

2. **Add an entry to `templates/changelog.html`** at the TOP of the entries list (newest first), using this format:
   ```html
   <div class="changelog-entry" id="YYYY-MM-DD-slug">
     <div class="changelog-date">YYYY-MM-DD</div>
     <h2 class="changelog-title">short title</h2>
     <p class="changelog-desc">1-2 sentence summary of what changed.</p>
     <a class="changelog-prompt-link" href="/static/changelog/YYYY-MM-DD-NNN-slug.txt">read the prompt</a>
     <a class="changelog-permalink" href="/changelog#YYYY-MM-DD-slug">#</a>
   </div>
   ```
   - The `id` attribute enables direct linking: `anky.app/changelog#2026-02-14-video-studio`
   - Keep descriptions concise but specific about what shipped

3. **Do this at the end of every session**, right before the final build + deploy.

## Deployment

- Build: `cargo build --release`
- Restart: `systemctl --user restart anky.service`
- Always build and restart after changes unless told otherwise.

## Payments

- All paid features use x402 wallet payments (USDC on Base). No API key payment paths.
- Treasury address comes from config. Users send USDC, pass tx hash as `payment-signature` header.
