# Anky Local-First Protocol

**Status:** draft v1 · updated 2026-05-06 for Sojourn 9 Solana path
**Supersedes:** sealed-write pipeline (`/api/sealed-write`, Nitro enclave)

This document is the contract between the iOS app and the Anky backend for the local-first architecture. Both sides implement to this spec. Any behavior not described here is out of scope and should not be assumed.

---

## The `.anky` session format

A writing session is a string where each line is one keystroke: `{delta_ms} {character}`. The first line's delta is `0`. Characters are literal except typed spaces, which are encoded as the exact `SPACE` token in the current mobile/SP1 protocol. No enter, no backspace — only forward keystrokes. The SHA-256 of this string's UTF-8 bytes is the `session_hash`. See `https://anky.app/spec.md` for full details.

This protocol transmits and stores the raw `.anky` string (or its derived artifacts), not flattened plaintext. Plaintext is flattened only transiently server-side for the Claude call.

---

## Principles

1. **The iOS device is the source of truth for writing sessions.** The server is a derived-artifact store and a coordination point for Solana + image generation.
2. **The backend never persists plaintext writing from authenticated users.** Not in databases, not in logs, not in error messages, not in request traces.
3. **Plaintext may transit server RAM** only for the duration of a single Claude API call, then is discarded. This is the same trust model as any AI chat app (Hailee, Claude.ai, ChatGPT).
4. **A writing session is never lost because of the network.** Local persistence is unconditional. Server submit is best-effort with unbounded retry.
5. **Simplicity is a feature.** If the privacy policy can't be explained in one paragraph, the architecture is wrong.

---

## What the server stores

For every authenticated anky, exactly these fields:

| Field | Type | Source |
|---|---|---|
| `id` | uuid | server-generated |
| `wallet_address` | text | authenticated user |
| `session_hash` | text (hex) | client-computed SHA-256 of exact `.anky` UTF-8 bytes |
| `duration_seconds` | int | client |
| `word_count` | int | client |
| `kingdom` | text | derived from `started_at` via Ankyverse calendar |
| `sojourn` | int | derived |
| `started_at` | timestamptz | client |
| `reflection` | text | Claude output |
| `title` | text | Claude output |
| `image_prompt` | text | Claude output |
| `image_url` | text | R2 / CDN |
| `solana_signature` | text | Anky Seal Program `seal_anky` tx when sealed |
| `created_at` | timestamptz | server |

## What the server never stores

- The writing plaintext.
- Any derivative that could reconstruct the writing (embeddings of the raw text, first-N-characters, etc.).
- The Claude request body after the response is returned.
- Any log line that contains the writing.

---

## The submit endpoint

### `POST /api/anky/submit`

**Auth:** `Authorization: Bearer <token>` (existing iOS session token).

**Request body** (application/json):

```json
{
  "session_hash": "<sha256 hex, 64 chars>",
  "duration_seconds": 483,
  "word_count": 612,
  "kingdom": "eleasis",
  "started_at": "2026-04-13T14:22:00Z",
  "wallet_signature": "<base58 signature of session_hash by user's Solana wallet>",
  "session": "<.anky-format keystroke stream, held in RAM only>"
}
```

**Server behavior:**

1. Verify bearer token → resolve user + wallet.
2. Verify `wallet_signature` is a valid Solana signature of `session_hash` by the user's wallet.
3. Recompute SHA-256 of the `session` UTF-8 bytes, assert it equals `session_hash`. Reject with `400` if mismatch.
4. Check idempotency: if an anky with this `session_hash` already exists for this wallet, return its existing derived artifacts as a complete SSE stream and exit.
5. Flatten the `.anky` keystroke stream into plaintext (extract the character column) and open a streaming Claude call with the plaintext. Both variables live only in this function's stack frame.
6. Stream events to client (see below).
7. On completion: generate image via existing pipeline (R2 upload), optionally record public Anky Seal Program metadata, persist derived artifacts, emit terminal events.
8. Drop the session + plaintext from memory. Do not log them. Do not include them in error reports.

**Response:** `text/event-stream` (SSE)

Events, in order:

```
event: accepted
data: {"anky_id": "<uuid>"}

event: title
data: {"title": "..."}

event: reflection_chunk
data: {"text": "..."}
# repeated, one per Claude token batch

event: reflection_complete
data: {"reflection": "<full text>"}

event: image_url
data: {"image_url": "https://..."}

event: solana
data: {"signature": "<tx sig>"}

event: done
data: {"anky_id": "<uuid>"}
```

