# iOS Agent Prompt — Local-First Migration

You are working on the Anky iOS app. Your job is to migrate the app from the current "sealed write" architecture (where writing was encrypted on-device and sent to a Nitro enclave via the backend) to a **local-first** architecture where the device is the source of truth and the backend only stores derived artifacts.

Read this entire prompt before writing any code.

---

## Background

Anky is a writing-practice app. A "writing session" is an 8-minute (≥480 seconds, ≥50 words) stream-of-consciousness session where the user cannot use backspace, cannot press enter, and cannot edit — only forward keystrokes. Every keystroke is recorded with its delta-ms timing. The result is a `.anky` string:

```
0 i
364 m
353  
309 b
131 a
```

Each line is `{delta_ms} {character}`. SHA-256 of the `.anky` string's UTF-8 bytes = `session_hash`. This hash is logged on Solana via spl-memo to prove the session existed.

Once a session completes, Claude generates a reflection + title + image prompt, and an image is generated from the prompt. Together these + the session_hash + Solana tx = an "anky" — the artifact of the session.

The user's identity is a Solana wallet held on the phone.

---

## Why this migration

The previous architecture ran a $90/mo AWS Nitro Enclave on EC2 that decrypted sealed envelopes server-side. We are killing the enclave. The new architecture is simpler, cheaper, more private, and tells a story users actually trust:

> Your writing lives on your phone. When you finish a session, we send the text to Claude to generate a reflection and image — exactly like any other AI app — and immediately forget it. We store the reflection, the image, and a cryptographic hash that proves your session happened. We never store your writing.

This is the Hailee-style promise. The iOS app is what makes it true.

---

## The contract

The backend team has committed to a protocol. Your implementation must match it exactly. Here it is verbatim:

<<<BEGIN PROTOCOL>>>

# Anky Local-First Protocol

## The `.anky` session format

A writing session is a string where each line is one keystroke: `{delta_ms} {character}`. The first line's delta is `0`. Characters are literal (including spaces). No enter, no backspace — only forward keystrokes. The SHA-256 of this string's UTF-8 bytes is the `session_hash`.

## Principles

1. The iOS device is the source of truth for writing sessions.
2. The backend never persists plaintext writing from authenticated users.
3. Plaintext may transit server RAM only for the duration of a single Claude API call, then is discarded.
4. A writing session is never lost because of the network. Local persistence is unconditional. Server submit is best-effort with unbounded retry.
5. Simplicity is a feature.

## The submit endpoint

### `POST /api/anky/submit`

**Auth:** `Authorization: Bearer <token>` (existing iOS session token).

**Request body:**

```json
{
  "session_hash": "<sha256 hex, 64 chars>",
  "duration_seconds": 483,
  "word_count": 612,
  "kingdom": "eleasis",
  "started_at": "2026-04-13T14:22:00Z",
  "wallet_signature": "<base58 signature of session_hash by user's Solana wallet>",
  "session": "<full .anky keystroke stream>"
}
```

**Response:** `text/event-stream` (SSE)

Events, in order:

```
event: accepted        data: {"anky_id": "<uuid>"}
event: title           data: {"title": "..."}
event: reflection_chunk data: {"text": "..."}   # repeated
event: reflection_complete data: {"reflection": "<full text>"}
event: image_url       data: {"image_url": "https://..."}
event: solana          data: {"signature": "<tx sig>"}
event: done            data: {"anky_id": "<uuid>"}
```

On error: `event: error  data: {"stage": "claude|image|solana|persist", "retryable": true, "message": "..."}`.

Errors during `solana` or `image` still persist the anky with partial fields and return an `anky_id` — the client considers the session submitted. Missing fields get filled in by server-side retry workers.

Errors during `claude` do NOT persist anything. The client retries the whole submit later.

### Idempotency

`session_hash` is the idempotency key scoped to wallet. Resubmitting the same session is safe and returns the existing anky.

## Deprecated (the app must stop calling these)

- `POST /api/sealed-write`
- `GET /api/anky/public-key`
- `POST /api/sessions/seal`
- Any endpoint related to the Nitro enclave

These return `410 Gone`. Remove all code paths that call them.

<<<END PROTOCOL>>>

---

## Your tasks

### 1. On-device session storage (SQLite + iCloud backup)

