# Mobile Seed Identity

This is the new identity model for the native iOS app.

## Product Model

- Identity is created locally on first app open.
- The app generates a 24-word recovery phrase on-device.
- The phrase never reaches the backend.
- The phrase derives a Solana/Ed25519 keypair locally.
- The private key lives in iOS Keychain and should be syncable with iCloud Keychain.
- The backend only sees the public key (`wallet_address`) and signed challenges.
- If the user loses the phrase and the local Keychain copy, they lose access forever.
- "Reboot identity" means wiping the local key material and creating a new phrase.

## Backend Endpoints

### `POST /swift/v2/auth/challenge`

Request:

```json
{
  "wallet_address": "Base58PublicKey"
}
```

Response:

```json
{
  "ok": true,
  "challenge_id": "uuid",
  "message": "anky.app seed identity sign in\n\npublic key: ...",
  "expires_at": "2026-03-16 16:20:00"
}
```

Behavior:

- Validates the public key shape.
- Creates a one-time challenge that expires after 10 minutes.
- The app must sign the exact `message` bytes locally.

### `POST /swift/v2/auth/verify`

Request:

```json
{
  "wallet_address": "Base58PublicKey",
  "challenge_id": "uuid",
  "signature": "Base58Signature"
}
```

Response:

```json
{
  "ok": true,
  "session_token": "uuid",
  "user_id": "uuid",
  "wallet_address": "Base58PublicKey"
}
```

Behavior:

- Verifies the Ed25519 signature against the stored challenge message.
- Finds or creates the Anky user for that public key.
- Consumes the challenge so it cannot be replayed.
- Returns a normal Anky session token.

### `DELETE /swift/v2/auth/session`

- Same behavior as `DELETE /swift/v1/auth/session`.
- Invalidates the bearer token.

### `GET /swift/v2/me`

- Same behavior as `GET /swift/v1/me`.
- Useful to confirm the restored identity after recovery.

### `GET /swift/v2/writings`

- Same behavior as `GET /swift/v1/writings`.
- Because short sessions are local-only in v2, this list effectively becomes the user's persisted ankys and any older stored writings.

### `POST /swift/v2/write`

Request:

```json
{
  "text": "stream of consciousness...",
  "duration": 486.2,
  "session_id": "optional-client-session-id",
  "keystroke_deltas": [0.11, 0.08, 0.42]
}
```

Response for a short or failed session:

```json
{
  "ok": true,
  "session_id": "uuid",
  "is_anky": false,
  "word_count": 184,
  "flow_score": 0.71,
  "persisted": false
}
```

Response for a true anky:

```json
{
  "ok": true,
  "session_id": "uuid",
  "is_anky": true,
  "word_count": 412,
  "flow_score": 0.83,
  "persisted": true,
  "anky_id": "uuid",
  "wallet_address": "Base58PublicKey"
}
```

Behavior:

- Requires bearer auth from the new seed-identity sign-in flow.
- Sessions under the real anky threshold are not written to the database.
- A real anky remains `>= 8 minutes` and `>= 300 words`.
- Real ankys are persisted and kick off the normal background generation pipeline.
- If the authenticated user does not have a wallet/public key attached, the endpoint rejects the request.

## Swift Client Responsibilities

- Generate the 24-word phrase locally with a BIP39-compatible library.
- Derive the Solana/Ed25519 keypair locally from that phrase.
- Store the private key in Keychain.
- Prefer `kSecAttrSynchronizable` for iCloud Keychain backup if that matches the desired UX.
- Show the recovery phrase exactly once during onboarding, with explicit warning copy.
- Never send the phrase to the backend.
- On cold start:
  - load key from Keychain
  - if present, request challenge
  - sign message
  - verify
  - store returned `session_token` in Keychain
- On recovery:
  - user enters 24 words locally
  - app derives the same keypair
  - app signs a new challenge
  - backend restores the same identity because the public key matches

## Current Boundary

- The new seed identity auth path exists on the backend.
- The new mobile write path for this model is `POST /swift/v2/write`.
- The legacy mobile write path (`/swift/v1/write`) still uses the older behavior and persists short sessions.
- The web `NOW` flow is already on the newer product behavior, but it still uses anonymous UUID identity rather than seed identity.
