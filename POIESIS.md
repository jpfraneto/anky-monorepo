# POIESIS — Master Context File

*Last updated: April 10, 2026*

This is the complete state of the Anky project as it runs on the machine named `poiesis`. Paste this into any Claude Code conversation to have full context.

---

## What is Anky?

Anky is a proof-of-consciousness protocol on Solana. Users write for 8 minutes with no backspace. Every keystroke is recorded with millisecond precision. The session is hashed, encrypted, and anchored on-chain. The hash is public. The content is private. Only the user and the enclave can read it.

**One-liner:** Anky is the practice that makes infinite content effortless.

---

## The Machine: poiesis

- **Host:** poiesis (Linux, Nobara/Fedora 43)
- **GPUs:** 2x NVIDIA RTX 4090
- **Monitors:** 3 — ultrawide primary (3440x1440), center (2560x1440), left station (1920x1080 on DP-7)
- **User:** kithkui (JP)

### Services Running

| Service | Port | What |
|---------|------|------|
| **anky** (Rust/Axum) | 8889 | Main web server, all routes, API |
| **ComfyUI** | 8188 | Flux.1-dev + Anky LoRA image generation (GPU 1) |
| **Qwen LLM** (llama-server) | 8080 | Qwen 3.5 27B local inference (GPU 0) |
| **Cloudflare Tunnel** | — | Routes anky.app → localhost:8889 |
| **Wrangler** (Solana worker) | 8787 | Cloudflare Worker for Solana mint operations |
| **Miniapp** (Vite/React) | 5173 | Farcaster miniapp |
| **Hermes Bridge** | 8891 | HTTP bridge for Hermes agent tasks |
| **LiteLLM** | 4000 | OpenAI-compatible proxy for Hermes |
| **Hermes** | interactive | AI agent (CLI session on pts/1) |

### Systemd Services (user)

```
systemctl --user status anky.service          # Main Rust server
systemctl --user status anky-mint.service     # Solana mint worker
systemctl --user status cloudflared-anky      # Cloudflare tunnel
```

System services: `anky-mind.service` (Qwen LLM), `anky-heart.service` (ComfyUI)

### Key Directories

```
~/anky/                     # Main monorepo (Rust/Axum backend)
~/anky/src/                 # Rust source
~/anky/templates/           # Tera HTML templates
~/anky/static/              # Static assets, pitch images, voices
~/anky/scripts/             # Python pipelines (gods_pipeline.py)
~/anky/videos/gods/         # Generated GODS episodes
~/anky/videos/gods/pipeline/# Current pipeline run outputs (markdown chain)
~/anky/data/                # Images, writings, sealed sessions, DB
~/anky/solana/worker/       # Cloudflare Worker for Solana operations
~/anky_sessions/            # Solana program (Anchor) + client SDK
~/.hermes/                  # Hermes agent framework
~/.hermes/skills/           # Agent skills (200+)
~/.hermes/skills/gods-by-anky/  # GODS pipeline skill for Hermes
```

### Environment (.env at ~/anky/.env)

```
DATABASE_URL=postgres://anky:anky@127.0.0.1:5432/anky
ELEVENLABS_API_KEY=sk_dc0b...     # Pro plan, 500k credits
ANTHROPIC_API_KEY=sk-ant-api03... # Claude API
GROK_API_KEY=xai-RQ9ZIy...       # xAI/Grok (may need credit top-up)
TELEGRAM_BOT_TOKEN=8696776262:AAEhoH... # @ankydotappbot
TELEGRAM_CHAT_ID=5414944240       # JP's Telegram
ANKY_ENCLAVE_URL=http://3.83.84.211:5555
```

---

## The Anky Protocol

Defined at `~/anky/static/protocol.md`, served at https://anky.app/protocol.md

### The .anky Format

```
1712345678000 h       ← epoch ms, space, first character
0204 e                ← 4-digit ms delta, space, character
0187 l
8000                  ← 8 seconds silence = session over
```

