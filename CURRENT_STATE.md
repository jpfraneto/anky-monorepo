# Anky Backend — Current State
Last updated: 2026-04-22 (session 21 — Sojourn 9 constitution synced to `/sojourn9.md`)

## Session 21 (2026-04-22) — Sojourn 9 constitution sync

### What shipped
- `/sojourn9.md` now serves the current canonical Sojourn 9 constitution instead of the older draft text.
- The hosted markdown matches `sojourn9/constitution/SOJOURN_9.md`, so the public URL and the constitutional source are aligned again.

## Session 20 (2026-04-12) — i18n infrastructure + 9 languages

### What shipped
- **New module `src/i18n.rs`**: JSON-file locale loader, Tera `t(key=..., lang=lang)` function, request-language resolver (query → cookie → Accept-Language → en), client-side JSON bundle builder.
- **9 locale files** in `locales/`: `en, es, fr, pt, de, it, ja, ko, zh` (44 keys each).
- **AppState gained `i18n: Arc<I18n>`**. `main.rs` loads locales at startup, registers the Tera function, passes the Arc into state.
- **Automatic detection**: the `Accept-Language` HTTP header is the primary signal. No picker, no URL param required. `?lang=xx` is also supported and persists to an `anky_lang` cookie (1-year).
- **Templates converted**: `landing.html` (the main chat/writing/timer flow), `you.html` (profile), `anky.html` (detail page + reply UI). Each template now emits `<html lang>` correctly and inlines a `window.__LANG__` + `window.__I18N__` bridge so inline JS strings (e.g., "anky is reading…", drawer empty states, session-divider "now") translate via `window.t(key)`.
- **Page handlers updated**: `home`, `write_page`, `you_page`, `anky_detail` in `src/routes/pages.rs` now call `i18n::inject_into_context(…)` before rendering. `you_page` and `anky_detail` return `(CookieJar, Html<String>)` so `?lang=` can set the cookie.

### Chat-bubble CSS fix (same session)
- Desktop anky reflection bubble was shrink-wrapping to ~15 chars wide. Fixed in `static/style.css:612` by giving `.chat-bubble.anky` a `min-width: min(100%, 520px)` and swapping `word-break: break-word` for `overflow-wrap: break-word`.
- SSE reflection sometimes cut off at the tail. After `done`/`error`, client now re-fetches `/api/v1/anky/{id}` and replaces the bubble with the longer DB-saved reflection (`templates/prompt.html:567,584`).

### Not in this pass (deliberately — "simplicity wins")
- No language picker UI. Automatic from device.
- No extraction from legacy templates: `home.html` (the `/write` page), miniapp React app, iOS app — all still English-only.
- Translations are LLM-generated best-effort; Anky's contemplative voice (e.g., "what is alive in you right now") would benefit from a native-speaker pass per language before polishing.
- Reflection output language already mirrors the writer's language via the LLM prompt ("Respond in their language") — unchanged.

### How to add another language
Drop a new file `locales/<code>.json` with the same keys as `en.json` and restart. The loader picks it up at startup; `resolve_request_lang` will match it from `Accept-Language` automatically.



## Solana Hackathon Ship (Colosseum, deadline Mon Apr 7)

### Architecture decisions (2026-04-05)
- **Everything is Solana.** User identity = Solana wallet. iOS app generates BIP39 mnemonic, derives Ed25519 keypair (m/44'/501'/0'/0'), produces base58 Solana pubkey.
- **Two NFT collections on devnet:**
  - **Mirrors** (sojourn membership): 3,456 cap, depth-12 tree `ArmTPWNskwUsiZErvN1HKbqrupeiavkxVgS1ciLRQK6B`, collection `CXYtYYgnXx5Lbn5MmePHSePynjJwGyTSWnzhDBbbn4Dt`. One per user. The gate.
  - **Ankys** (per-writing artifacts): depth-10 tree `3SgBFS5gFmeMUNZQqQk1xGsoZAZewXVAiMVkzjeJBSd6`, collection `5AbvPKw84mXhYWNBo3iTCH1Nkpc7EZPHHk4q1ECz4rVW`, 1,024 capacity. Auto-minted at write-time for participants.
