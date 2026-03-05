---
name: memetics
version: 1.0.0
description: Custom token homepages on memetics.wtf. Deploy a bungalow — a branded landing page for your Solana token — in one API call.
homepage: https://memetics.wtf
metadata: {"category": "defi-tools", "api_base": "https://memetics.wtf"}
---

# Memetics — Custom Token Homepages

You have a Solana token. Dexscreener gives you a chart. Memetics gives you a **home**.

A **bungalow** is a custom-branded landing page for your token, hosted at `memetics.wtf/solana/{mint_address}`. You control the HTML. One API call to deploy. One API call to update.

---

## Quick Start

### 1. Build your page

Create a single `index.html` file with **all CSS and JS inlined**. No external dependencies except fonts (Google Fonts, etc). The page must be fully self-contained.

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>$YOUR_TOKEN</title>
  <style>
    /* all your CSS here, inlined */
  </style>
</head>
<body>
  <!-- your token homepage -->
  <script>
    // all your JS here, inlined
  </script>
</body>
</html>
```

### 2. Deploy it

```
POST https://memetics.wtf/api/v1/bungalow
Content-Type: application/json
payment-signature: 0x<tx_hash>

{
  "mint_address": "YourTokenMintAddress123456789pump",
  "html": "<!DOCTYPE html><html>...your full HTML string...</html>",
  "title": "My Token",
  "description": "One-line description of your project"
}
```

Cost: **$5.00 USDC** via x402 protocol on Base.

### 3. Done

Your bungalow is live at:

```
https://memetics.wtf/solana/YourTokenMintAddress123456789pump
```

---

## Payment — x402 Protocol

Every bungalow deployment costs **$5.00 USDC** paid via the x402 protocol.

### How to pay

1. Get the treasury address: `GET https://memetics.wtf/api/treasury`
2. Send **5.00 USDC** on Base (chain ID 8453) to the treasury address
3. Pass the transaction hash in the `payment-signature` header

```
POST https://memetics.wtf/api/v1/bungalow
Content-Type: application/json
payment-signature: 0x<64 hex chars tx hash>

{ ... }
```

Any wallet can pay. No API key needed. No registration. Just USDC and a tx hash.

### Payment flow

1. `payment-signature` header with raw tx hash (0x + 64 hex) → **wallet payment**
2. `payment-signature` header with x402 token → **x402 facilitator verification**
3. No payment header → **402 Payment Required** (response includes treasury address and cost)

**402 response example:**

```json
{
  "error": "payment required",
  "cost_usdc": 5.00,
  "treasury": "0x...",
  "chain": "base",
  "chain_id": 8453,
  "usdc_contract": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
  "accepts": ["x402", "raw_tx_hash"]
}
```

---

## Full API Reference

| Method | Endpoint | Auth | Purpose |
|--------|----------|------|---------|
| POST | `/api/v1/bungalow` | Payment | Deploy or update a bungalow |
| GET | `/api/v1/bungalow/{mint_address}` | None | Get bungalow metadata |
| DELETE | `/api/v1/bungalow/{mint_address}` | Payment sig (owner) | Remove a bungalow |
| GET | `/api/treasury` | None | Get USDC treasury address |
| GET | `/solana/{mint_address}` | None | View the live bungalow page |
| GET | `/api/v1/bungalows` | None | List recent bungalows |
| GET | `/health` | None | Service health check |

---

## Endpoints in Detail

### Deploy / Update Bungalow

```
POST https://memetics.wtf/api/v1/bungalow
Content-Type: application/json
payment-signature: 0x<tx_hash>

{
  "mint_address": "6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump",
  "html": "<!DOCTYPE html><html lang='en'>...</html>",
  "title": "anky",
  "description": "write yourself into existence"
}
```

**Request fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `mint_address` | string | yes | Solana token mint address |
| `html` | string | yes | Complete self-contained HTML (all CSS/JS inlined) |
| `title` | string | yes | Token/project name |
| `description` | string | no | One-line description |

**Response (201):**

```json
{
  "ok": true,
  "mint_address": "6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump",
  "url": "https://memetics.wtf/solana/6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump",
  "deployed_at": "2026-02-20T18:30:00Z"
}
```

**Updating:** POST to the same `mint_address` again with a new `payment-signature`. The old page is replaced. Each deployment costs $5 USDC.

### Get Bungalow Metadata