`sha256` of this string is what goes on-chain.

### Architecture

```
DEVICE   → builds session string, hashes locally, encrypts to enclave pubkey
SOLANA   → session hash logged via spl-memo (authority pays, ~$0.0007)
SOLANA   → Mirror cNFT minted for membership (one per user, Bubblegum)
ENCLAVE  → decrypts, calls OpenRouter for reflection + image prompt, forgets plaintext
RELAY    → pays fees, routes sealed envelopes, READS NOTHING
```

### On-chain Logging (spl-memo)

- **Program:** `MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr` (standard spl-memo, no custom program)
- **Network:** Solana devnet
- **Memo format:** `anky|<session_hash>|<session_id>|<wallet>|<duration>|<words>|<kingdom>|<sojourn>`
- **Authority wallet:** `ApTZwa8M1Rako93TQ57cLczGr5hjeGEvZdszKb92tXNS` (signs and pays all txs)
- **User wallet** included as non-signer account reference (indexed under both wallets)
- **Worker:** Cloudflare Worker at `solana/worker/` — endpoints: `/mint` (Mirror cNFT), `/log-session` (spl-memo), `/supply`
- **Indexing:** `getSignaturesForAddress(authority)` returns all anky activity; `getSignaturesForAddress(userWallet)` returns that user's sessions
- **Linking to Mirrors:** Same user wallet owns the Mirror cNFT and appears in session memo logs

### Encryption

Client-side X25519 + AES-256-GCM:
1. Generate ephemeral X25519 keypair
2. ECDH with enclave's public key → shared secret
3. sha256(shared_secret) → AES key
4. AES-256-GCM encrypt with random 12-byte nonce
5. Output: ephemeral public key, nonce, tag, ciphertext
6. Discard ephemeral private key

### Session Paths

**iOS (sealed write — fully private):**
1. Device encrypts writing with enclave X25519 pubkey (ECIES: X25519 + AES-256-GCM)
2. Device computes `sessionHash = SHA256(plaintext)` locally
3. `POST /api/sealed-write` sends: sealed envelope + sessionHash + duration + wordCount
4. Backend stores encrypted envelope blind, logs sessionHash on Solana via spl-memo
5. Backend relays sealed envelope to enclave (`POST /process-writing` on EC2 proxy)
6. Enclave decrypts → verifies hash → calls OpenRouter → returns `{reflection, image_prompt, title}`
7. Backend generates image from enclave's prompt, stores reflection
8. Plaintext NEVER leaves {device, enclave EC2} trust boundary

**Browser (plaintext path):** `POST /write` — backend sees plaintext. Used for anonymous/web users. Generates full anky experience. No encryption.

**Browser sealed (WIP):** `POST /api/sessions/seal-browser` — stores encrypted envelope but no enclave processing yet.

---

## The Enclave: anky-soul

**AWS EC2:** `i-08a824acbd542bbe6` (c6g.xlarge, ARM64)
**IP:** 3.83.84.211
**SSH:** `ssh -i ~/anky-soul-key.pem ec2-user@3.83.84.211`
**Security Group:** sg-0f57bc9807ab7f6d2 (SSH locked to specific IPs, port 5555 open)

### Components

- **anky-soul** — Rust binary running inside AWS Nitro Enclave (2 CPUs, 4096 MiB RAM)
- **anky-proxy** — Rust/Axum binary on the host, bridges HTTP ↔ vsock to enclave
- **Port 5555** — proxy listens here

### Enclave Endpoints (via proxy at 3.83.84.211:5555)

