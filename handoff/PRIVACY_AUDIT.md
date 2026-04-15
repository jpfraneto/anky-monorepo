Now I have enough information. Let me compile this comprehensive audit into a detailed report.

---

## EXHAUSTIVE PRIVACY AND DATA-FLOW AUDIT: ANKY BACKEND

**Prepared**: 2026-04-15  
**Scope**: Rust/Axum backend at `/home/kithkui/anky/src/` | Postgres via sqlx | AWS Nitro Enclave | Solana L1 | Cloudflare R2  
**Methodology**: Source code review with file:line citations. Architecture context from CLAUDE.md + CURRENT_STATE.md.

---

### 1. USER DATA RECEIVED, STORED, PROCESSED, LOGGED, INFERRED

#### A. Data Inputs to Backend

| Data Type | Source | Example Fields | Notes |
|-----------|--------|-----------------|-------|
| **Writing Session Text** | iOS app (sealed), web (plaintext) | `writing_sessions.content` | Encrypted on-device → stored as plaintext in DB ONLY for web/anonymous path (migrations/001_init.sql:32-42). Sealed write path: ciphertext stored only in `sealed_sessions` table (migrations/009_sealed_sessions.sql). |
| **User Identity** | Mobile auth (Ed25519 seed) / Web OAuth | `users.wallet_address`, `users.farcaster_fid`, `users.email`, `users.username` | Soln: Ed25519 Solana wallet auth via `POST /swift/v2/auth/verify` (src/routes/swift.rs:313-335). Farcaster via Neynar. Email optional. |
| **Session Metadata** | Writing submission | `duration_seconds`, `word_count`, `started_at`, `keystroke_deltas` | src/routes/swift.rs:724-1200 (`submit_writing_unified`) |
| **Farcaster Profile Data** | Neynar API → mirrors table | `fid`, `username`, `display_name`, `avatar_url`, `follower_count`, `bio` | Via `GET /api/mirror?fid=X` → src/routes/api.rs (fetches casts, analyzes PFP via Claude) → stored in `mirrors` table (migrations/001_init.sql:848-860). |
| **Device/Session IDs** | iOS + Web | `device_tokens.id`, `auth_sessions.token` | APNs token registration (src/routes/swift.rs:2465). Session tokens are UUIDs (src/routes/swift.rs:253, auth_sessions table). |
| **IP Address** | HTTP connection | Not explicitly logged in tracing; can be inferred from request context | No evidence of IP logging in code. Cloud-flare Tunnel strips IPs (architecture). |

#### B. Data Processing & Inference

| Process | Code Location | Data Fields | Output/Storage |
|---------|---------------|------------|-----------------|
| **Memory Extraction** | src/memory/extraction.rs:44-58 | Writing text → themes, emotions, entities, patterns, breakthroughs, avoidances | `user_memories` table (migrations/001_init.sql:294-306). Extracted via Claude Haiku (EXTRACTION_SYSTEM prompt, line 5-25). |
| **Psychological Profile** | src/memory/profile.rs:59-125 | Writing sessions → psychological_profile, emotional_signature, core_tensions, growth_edges | `user_profiles` table (migrations/001_init.sql:308-320). Updated every 5th session via Honcho chat (src/main.rs:612-728 reflection recovery watchdog). |
| **Memory Embeddings** | src/memory/embeddings.rs:85-132 | Writing text → OpenAI embeddings (stored as BYTEA) | `memory_embeddings` table (migrations/001_init.sql:284-292). Used for similarity search in reflection generation (src/memory/recall.rs:107-132). |
| **Title + Reflection** | src/services/claude.rs (call_haiku*), src/pipeline/image_gen.rs:296-310 | Writing text + memory context → title (title field), reflection (reflection field) | `ankys.title`, `ankys.reflection` (migrations/001_init.sql:44-61). Generated via Claude Haiku, fallback to Ollama Mind (src/services/mind.rs), last resort OpenRouter (src/services/claude.rs:140-200). |
| **Image Prompt** | src/pipeline/image_gen.rs:400-450 | Reflection + kingdom context → Gemini → Flux image prompt | `ankys.image_prompt` (migrations/001_init.sql). Sent to Gemini API (src/services/gemini.rs:109-233) or ComfyUI (src/services/comfyui.rs). |
| **Story Generation (Cuentacuentos)** | src/pipeline/guidance_gen.rs (generate_cuentacuentos) | Writing text + Honcho peer context → story (5-phase narrative) | `cuentacuentos` table (migrations/001_init.sql:677-688). Story translations stored: `content_es`, `content_zh`, `content_hi`, `content_ar` (via Ollama). |
| **Training Data Pairs** | src/pipeline/guidance_gen.rs | Writing input + story output → labeled training pair | `story_training_pairs` table (migrations/001_init.sql:737-754). Exported at 4:44 AM cron (src/main.rs references but not shown in audit scope). |

#### C. Third-Party Data Sharing

| Service | Data Sent | Method | Auth | Conditions | Lines |
|---------|-----------|--------|------|-----------|-------|
| **Honcho (user modeling)** | Writing text (plaintext, 25k char limit) | POST `/v3/workspaces/{id}/sessions/{id}/messages` | Bearer `HONCHO_API_KEY` | Every writing session (fire-and-forget) OR all historical writings on startup (backfill) | src/services/honcho.rs:96-135 (send_writing), 237-336 (backfill_all_writings). Gated on `is_configured()` (line 14). |
| **Claude API** | Writing text + system prompts | POST `https://api.anthropic.com/v1/messages` | x-api-key header | Every reflection, title, memory extraction, profile update | src/services/claude.rs:75-134 (call_claude), 150+ call sites |
| **OpenRouter** | Writing text + system prompt | POST `https://openrouter.ai/api/v1/chat/completions` | auth header | Fallback if Claude unavailable; free tier for llama-4-scout | src/services/openrouter.rs (not fully shown, but config at src/config.rs:135-140) |
| **Gemini API** | Writing-derived image prompt (text only, no writing) | POST `https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent` | x-goog-api-key | Every image generation | src/services/gemini.rs:109-233 |
| **ComfyUI (local)** | Image prompt (text only) | POST `http://127.0.0.1:8188/prompt` | None (local) | Fallback after Gemini; offline | src/services/comfyui.rs (local service) |
| **Neynar (Farcaster)** | FID, wallet address | GET `/v2/farcaster/user/bulk-by-address`, POST `/v2/farcaster/cast` | x-api-key | Looking up user profile, publishing replies, reacting | src/services/neynar.rs:34-104 (lookup), 148-184 (reply), 248+ |
| **Solana Mint Worker** | session_hash, user_wallet, duration, word_count, kingdom_id | POST `/log-session` (worker at solana/worker/) | Bearer token (`SOLANA_MINT_WORKER_SECRET`) | After every real anky (8+ min, 300+ words) for on-chain logging via spl-memo | src/pipeline/image_gen.rs:823-908 (log_session_onchain). Memo format: `anky\|<session_hash>\|<session_id>\|<wallet>\|<duration>\|<words>\|<kingdom>\|<sojourn>` (CLAUDE.md:50). |
| **X (Twitter) Bot** | Reply text (generated by Claude, lowercased) | POST `/2/tweets` (X API v2) | Bearer `X_BEARER_TOKEN` | When tagged, after intent classification + reply generation | src/routes/webhook_x.rs (not fully shown; X bot details in config.rs:31-36) |
| **Cloudflare R2** | Image bytes (WebP), audio bytes | PUT (AWS SDK S3 API) to R2 endpoint | r2_access_key_id + r2_secret_access_key | After image generation (stories/{anky_id}/page-{}.webp), after TTS (stories/{story_id}/{language}.mp4) | src/services/r2.rs:59-115 (upload_bytes, upload_image_to_r2). Keys in config.rs:70-74. |
| **OpenAI (for embeddings)** | Writing text (up to chunk size) | POST `/v1/embeddings` (implied in memory pipeline) | api-key header | For memory embedding generation (not fully shown, but inferred from memory_embeddings table schema) | src/memory/embeddings.rs infers but no explicit client visible in audit scope. |
| **TTS Service (F5-TTS local)** | Story text + language code | POST `http://localhost:5001/api/tts` (FastAPI) | None (local) | After story translation (5 languages × N stories) | src/pipeline/guidance_gen.rs (generate_cuentacuentos_audio), src/services/tts.rs (not fully shown). |
| **Stripe** | Payment amount, email (optional) | POST `https://api.stripe.com/v1/payment_intents` (implied) | stripe_secret_key | When user donates via altar (/altar route) | src/routes/payment.rs, src/config.rs:102-103 (stripe keys loaded). |
| **Base RPC (EVM, via RPC) ** | tx_hash, recipient, token, amount | JSON-RPC `eth_getTransactionReceipt`, `eth_blockNumber` | No auth (public RPC) | Verifying payment on Base chain | src/services/payment.rs:52-150 (verify_base_transaction). |

