# Anky Backend — Current State
Last updated: 2026-03-28 (session 15)

## What's working
- `cargo check` passes on the current tree, and the mobile seed-signature unit test passes via `cargo test verifies_seed_auth_signatures`.
- **All text inference is now cloud-only (Claude Haiku)**: Every Ollama/qwen3.5 call has been replaced with Claude Haiku. Quick feedback, reflections, suggested replies, image prompts, writing formatting, classifications, cuentacuentos stories+translations, memory extraction, psychological profiles, chat, system summaries — all Haiku. Local embeddings removed; Honcho handles semantic context. Local GPUs are now exclusively for Flux image generation.
- **On-chain minting (ERC1155 birthSoul on Base)**: `prepare-mint` verifies eligibility (complete anky, not minted, not gas-funded), rate-limits to 1 per wallet per hour, computes session CID (sha256 of writing), signs EIP-712 BirthPayload with `ANKY_WALLET_PRIVATE_KEY`, estimates gas, funds the user's wallet with 2x gas cost via EIP-1559 ETH transfer, and returns all params the iOS app needs to build + submit the birthSoul tx. `confirm-mint` verifies the on-chain receipt (status=1, to=contract), parses the SoulBorn event for token_id, and marks the anky as minted. Public metadata endpoint at `/api/v1/anky/{id}/metadata` serves ERC1155-compliant JSON (name, description, image, attributes). Contract: `0x19a36545CC4707870ad53CaADd23B7A70642F304`. DB columns added: `gas_funded_at`, `session_cid`, `metadata_uri`, `token_id` on ankys.
- **Social reply pipeline now has Honcho context + interaction history**: when someone tags Anky on X or Farcaster, the reply pipeline fetches their Honcho peer context (accumulated from their writing sessions) and their past interaction history before generating a reply. This gives Anky memory of who it's talking to.
- **Unified social voice with lowercase enforcement**: `ANKY_CORE_IDENTITY` and the reply system prompt are rewritten to enforce lowercase always. All Claude-generated replies are `.to_lowercase()`'d as a safety net. All hardcoded reply strings are lowercase.
- **Thread-aware reply generation**: `generate_anky_reply` now instructs Claude to split long replies into "slides" separated by `---`. The `AnkyReply` enum has a new `Thread(Vec<String>)` variant. `enforce_thread_limits()` hard-splits any slide exceeding platform limits (280 chars for X, 1024 for Farcaster) at sentence/word boundaries. `x_bot::reply_thread()` and `neynar::reply_thread()` post each slide as a chained reply. Both webhook handlers (X + Farcaster) handle the `Thread` variant and also auto-split single `Text` replies that exceed limits.
- `/you` profile page redesigned: shows user PFP, display name, username, bio (from psychological profile), stats row, and a 3-per-row Instagram-style grid of the user's completed ankys. A toggle switches to list view (1-per-row with title + reflection). New API endpoint `GET /swift/v2/you/ankys` returns the user's completed ankys with image URLs.
- **social_peers table**: maps `(platform, platform_user_id)` to `honcho_peer_id` and optionally to a linked `user_id`. Auto-created on first interaction, tracks interaction count and timestamps. Enables cross-platform identity when the same person uses both X and Farcaster.
- Story generation now uses the full anky.soul.md as its system prompt (`prompts/cuentacuentos_system.md`, included via `include_str!`). The prompt contains: territory detection heuristics with word-level signals, kingdom-specific story tone/pacing/metaphor-language specs, full narrator voice definition (first-person, present-tense, embodied, curious), structural constraints (no false comfort, no interpretation, no resolution of unresolved things, no spiritual jargon), and cross-territory handling.
- Training data pipeline: every generated cuentacuentos now logs a `(writing_input, story_output, metadata)` pair to the `story_training_pairs` table, ready for future LoRA fine-tuning export. Table tracks chakra, kingdom, city, played status, and has an `exported_at` column for the 4:44 AM cron.
- Anky detail page (`/anky/{id}`) now shows heart/mind tabs when both a cuentacuentos story and a Claude reflection exist for the same writing session. Heart tab shows the story with kingdom/city metadata; mind tab shows the reflection.
- Mobile web design system is live: `static/mobile.css` applies iOS-spec design tokens on viewports <= 768px, with a 72px fixed bottom nav (historias/anky/tu), desktop chrome hidden, and thin typography. The nav hides during active writing states (writing/paused/ended/reflection).
- `/stories` page renders cuentacuentos history from `/swift/v2/cuentacuentos/history` and `/swift/v2/cuentacuentos/ready`, with a full-screen story player overlay for phase-by-phase viewing.
- `/you` page renders the psychological mirror from `/swift/v2/you`, showing profile insights, emotional signature, tensions, growth edges, Honcho context, and writing stats.
- The single-binary Rust/Axum server boots from `src/main.rs`, opens the SQLite database at `data/anky.db`, runs `src/db/migrations.rs`, mounts the mobile router, and listens on `0.0.0.0:8889`.
- `/swift/v1` Privy auth, `/swift/v2` seed challenge/verify, shared bearer-session logout, and `/swift/v1|v2/me` are all routed through the same auth/session tables.
- `/swift/v1|v2/writings` plus `/swift/v1/write` and `/swift/v2/write` are wired end-to-end: thresholding, flow-score calculation, writing upsert, and post-write fan-out.
- The v2 seed-identity rule "only real ankys persist" is implemented: sub-threshold writes return `persisted: false` and skip the DB; real ankys persist and spawn downstream work.
- Persisted `/swift/v2/write` ankys also mirror their raw text into `data/writings/{wallet_address}/{timestamp_unix}.txt`.
- Child profile create/list/detail routes are fully wired to `child_profiles` and require a bearer session tied to a stored seed wallet.
- Cuentacuentos ready/history/complete/assign routes are wired to `cuentacuentos` and `cuentacuentos_images`, including child-scoped lookup and ready-response image URL decoration.
- The Anky image pipeline is wired end-to-end: Ollama prompt generation, Gemini image generation with Flux/ComfyUI fallback, WebP + thumbnail generation, fallback title/reflection, formatted writing, and memory extraction.
- **R2 CDN image upload + .anky story format**: After image generation, the pipeline converts to WebP and uploads to Cloudflare R2 (`stories/{anky_id}/page-0.webp`). The `.anky` format (YAML frontmatter + `:::page` blocks with CDN URLs and reflection text) is assembled and stored in `ankys.anky_story`. Exposed as `anky_story` field in `GET /api/v1/anky/{id}`. Uses the existing R2 service (`src/services/r2.rs`) with a new `upload_image_to_r2` function. Model: `src/models/anky_story.rs`. Gracefully degrades if R2 is not configured.
- The cuentacuentos image pipeline is wired end-to-end: prompt rows are inserted per story phase, generated sequentially on the local GPU, and retried in the startup worker.
- **TTS pipeline (F5-TTS)**: Local FastAPI service on `localhost:5001` (GPU 0, systemd user service `anky-tts.service`). Cross-lingual voice cloning from a single reference clip (`/home/kithkui/anky-tts/reference_voice.wav`) — same voice identity across EN/ES/ZH/HI/AR. `cuentacuentos_audio` table tracks per-language audio with status/retry. Auto-triggered after translations complete. `GET /api/v1/stories/{id}/voice` falls back to TTS when no human recording exists. 10 stories × 5 languages = 48 audio tracks generated (1 Hindi failure pending retry).
- **Mobile settings**: `GET/PATCH /swift/v2/settings` for cross-device preferences sync including `preferred_language`. `/me` also returns `preferredLanguage`.
- **History images fix**: `/swift/v2/cuentacuentos/history` now includes image URLs (was missing, caused "no images" bug in iOS app).
- **Recording identity**: `list_recordings` and `get_voice` now return recorder `userId` and `username`.
- **Reflection delivery is now triple-redundant**: (1) If Claude streaming fails during SSE, Ollama generates a fallback reflection in the same response. (2) A watchdog runs every 5 minutes and recovers any `complete` ankys with missing reflections (tries Claude first, Ollama fallback). (3) Frontend retries the SSE connection up to 3 times with backoff before giving up.
- **Post-writing UX fixed**: SSE "done" event was delayed because a cloned mpsc sender kept the channel alive after Claude finished. Fixed by dropping the fallback sender immediately on success. Added animated progress labels ("anky is sitting with what you wrote...", "anky is finding the right words...") while waiting for first chunk. Added console.log throughout the entire post-writing JS flow for debugging. SSE keep-alive pings are now filtered out so they don't pollute reflection text.
- The mobile seed-auth cryptography has direct unit coverage: `verify_seed_auth_signature()` is tested against a generated secp256k1 keypair.