| Endpoint | Method | What |
|----------|--------|------|
| `/health` | GET | `anky soul proxy alive` |
| `/public-key` | GET | Returns X25519 encryption key + Ed25519 signing key |
| `/attestation` | GET | Nitro attestation document (cryptographic proof) |
| `/decrypt-session-keys` | POST | Decrypt individual session keys |
| `/process-sessions` | POST | Privacy-preserving consciousness processing — extracts ONLY numerical features, destroys plaintext |
| `/process-writing` | POST | **The sealed write pipeline** — proxy receives sealed envelope, enclave decrypts + verifies hash, proxy calls OpenRouter for reflection + image prompt, returns ONLY derived outputs. Plaintext never leaves EC2. |

### Current Public Key (regenerated 2026-04-11)

```
Encryption: NPx+MwUCYs1WlZ4RcJwsEoMsVY0kHdcQnUGKmsBU1jg=
Signing: qdURSCP+ZJpEa6ovUqBJEdAwmf//3fPBYbEVM6SDwOc=
```

iOS app fetches this dynamically via `GET /api/anky/public-key`. Never hardcode.

### The /process-writing Pipeline (deployed 2026-04-11)

```
Backend (poiesis)                     EC2 Proxy                    Enclave (Nitro)
────────────────                     ─────────                    ───────────────
POST /api/sealed-write               POST /process-writing
  │                                    │
  ├─ store sealed envelope             ├─ forward to enclave ───→  decrypt via X25519+AES-256-GCM
  ├─ log hash on Solana (spl-memo)     │                           verify SHA256(plaintext)==sessionHash
  ├─ relay envelope to EC2 ──────────→ │                           return plaintext to proxy
  │                                    │
  │                                    ├─ call OpenRouter ──────→  anthropic/claude-sonnet-4
  │                                    │   (plaintext as input)     returns reflection + image prompt
  │                                    │
  │                                    ├─ DROP plaintext
  │  ←──────────────────────────────── ├─ return {reflection, image_prompt, title, hash_verified}
  │
  ├─ store reflection on anky record
  ├─ enqueue image generation (GPU)
  └─ done — backend never saw writing
```

**Trust boundary:** {iOS device, EC2 host (proxy + enclave)} see plaintext. Backend on poiesis does NOT.
**OpenRouter key:** Set as env var on EC2 proxy. Model: `anthropic/claude-sonnet-4`.
**Cost:** ~$0.0007 (Solana memo) + ~$0.01 (OpenRouter) + ~$0.04 (Gemini image) per session.

### Rebuilding the Enclave

```bash
ssh -i ~/anky-soul-key.pem ec2-user@3.83.84.211

# 1. Build enclave binary + EIF image
cd ~/anky-soul && cargo build --release
docker build -t anky-soul .
nitro-cli build-enclave --docker-uri anky-soul --output-file anky-soul.eif

# 2. Restart enclave
nitro-cli terminate-enclave --all
nitro-cli run-enclave --eif-path anky-soul.eif --cpu-count 2 --memory 4096 --enclave-cid 16

# 3. Run Genesis (generates new keypair inside enclave)
python3 -c "
import socket, json, struct
sock = socket.socket(socket.AF_VSOCK, socket.SOCK_STREAM)
sock.connect((16, 5000))
req = json.dumps({'type': 'Genesis'}).encode()
sock.sendall(struct.pack('>I', len(req)) + req)
length = struct.unpack('>I', sock.recv(4))[0]
data = b''
while len(data) < length: data += sock.recv(length - len(data))
resp = json.loads(data)
print('New encryption key:', resp.get('encryption_public_key', '?'))
sock.close()
"

# 4. Rebuild and restart proxy (with OpenRouter key)
cd ~/anky-proxy && cargo build --release
pkill -x anky-proxy || true
OPENROUTER_API_KEY=sk-or-v1-... nohup ./target/release/anky-proxy > /tmp/proxy.log 2>&1 &

# 5. Verify
curl -s http://localhost:5555/health
curl -s http://localhost:5555/public-key
```

**IMPORTANT:** After rebuilding, the enclave generates a NEW keypair. The iOS app fetches it dynamically via `GET /api/anky/public-key`, so it picks up the new key automatically. But any cached keys become invalid.