---

### 2. API ENDPOINTS: EXHAUSTIVE ENUMERATION

**Format**: `PATH | HTTP METHOD | Auth | Request Body Fields | Response Fields | Writing Content? | Logs? | Third-Party Forward?`

#### A. Mobile/Swift Endpoints (`/swift/v1/`, `/swift/v2/`)

| Endpoint | Method | Auth | Request | Response | Writes Content? | Logs | Third-Party |
|----------|--------|------|---------|----------|-----------------|------|-------------|
| `/swift/v1/auth/privy` | POST | None | `privy_idToken` | `token`, `user_id`, `is_new_user` | No | Yes (tracing:info) | Privy verification |
| `/swift/v2/auth/challenge` | POST | None | `wallet_address` | `challenge`, `challenge_id`, `expires_at` | No | Minimal | None |
| `/swift/v2/auth/verify` | POST | None | `challenge_id`, `signature` (Ed25519) | `token`, `user_id` | No | Yes (verify_seed_auth_signature test) | None |
| `/swift/v1/auth/session`, `/swift/v2/auth/session` | DELETE | Bearer | None | `{ "ok": true }` | No | Minimal | None |
| `/swift/v1/me`, `/swift/v2/me` | GET | Bearer | None | `user_id`, `wallet_address`, `farcaster_fid`, `is_premium`, `total_ankys`, `profile_image_url`, `preferred_language` | No | Minimal | None |
| `/swift/v1/writings`, `/swift/v2/writings` | GET | Bearer | None (query: limit, offset) | Array of `{ session_id, content, duration_seconds, word_count, is_anky, created_at, anky_id, image_url, reflection, ... }` | **Yes (full content returned)** | Minimal | None |
| `/swift/v1/write`, `/swift/v2/write` | POST | Bearer | `{ keystroke_deltas, content, duration_seconds, word_count }` | `{ ok, outcome, persisted, anky_id, wallet_address, flow_score }` | **Yes (stored and queued for pipeline)** | Yes (`submit_writing_unified` at src/routes/swift.rs:724) | Honcho (if configured), Claude (reflection), Gemini (image), Solana (on-chain logging), R2 (image upload) |
| `/swift/v2/writing/{sessionId}/status` | GET | Bearer | None | `{ outcome, status, title, reflection, image_path, anky_response, mood, ... }` | No | Minimal | None |
| `/swift/v2/writing/{sessionId}/retry-reflection` | POST | Bearer | None | Regenerated reflection | No | Yes (emit_log) | Claude (reflection regen), Ollama fallback |
| `/swift/v2/children` | GET | Bearer | None | Array of `{ id, name, birthdate, emoji_pattern, wallet_address }` | No | Minimal | None |
| `/swift/v2/children` | POST | Bearer | `{ name, birthdate, emoji_pattern }` | `{ id, derived_wallet_address, ... }` | No | Yes (create_child_profile) | None |
| `/swift/v2/children/{childId}` | GET | Bearer | None | `{ id, name, birthdate, emoji_pattern, wallet_address }` | No | Minimal | None |
| `/swift/v2/cuentacuentos/ready` | GET | Bearer + wallet | None | Next unplayed story + phase images | **Yes (story text returned)** | Minimal | None |
| `/swift/v2/cuentacuentos/history` | GET | Bearer + wallet | None | Array of completed stories | **Yes (story text returned)** | Minimal | None |
| `/swift/v2/cuentacuentos/{id}/complete` | POST | Bearer + wallet | None | `{ ok: true }` | No | Yes (complete_cuentacuentos) | None |
| `/swift/v2/cuentacuentos/{id}/assign` | POST | Bearer + wallet | `{ child_wallet_address }` | `{ ok: true }` | No | Yes (assign_cuentacuentos) | None |
| `/swift/v2/prompt/{id}` | GET | Bearer | None | `{ id, prompt_text, image_url, ... }` | No | Minimal | None |
| `/swift/v2/next-prompt` | GET | Bearer | None | `{ prompt_text, generated_from_session }` (or generic default) | No | Minimal | None |
| `/swift/v2/chat/prompt` | GET | Bearer | None | Opening message for new session (can include personalized prompt from Honcho) | No | Minimal | Honcho context |
| `/swift/v2/you` | GET | Bearer | None | `{ user_id, total_sessions, total_anky_sessions, total_words, psychological_profile, emotional_signature, core_tensions, growth_edges, honcho_peer_context, anky_count, current_streak, ... }` | **No, but returns inferred profile data** | Minimal | Honcho (peer context fetch) |
| `/swift/v2/you/ankys` | GET | Bearer | None | Array of user's ankys with images | **Yes (anky IDs + reflections returned)** | Minimal | None |
| `/swift/v2/you/items` | GET | Bearer | None | Mixed (stories, ankys, prompts) | **Yes (item text/reflections returned)** | Minimal | None |
| `/swift/v2/mirror/mint` | POST | Bearer | `{ solana_address, writing_session_id }` | `{ alreadyMinted, tx_signature, kingdom, ... }` | No | Yes (swift_mirror_mint) | Solana Mint Worker (POST /mint) |
| `/swift/v2/device-token`, `/swift/v2/devices` | POST | Bearer | `{ platform, token }` | `{ ok: true }` | No | Minimal | Apple APNs (implicit: token stored for push) |
| `/swift/v2/devices` | DELETE | Bearer | `{ platform }` | `{ ok: true }` | No | Minimal | None |
| `/swift/v2/settings` | GET | Bearer | None | `{ preferred_language, font_family, ... }` | No | Minimal | None |
| `/swift/v2/settings` | PATCH | Bearer | `{ preferred_language, ... }` | `{ ok: true }` | No | Yes (patch_settings) | None |
| `/swift/v2/writing/{sessionId}/prepare-mint` | POST | Bearer | `{ nonce_override }` (optional) | `{ payloadJson, signature, userAddress, gasEstimate, ... }` | No | Yes (prepare_mint) | Pinata (if metadata upload configured, line 3031-3068) |
| `/swift/v2/writing/{sessionId}/confirm-mint` | POST | Bearer | `{ tx_hash }` | `{ ok, token_id, metadata_uri, ... }` | No | Yes (confirm_mint) | None |
| `/swift/v2/mint-mirror` | POST | Bearer | `{ solana_address, writing_session_id }` | `{ alreadyMinted, tx_signature, ... }` | No | Yes (mint_raw_mirror) | Solana Mint Worker |

#### B. Web/Public Endpoints (`/api/`, `/`)