## What was removed (2026-03-19)
All sadhana, meditation, breathwork, and facilitator code has been deleted from the codebase:
- `src/routes/meditation.rs` — deleted entirely.
- `src/routes/swift.rs` — removed all sadhana, meditation, breathwork, and facilitator handler functions; removed `meditation` and `breathwork` fields from `SpawnedPipelines` and `WritingStatusResponse`; removed `GuidanceStatusInfo` and `BreathworkStatusInfo` structs; removed post-write guidance spawn blocks. Only `set_premium` was kept from those sections.
- `src/routes/mod.rs` — removed `pub mod meditation;` declaration and all sadhana/meditation/breathwork/facilitator/admin-facilitator route registrations and web meditation routes.
- `src/pipeline/guidance_gen.rs` — removed all meditation/breathwork generation functions (`detect_breathwork_style`, prompt builders, `generate_meditation_premium/free`, `generate_breathwork_premium/free`, `queue_post_writing_guidance`, `queue_daily_guidance`, `process_free_queue`, `parse_script`). Only cuentacuentos generation code remains.
- `src/main.rs` — removed the guidance queue worker spawn block.
- `src/models/mod.rs` — removed all meditation-related structs.
- `src/db/migrations.rs` — removed CREATE TABLE statements for `meditation_sessions`, `user_interactions`, `user_progression`, `sadhana_commitments`, `sadhana_checkins`, `breathwork_sessions`, `breathwork_completions`, `personalized_meditations`, `personalized_breathwork`, `facilitators`, `facilitator_reviews`, `facilitator_bookings`. Old tables remain harmlessly in SQLite.
- `src/db/queries.rs` — removed all meditation, breathwork, sadhana, and facilitator query functions and structs.

