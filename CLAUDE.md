# Anky Backend — Agent Operating Manual

## Shared Memory

Before starting any session, read `CURRENT_STATE.md` in full.
It is the authoritative record of what is working, what is broken, and what is deferred.
Update it at the end of every session that makes meaningful changes.
Do not rely on conversation history.
`CURRENT_STATE.md` is the truth.

## What Anky Is

Anky is a writing-practice system built on the premise that 8 minutes of uninterrupted stream-of-consciousness writing reveals something real about the writer. The system has two surfaces (Farcaster miniapp + iOS app), one identity layer (Solana wallet), and one privacy layer (AWS Nitro Enclave).

The backend never sees plaintext writing from authenticated users. The iOS app encrypts on-device with the enclave's X25519 public key, computes the session hash locally, and sends the sealed envelope to `POST /api/sealed-write`. The backend stores it blind, logs the hash on Solana via spl-memo, and relays the sealed envelope to the enclave for processing. The enclave decrypts, calls OpenRouter for reflection + image prompt, and returns only derived outputs. The backend generates the image from the enclave's prompt. Plaintext never leaves {iOS device, EC2 host}.

**Sojourn 9** is the current 96-day cycle (started 2026-03-03, ends 2026-06-07). 3,456 participants max. Each participant's first writing session mints a Mirror cNFT (their seat). Every subsequent anky mints an Anky cNFT (the artifact).

## The Ankyverse Calendar

The Ankyverse started on **2023-08-10T05:00:00-04:00**. Each cycle = 96-day sojourn + 21-day Great Slumber (117 days total). Within a sojourn, 12 resonance waves of 8 days each cycle through 8 kingdoms:

Primordia (Root) → Emblazion (Sacral) → Chryseos (Solar Plexus) → Eleasis (Heart) → Voxlumis (Throat) → Insightia (Third Eye) → Claridium (Crown) → Poiesis (Transcendent)

The kingdom of the day determines the cNFT metadata for ankys written that day.

## Architecture

- **Backend**: Rust/Axum server on `poiesis`, port 8889. Systemd user service `anky.service`.
- **Database**: Postgres via sqlx (`PgPool`), with a rusqlite-compatible `Connection` wrapper that translates `?1` params to `$1`. Migrations in `migrations/*.sql`, run at startup via `sqlx::migrate!`.
- **Public access**: `anky.app` → Cloudflare tunnel → localhost:8889.
- **Enclave**: AWS Nitro on EC2 (`c6g.xlarge`, `3.83.84.211:5555`). Runs `anky-proxy` (HTTP→vsock bridge) + `anky-soul` (inside enclave). Endpoints: `/health`, `/public-key`, `/attestation`, `/process-writing` (sealed write pipeline — decrypts, calls OpenRouter, returns reflection + image prompt). The proxy has the OpenRouter API key as env var. Plaintext never leaves the EC2 host.
- **Solana mint worker**: Runs locally via `wrangler dev` on port 8787. Systemd service `anky-mint.service`. Mints Bubblegum cNFTs via Metaplex Umi. Secrets in `solana/worker/.dev.vars`.
- **Text inference**: Mind (local llama-server) → Claude → OpenRouter fallback chain.
- **Image generation**: Gemini → Flux/ComfyUI (local GPU 1) fallback. R2 CDN upload when configured.
- **Redis/Valkey**: Job persistence and crash recovery at `REDIS_URL`.

## Two Solana Collections (Devnet)

### Mirrors (Sojourn membership)
- **Tree**: `ArmTPWNskwUsiZErvN1HKbqrupeiavkxVgS1ciLRQK6B` (depth 12, 4,096 capacity)
- **Collection**: `CXYtYYgnXx5Lbn5MmePHSePynjJwGyTSWnzhDBbbn4Dt`
- **Purpose**: 3,456 membership cNFTs. One per user. Minted via `POST /mirror/mint` or `POST /api/mirror/solana-mint`.
- **Gate**: Only users with a Mirror cNFT can persist ankys and receive Anky cNFTs.

### Session Logging (spl-memo, no custom program)
- **Program**: `MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr` (standard spl-memo)
- **Purpose**: Every real anky (8+ min) gets its session hash logged on-chain via spl-memo. Authority wallet pays (~$0.0007).
- **Logged automatically** at write-time via `log_session_onchain()` in `src/pipeline/image_gen.rs`.
- **Memo format**: `anky|<session_hash>|<session_id>|<wallet>|<duration>|<words>|<kingdom>|<sojourn>`
- **User wallet** included as non-signer account reference — indexed under both authority and user wallet.
- **Querying**: `getSignaturesForAddress(authority)` for all activity, `getSignaturesForAddress(userWallet)` for one user.