- **Farcaster miniapp now mints.** The miniapp generates a mirror from the user's Farcaster presence, then mints a Mirror cNFT. It's a full onboarding surface.
- **Per-anky minting is live.** Every real anky (8+ min, 300+ words) written by a sojourn participant auto-mints a cNFT on the Ankys tree. Happens at write-time, not after image generation.
- **AWS Nitro Enclave is live** at `3.83.84.211:5555`. `GET /api/anky/public-key` proxies to the enclave. Sealed sessions stored via `POST /api/sessions/seal`. Backend never sees plaintext from authenticated users.
- **Anonymous users see ephemeral writing** — no persistence, just copy-to-platform buttons (X, Warpcast, Claude, ChatGPT) after writing.
- **No explicit sequence counter.** Bubblegum tree assigns leaf indices automatically.

### Solana infrastructure status
- **Devnet tree created**: `solana/setup/tree-devnet.json` — tree `ArmTPWNskwUsiZErvN1HKbqrupeiavkxVgS1ciLRQK6B`, collection `CXYtYYgnXx5Lbn5MmePHSePynjJwGyTSWnzhDBbbn4Dt`, maxDepth=12, 4,096 capacity.
- **Mainnet tree**: NOT YET CREATED. Run `cd solana/setup && SOLANA_NETWORK=mainnet-beta HELIUS_API_KEY=xxx npm start`. Authority needs ~2-3 SOL.
- **Cloudflare Worker** (`solana/worker/`): Code complete (234 lines, Umi + mpl-bubblegum v4.3.1). `POST /mint` accepts `{mirror_id, recipient, name, uri, kingdom, symbol}` and mints cNFT with `leafOwner = recipient`. NOT YET DEPLOYED. Deploy: `cd solana/worker && wrangler deploy`, then set secrets.
- **Env vars needed on poiesis**: `SOLANA_MINT_WORKER_URL`, `SOLANA_MINT_WORKER_SECRET`, `SOLANA_MERKLE_TREE`, `SOLANA_COLLECTION_MINT`, `SOLANA_AUTHORITY_PUBKEY`, `HELIUS_API_KEY`.

### Code changes (this session)
- **Metadata creator address**: Both `GET /api/mirror/collection-metadata` and `GET /api/mirror/metadata/{id}` now read `SOLANA_AUTHORITY_PUBKEY` from config instead of empty string.
- **Collection image**: `static/anky-collection.png` added so collection metadata image URL resolves.
- **Farcaster miniapp**: Removed mint button, mint JS, mint CSS. After mirror generation, shows CTA: "this is your surface. anky saw your public presence. want to show it what's inside?" → link to iOS App Store. Share button always enabled after mirror loads.
- **iOS mint endpoint refactored**: `POST /swift/v2/mint-mirror` now requires `{solana_address, writing_session_id}`. Validates base58 (rejects 0x-prefixed ETH addresses). Validates writing session is real anky (8+ min) owned by the user. Handles already-minted gracefully: returns `{alreadyMinted: true, existingTxSignature, ...}` instead of an error. Added `get_user_existing_mint` DB query.
- **Metadata attributes**: Raw mirrors now include `Writer` (Solana pubkey) and `Sojourn` (9) attributes.
- **Config**: Added `solana_authority_pubkey` field + `SOLANA_AUTHORITY_PUBKEY` env var.

### Blocking items for Monday ship
1. Create mainnet merkle tree (fund authority, run setup script)
2. Deploy Cloudflare Worker + set 5 secrets
3. Set env vars on poiesis
4. iOS app: replace EVM crypto with Solana Ed25519 derivation (BIP39 → m/44'/501'/0'/0')
5. iOS app: call `POST /swift/v2/mint-mirror` with `{solana_address, writing_session_id}` on first seal