## What's unvalidated at runtime
- From repo evidence alone, there is no end-to-end smoke test or operational probe for the v2 children + cuentacuentos flows; they compile and are routed, but there is no in-repo proof of live traffic exercising them.
- The vertical story-image branch of ComfyUI compiles and is called from the cuentacuentos pipeline, but there is no automated coverage for the full `/swift/v2/write` -> story -> images path.
- The startup workers for X stream, Farcaster backfill, checkpoint recovery, and failed-job retries are active codepaths, but this document only confirms compile-time wiring, not current production health.

## Known gaps
- The runtime schema source of truth is SQLite in `src/db/migrations.rs`; the standalone SQL files under `migrations/20260317_*` are Postgres-shaped and are not used by the running server.
- `COMFYUI_URL` is loaded into `Config`, but the active ComfyUI service still hardcodes `http://127.0.0.1:8188`.
- Mobile anky generation currently passes the literal string `"mobile"` into `generate_anky_from_writing()`. That placeholder is then reused for memory-aware fallback reflection and the post-image memory pipeline, so mobile ankys are not actually memory-personalized to the real caller.
- Mobile admin auth is not actually admin-restricted yet. Any valid bearer session can toggle premium.
- The new writing archive is intentionally path-public for now: `/data/writings/*` has no auth layer.
- Cuentacuentos readiness is story-first, not image-complete: `GET /swift/v2/cuentacuentos/ready` returns the oldest unplayed story and fills phase `image_url` with `null` when images are still pending or failed.
- There is no `src/pipeline/comfyui.rs`; the real implementation lives in `src/services/comfyui.rs`.

## Database
- Live mobile schema is defined in `src/db/migrations.rs`.
- `users` - canonical backend user row; carries wallet address, Privy DID, username, email, premium flags, and other identity metadata.
- `auth_sessions` - bearer session tokens for web and mobile clients.
- `auth_challenges` - one-time seed-identity challenge messages plus expiry and consumed state.
- `writing_sessions` - persisted writings, threshold outcome, flow metrics, lifecycle state, and quick feedback text.
- `ankys` - generated image/reflection records linked to an anky writing.
- `user_profiles` - longitudinal psychological profile, tensions, and growth edges.
- `child_profiles` - parent wallet -> child derived wallet map, birthdate, and 12-item emoji pattern.
- `cuentacuentos` - generated parent-to-child story (English primary), Ankyverse placement, translations, optional child assignment, guidance-phase JSON, and played state.
- `cuentacuentos_images` - one image job per story phase, with prompt, status, attempts, and final URL.
- `next_prompts` - personalized writing prompts generated after each session, keyed by user_id.
- `device_tokens` - APNs device tokens for silent push notifications.
- Note: old tables (meditation_sessions, user_progression, sadhana_*, breathwork_*, personalized_*, facilitator*) still exist in SQLite but are no longer created by migrations or referenced by code.

## Honcho integration
- Honcho user modeling API is integrated as an optional identity layer, gated on `HONCHO_API_KEY` env var.
- Every writing is sent to Honcho fire-and-forget.
- Cuentacuentos story generation injects Honcho peer context into the Claude prompt.
- Fallback reflection generation in the image pipeline prefers Honcho context over local memory recall.
- Every 5th anky session, the memory pipeline uses Honcho to populate user profile fields.