On error at any stage, emit a final event and close the stream:

```
event: error
data: {"stage": "claude|image|solana|persist", "retryable": true, "message": "..."}
```

An error during `solana` or `image` still persists the anky with partial fields and returns an `anky_id` — the client considers the session submitted. Missing fields are filled by server-side retry workers, not by the client.

An error during `claude` does NOT persist anything. The client retries the whole submit later.

### Idempotency

- `session_hash` is the idempotency key, scoped to wallet.
- Resubmitting the same `session_hash` is safe and returns the existing anky.
- Writing with identical `.anky` bytes submitted from two devices produces one anky (same hash). Reconstructed prose is not a valid hashing input.

### Timeouts

- SSE connection may stay open up to 180 seconds.
- Client should treat connection close without `done` or `error` as retryable.

---

## Deprecations

| Endpoint / table | New state |
|---|---|
| `POST /api/sealed-write` | Returns `410 Gone` with body `{"error": "deprecated", "use": "/api/anky/submit"}`. Old iOS builds must fail loudly. |
| `GET /api/anky/public-key` | Returns `410 Gone`. |
| `POST /api/sessions/seal` | Returns `410 Gone`. |
| `sealed_sessions` table | Read-only. No writes. Data retained for existing records but not queried by new code paths. |
| `content` column on `ankys` | For rows created via `/api/anky/submit`: always NULL. Existing rows retain content until a cleanup migration is run separately. |
| Enclave infrastructure (`3.83.84.211`) | Stopped. No code path references it. |

## Unauthenticated Web Users

Unchanged. The `/write` plaintext path for unauthenticated web sessions remains. This legacy web-session writing is transient by design: it lives in request memory, generates a reflection + image, and is never persisted anywhere. This path does not need local-first treatment because there is no durable user session to return to later.

## Farcaster miniapp users

The miniapp continues to use the existing `/write` endpoint. The 3,456-seat Mirror cap remains on the miniapp as a social/public mechanic. The miniapp is the public funnel; it is intentionally *not* local-first because it has no persistent device.

---

## iOS invariants (testable)

These properties must hold on the iOS app and will be verified by the iOS implementation:

1. **Autosave:** writing persists to on-device SQLite within 500ms of the last keystroke. Killing the app mid-session loses at most 500ms of text.
2. **Crash recovery:** on app launch, any in-progress session is restorable from local storage.
3. **Offline submit:** completing a session with no network enqueues the submit. The submit retries with exponential backoff (1s, 2s, 4s, … capped at 5min) indefinitely until success or user deletion.
4. **No duplicate mint:** the submit queue uses `session_hash` as the dedupe key; retries after partial success are safe.
5. **No loss on failure:** a failed submit never deletes the local writing. Local writing is only deleted when the user explicitly deletes the session.
6. **Writing is always kept locally and backed up to iCloud.** There is no opt-out. The on-device SQLite DB is included in the app's iCloud backup domain, encrypted with the user's iCloud key. If the user loses their phone and restores from iCloud, all their writing returns.
7. **Derived artifacts persist locally:** reflection, title, and image URL are cached on-device so the anky grid works offline.

---

## Privacy policy (to be shown to users)

Short version:

> Your writing lives on your phone. When you finish an 8-minute session, we send the text to Claude to generate a reflection and image prompt — exactly like any other AI app — and immediately forget it. We store the reflection, the image, and a cryptographic hash that proves your session happened. We never store your writing.

Full policy will reference this doc as the implementation backing the promise.

---

## Open questions

- **Cross-device sync:** via iCloud backup. Writing survives device loss and restores on a new phone. Multi-device simultaneous use is out of scope for v1.
- **Claude provider:** direct Anthropic API or via OpenRouter. Either works. No architectural impact.

---

## Fresh start

This protocol is v2. Existing `sealed_sessions` data and existing `ankys` rows with populated `content` are preserved as a v1 artifact — read-only, not queried by new code paths, not migrated. The new system begins with an empty state. Users re-onboard into the new flow; old wallets and Mirror-era artifacts remain legacy history, while active Sojourn 9 access and scoring use Metaplex Core Looms plus Anky Seal Program receipts.
