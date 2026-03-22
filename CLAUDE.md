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
- Claude handles premium writing-derived generation and other cloud LLM tasks.
- Ollama is the local text model layer. The current configured default is `qwen3.5:35b` at `http://localhost:11434`.
- ComfyUI is the local Flux image layer. The live implementation is in `src/services/comfyui.rs`; current runtime code uses localhost `8188` for Flux + the Anky LoRA workflow, even though `COMFYUI_URL` also exists in config.
- `poiesis` is the operational center: Rust server, SQLite, Ollama, ComfyUI, and the surrounding worker loops all assume that machine-local deployment model.

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