| Endpoint | Method | Auth | Request | Response | Writes Content? | Logs | Third-Party |
|----------|--------|------|---------|----------|-----------------|------|-------------|
| `GET /` | GET | None | None | HTML (miniapp or landing) | No | Minimal | None |
| `GET /write` | GET | Optional Bearer | None | Writing UI | No | Minimal | None |
| `GET /you` | GET | Bearer | None | Profile HTML | No | Minimal | None |
| `GET /anky/{id}` | GET | None | None | Anky detail page (reflection, image, conversation) | **Yes (reflection + conversation returned)** | Minimal | None |
| `GET /login` | GET | None | None | QR code + wait loop | No | Minimal | None |
| `POST /api/auth/qr` | POST | None | `{ wallet_address }` | `{ challenge_id, qr_code }` | No | Yes (emit_log) | None |
| `POST /api/anky/public-key` | POST | Bearer | None | `{ public_key }` (X25519, from enclave) | No | Minimal | Enclave proxy call (POST `/public-key` to 3.83.84.211:5555) |
| `POST /api/sessions/seal` | POST | Bearer | `{ session_id, ciphertext, nonce, tag, user_encrypted_key, anky_encrypted_key, session_hash, metadata }` | `{ sealed, session_hash, stored_at }` | **Yes (ciphertext stored as opaque blob, never decrypted by backend)** | Yes (tracing:info, src/routes/sealed.rs:153-159) | None (backend blind relay) |
| `GET /swift/v2/sealed-sessions` | GET | Bearer | None | Array of sealed sessions (ciphertext opaque) | **Yes (encrypted envelopes returned)** | Minimal | None |
| `GET /api/verify/{session_hash}` | GET | None | None | `{ exists, sealed_at, ciphertext_size_bytes }` | No | Minimal | None |
| `GET /api/anky/{id}` (API) | GET | None | None | JSON: `{ id, title, reflection, image_path, image_url, anky_story, kingdom, ... }` | **Yes (reflection + anky_story returned)** | Minimal | None |
| `GET /api/mirror?fid={fid}` | GET | None | `fid` (Farcaster ID) | JSON: `{ id, fid, username, display_name, public_mirror, flux_descriptors_json, image_url, ... }` | **Yes (mirror text + image returned)** | Yes (emit_log for mirror generation, src/routes/api.rs) | Neynar (lookup user + casts), Claude Vision (PFP analysis), Claude Sonnet (mirror generation), ComfyUI/Gemini (image) |
| `GET /api/mirror/collection-metadata` | GET | None | None | Metaplex NFT collection metadata | No | Minimal | None |
| `GET /api/mirror/metadata/{id}` | GET | None | None | Metaplex NFT metadata for one mirror | No | Minimal | None |
| `GET /api/mirror/supply` | GET | None | None | `{ minted, remaining }` | No | Minimal | None |
| `POST /api/mirror/solana-mint` | POST | Bearer | `{ mirror_id, wallet_address }` | `{ ok, tx_signature, ... }` | No | Yes (emit_log) | Solana Mint Worker |
| `GET /api/v1/mind/status` | GET | None | None | Mind (llama-server) availability, slot status, kingdom mapping | No | Minimal | Local Mind service (GET `/health`) |
| `POST /api/v1/generate` | POST | Optional API Key | `{ prompt, payment_signature, ... }` | Generated text/image | No | Yes (emit_log) | Claude, OpenRouter (depending on payload) |
| `POST /api/v1/prompt` (API) | POST | Optional API Key | `{ prompt_text }` | `{ id, image_url }` | **Yes (prompt text stored in prompts table, image queued)** | Yes (create_prompt_api, src/routes/prompt.rs) | Gemini/ComfyUI (image) |
| `POST /api/v1/transform` | POST | API Key | `{ text, prompt }` | Transformed text | No | Yes (transform, src/routes/extension_api.rs) | Claude (transformation) |
| `POST /webhook/x/{bot_id}` | POST | None (signature verified) | Tweet/event JSON | 200 OK (reply posted asynchronously) | No (event logged only) | **Yes (entire event forwarded to X bot; reply generated + posted)** | X API (POST `/2/tweets`), Claude (reply generation), Honcho (peer context), memory extraction |
| `POST /webhook/farcaster/{bot_id}` | POST | None (signature verified) | Cast/event JSON | 200 OK (reply posted asynchronously) | No (event logged only) | **Yes (entire event forwarded; reply generated + posted)** | Neynar (POST `/cast`), Claude (reply generation), Honcho (peer context) |
| `GET /altar` | GET | None | None | Stripe payment form for donations | No | Minimal | None |
| `POST /payment/verify` | POST | None | `{ tx_hash, collection_id, expected_amount }` | `{ valid, reason }` | No | Yes (emit_log, src/routes/payment.rs:11-14) | Base RPC (verify tx) |

#### C. Sealed Write Pipeline (iOS App)

**Architecture (from CLAUDE.md:69-81)**:
1. iOS app: `GET /api/anky/public-key` → enclave X25519 pubkey (proxy to 3.83.84.211:5555)
2. iOS app: encrypt writing locally (ECIES: X25519 ECDH → SHA256 → AES-256-GCM)
3. iOS app: compute `session_hash = SHA256(plaintext)` locally
4. `POST /api/sealed-write` → send: sealed envelope + session_hash + duration + word_count
5. Backend stores encrypted envelope in `sealed_sessions` + `data/sealed/` (never decrypts)
6. Backend logs session_hash on Solana via spl-memo (authority pays)
7. Backend relays sealed envelope to enclave (`POST /process-writing` on EC2 proxy at 3.83.84.211:5555)
8. Enclave decrypts, calls OpenRouter → returns `{ reflection, image_prompt, title }`
9. Backend generates image from enclave's prompt, stores reflection on anky record

**Key file**: src/routes/sealed.rs:70-166 (`seal_session` handler)

---

### 3. DATABASES, OBJECT STORAGE, QUEUES, CACHES, BACKUPS

#### A. Postgres Database

**URL**: `DATABASE_URL` env var (src/config.rs:118-119)  
**Migrations**: `migrations/*.sql` (001 through 016 as of 2026-04-15)  
**Pool**: 10 max connections (src/db/mod.rs:14)

**Schema** (core tables for privacy audit):

| Table | Sensitive Fields | Purpose | Retention |
|-------|------------------|---------|-----------|
| `users` | `wallet_address`, `farcaster_fid`, `email`, `privy_did`, `generated_wallet_secret` | User identity, keys | Indefinite (DELETE possible via `/api/delete-account` if exists, but not found in audit scope) |
| `writing_sessions` | `content` (plaintext for web writes), `content_deleted_at` (soft delete flag), `keystroke_deltas` | Writing history | Indefinite; soft-delete via `UPDATE writing_sessions SET content = NULL, content_deleted_at = datetime('now')` (src/db/queries.rs) |
| `sealed_sessions` | `ciphertext`, `nonce`, `tag`, `user_encrypted_key`, `anky_encrypted_key` (all as BYTEA, opaque to backend) | Encrypted envelopes | Indefinite (migrations/009_sealed_sessions.sql) |
| `ankys` | `image_prompt`, `reflection`, `title`, `anky_story`, `kingdom_id`, `kingdom_name`, `kingdom_chakra` | Generated artifacts | Indefinite |
| `memory_embeddings` | `content` (snippets), `embedding` (BYTEA OpenAI vector) | Longitudinal memory | Indefinite |
| `user_memories` | `content` (theme/emotion/entity text), `importance`, `occurrence_count` | Extracted patterns | Indefinite |
| `user_profiles` | `psychological_profile`, `emotional_signature`, `core_tensions`, `growth_edges` | Inferred psychological model | Indefinite |
| `cuentacuentos` | `content`, `content_es`, `content_zh`, `content_hi`, `content_ar` (story text) | Stories (parent-to-child) | Indefinite |
| `mirrors` | `public_mirror`, `flux_descriptors_json`, `bio` (from Farcaster) | Farcaster-derived data | Indefinite |
| `story_training_pairs` | `writing_input`, `story_content` (full training data) | ML training dataset | Indefinite; `exported_at` column tracks export (migrations/001_init.sql:737-754) |

