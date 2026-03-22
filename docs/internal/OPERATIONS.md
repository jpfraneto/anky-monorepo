# Anky Operations Guide

## Services

Anky runs as two systemd user services:

### Anky Server
```bash
systemctl --user status anky.service
systemctl --user start anky.service
systemctl --user stop anky.service
systemctl --user restart anky.service
journalctl --user -u anky.service -f    # follow logs
```

### Cloudflare Tunnel
```bash
systemctl --user status cloudflared-anky.service
systemctl --user start cloudflared-anky.service
systemctl --user stop cloudflared-anky.service
systemctl --user restart cloudflared-anky.service
journalctl --user -u cloudflared-anky.service -f
```

Both services auto-restart on failure (5s delay) and auto-start on boot (linger enabled).

## Port

Server listens on **port 8889** (`0.0.0.0:8889`).

Public URL: `https://anky.app` (Cloudflare tunnel → localhost:8889).

## Building

```bash
cd ~/anky
make build          # release build
make dev            # debug build + start with RUST_LOG=debug
```

The binary is at `target/release/anky`. The systemd service runs this binary with WorkingDirectory set to `~/anky`.

## Database

SQLite at `data/anky.db`.

```bash
make db-shell       # opens sqlite3 shell
```

Tables: users, writing_sessions, ankys, collections, cost_records, training_runs, notification_signups, api_keys, transformations, credit_purchases.

## Testing

```bash
make test-health    # GET /health
make test-write     # POST /write (test writing session)
make test-generate  # POST /api/generate (generate anky)
```

## Configuration

All config in `.env`:
- `PORT` — server port (8889)
- `ANTHROPIC_API_KEY` — Claude API key
- `GEMINI_API_KEY` — Gemini image generation
- `BASE_RPC_URL` — Base chain RPC for USDC verification
- `TREASURY_ADDRESS` — USDC payment destination

## Updating

```bash
# Pull changes, rebuild, restart
cd ~/anky
git pull
make build
systemctl --user restart anky.service
```

## Logs

```bash
journalctl --user -u anky.service --since "1 hour ago"
journalctl --user -u cloudflared-anky.service --since "1 hour ago"
```