### Ankys cNFT Tree (reserved for future milestones)
- **Tree**: `3SgBFS5gFmeMUNZQqQk1xGsoZAZewXVAiMVkzjeJBSd6` (depth 10, 1,024 capacity)
- **Collection**: `5AbvPKw84mXhYWNBo3iTCH1Nkpc7EZPHHk4q1ECz4rVW`
- **Purpose**: Reserved for milestone cNFTs (not per-session). The `/mint-anky` worker endpoint exists but is not called automatically.
- **Future**: Anky (as an entity) may reward users with cNFTs at meaningful moments — streak completions, kingdom completions, sojourn completions.

**Authority wallet**: `ApTZwa8M1Rako93TQ57cLczGr5hjeGEvZdszKb92tXNS` (keypair in `solana/setup/authority.json`).

Worker endpoints: `POST /mint` (mirrors), `POST /mint-anky` (milestone cNFTs, reserved), `POST /log-session` (spl-memo session logging), `GET /supply`.

## Privacy Architecture

The backend is a **blind relay** for authenticated users:

1. iOS app fetches enclave pubkey via `GET /api/anky/public-key` (X25519)
2. iOS encrypts writing on-device (ECIES: X25519 ECDH → SHA256 → AES-256-GCM)
3. iOS computes `session_hash` = SHA256(plaintext) locally
4. `POST /api/sealed-write` sends: sealed envelope + session_hash + duration + word_count
5. Backend stores encrypted envelope blind in `sealed_sessions` table + `data/sealed/`
6. Backend logs session_hash on Solana via spl-memo (authority pays)
7. Backend relays sealed envelope to enclave (`POST /process-writing` on EC2 proxy)
8. Enclave decrypts, verifies hash, calls OpenRouter → returns `{reflection, image_prompt, title}`
9. Backend generates image from enclave's prompt, stores reflection on anky record
10. Plaintext NEVER leaves {iOS device, EC2 host} trust boundary

**Trust model:** The EC2 proxy sees plaintext during the OpenRouter call (same machine as enclave). The backend on poiesis does NOT.

**`GET /api/verify/{session_hash}`** proves a writing exists without revealing content.

**The plaintext `/write` path** still works for anonymous/web users whose writing is ephemeral. Authenticated users use the sealed write path.

## Three User States (Web)

### Anonymous (no phone, no login)
- Writes for 8 minutes → sees their writing + platform share buttons (X, Warpcast, Claude, ChatGPT)
- Nothing persisted. Writing is ephemeral. Disappears when they close the tab.
- Altar on the left: Stripe payment to gift money to anky.
- Top-right shows "not sealed" badge linking to `/login`.

### Authenticated via QR (phone scanned, has wallet, no Mirror cNFT)
- Phone is the authority. Browser is a writing surface.
- Can see the 3,456 slot counter. Can claim a mirror via the Farcaster miniapp or iOS.
- Writing can be sealed via phone, but ankys are not minted until they have a Mirror.

### Sojourn participant (has Mirror cNFT)
- Full experience: write → phone seals → enclave processes → reflection → anky cNFT minted
- Profile shows anky grid. Each anky is tappable → conversation view.
- Nav PFP = latest anky image.

## Farcaster Miniapp

The miniapp is a **React app** (Vite + React + TypeScript) in `miniapp/`. It is a **chat interface** — everything is a conversation between anky and the user.

**Serving**: `anky.app` root (`/`) serves the React build when User-Agent contains `Farcaster` or `Warpcast`, or when `sec-fetch-dest` is `iframe`. The Rust server reads `static/miniapp/index.html` at runtime (not baked in), falling back to `templates/miniapp.html` if the React build doesn't exist.

**Dev workflow**:
```bash
cd miniapp && npm run dev    # local dev with HMR, proxies API calls to localhost:8889
cd miniapp && npm run build  # production build → static/miniapp/ (no Rust recompile needed)
```

**Layout**: One chat screen. Top-left button → altar overlay. Top-right PFP → profile overlay.

**Chat flow**:
1. Farcaster SDK init → get user context (fid, username, pfpUrl)
2. New user: anky greets by name → generates mirror from Farcaster presence → shows mirror card in chat → user can seal (mint cNFT) inline
3. Returning user: anky greets with personalized prompt (from `/api/miniapp/prompt?fid=X`)
4. User taps "write" → full-screen writing overlay (8-min timer, no backspace, chakra bar, idle detection)
5. Submit → user's writing appears as message → anky reflects via SSE stream (`/api/stream-reflection/{ankyId}`)
6. Profile overlay: anky grid (from `/api/my-ankys`), tap any anky → conversation view showing writing + image + reflection