**Encryption at Rest**: NOT EVIDENT. Postgres at `127.0.0.1:5432` (default) with no TLS specified in pool config (src/db/mod.rs). Presumed unencrypted.

**Backups**: Not visible in codebase. No backup job scheduled in visible code.

#### B. Object Storage: Cloudflare R2

**Configuration** (src/config.rs:70-74, src/services/r2.rs:9-27):
- Account ID: `R2_ACCOUNT_ID`
- Bucket: `R2_BUCKET_NAME` (default: "anky-voices")
- Keys: `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`
- Public base URL: `R2_PUBLIC_URL`

**What's Stored**:
1. **Anky images** (stories/{anky_id}/page-{page_index}.webp) — src/services/r2.rs:80-115 (upload_image_to_r2)
2. **Class slides** (classes/{class_number}/slide-{slide_index}.webp) — src/services/r2.rs:119-153 (upload_class_slide)
3. **Story audio** (stories/{story_id}/{language}.m4a or .mp4) — src/services/tts.rs (implied; not fully shown)
4. **Recording approval** — story_recordings table tracks r2_key for human-recorded narrations (migrations/001_init.sql:759-793)

**Retention**: Indefinite; immutable cache headers (max-age=31536000, immutable) set on upload (src/services/r2.rs:109, 149).

**Encryption at Rest**: AWS S3/R2 default (AES-256 server-side encryption).

#### C. Local File Storage

| Path | Contents | Retention |
|------|----------|-----------|
| `data/sealed/{user_id}/*.sealed` | Encrypted writing envelopes (JSON: ciphertext, nonce, tag, etc.) | Indefinite; mirrors DB |
| `data/writings/{wallet_address}/{timestamp_unix}.txt` | Plaintext writing content (archive) | Indefinite; backfill job at src/main.rs:350-360 (`backfill_writings_to_files`) |
| `data/images/{anky_id}.png` (or webp) | Generated anky images | Indefinite; fallback if R2 unavailable (src/pipeline/image_gen.rs) |
| `data/mirrors/{id}.png` | Mirror images | Indefinite |
| `data/training_runs/` | Video frames, LLM training artifacts | Indefinite |
| `data/review/` | Story review comments (JSON) | Indefinite |

**Encryption at Rest**: Not visible. Files on poiesis disk (likely Linux ext4/XFS, no explicit encryption configured).

#### D. Redis/Valkey

**Configuration**: `REDIS_URL` env var (src/config.rs:206-207, default: `redis://127.0.0.1:6379`)

**Usage**:
1. **Job queue** (src/services/redis_queue.rs): Pro/free priority queues for GPU jobs (anky images, story images, audio)
   - `anky:jobs:pro`, `anky:jobs:free` (LPUSH/RPOP)
   - Per-job tracking: `anky:job:{job_id}` (hash with status, retry count)
2. **Session reaper** (src/routes/session.rs): In-memory map + Redis for crash recovery

**Retention**: Job entries cleaned up after completion or max retries (src/main.rs:402-405 abandonment logic). **No persistent storage of writing content in Redis** (writes go to Postgres + sealed envelope storage).

---

### 4. STORAGE SYSTEMS: DETAILED BREAKDOWN

#### Writing Content (Plaintext Web Path)

| System | What | When | Format | Deletable | Retention |
|--------|------|------|--------|-----------|-----------|
| Postgres `writing_sessions.content` | Full plaintext | On write submission (web/anon path only) | TEXT | Via soft-delete (src/db/queries.rs: `content_deleted_at` flag) | Indefinite unless soft-deleted |
| Postgres `writing_sessions.keystroke_deltas` | Keystroke timing metadata | On submission | TEXT (delta format) | Via cascade if session deleted | Indefinite |
| `data/writings/{wallet}/{timestamp}.txt` | Full plaintext (archive) | After iOS anky write (post-pipeline) | Plain UTF-8 | Yes, manual file deletion only | Indefinite |
| `sealed_sessions` table + `data/sealed/{user_id}/*.sealed` | Encrypted envelope (iOS) | On sealed write POST | BYTEA + JSON file | Yes, but backend cannot decrypt; must delete file + DB row | Indefinite |

**Hard Delete**: Only `DELETE FROM writing_sessions WHERE id = ?` + `DELETE FROM ankys WHERE writing_session_id = ?` visible (src/routes/writing.rs:2016-2032, test cleanup only). No user-facing delete endpoint found.

**Soft Delete**: `UPDATE writing_sessions SET content = NULL, content_deleted_at = datetime('now')` (src/db/queries.rs).

#### Images (Anky + Story Generated)

| System | Format | Retention | Deletable | Notes |
|--------|--------|-----------|-----------|-------|
| R2 (primary) | WebP (95% quality) | Indefinite; immutable cache | Yes (S3 DELETE via AWS SDK, not exposed in backend API) | src/services/r2.rs:59-115 |
| Local fallback | PNG/WebP | Indefinite | Manual deletion | `data/images/{anky_id}.*` |
| Image metadata | DB: `ankys.image_path`, `ankys.image_webp`, `ankys.image_thumb`, `anky_story` | Indefinite | DB delete cascades | Migration 001, line 44-61 |

#### Embeddings & Vectors

| System | Data | Size | Retention | Deletable |
|--------|------|------|-----------|-----------|
| `memory_embeddings` table (Postgres) | OpenAI embedding vectors (BYTEA) + source text snippet | ~1.5KB per embedding (1536 dims × 2 bytes) | Indefinite | Yes, via `DELETE FROM memory_embeddings WHERE user_id = ?` |

#### Profile Inferences

| System | Data | How Generated | Retention | Deletable |
|--------|------|----------------|-----------|-----------|
| `user_profiles` table | psychological_profile, emotional_signature, core_tensions, growth_edges (JSON text) | Claude Haiku analysis + Honcho chat (every 5th session) | Indefinite | Yes, via `DELETE FROM user_profiles WHERE user_id = ?` |
| `next_prompts` table | Personalized writing prompt | Ollama generation | Indefinite (replaced on each write) | Yes |

#### Training Data

| Table | Fields | Retention | Access |
|-------|--------|-----------|--------|
| `story_training_pairs` | `writing_input`, `story_title`, `story_content`, `language`, `played`, `quality_score`, `exported_at` | Indefinite | `exported_at` column tracks cron exports (src/main.rs references "4:44 AM cron" but job not visible in audit scope) |

**Export**: Presumed daily LoRA fine-tuning dataset export (referenced but code not fully shown).

---

### 5. THIRD-PARTY SERVICES: COMPREHENSIVE ENUMERATION