Create or update an on-device SQLite database (e.g. via GRDB or SQLite.swift — whichever the app already uses; check the project first). Schema:

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,           -- UUID generated on device
    anky_string TEXT NOT NULL,     -- full .anky keystroke stream
    session_hash TEXT UNIQUE,      -- computed once session completes
    started_at TEXT NOT NULL,      -- ISO8601
    ended_at TEXT,                 -- ISO8601, null while in progress
    duration_seconds INTEGER,
    word_count INTEGER,
    kingdom TEXT,
    sojourn INTEGER,
    submit_state TEXT NOT NULL,    -- 'in_progress' | 'pending' | 'submitting' | 'submitted' | 'partial' | 'failed'
    submit_attempts INTEGER NOT NULL DEFAULT 0,
    last_submit_error TEXT,
    anky_id TEXT,                  -- server UUID once accepted
    title TEXT,                    -- from server
    reflection TEXT,               -- from server
    image_url TEXT,                -- from server
    solana_signature TEXT,         -- from server
    created_at TEXT NOT NULL
);
```

The DB file MUST be located in the app's Documents directory (or another iCloud-backed container) so it is included in the user's iCloud backup. Do NOT place it in Caches or tmp. Verify that `NSURLIsExcludedFromBackupKey` is NOT set on the DB file.

### 2. Writing UI: autosave per keystroke

On every accepted keystroke in the writing view:

- Append `{delta_ms} {character}\n` to an in-memory buffer.
- Debounce an async write to SQLite: flush the buffer to the `sessions.anky_string` column every 300ms (or on the next keystroke if >300ms has passed since the last flush).
- If the app is backgrounded or killed, at most 300ms of writing is lost.

On writing view load:

- If a session with `submit_state = 'in_progress'` exists, offer to resume it. The session is recoverable from SQLite alone — do not rely on any in-memory state.

Do not change the existing keystroke-capture logic or banned-keys enforcement (enter, backspace, editing operations all remain blocked). This task is only about where the data lives as it's captured.

### 3. Session completion

When the user finishes a session (≥480s and ≥50 words, or when they manually end):

- Mark `ended_at`, compute `duration_seconds` and `word_count`.
- Compute `session_hash = SHA256(anky_string.utf8Bytes)`.
- Compute `kingdom` and `sojourn` from `started_at` via the Ankyverse calendar (existing logic in the app — do not reimplement).
- Set `submit_state = 'pending'`.
- Sign `session_hash` with the user's Solana wallet (existing wallet module in the app).
- Enqueue the session for submit (see task 4).

### 4. Submit queue with unbounded retry

Implement a persistent submit queue. On each tick:

- Find all sessions with `submit_state IN ('pending', 'partial', 'failed')`.
- For each, attempt `POST /api/anky/submit` as an SSE stream.
- Set `submit_state = 'submitting'` during the attempt.
- Stream events as they arrive; update the row incrementally:
  - `accepted` → store `anky_id`
  - `title` → store `title`
  - `reflection_chunk` → append to `reflection` (show in UI as it streams)
  - `reflection_complete` → finalize `reflection`
  - `image_url` → store `image_url`
  - `solana` → store `solana_signature`
  - `done` → set `submit_state = 'submitted'`
  - `error` with stage `solana` or `image` → set `submit_state = 'partial'`, `anky_id` is still valid, session is "submitted"; background worker will poll `/api/anky/{id}` later to fill missing fields
  - `error` with stage `claude` or `persist` → set `submit_state = 'failed'`, increment `submit_attempts`
- On transport failure (connection reset, timeout, no `done`) → treat as retryable, set `submit_state = 'failed'`, increment attempts.

Retry strategy: exponential backoff starting at 1s, doubling, capped at 5 minutes. No maximum attempt count. The queue retries until success or until the user deletes the session.

Retries are safe because `session_hash` is the server's idempotency key.

### 5. Polling for partial submissions

For any session with `submit_state = 'partial'` and an `anky_id`, poll `GET /api/anky/{anky_id}` every 30 seconds. When the response shows all fields populated (`image_url`, `solana_signature`), set `submit_state = 'submitted'`.

### 6. Settings / privacy screen

Add or update a privacy/settings screen with this copy, verbatim, at the top:

> Your writing lives on this phone. When you finish a session, we send the text to Claude to generate a reflection and image — exactly like any other AI app — and immediately forget it. We store the reflection, the image, and a cryptographic hash that proves your session happened. We never store your writing.

Below the copy, show:

- **Where your writing lives**: "On this device, backed up to your iCloud. [N] sessions stored."
- **Export all writing**: button that dumps all `.anky` strings + derived artifacts as a `.zip` the user can save anywhere.
- **Delete a session**: from the sessions list, long-press → delete. Deleting removes the local row. The derived artifacts on the server remain (they're de-identified except for the wallet).

Do NOT add a "keep writing on device" toggle. Writing is always kept locally and backed up to iCloud — there is no opt-out.

### 7. Rip out sealed-write code

Remove all code that:

- Fetches the enclave public key (`GET /api/anky/public-key`).
- Encrypts writing with X25519/AES-GCM for enclave submission.
- Calls `POST /api/sealed-write` or `POST /api/sessions/seal`.
- References the enclave, Nitro, attestation, or `3.83.84.211`.

Delete the code. Don't leave it behind feature flags.

### 8. Verify testable invariants

Before reporting back, manually verify each:

1. **Autosave**: start a session, type 50 characters, force-quit the app after the last keystroke, relaunch. Writing is present (minus at most the last 300ms).
2. **Crash recovery**: during an in-progress session, kill the app. On relaunch, the app offers to resume the session with all keystrokes intact.
3. **Offline submit**: complete a session in airplane mode. Verify the session is saved with `submit_state = 'pending'`. Re-enable network. Verify the submit succeeds within 2 retries.
4. **No duplicate mint**: successfully submit a session, then manually re-trigger submit for the same row. Verify the server returns the existing anky (same `anky_id`), not a new one.
5. **No loss on failure**: submit a session with the backend deliberately returning 500. Verify `submit_state = 'failed'` and `anky_string` is still intact in SQLite.
6. **iCloud backup**: check that the SQLite DB file does NOT have `NSURLIsExcludedFromBackupKey` set. Use `URLResourceValues` to verify.

---

## Out of scope

- Multi-device simultaneous use (one device per user for v1).
- Any changes to the Farcaster miniapp or web.
- Any changes to wallet creation, cNFT minting flow, or Solana signing primitives — reuse what's there.
- Any changes to the writing UI's visual design or banned-keys enforcement.

---

## Report back

When done, reply with:

1. A list of files you created or modified.
2. The result of each of the 6 invariant checks above — pass / fail / couldn't test, with a one-sentence note each.
3. Any deviations from this prompt, with reasoning.
4. Anything you think is wrong with the protocol or the approach — you have fresh eyes and the backend team values the pushback.

Do not report back with a summary of what the protocol says. We wrote it; we know. Report what you did.
