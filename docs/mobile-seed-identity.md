# Mobile Seed Identity

This is the current identity model for the native iOS app.

It is Base/EVM-based, not Solana-based.

## Product Model

- Identity is created locally on first app open.
- The app generates a 24-word recovery phrase on-device.
- The phrase never reaches the backend.
- The phrase derives a Base/EVM secp256k1 keypair locally.
- The backend only sees the derived `0x...` wallet address and signed challenges.
- The private key should live in iOS Keychain and may be synced through iCloud Keychain.
- If the user loses the recovery phrase and the local Keychain copy, they lose access forever.
- "Reboot identity" means wiping the local key material and creating a completely new identity.

## Canonical Derivation

- BIP39 mnemonic
- EVM/secp256k1 account
- canonical derivation path: `m/44'/60'/0'/0/0`

Freeze that path before public rollout and do not drift later.

## Backend Endpoints

### `POST /swift/v2/auth/challenge`

Request:

```json
{
  "wallet_address": "0x1234..."
}
```

Response:

```json
{
  "ok": true,
  "challenge_id": "uuid",
  "message": "anky.app base identity sign in\n\naddress: 0x1234...\nchallenge id: ...",
  "expires_at": "2026-03-16 20:07:23"
}
```

Behavior:

- validates that `wallet_address` is a valid EVM address
- creates a one-time challenge valid for 10 minutes
- the app must sign the exact returned `message`
- signing semantics are Ethereum `personal_sign` / EIP-191 style

### `POST /swift/v2/auth/verify`

Request:

```json
{
  "wallet_address": "0x1234...",
  "challenge_id": "uuid",
  "signature": "0x..."
}
```

Response:

```json
{
  "ok": true,
  "session_token": "uuid",
  "user_id": "uuid",
  "wallet_address": "0x1234..."
}
```

Behavior:

- verifies the EVM signature by recovering the signer address from the stored challenge
- finds or creates the Anky user for that wallet address
- consumes the challenge so it cannot be replayed
- returns a normal backend session token

### `DELETE /swift/v2/auth/session`

- invalidates the current bearer token

### `GET /swift/v2/me`

- returns the authenticated user profile
- includes `wallet_address`, `total_writings`, and `total_ankys`

### `GET /swift/v2/writings`

- returns persisted writings for the authenticated identity
- because short sessions are local-only in v2, this list should effectively represent real persisted ankys and any older server-side writings

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

Short-session response:

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

Real-anky response:

```json
{
  "ok": true,
  "session_id": "uuid",
  "is_anky": true,
  "word_count": 412,
  "flow_score": 0.83,
  "persisted": true,
  "anky_id": "uuid",
  "wallet_address": "0x1234..."
}
```

Behavior:

- requires bearer auth from the seed-identity flow
- sessions under the true anky threshold are not written to the database
- a true anky remains `>= 8 minutes` and `>= 300 words`
- real ankys are persisted and kick off the normal generation pipeline

## Swift Client Responsibilities

- generate the 24-word phrase locally
- derive the Base/EVM private key locally
- derive the `0x...` address locally
- store the private key in Keychain
- sign the backend challenge with `personal_sign` / EIP-191 semantics
- never send the recovery phrase to the backend
- show the phrase once during onboarding with explicit warning copy

Cold start flow:

1. load key from Keychain
2. derive `0x...` address
3. request challenge
4. sign the returned message locally
5. verify and receive `session_token`
6. store `session_token` in Keychain

Recovery flow:

1. user enters the 24 words locally
2. app derives the same EVM account locally
3. app signs a fresh challenge
4. backend restores the same identity because the recovered wallet address matches

## Current Boundary

- The Base/EVM seed identity auth path exists on the backend.
- The new mobile write path for this model is `POST /swift/v2/write`.
- The legacy mobile path `/swift/v1/*` still exists, but the new product direction should target `/swift/v2/*`.
- The web `NOW` flow still uses anonymous UUID identity today; it has not yet been moved to seed identity.