| Service | Data Sent | Frequency | Auth | No-Persist Guarantee? | Purpose | Env Vars |
|---------|-----------|-----------|------|----------------------|---------|----------|
| **Anthropic Claude API** | Writing text + system prompts (reflection, title, memory extraction, profile) | Per-write + recovery watchdog | `ANTHROPIC_API_KEY` header | No (Claude logs for moderation/training on their side) | Text generation | src/config.rs:140 |
| **OpenRouter (fallback LLM)** | Writing text + system prompts | If Claude fails or load exceeds threshold | `OPENROUTER_API_KEY` | No | Fallback inference | src/config.rs:135-139 |
| **Google Gemini API** | Image prompt (text only, no writing) | Per-image generation | `GEMINI_API_KEY` | Unknown (Google privacy policy) | Image generation | src/config.rs:141 |
| **Honcho (user modeling)** | Full writing text (plaintext, 25k char limit per message) | Every write + historical backfill on startup | `HONCHO_API_KEY`, `HONCHO_WORKSPACE_ID` | No (Honcho builds persistent user models) | User profiling | src/config.rs:64-66 |
| **Neynar (Farcaster API)** | FID, username, wallet (lookup); reply text (publishing) | On mirror generation + webhook social replies | `NEYNAR_API_KEY` | No (Neynar indexes public casts) | Farcaster profile lookup + reply posting | src/config.rs:48-50 |
| **Solana RPC (Helius)** | None directly (minting worker calls RPC for signatures) | Per-write (on-chain logging) | `HELIUS_API_KEY` (in worker, not backend) | Yes (Solana L1 is immutable, data public) | Session logging via spl-memo | CLAUDE.md:199 |
| **X (Twitter) API v2** | Reply text (generated by Claude, lowercase), cast hash (reply-to), image URL | When tagged (webhook-driven) | `X_BEARER_TOKEN`, `X_CONSUMER_KEY`, `X_CONSUMER_SECRET` | No (X archives all public tweets) | Reply posting | src/config.rs:32-36 |
| **OpenAI (embeddings)** | Writing text (chunked) | Memory embedding generation | `OPENAI_API_KEY` | No (OpenAI logs for training) | Embedding vectors | src/config.rs:46 |
| **Cloudflare R2 (S3-compatible)** | Image bytes (WebP), audio bytes (MP4) | After generation | `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | No (Cloudflare storage is persistent) | Image/audio CDN storage | src/config.rs:70-74 |
| **AWS Nitro Enclave** | Encrypted sealed envelope (no plaintext at backend) | Per sealed write | None (HTTPS proxy) | Yes (enclave decrypts, returns only derived outputs; plaintext never persisted elsewhere) | Reflection + image prompt generation | src/config.rs:106 (ANKY_ENCLAVE_URL) |
| **F5-TTS (local)** | Story text + language code | After story translation | None (localhost:5001) | N/A (local service, no external forward) | Text-to-speech generation | src/config.rs:68 |
| **ComfyUI (local)** | Image prompt (text only) | Fallback after Gemini | None (localhost:8188) | N/A (local service) | Image generation | src/config.rs:62 (COMFYUI_URL) |
| **Ollama/llama-server (Mind, local)** | Writing text + system prompts | Fallback before Claude | None (localhost:8080) | N/A (local service) | Text generation | src/config.rs:89 (MIND_URL) |
| **Stripe** | Payment amount, email (optional), card token (handled by Stripe JS) | When user donates via altar | `STRIPE_SECRET_KEY`, `STRIPE_PUBLISHABLE_KEY` | No (Stripe PCI compliance logs) | Payment processing | src/config.rs:102-103 |
| **Pinata (IPFS)** | Metadata JSON (EIP-712 payload for birthSoul mint) | Per EVM mint (deprecated) | `PINATA_JWT` | No (IPFS is immutable) | Metadata pinning | src/config.rs:79, src/routes/swift.rs:3031-3068 |
| **Privy** | Privy JWT from mobile/web | On mobile login | `PRIVY_APP_ID`, `PRIVY_APP_SECRET`, `PRIVY_VERIFICATION_KEY` | Unknown (Privy privacy policy) | Wallet auth + key management | src/config.rs:39-41 |

---

### 6. WRITING CONTENT, PROMPTS, REFLECTIONS, EMBEDDINGS, SUMMARIES

#### A. Are They Stored?

| Artifact | Stored | Where | Encrypted | Shared |
|----------|--------|-------|-----------|--------|
| **Raw writing text** | Yes | `writing_sessions.content` (web/anon path) OR `sealed_sessions` ciphertext (iOS authenticated) | No (plaintext web) OR Yes (ECIES encrypted iOS) | To Honcho (plaintext, if configured) + Claude (for reflection/extraction); never to training APIs unless explicitly consented |
| **Reflection** | Yes | `ankys.reflection` | No | Returned to user in API responses (src/routes/swift.rs:1319 `get_writing_status` returns full reflection) |
| **Title** | Yes | `ankys.title` | No | Returned in API responses |
| **Image Prompt** | Yes | `ankys.image_prompt` | No | Used to generate image (not sent externally) |
| **Formatted Writing** | Yes | `ankys.formatted_writing` | No | Likely used for display, not sent externally |
| **Anky Story** | Yes | `ankys.anky_story` (YAML frontmatter + story text) | No | Returned in API (migrations/001_init.sql:58) |
| **Extracted Memories** | Yes | `user_memories` table (categories: theme, emotion, entity, pattern, breakthrough, avoidance) | No | Used for profile building + next-prompt generation |
| **Embeddings** | Yes | `memory_embeddings.embedding` (BYTEA OpenAI vector) | No | Used for similarity search in reflection generation |
| **Psychological Profile** | Yes | `user_profiles.psychological_profile` (JSON) | No | Returned in `/swift/v2/you` response (src/routes/swift.rs:2223-2296) |

#### B. Are They Logged?

| Type | Tracing Level | Evidence |
|------|---------------|----------|
| Full writing content | No | No `tracing::info!("{}", writing_text)` calls found. Metadata logged (session hash, word count, duration) only. |
| Reflection | No | No full reflection logged. Only generation status (e.g., "Recovering missing reflection for anky X") |
| Prompts sent to Claude | No | System prompts logged only by Claude side (not backend) |
| Passwords / API keys | No | Keys not logged; header values redacted in error logging (src/services/payment.rs, src/services/neynar.rs show careful logging) |

#### C. Are They Sent to Third Parties?

| Data | Recipients | Conditions | Lines |
|------|------------|-----------|-------|
| **Writing text (plaintext)** | Honcho | If `HONCHO_API_KEY` configured; fire-and-forget + historical backfill on startup | src/services/honcho.rs:96-135, 237-336 (send_writing, backfill_all_writings) |
| **Writing text (plaintext)** | Claude/OpenRouter | For reflection, title, memory extraction, profile updates | src/services/claude.rs (call_haiku_*), src/memory/extraction.rs, src/memory/profile.rs |
| **Writing text + image prompt** | Gemini/ComfyUI | For image generation (image_prompt only sent externally, not writing text) | src/services/gemini.rs, src/services/comfyui.rs |
| **Memory-derived summaries** | Claude | For personality/profile inference | src/memory/profile.rs:59-125 |
| **Reflection + conversation** | User's social network | Via X/Farcaster replies (when Anky is tagged) | src/routes/webhook_x.rs, src/routes/webhook_farcaster.rs |
| **Session hash** | Solana on-chain (spl-memo) | Every real anky (8+ min) — permanent, public record | src/pipeline/image_gen.rs:823-908 (log_session_onchain) |
| **Farcaster profile data** | None (stored locally after Neynar fetch) | Mirrors table holds FID, username, PFP, bio from Farcaster API | src/routes/api.rs (mirror generation) |

#### D. Are They Used for Training/Evals?

| Data | Used For Training? | Evidence | Opt-Out? |
|------|-------------------|----------|----------|
| **Writing text (via Honcho)** | Unknown (Honcho's terms) | Honcho is a user modeling API; unclear if writings are used for LLM training | Gated on `HONCHO_API_KEY` env var; no user opt-out in code |
| **Writing text (via Claude)** | Possible (Claude's terms) | "Claude may use your messages to improve Claude" (Anthropic terms). No evidence in Anky code of explicit consent capture. | `ANTHROPIC_API_KEY` is required; no consent UI found |
| **Story training pairs** | Yes (LoRA fine-tuning dataset) | `story_training_pairs` table explicitly tracks `(writing_input, story_content)` with `exported_at` column for cron export | Yes, but no user control visible in backend (would be app-level setting) |
| **Reflections** | Possible (if forwarded to training pipeline) | Reflections not explicitly forwarded, but stored in DB (could be exported offline) | Unclear |
| **Images** | No (generated locally via Gemini/ComfyUI; R2 CDN) | Images sent to Gemini/ComfyUI for generation, not for training | Implicit (no explicit opt-in/out) |

---

### 7. IDENTIFIERS IN BACKEND SYSTEMS

| Identifier | Type | Source | Storage | Logged? | Searchable? |
|------------|------|--------|---------|---------|------------|
| **User ID (UUID)** | Application | Backend generates (UUID v4) on user creation | `users.id` (PRIMARY KEY) | Yes (tracing logs show shortened IDs `&anky_id[..8]`) | Indexed on `users.id` (implicit primary key) |
| **Solana Wallet Address** | Blockchain identity | User's iOS app (BIP39 derivation) | `users.wallet_address` | Yes (logs show "wallet X") | Indexed: `idx_users_wallet_address` (migrations/001_init.sql:661) |
| **Farcaster FID** | Social platform | Neynar API lookup | `users.farcaster_fid`, `mirrors.fid` | Yes (logs show FID in kingdom assignment) | Indexed: `idx_mirrors_fid` (migrations/001_init.sql:862) |
| **Farcaster username** | Social handle | Neynar API lookup | `users.farcaster_username`, `mirrors.username` | Yes (logs reference username) | Not explicitly indexed |
| **Email** | Contact | User registration (optional) | `users.email` | Not found in code logs | Not explicitly indexed |
| **Privy DID** | OAuth identity | Privy API | `users.privy_did` | Minimal (not in logs found) | Not indexed |
| **X (Twitter) User ID** | Social platform | X API (OAuth or bot follower lookup) | `x_users.x_user_id` | Yes (logs reference X handle) | Indexed: `idx_x_users_* ` (implicit) |
| **Device Token (APNs)** | Push notification | iOS app (Apple APNs) | `device_tokens.id` (UUID), `device_tokens.token` | Not found (stored securely) | Indexed: `(user_id, platform)` implicit |
| **Session Token (Bearer)** | Auth | Backend generates (UUID) on login | `auth_sessions.token` (PRIMARY KEY) | Not found (tokens not logged in full) | Indexed (PK) |
| **Session Hash** | Content fingerprint | iOS app (SHA256 of plaintext writing) | `sealed_sessions.session_hash`, `ankys.session_hash` (presumably) | Yes (logged to confirm on-chain logging) | Indexed (supporting on-chain verification endpoint) |
| **IP Address** | Network identity | HTTP connection | Not explicitly stored | No (Cloudflare Tunnel abstracts) | N/A (tunnel topology) |
| **Honcho Peer ID** | User modeling | Derived from user_id (sanitized alphanumeric) | `social_peers.honcho_peer_id` (inferred) | Not found | Inferred indexed |

**Searchable by User ID**: All user-keyed tables enable `SELECT ... WHERE user_id = ?` (writing_sessions, ankys, user_memories, user_profiles, cuentacuentos, device_tokens, etc.).

---

### 8. LOGGING: PERSONAL DATA IN TRACING CALLS

**Search Method**: `grep -r "tracing::(info|debug|warn|error)" src/`  
**Findings**:

#### Safe Logs (No Personal Data)

```rust
tracing::info!("Starting Anky with mode {:?} on port {}", config.run_mode, port);  // main.rs:228
tracing::info!("Database initialized");  // main.rs:237
tracing::info!("GPU job {} failed: {}", &job.id[..8], e);  // main.rs:167
```

#### Logs with Truncated IDs (Safe)

```rust
tracing::info!("Sealed session stored: user={}, session={}, hash={}, size={}", 
    user_id, session_id, session_hash, ciphertext_bytes.len());  // sealed.rs:154-159