```
GET https://memetics.wtf/api/v1/bungalow/{mint_address}
```

**Response:**

```json
{
  "mint_address": "6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump",
  "title": "anky",
  "description": "write yourself into existence",
  "url": "https://memetics.wtf/solana/6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump",
  "deployed_at": "2026-02-20T18:30:00Z",
  "updated_at": "2026-02-20T18:30:00Z"
}
```

### List Bungalows

```
GET https://memetics.wtf/api/v1/bungalows?limit=20&offset=0
```

Returns an array of bungalow metadata objects, newest first.

### Treasury

```
GET https://memetics.wtf/api/treasury
```

```json
{
  "address": "0x...",
  "chain": "base",
  "chain_id": 8453,
  "usdc_contract": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
  "bungalow_cost_usdc": 5.00
}
```

---

## HTML Requirements

Your `index.html` must be:

1. **Self-contained** — all CSS in `<style>` tags, all JS in `<script>` tags
2. **No external scripts** — no CDN links to JS libraries (Google Fonts CSS is OK)
3. **Responsive** — must work on mobile and desktop
4. **Max size: 500KB** — keep it lean

Memetics serves your HTML directly. No sandboxing, no iframe. Your page IS the page at that URL.

---

## Costs

| Action | Cost |
|--------|------|
| Deploy bungalow | $5.00 USDC |
| Update bungalow | $5.00 USDC |
| View bungalow | Free |
| API metadata queries | Free |

USDC contract on Base: `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`

---

## For Claude Code — Backend Implementation Guide

This section is for the developer building the memetics.wtf backend. Use this as the spec.

### Architecture

- **Stack:** Rust / Axum (or your preferred stack)
- **Database:** Store bungalows with columns: `mint_address` (PK), `html` (TEXT), `title`, `description`, `deployer_tx_hash`, `deployed_at`, `updated_at`
- **Payment verification:** Validate the `payment-signature` header contains a real USDC transfer of ≥ $5.00 to the treasury address on Base (chain ID 8453). Use an RPC call to `eth_getTransactionReceipt` to verify.

### Route structure

```
GET  /solana/{mint_address}     → serve the stored HTML directly (Content-Type: text/html)
POST /api/v1/bungalow           → deploy/update (requires payment)
GET  /api/v1/bungalow/{mint}    → JSON metadata
DELETE /api/v1/bungalow/{mint}  → remove (verify deployer ownership via tx signer)
GET  /api/v1/bungalows          → list all, paginated
GET  /api/treasury              → return treasury address + cost
GET  /health                    → uptime check
```

### Payment verification logic (x402)

```
fn verify_payment(tx_hash: &str, expected_amount_usdc: f64) -> Result<bool> {
    // 1. Call Base RPC: eth_getTransactionReceipt(tx_hash)
    // 2. Parse logs for USDC Transfer event
    //    - topic[0] = keccak256("Transfer(address,address,uint256)")
    //    - topic[2] = treasury address (padded to 32 bytes)
    //    - data = amount (USDC has 6 decimals, so $5.00 = 5_000_000)
    // 3. Verify: to == treasury, amount >= expected, tx confirmed
    // 4. Check tx_hash hasn't been used before (prevent replay)
    // 5. Return Ok(true) if valid
}
```

### Key implementation notes

- **Replay protection:** Store every `payment-signature` tx hash. Reject duplicates.
- **HTML sanitization:** Optional — since the deployer is paying, they own the content. But consider stripping `<script>` tags that load external domains if you want to prevent phishing.
- **Rate limiting:** None needed — the $5 cost IS the rate limit.
- **CORS:** Allow `*` on GET endpoints. Restrict POST/DELETE.
- **Caching:** Serve bungalow HTML with `Cache-Control: public, max-age=300` (5 min). Bust on update.
- **Size limit:** Reject HTML payloads > 500KB.

### Database migration

```sql
CREATE TABLE IF NOT EXISTS bungalows (
    mint_address TEXT PRIMARY KEY,
    html TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT DEFAULT '',
    deployer_address TEXT NOT NULL,
    deployer_tx_hash TEXT NOT NULL UNIQUE,
    deployed_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS used_tx_hashes (
    tx_hash TEXT PRIMARY KEY,
    used_at TEXT NOT NULL DEFAULT (datetime('now')),
    mint_address TEXT NOT NULL
);
```

---

Deploy your token's home. The chart is on dexscreener. The story is on memetics.