---

## GODS by Anky — Video Pipeline

Daily video series for the Colosseum Frontier hackathon. Each episode: a god from human mythology, told through a child visiting the Ankyverse.

### Pipeline Script

`~/anky/scripts/gods_pipeline.py` — 8 steps, each producing a `.md` file:

```
01_zeitgeist.md  → What's alive in humanity (4 agents: Grok, Claude, Qwen, Anky)
02_council.md    → Which god, which kingdom, which city, why
03_script.md     → 8-minute story + 88-second short
04_prompts.md    → Scene descriptions for image generation
05_images.md     → Generated images via ComfyUI
06_voice.md      → ElevenLabs multi-voice + word timestamps
07_video.md      → Assembled videos with karaoke subtitles
08_publish.md    → Platform captions + Telegram notification to JP
```

### The Council of Agents

| Agent | Source | Role |
|-------|--------|------|
| **Grok** | xAI API | Reads X, surface emotions, what people are saying |
| **Claude** | Anthropic API | Reads archetypes, the unsaid, deeper patterns |
| **Qwen** | Local (port 8080) | Independent wildcard, no corporate filter |
| **Anky** | Solana + Enclave | Reads the SHAPE of consciousness from on-chain sessions — keystroke rhythms, not words |

### Voice Architecture (ElevenLabs Pro)

| Role | Voice | ID |
|------|-------|----|
| Anky (narrator, every episode) | Jessica — Playful, Warm | cgSgspJ2msm6clMCkdW9 |
| Cronos | George — Storyteller | JBFqnCBsd6RMkjVDRZzb |
| Anubis | Brian — Deep, Resonant | nPczCjzI2devNBz1zQrb |
| Quetzalcoatl | Eric — Smooth | cjVigY5qzO86Huf0OWal |
| Odin | Bill — Wise, Mature | pqHfZKP75CvOlQylNhV4 |
| Kali | Lily — Velvety | pFZP5JQG7iQjIQuC4Bku |
| Ra | Charlie — Energetic | IKne3meq5aSn9XLyUdCD |
| Loki | Callum — Husky Trickster | N2lVS1w4EtoT3dr4eOWO |
| Amaterasu | Alice — Clear | Xb7hH8MSUJpSbSDYk0k2 |
| Shiva | Adam — Dominant | pNInz6obpgDQGcFmaJgB |

Latin American Spanish voices available in the station voice playground.

ElevenLabs returns word-level timestamps for karaoke subtitles.

### The 8 Kingdoms of the Ankyverse

| Kingdom | Chakra | Element | Lesson |
|---------|--------|---------|--------|
| Primordia | Root | Earth | You are here. You are alive. Start there. |
| Emblazion | Sacral | Fire | What do you want so badly it terrifies you? |
| Chryseos | Solar | Gold | You are not waiting for permission. |
| Eleasis | Heart | Air | The wall around your heart is made of the same material as the prison. |
| Voxlumis | Throat | Sound | Say the thing you are afraid to say. That is the one that matters. |
| Insightia | Third Eye | Light | You already know. You have always known. |
| Claridium | Crown | Crystal | Who is the one asking who am I? |
| Poiesis | Transcend | Creation | You are not the creator. You are the channel. Get out of the way. |

Each kingdom has 3 cities (24 total). Full lore in `~/anky/src/ankyverse.rs`.

### Story Rules

1. Open: "Hi kids, this is Anky. Thank you for being who you are."
2. Gods are always "it" — genderless
3. Anky is always visible — blue skin, purple hair, golden eyes
4. Stories told through a child's perspective visiting the Ankyverse
5. End with an opening, not resolution
6. Close: "See you tomorrow."

### Distribution