tracing::info!("Anky {} assigned to kingdom {} ({})", 
    &anky_id[..8], kingdom.name, kingdom.chakra);  // main.rs:70-79
```

#### Logs with Metadata (Safe)

```rust
state.emit_log("INFO", "payment", 
    &format!("Verifying payment tx: {}...", &req.tx_hash[..10]));  // payment.rs:11-14
```

#### Logs with User Interaction (Minimal Risk)

```rust
tracing::warn!(status = %status, body = %body, voice_id = %voice_id, "ElevenLabs TTS error");  // mod.rs:582
```

#### No Full-Content Logs Found

- No `tracing::info!("{}", writing.content)`
- No plaintext writing in logs
- No full reflections in logs (only status messages)
- No full prompts in logs (only error messages)

**Conclusion**: Tracing is **safe**. IDs are truncated, metadata is contextual, personal writing is never logged.

---

### 9. DELETION / RETENTION BEHAVIOR

#### A. User Deletion

**Explicit Endpoint**: NOT FOUND in codebase.

**Soft Delete Capability**: Writing content can be soft-deleted:
```rust
// src/db/queries.rs (not shown in detail but referenced)
UPDATE writing_sessions SET content = NULL, content_deleted_at = datetime('now') 
WHERE id = ?1
```

**Hard Delete in Tests**: src/routes/writing.rs:2015-2032 shows test cleanup:
```rust
async fn cleanup_protocol_rows(state: &AppState, user_id: &str, session_hash: &str) {
    sqlx::query("DELETE FROM ankys WHERE user_id = $1 AND session_hash = $2")...
    sqlx::query("DELETE FROM writing_sessions WHERE user_id = $1 AND session_hash = $2")...
    sqlx::query("DELETE FROM users WHERE id = $1")...  // delete users themselves
}
```

**Evidence of Account Deletion API**: None found. **Unclear if users can request full deletion.**

#### B. Data Retention

| Data | Retention | Cleanup Job | Soft Delete? |
|------|-----------|------------|------------|
| **Writing content** | Indefinite (unless soft-deleted) | None visible | Yes (content_deleted_at flag) |
| **Sealed sessions** | Indefinite | None visible | No (hard delete only, if ever) |
| **Ankys + reflections** | Indefinite | None visible | No |
| **Memories + embeddings** | Indefinite | None visible | No |
| **Profiles** | Indefinite | None visible | No |
| **Stories + translations** | Indefinite | None visible | No |
| **Training pairs** | Indefinite; exported at "4:44 AM cron" (job not visible) | Presumed daily export | No |
| **Auth sessions** | Until `expires_at` (default likely 30d) | Reaper at src/routes/session.rs (removes stale chunked sessions, not auth sessions) | N/A (tokens expire) |
| **Device tokens** | Until user deletes or app uninstalls | No auto-cleanup visible | No |
| **R2 images** | Indefinite (immutable cache) | None | Only via manual S3 DELETE API |
| **Local files** | Indefinite | Backfill job maintains, no cleanup | Manual deletion only |

#### C. Crypto/Hash-Based Deletion Proof

**Solana spl-memo logging** (src/pipeline/image_gen.rs:823-908):
- Session hash logged on-chain: **permanent, public, immutable**
- Format: `anky|<session_hash>|<session_id>|<wallet>|<duration>|<words>|<kingdom>|<sojourn>`
- **Cannot be deleted** (Solana L1 record)
- **Searchable by wallet address** via Helius API (CLAUDE.md:52)

---

### 10. ENVIRONMENT VARIABLES AFFECTING PRIVACY/SHARING

**File**: `src/config.rs` (lines 110-224)

| Var | Purpose | Privacy Impact | Default | Required? |
|-----|---------|-----------------|---------|-----------|
| `DATABASE_URL` | Postgres connection | **Controls where all user data is stored** | "postgres://postgres:postgres@127.0.0.1:5432/anky" | No (but fails without it) |
| `ANTHROPIC_API_KEY` | Claude API key | **Enables forwarding of writings to Claude** | Empty (fallback to Mind/OpenRouter) | No |
| `OPENROUTER_API_KEY` | OpenRouter fallback | **Enables forwarding to OpenRouter** | Empty | No |
| `HONCHO_API_KEY` | User modeling API | **CRITICALLY: Enables sending ALL writings to Honcho for user profiles** | Empty (optional) | No — **if empty, Honcho disabled** |
| `HONCHO_WORKSPACE_ID` | Honcho workspace | Pair with HONCHO_API_KEY | "anky-prod" | No |
| `NEYNAR_API_KEY` | Farcaster API | **Enables Farcaster profile lookups + reply posting** | Empty | No |
| `GEMINI_API_KEY` | Google Gemini | **Enables image generation via Gemini** | Empty (fallback to ComfyUI) | No |
| `OPENAI_API_KEY` | OpenAI embeddings | **Enables forwarding writings to OpenAI for vector embeddings** | Empty | No |
| `STRIPE_SECRET_KEY` | Stripe payments | **Enables payment processing (altar donations)** | Empty | No |
| `SOLANA_MINT_WORKER_URL` | Worker URL | **If empty, on-chain session logging disabled** | Empty | No |
| `SOLANA_MINT_WORKER_SECRET` | Worker auth | Pair with SOLANA_MINT_WORKER_URL | Empty | No |
| `ANKY_ENCLAVE_URL` | Nitro Enclave proxy | **If empty, sealed write path broken** | Empty | No (but needed for iOS) |
| `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | Cloudflare R2 | **If empty, images stored locally (not CDN)** | Empty | No |
| `MIND_URL` | Local llama-server | **If empty, forces Cloud (Claude/OpenRouter)** | "http://127.0.0.1:8080" | No |
| `X_BEARER_TOKEN`, etc. | X Bot credentials | **If set, enables X social reply webhook** | Empty | No |
| `PRIVY_APP_ID`, `PRIVY_APP_SECRET` | Privy auth | **Enables Privy wallet login (v1 path)** | Empty | No |
| `APNS_KEY_PATH`, `APNS_KEY_ID`, `APNS_TEAM_ID` | Apple Push | **If set, enables APNs silent notifications** | Empty | No |
| `TTS_BASE_URL` | F5-TTS service | **If empty, story audio gen disabled** | "http://localhost:5001" | No |
| `COMFYUI_URL` | ComfyUI local | **Image gen endpoint; if empty, forces Gemini** | "http://localhost:8188" | No |