### EVM paths (dormant, not deleted)
- `POST /swift/v2/writing/{sessionId}/prepare-mint` and `/confirm-mint` still exist for Base ERC1155.
- Not used by current iOS flow. May be useful later.

## What's working
- `cargo check` passes on the current tree, and the mobile seed-signature unit test passes via `cargo test verifies_seed_auth_signatures`.
- **Local-first text inference (Mind → Claude → OpenRouter)**: All text inference calls now follow a 3-tier fallback chain: (1) Mind (local llama-server with qwen3.5-27b at `MIND_URL`), (2) Claude Haiku (cloud), (3) OpenRouter (last resort). This applies to `call_haiku`, `call_haiku_with_system`, `call_haiku_with_system_max`, `call_haiku_with_fallback`, `generate_title_and_reflection_with_memory`, and `generate_anky_reply`. If `MIND_URL` is empty or Mind is unreachable, all existing cloud paths work exactly as before. Ollama is gone; the Mind uses llama-server's OpenAI-compatible API with `<think>` block stripping for qwen3.5-27b.
- **8 Kingdoms system**: Each anky is assigned to one of 8 kingdoms (Primordia, Emblazion, Chryseos, Eleasis, Voxlumis, Insightia, Claridium, Poiesis) based on the user's Farcaster FID or a session hash. Kingdom assignment happens in the GPU job worker before image generation. Each kingdom has a chakra, element, system addendum for text generation, and image prompt flavor that is appended to the ComfyUI/Flux prompt. DB columns: `kingdom_id`, `kingdom_name`, `kingdom_chakra` on ankys. API: kingdom fields exposed in `GET /api/v1/anky/{id}`.
- **Mind status endpoint**: `GET /api/v1/mind/status` returns availability, slot status, and kingdom mapping for each slot.
- **Redis job persistence**: `src/services/redis_queue.rs` provides a Redis-backed job queue with pro/free priority, processing tracking, and crash recovery. On startup, orphaned processing jobs are re-queued (max 5 retries).
- **Retry watchdog with backoff**: The failed anky retry loop now has `MAX_ANKY_RETRIES = 5`, exponential backoff (`2^N * 5` minutes), and `retry_count`/`last_retry_at` tracking. After max retries, ankys are marked `abandoned` instead of retrying forever.
- **Systemd service files**: `deploy/anky-mind.service` (llama-server on GPU 0) and `deploy/anky-heart.service` (ComfyUI on GPU 1) with install instructions in `deploy/README.md`.
- **On-chain minting (ERC1155 birthSoul on Base)**: `prepare-mint` verifies eligibility (complete anky, not minted, not gas-funded), rate-limits to 1 per wallet per hour, computes session CID (sha256 of writing), signs EIP-712 BirthPayload with `ANKY_WALLET_PRIVATE_KEY`, estimates gas, funds the user's wallet with 2x gas cost via EIP-1559 ETH transfer, and returns all params the iOS app needs to build + submit the birthSoul tx. `confirm-mint` verifies the on-chain receipt (status=1, to=contract), parses the SoulBorn event for token_id, and marks the anky as minted. Public metadata endpoint at `/api/v1/anky/{id}/metadata` serves ERC1155-compliant JSON (name, description, image, attributes). Contract: `0x19a36545CC4707870ad53CaADd23B7A70642F304`. DB columns added: `gas_funded_at`, `session_cid`, `metadata_uri`, `token_id` on ankys.
- **Social reply pipeline now has Honcho context + interaction history**: when someone tags Anky on X or Farcaster, the reply pipeline fetches their Honcho peer context (accumulated from their writing sessions) and their past interaction history before generating a reply. This gives Anky memory of who it's talking to.
- **Farcaster community writing prompts**: When someone tags Anky in a top-level cast, Claude Haiku classifies intent via `classify_community_question()` — detects questions, prompts, or invitations directed at the audience (not just chatting with Anky). When detected, Claude reframes the question into a deeper, personal writing prompt (via `reframe_as_writing_prompt`), stores it in the prompts table, and the reply system is forced to include a write invitation link (`https://anky.app/write?p={id}`). The system prompt has a new COMMUNITY QUESTIONS section that tells Anky to always end with the write link.
- **Programming classes system**: Full infrastructure for 8-slide, 8-minute programming lessons generated from coding sessions. DB table `programming_classes` (class_number, title, concepts_json, slide_urls_json). Templates at `/classes` (index) and `/classes/{number}` (player with 60s auto-advance, progress bar, keyboard nav, preloaded images). Slide images generated via ComfyUI + uploaded to R2 at `classes/{n}/slide-{i}.webp` with immutable cache headers. Local fallback at `data/classes/`. Generation triggered via `POST /api/v1/classes/generate` with session summary. Claude generates concepts + Flux image prompts featuring Anky in teaching scenes.
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
- `POST /swift/v2/mint-mirror` | auth: bearer | Mint Sojourn 9 cNFT from writing session. Body: `{solana_address, writing_session_id}`. Returns `{alreadyMinted, tx_signature, kingdom, ...}`.
- `POST /swift/v2/writing/{sessionId}/prepare-mint` | auth: bearer | EIP-712 sign + gas fund for birthSoul mint (EVM, dormant).
- `POST /swift/v2/writing/{sessionId}/confirm-mint` | auth: bearer | Verify mint tx receipt, parse token ID (EVM, dormant).
- `GET /api/v1/anky/{id}/metadata` | public | ERC1155-compliant metadata JSON.
- `GET /api/mirror/metadata/{id}` | public | Metaplex-compatible cNFT metadata (Sojourn 9).
- `GET /api/mirror/collection-metadata` | public | Metaplex collection metadata.
- `GET /api/mirror/supply` | public | Mint count and remaining supply.
- `POST /swift/v1/admin/premium` | auth: bearer (not admin-scoped) | Toggle premium.