**Key API endpoints used by miniapp**:
- `GET /api/miniapp/onboarding?fid=X` — check if user is onboarded
- `POST /api/miniapp/onboard` — generate wallet + assign kingdom
- `GET /api/mirror?fid=X` — generate mirror from Farcaster presence
- `POST /api/mirror/solana-mint` — mint Mirror cNFT
- `GET /api/miniapp/prompt?fid=X` — personalized writing prompt
- `POST /write` — submit writing session
- `GET /api/stream-reflection/{ankyId}` — SSE reflection stream
- `GET /api/my-ankys` — user's completed ankys
- `GET /api/me` — user profile

## iOS App Endpoints

The iOS app uses `/swift/v2/*` endpoints with Bearer token auth:

- `POST /api/sealed-write` → `SealedWriteResponse` (ok, session_id, session_hash, anky_id, is_anky) — **primary write path for authenticated users**, encrypted on-device
- `POST /swift/v2/write` → `MobileWriteResponse` (ok, outcome, persisted, spawned.anky_id, wallet_address, flow_score) — **fallback plaintext path**
- `GET /swift/v2/writing/{sessionId}/status` → polls for anky_response, next_prompt, mood, anky status
- `GET /swift/v2/me` → user profile (is_premium, total_ankys, wallet_address, profile_image_url evolves to latest anky)
- `GET /swift/v2/writings` → writing history with full anky_image_path URLs
- `POST /mirror/mint` → per-session Mirror cNFT mint (idempotent, one per user)
- `POST /api/sessions/seal` → store encrypted writing envelope
- `GET /api/anky/public-key` → enclave encryption public key

## Mirror Generation Pipeline

`GET /api/mirror?fid=X` (in `src/routes/api.rs`):

1. Fetch Farcaster profile + recent casts via Neynar API
2. Analyze PFP with Claude Vision
3. Generate public_mirror text + gap + 10 flux_descriptors via Claude Sonnet (→ Haiku → OpenRouter fallback)
4. Derive 8 kingdom items via Claude
5. Generate Anky image via ComfyUI using flux_descriptors
6. Persist to `mirrors` table + `data/mirrors/{id}.png`

## Product Stance

- The writing session is sacred. Never modify anything that touches the write path without explicit instruction.
- Privacy is structural. The backend never sees plaintext from authenticated users. The enclave is the only decryption point.
- Identity = Solana wallet. Always. The phone holds the wallet. The web is a writing surface.
- The sojourn has 3,456 seats. The Mirror cNFT is the gate. The Anky cNFTs are the artifacts.

## Engineering Stance

- Follow existing patterns. Before adding a new abstraction, check if the pattern already exists.
- Database is Postgres via sqlx. The `Connection` wrapper translates rusqlite-style `?1` params to `$1`.
- Migrations live in `migrations/*.sql`. Run automatically at startup.
- `cargo fmt` and `cargo check` must pass before any session ends.
- Never hardcode credentials. Read from environment variables or config struct.
- **Documentation is first-class.** Always update `CURRENT_STATE.md`, `CLAUDE.md`, and changelog when making changes.
- Public markdown routes are product surface. Keep `/sojourn9.md` aligned with the current Sojourn 9 constitutional text.

## Deployment

- Build: `cargo build --release`
- Restart: `systemctl --user restart anky.service`
- Mint worker: `systemctl --user restart anky-mint.service`
- Always build and restart after changes unless told otherwise.

## Key Services on poiesis

| Service | Port | Systemd Unit |
|---------|------|-------------|
| Anky server | 8889 | `anky.service` |
| Mint worker | 8787 | `anky-mint.service` |
| llama-server (Mind) | 8080 | `anky-mind.service` |
| ComfyUI (Heart) | 8188 | `anky-heart.service` |
| Cloudflare tunnel | — | `cloudflared-anky.service` |

## Key External Services

| Service | URL |
|---------|-----|
| Anky Soul Enclave | `http://3.83.84.211:5555` |
| Solana RPC (devnet) | Helius devnet via API key |
| Neynar (Farcaster) | `api.neynar.com` |
| OpenRouter (LLM fallback) | `openrouter.ai` |

## Payments

- Altar: Stripe payments on web (`/altar`).
- All other paid features use x402 wallet payments (USDC on Base) via `payment-signature` header.