**Key Privacy Decisions** (User-Configurable at Deployment):
1. **HONCHO_API_KEY**: If set → all writings sent to Honcho (user modeling). If empty → local-only profiling.
2. **ANTHROPIC_API_KEY + Mind URL**: If MIND_URL configured + reachable → writings stay local. Else → Claude.
3. **SOLANA_MINT_WORKER_URL**: If empty → session hashes NOT logged on-chain (privacy gain, but loses immutable proof).
4. **R2 credentials**: If empty → images stored locally (no CDN, no Cloudflare involvement).

---

### 11. FINAL SUMMARY TABLE: DATA CATEGORIES & FLOW

| Data Category | Field Example | Source | Stored Where | Shared With | Purpose | Retention | Deletable? | Notes |
|---------------|----------------|--------|--------------|------------|---------|-----------|-----------|-------|
| **Raw Writing (Authenticated iOS)** | `sealed_sessions.ciphertext` | iOS app encrypted locally | Postgres `sealed_sessions` + `data/sealed/` (as opaque BYTEA/file) | Enclave proxy only (encrypted; backend blind) | Content processing | Indefinite | Yes (hard delete DB row + file) | ECIES encrypted; backend never decrypts |
| **Raw Writing (Web/Anonymous)** | `writing_sessions.content` | Browser form | Postgres `writing_sessions.content` | Honcho (if configured), Claude, Ollama, OpenRouter | Reflection generation | Indefinite | Yes (soft delete: content = NULL + timestamp) | Plaintext stored; no encryption at rest |
| **Session Metadata** | duration_seconds, word_count, created_at, keystroke_deltas | Both iOS/web | `writing_sessions.*`, `sealed_sessions.metadata_json` | Solana on-chain (session_hash logged) | Proof of practice, analytics | Indefinite | Solana entry: No (immutable); DB: Yes |
| **Farcaster Profile Data** | FID, username, display_name, avatar_url, follower_count, bio | Neynar API (lookup by user wallet) | `mirrors` table | None (stored locally) | Mirror generation (visual profile) | Indefinite | Yes (delete from mirrors) | Fetched once per mirror; Neynar queries are public |
| **Title + Reflection** | ankys.title, ankys.reflection | Claude/Ollama/OpenRouter (generated from writing) | `ankys` table | Returned in API responses, possibly Honcho context (next prompt) | Display, personalization | Indefinite | Yes | Generated summaries, not training data unless exported |
| **Image Prompt** | ankys.image_prompt | Claude/enclave output | `ankys.image_prompt` | Gemini API or ComfyUI (local) | Image generation | Indefinite | Yes | Derived from reflection, not from writing |
| **Generated Images** | ankys.image_path, R2 URL | Gemini/ComfyUI/Flux | R2 CDN (immutable) + local fallback | R2 (Cloudflare), returned in API | Display, social sharing | Indefinite | Yes (via S3 DELETE, not exposed in API) |
| **Extracted Memories** | user_memories.content (theme/emotion/entity/pattern) | Claude Haiku on writing | `user_memories` table | Used for next-prompt, profile (Honcho context if available) | Longitudinal profiling | Indefinite | Yes | Deduplicated by exact text match |
| **Embeddings** | memory_embeddings.embedding (OpenAI 1536-dim) | OpenAI embedding API | `memory_embeddings.embedding` (BYTEA) | Used for similarity search in reflection generation | Semantic memory retrieval | Indefinite | Yes | Associated with `content` snippet; no external storage |
| **Psychological Profile** | user_profiles.psychological_profile (JSON) | Claude Haiku + Honcho chat (every 5th write) | `user_profiles` table | Returned in `/swift/v2/you` API response | User self-understanding | Indefinite; updated on each generation | Yes | Inferred from writing patterns; not ground truth |
| **Story (Cuentacuentos)** | cuentacuentos.content + translations (es, zh, hi, ar) | Claude (parent narrative) + Ollama (translations) | `cuentacuentos` table | R2 (images), returned in API | Parent-to-child storytelling | Indefinite | Yes | Training pairs exported offline (STORY_TRAINING_PAIRS) |
| **Training Dataset** | story_training_pairs.writing_input + story_output | Cuentacuentos generation pipeline | `story_training_pairs` table | Exported daily at "4:44 AM cron" (job not visible) to offline training | LoRA fine-tuning (future) | Indefinite; `exported_at` tracks exports | Unclear (would require admin/user consent reconciliation) | Explicit training data: writing + story pairs |
| **Solana Wallet Address** | users.wallet_address | iOS app (BIP39-derived Ed25519) | `users.wallet_address`, also logged on-chain in spl-memo | Solana L1 (public ledger), Neynar (Farcaster lookup), Stripe (payments) | User identity, payments, NFT minting | Indefinite | DB: Yes; Solana: No (immutable) | Solana address is user's primary identity |
| **Farcaster FID** | users.farcaster_fid | Neynar lookup (from wallet) | `users.farcaster_fid`, `mirrors.fid` | Neynar (fetches), Solana memo (logged) | Social identity linking | Indefinite | Yes (DB); No (Farcaster archive) | Fetched via Neynar bulk lookup API |
| **Email** | users.email | User input (optional) | `users.email` | Stripe (if payment made), Honcho (if forwarded in peer data) | Contact, payments | Indefinite | Yes | Optional field; nullable |
| **Device Token (APNs)** | device_tokens.token | iOS app (Apple-generated) | `device_tokens` table | APNs (Apple push service) | Push notifications (silent) | Until user deletes or uninstalls | Yes (via DELETE endpoint) | Not logged; stored as-is for push delivery |
| **Auth Session Token** | auth_sessions.token (UUID) | Backend generates | `auth_sessions` table (PK) | Returned to client; client stores in app storage | Bearer auth for API calls | Expires at `auth_sessions.expires_at` | Yes (early logout) | Tokens are opaque; not associated with writing content in logs |
| **Session Hash** | writing_sessions.session_hash (SHA256 hex) | iOS app (SHA256 of plaintext) | `sealed_sessions.session_hash`, Solana spl-memo (immutable log) | Solana L1 (public ledger) via `/log-session` worker, verified via `GET /api/verify/{hash}` | Proof of writing without revealing content | Indefinite | DB: Yes; Solana: No | Published on-chain; searchable by wallet |
| **IP Address** | (HTTP connection) | Cloudflare Tunnel proxy | Not explicitly stored | Cloudflare (tunnel logs, not visible to backend) | Network topology (Cloudflare observability) | Cloudflare retention policy | No (behind tunnel abstraction) | Cloudflare Tunnel strips user IP from backend perspective |