## Active pipelines
- Anky post-writing image pipeline: triggered by a real anky write or the failed-anky retry worker.
- Writing archive pipeline: triggered after every persisted `/swift/v2/write` anky.
- Cuentacuentos pipeline: triggered only by persisted `/swift/v2/write` ankys. Claude generates story, images generated via Flux, translations via Ollama.
- Cuentacuentos image pipeline: triggered by the story pipeline and by the retry worker.
- Next-prompt pipeline: after every persisted writing (short or anky), Ollama generates a personalized prompt stored in `next_prompts` for the next session.

## Mobile endpoints
- `POST /swift/v1/auth/privy` | auth: none | Verify Privy token, find/create user, mint session.
- `POST /swift/v2/auth/challenge` | auth: none | Create seed-identity challenge.
- `POST /swift/v2/auth/verify` | auth: none | Verify EIP-191 signature, mint session.
- `DELETE /swift/v1/auth/session` and `DELETE /swift/v2/auth/session` | auth: bearer | Logout.
- `GET /swift/v1/me` and `GET /swift/v2/me` | auth: bearer | User profile.
- `GET /swift/v1/writings` and `GET /swift/v2/writings` | auth: bearer | Writing history.
- `POST /swift/v1/write` and `POST /swift/v2/write` | auth: bearer | Submit writing.
- `GET /swift/v2/writing/{sessionId}/status` | auth: bearer | Poll downstream pipeline status.
- `GET/POST /swift/v2/children` | auth: bearer+wallet | Child profile CRUD.
- `GET /swift/v2/children/{childId}` | auth: bearer+wallet | Single child profile.
- `GET /swift/v2/cuentacuentos/ready` | auth: bearer+wallet | Next unplayed story.
- `GET /swift/v2/cuentacuentos/history` | auth: bearer+wallet | Story history.
- `POST /swift/v2/cuentacuentos/{id}/complete` | auth: bearer+wallet | Mark story played.
- `POST /swift/v2/cuentacuentos/{id}/assign` | auth: bearer+wallet | Assign story to child.
- `GET /swift/v2/next-prompt` | auth: bearer | Precomputed personalized writing prompt (defaults to generic if none yet).
- `GET /swift/v2/you` | auth: bearer | Full user profile with Honcho peer context for the You tab.
- `POST /swift/v2/device-token` | auth: bearer | Register APNs device token for silent push.
- `POST /swift/v2/writing/{sessionId}/prepare-mint` | auth: bearer | EIP-712 sign + gas fund for birthSoul mint.
- `POST /swift/v2/writing/{sessionId}/confirm-mint` | auth: bearer | Verify mint tx receipt, parse token ID.
- `GET /api/v1/anky/{id}/metadata` | public | ERC1155-compliant metadata JSON.
- `POST /swift/v1/admin/premium` | auth: bearer (not admin-scoped) | Toggle premium.

## Retry and background workers
- Session reaper: every 2 seconds, kills stale chunked agent sessions.
- Failed anky + prompt retry loop: every 5 minutes.
- Cuentacuentos image retry loop: every 5 minutes.
- Checkpoint recovery watchdog: every 5 minutes after 60s boot delay.
- Farcaster backfill loop: every 120 seconds after 30s boot delay.
- X filtered-stream reconnect loop: continuous with exponential backoff.
- System summary worker: every 30 minutes.

## Web seed auth (2026-03-19)
- Web login page (`/login`) now uses 12-word BIP39 seed identity instead of Privy.
- Three flows: generate new identity, import existing recovery phrase, unlock stored vault.
- Private key encrypted client-side with PBKDF2 (600k iterations) + AES-GCM, stored in localStorage as `anky_seed_vault`.
- `POST /auth/seed/verify` sets `anky_session` and `anky_user_id` cookies (same auth as mobile `/swift/v2/auth/verify`).
- `POST /auth/seed/logout` clears cookies and invalidates session.
- `auth_seed_verify_inner()` in `swift.rs` is the shared core verification function used by both mobile and web.
- Privy endpoints remain alive but the web login no longer links to them.
- iOS specs updated: 24-word references changed to 12-word.

## Next priorities
- Pass the real mobile `user_id` into `generate_anky_from_writing()` so fallback reflection + memory extraction operate on the correct user.
- Lock down auth on mobile admin path before more real traffic hits it.
- Make `Config.comfyui_url` the real source of truth everywhere.
- Add end-to-end smoke coverage for `/swift/v2/write` -> anky image -> cuentacuentos -> phase images.
- Decide whether `GET /swift/v2/cuentacuentos/ready` should wait for a fully imaged story or continue returning text-first stories with nullable phase images.