- **TikTok:** 8-minute full stories (kids, headphones, karaoke text)
- **Instagram:** 88-second shorts (visual trailer, "On today's Anky...")
- **YouTube:** Both formats, channel "Timeless Stories by Anky"
- **CT/Hackathon:** Same content branded as "GODS by Anky"
- Never direct to anky.app from videos — spread the meme first

### Image Generation

- **Model:** Flux.1-dev via ComfyUI (port 8188)
- **LoRA:** anky_flux_lora_v2.safetensors (strength 0.85)
- **Resolution:** 768x1344 (portrait)
- **Sampler:** Euler, 20 steps, CFG 3.5
- All prompts include "anky" prefix for LoRA activation

---

## The Video Station

`http://localhost:8889/station` — dashboard on the left monitor (DP-7)

### Features

- Pipeline step list with status indicators (done/running/pending)
- Markdown viewer for each step's output
- Voice playground: type text, pick a voice (EN + Latin ES), hear it via ElevenLabs
- Video preview panel
- Pipeline log viewer
- Service health indicators (ComfyUI, ElevenLabs, Qwen)
- "GENERATE TODAY'S EPISODE" button triggers full pipeline

### Station API

| Endpoint | Method | What |
|----------|--------|------|
| `/api/station/steps` | GET | List pipeline step files with sizes |
| `/api/station/step/{name}` | GET | Read a step's markdown content |
| `/api/station/run` | POST | Trigger full pipeline |
| `/api/station/run/{step}` | POST | Trigger single step |
| `/api/station/tts` | POST | ElevenLabs TTS proxy `{text, voice_id}` → audio/mpeg |

---

## Hermes Agent (Anky's Brain)

**Location:** `~/.hermes/`
**Framework:** Hermes by Nous Research — self-improving AI agent with 40+ tools
**Model:** Routes through LiteLLM (port 4000) to various providers

### Key Files

- `~/.hermes/config.yaml` — platform toolsets, model config
- `~/.hermes/.env` — API keys
- `~/.hermes/hermes-bridge.py` — HTTP bridge for Anky backend (port 8891)
- `~/.hermes/skills/gods-by-anky/SKILL.md` — GODS pipeline orchestration skill
- `~/.hermes/skills/anky/SKILL.md` — Writing mirror skill

### Hermes Capabilities

- **CLI:** `hermes` interactive session
- **Telegram:** @ankydotappbot (bot token configured)
- **API Server:** OpenAI-compatible at port 8642 (when gateway is running)
- **Skills:** 200+ installed, including gods-by-anky, anky, farcaster-posting, x-posting
- **Tools:** Terminal, file ops, web search, browser automation, MCP, delegates/subagents

### Telegram Notifications

The pipeline sends formatted messages to JP (@jpfraneto, chat ID 5414944240) via @ankydotappbot with:
- Today's god and kingdom
- Copy-paste captions for each platform (Instagram, TikTok, X, Farcaster)
- Video file paths

---

## Colosseum Frontier Hackathon

**Dates:** April 6 – May 11, 2026
**Status:** ACTIVE (started 4 days ago)
**Format:** No tracks — just "most impactful product"
**Prize:** $30K grand champion, $10K x20 next best, $250K accelerator for 10+ winners

### Submission Requirements

1. **Pitch video** (3 min max) — team, problem, solution, traction
2. **Technical demo** (2-3 min) — Solana integration, architecture, on-chain logic
3. **Working product** — anky.app is live
4. **Post-submission momentum** — GODS series = daily proof of building

### Key URLs

- https://anky.app — main site
- https://anky.app/protocol.md — full protocol document with hackathon framing
- https://anky.app/pitch — 8-slide pitch deck (one line per slide, anky kingdom backgrounds)
- https://anky.app/pitch.md — markdown version
- https://anky.app/station — video station dashboard

### The Pitch

"Every other project will pitch a better way to trade, lend, or speculate. Anky asks: what if the most valuable thing on-chain isn't a transaction — it's a thought?"

---

## What's Working