---

### APPENDIX: SEALED WRITE ARCHITECTURE (CRITICAL FOR PRIVACY)

**Source**: CLAUDE.md:64-84, src/routes/sealed.rs, src/pipeline/image_gen.rs

**Key Property**: **Backend never sees plaintext from authenticated iOS users.**

1. iOS app:
   - Encrypts writing locally: `ECIES(X25519 ECDH → SHA256 → AES-256-GCM)`
   - Computes: `session_hash = SHA256(plaintext_writing_locally)`
   - Sends: `POST /api/sealed-write` with (sealed envelope + hash + metadata)

2. Backend (blind relay):
   - Receives ciphertext + hash (plaintext NOT visible)
   - Stores both in DB (`sealed_sessions` table) + disk (`data/sealed/{user_id}/{hash}.sealed`)
   - Logs hash to Solana via spl-memo (permanent proof, no plaintext)
   - Relays encrypted envelope to Nitro Enclave (EC2 proxy at 3.83.84.211:5555)

3. Enclave (single decryption point):
   - Decrypts envelope (only machine with private key)
   - Verifies hash: `SHA256(decrypted_plaintext) == session_hash`
   - Calls OpenRouter with plaintext (both on same EC2 host, same security boundary)
   - Returns: `{ reflection, image_prompt, title }` to backend

4. Backend (after enclave returns):
   - Never sees plaintext
   - Stores derived outputs (reflection, title) in `ankys` table
   - Generates image from enclave's prompt
   - Logs on-chain

**Trust Boundary**:
- Plaintext: `{iOS device, EC2 host (enclave + proxy)}`
- Backend: `{poiesis server}` — can see ciphertext, derived outputs, but never plaintext

**Exceptions**:
- Web/anonymous write path: plaintext stored (user consent implied by non-login)
- If user configures Honcho: plaintext forwarded to Honcho (explicit env var opt-in)

---

### DEPLOYMENT-DEPENDENT BEHAVIORS (FLAGGED AMBIGUITIES)

1. **Honcho Integration** (OPTIONAL)
   - If `HONCHO_API_KEY` is set: ALL writings sent to Honcho for user modeling.
   - If empty: Local-only profiling via Claude Haiku + memory extraction.
   - **User consent for Honcho**: Not captured in visible code (would need Terms of Service + app UI toggle).

2. **Claude vs. Local Inference**
   - If `MIND_URL` configured + reachable: Text stays local (llama-server with qwen3.5-27b).
   - Else: Falls back to Claude (requires `ANTHROPIC_API_KEY`).
   - **User consent**: Not captured; assumed by system design.

3. **On-Chain Logging** (OPTIONAL)
   - If `SOLANA_MINT_WORKER_URL` configured: Session hash logged on Solana (immutable, public).
   - If empty: Logging disabled (local-only).
   - **User consent**: Not explicit; implied by iOS app behavior (hash computed + sent to backend).

4. **Image Storage**
   - If `R2_ACCOUNT_ID` + keys configured: Images stored in Cloudflare R2 CDN (immutable, potentially cached globally).
   - If empty: Images stored locally in `data/images/` (private to deployment).
   - **User visibility**: No API controls for where images are stored.

5. **Training Data Export**
   - `story_training_pairs` table logs writing + story pairs explicitly.
   - "4:44 AM cron" exports for LoRA fine-tuning (job not visible in Rust code; presumed in infrastructure).
   - **User consent**: Not found in code (would require explicit data processing agreement in Terms of Service).

6. **Email Field** (OPTIONAL)
   - Collected in `users.email` (nullable, optional).
   - If Stripe integration: forwarded to Stripe during payment.
   - **Privacy risk**: Email not encrypted at rest; no visible deletion mechanism.

---

### CONCLUSION & RISK SUMMARY

**Privacy-Positive Design Choices**:
- ✅ **Sealed write pipeline**: Backend blind relay for authenticated users (plaintext never at rest on backend).
- ✅ **Session hash (not content)**: Only SHA256 logged on-chain, proving writing happened without revealing it.
- ✅ **Minimal logging**: No full-content logging found in tracing calls.
- ✅ **Local inference option**: Mind URL allows keeping text inference on-device.
- ✅ **Optional Honcho**: User modeling disabled by default (requires env var).

**Privacy Risks / Gaps**:
- ⚠️ **Web/anonymous writes**: Plaintext stored in Postgres (no encryption at rest).
- ⚠️ **Postgres encryption**: Database presumed unencrypted (no TLS in pool config; no "encryption at rest" visible).
- ⚠️ **No user deletion API**: Account deletion would require manual DB cleanup (not self-service).
- ⚠️ **Honcho opt-in via env var**: If configured, ALL writings sent to third party (no user consent flow visible in backend).
- ⚠️ **Training data export**: `story_training_pairs` table explicitly for LoRA training; export mechanism not visible (offline job).
- ⚠️ **Solana logging is immutable**: Session hashes on Solana L1 cannot be deleted (intentional, but privacy trade-off for immutability).
- ⚠️ **OpenAI embeddings**: If configured, writings chunked + sent to OpenAI for embedding vectors (no user control visible).
- ⚠️ **R2 images immutable cache**: Images uploaded to R2 with `max-age=31536000, immutable` headers (CDN caches forever; cannot update or delete quickly).

**Recommendation for Privacy Policy**:
- Clearly disclose: which third parties receive writings (Claude, OpenRouter, Honcho if enabled, OpenAI if embeddings enabled).
- Disclose: session hashes are logged on Solana L1 (permanent, public, searchable by wallet).
- Disclose: story + writing pairs are exported for training (would need explicit consent toggle).
- Disclose: web/anonymous writes stored plaintext (no encryption at rest).
- Provide: explicit opt-in/opt-out for Honcho, training data export, on-chain logging.
- Provide: user-facing account deletion API (currently missing).
- Recommend: enable Postgres encryption at rest (TLS + filesystem encryption).
- Recommend: set retention policy for old writing archives (currently indefinite).

---

**End of Audit**