## Retry and background workers
- Session reaper: every 2 seconds, kills stale chunked agent sessions.
- Failed anky + prompt retry loop: every 5 minutes.
- Cuentacuentos image retry loop: every 5 minutes.
- Checkpoint recovery watchdog: every 5 minutes after 60s boot delay.
- Farcaster backfill loop: every 120 seconds after 30s boot delay.
- X filtered-stream reconnect loop: continuous with exponential backoff.
- System summary worker: every 30 minutes.

## Web Auth Bridge (2026-04-03)
- Web login page (`/login`) is a phone-seal bridge.
- Browser creates a QR challenge via `POST /api/auth/qr`, waits, and wakes up when the iPhone app seals.
- Phone remains the authority for the wallet and swipe; the web only receives the resulting browser session.
- Live QR challenges now return the real TestFlight fallback via `ANKY_IOS_APP_URL=https://testflight.apple.com/join/WcRYyCm5`.
- QR seals now verify the iPhone's Ed25519 signature against the stored challenge token before minting the browser session.
- Browser logout is now a single `POST /auth/logout`.
- Old web-only Privy and seed auth routes were removed from the browser surface. Mobile auth endpoints remain unchanged.

## Next priorities
- Pass the real mobile `user_id` into `generate_anky_from_writing()` so fallback reflection + memory extraction operate on the correct user.
- Lock down auth on mobile admin path before more real traffic hits it.
- Make `Config.comfyui_url` the real source of truth everywhere.
- Add end-to-end smoke coverage for `/swift/v2/write` -> anky image -> cuentacuentos -> phase images.
- Decide whether `GET /swift/v2/cuentacuentos/ready` should wait for a fully imaged story or continue returning text-first stories with nullable phase images.