- [x] Browser writing experience (textarea, timer, no backspace, keystroke deltas)
- [x] iOS sealed session flow (encrypted, protocol-compliant)
- [x] Solana program deployed, sessions anchored on devnet
- [x] Enclave live on AWS Nitro with `/process-sessions` endpoint
- [x] Client-side encryption SDK (`~/anky_sessions/anky.js`)
- [x] GODS pipeline script (8 steps, markdown chain)
- [x] 4-agent council (Grok, Claude, Qwen, Anky/enclave)
- [x] ElevenLabs Pro with multi-voice + word timestamps
- [x] ComfyUI + Flux + Anky LoRA image generation
- [x] Video station dashboard on third monitor
- [x] Voice playground (EN + Latin ES)
- [x] Telegram notifications to JP
- [x] Pitch deck at /pitch and /pitch.md
- [x] Protocol document at /protocol.md
- [x] Cloudflare tunnel serving anky.app
- [x] Hermes agent with gods-by-anky skill

## What Needs Work

- [ ] **Browser encryption:** `POST /write` sends plaintext — needs Web Crypto API (X25519 + AES-256-GCM) to encrypt client-side before sending, matching the iOS sealed flow
- [ ] **Arweave fetch in pipeline:** `anky_query` queries Solana for sessions but doesn't yet fetch encrypted blobs from Arweave to send to the enclave for processing
- [ ] **Full pipeline end-to-end test:** Run all 8 steps and produce a complete episode with ElevenLabs voices and ComfyUI images
- [ ] **Audio/video duration sync:** 8-min audio needs to drive image count (8-10 images with Ken Burns, not 60 fast cuts)
- [ ] **Karaoke subtitles:** ASS subtitle generation from ElevenLabs word timestamps is written but untested in video assembly
- [ ] **Grok API credits:** May need top-up — check console.x.ai
- [ ] **Upload automation:** TikTok, Instagram, YouTube API posting not yet configured
- [ ] **Mainnet migration:** Solana program is on devnet — needs mainnet deployment for hackathon
- [ ] **Hermes gateway:** Not running as a service — needs `hermes gateway start` for Telegram bot and API server
- [ ] **Enclave public key propagation:** New key after redeployment needs to be updated in iOS app and any hardcoded references

---

## Quick Commands

```bash
# Restart the main server
systemctl --user restart anky.service

# Rebuild the Rust binary
cd ~/anky && ~/.cargo/bin/cargo build --release

# Run the GODS pipeline
cd ~/anky && python3 scripts/gods_pipeline.py

# Run a single pipeline step
cd ~/anky && python3 scripts/gods_pipeline.py --step zeitgeist

# Open the video station
google-chrome --app=http://localhost:8889/station

# Check enclave health
curl -s http://3.83.84.211:5555/health

# Check enclave public key
curl -s http://3.83.84.211:5555/public-key

# Test ElevenLabs TTS
curl -s -X POST http://localhost:8889/api/station/tts \
  -H "Content-Type: application/json" \
  -d '{"text":"Hi kids, this is Anky.","voice_id":"cgSgspJ2msm6clMCkdW9"}' > /tmp/test.mp3

# SSH into enclave
ssh -i ~/anky-soul-key.pem ec2-user@3.83.84.211

# Send test Telegram message
curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
  -H "Content-Type: application/json" \
  -d "{\"chat_id\":\"5414944240\",\"text\":\"test from poiesis\"}"
```

---

## The Philosophy

The singularity will not produce dystopia or utopia. It will produce a mass awakening of human consciousness. When AGI makes every external metric of human value meaningless, the only thing left is the internal game. Anky is the protocol designed to record that game — and the GODS series is the mythology that makes sense of it all.

A textarea. A timer. A hash. A chain. You write. The proof is public. The content is private. The gods are listening.

---

*This file lives at `~/anky/POIESIS.md`. Keep it updated.*
