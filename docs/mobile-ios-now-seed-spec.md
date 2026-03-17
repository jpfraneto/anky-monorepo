# iOS NOW + Base Seed Identity Spec

This is the implementation brief for the native iOS app.

It combines two product ideas:

1. a `NOW`-centered writing experience
2. a hidden Base/EVM seed identity that belongs to the user

The point is not to make people manage an account. The point is to let identity exist quietly in the background while the foreground stays radically simple: write now.

## Product Position

Anky mobile should feel like this:

- identity is created locally, not requested socially
- the app protects custody quietly under the hood
- unfinished writing stays private and local
- only a real anky crosses into persistence
- the UI keeps pulling the person back into `NOW`

This is not a feed app.
This is not a sign-up funnel.
This is a threshold practice.

## Canonical Product Rules

- A real anky is `>= 8 minutes` and `>= 300 words`.
- Anything below that threshold is not a real anky.
- Short writing remains local-only on device.
- Short writing must not unlock chat, reflections, or cloud history.
- A real anky is persisted to the backend.
- A real anky unlocks the fuller Anky experience.

## Identity Model

### Core Principle

The app creates a seed identity for the user on first open.

- 24-word recovery phrase
- generated locally on-device
- never sent to the backend
- used to derive a Base/EVM wallet
- recovered only by the user

The backend should know:

- the derived `0x...` address
- signed challenge proofs
- normal session tokens

The backend should never know:

- the recovery phrase
- the raw mnemonic seed

### Canonical Derivation

- BIP39 mnemonic
- EVM/secp256k1 account
- derivation path: `m/44'/60'/0'/0/0`

Do not drift from that path once the app ships.

### Identity Lifecycle

#### First app open

1. generate a 24-word phrase locally
2. derive the EVM private key locally
3. derive the `0x...` address locally
4. store the private key in iOS Keychain
5. present a recovery-phrase ceremony
6. authenticate silently against `/swift/v2/auth/*`

#### Normal daily use

- load the private key from Keychain
- derive the same `0x...` address
- request a challenge
- sign it locally using Ethereum `personal_sign` / EIP-191 semantics
- exchange it for a backend session token

The user should not experience this as “logging in.”

#### Recovery

- user enters the 24 words locally
- app re-derives the same EVM key
- app signs a new challenge
- backend restores the same identity because the wallet address matches

#### Reboot identity

- explicit user action only
- wipe local private key
- wipe local unfinished writing cache
- wipe local session token
- generate a brand-new phrase and address

This is a true identity reset.

### User-Facing Warnings

The copy should be direct:

- this phrase is your identity
- if you lose it, you lose your writings forever
- Anky cannot recover it for you
- the phrase never leaves your device

## Backend Contract

Base URL:

```text
https://anky.app
```

Target only the `/swift/v2/*` contract for new work.

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

- validates an EVM address
- stores a one-time challenge valid for 10 minutes
- returns a plain-text message to sign
- client must sign the exact returned message with Ethereum `personal_sign` / EIP-191 semantics

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

- backend computes the EIP-191 hash of the stored message
- backend recovers the signer address from the submitted signature
- backend compares that recovered address to `wallet_address`
- backend finds or creates the user for that address
- backend consumes the challenge
- backend returns a normal session token

### `GET /swift/v2/me`

Use this after login or restore to confirm identity.

Response includes:

- `user_id`
- `wallet_address`
- `total_writings`
- `total_ankys`

### `GET /swift/v2/writings`

Returns persisted server-side writings for the authenticated identity.

Important:

- because short sessions are local-only in v2, this list should effectively represent real ankys and any old server-side content

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

- requires bearer auth from the v2 seed flow
- short sessions are not persisted server-side
- only real ankys persist
- the `persisted` field is canonical and should drive client behavior

### `DELETE /swift/v2/auth/session`

- invalidates the current bearer token

## Required Client Architecture

### `SeedIdentityManager`

Responsibilities:

- generate mnemonic
- derive EVM private key
- derive `0x...` address
- sign challenge messages with EIP-191 semantics
- store/retrieve key material in Keychain
- wipe identity on reboot
- restore from phrase

### `AuthService`

Responsibilities:

- request challenge
- sign it via `SeedIdentityManager`
- verify and receive backend session token
- store session token in Keychain
- restore session on launch

### `WritingSessionStore`

Responsibilities:

- local in-progress text
- keystroke timing capture
- life/idle state
- pause/continue state
- local unfinished-session persistence

### `WritingAPI`

Responsibilities:

- submit completed sessions to `/swift/v2/write`
- fetch `/swift/v2/me`
- fetch `/swift/v2/writings`

### Top-Level App State

Recommended booleans:

- `hasLocalIdentity`
- `hasBackedUpPhrase`
- `isAuthenticated`
- `hasUnlockedFullExperience`
- `hasInProgressWriting`

The key distinction is:

- having an identity is not the same thing as having unlocked the full experience

## Onboarding Flow

Recommended sequence:

1. first open
2. brief explanation of the practice
3. generate identity silently
4. present recovery phrase ceremony
5. require user acknowledgment
6. authenticate silently
7. land on `WRITE NOW`

Do not put email, phone, social login, or conventional profile creation in front of the practice.

## Recovery Phrase Ceremony

The backup moment should feel:

- quiet
- serious
- beautiful
- minimal

The user should understand:

- this phrase restores the identity
- this phrase restores the writings
- Anky cannot recover it
- it never leaves the device

Avoid generic crypto-wallet onboarding aesthetics.

## Writing Experience Spec

### Core Visual Principle

The interface should keep pulling attention into `NOW`.

- no conventional text editor feel
- the current glyph dominates
- old text is ambient
- the present moment is the real object on screen

### Landing State

Show only:

- `WRITE NOW`
- `8 minutes`

### Active Writing

- the latest glyph is huge and centered
- the user should not feel like they are filling a normal text box
- typed history is secondary

### Bottom Ribbon

The bottom ribbon contains previous characters flowing endlessly to the left.

Rules:

- base font size is `16pt`
- apply rhythm/inter-keystroke multiplier on top
- letters are mostly white
- only a faint hint of chakra color remains
- the effect should be subtle, not loud

### Life / Idle Model

There are 2 lives.

Rules:

- after 3 seconds of inactivity, the active heart starts draining
- by 8 seconds of inactivity, that life is lost
- the center glyph stays visible during that window
- from 3s to 8s it should fade in sync with the heart drain
- over the same window it should crack/fracture open beautifully

Do not make the glyph disappear abruptly at 3 seconds.

### Pause / Continue State

If the user loses one life but still has another:

- freeze the writing state
- show `CONTINUE`
- keyboard input should resume immediately
- tapping `CONTINUE` should resume immediately
- the first resumed character must not be dropped

### Failed / Unfinished End State

If the user does not complete a real anky:

- no loading dots
- no fake processing state
- no reflection
- no chat

Only show:

- one subtle `try again` button
- a small `(8 minutes)` label below it

### Successful End State

If the user completes a real anky:

- persist it to the backend
- unlock the fuller shell
- make the reflective/conversation layer available

## Local Persistence Rules

Store locally:

- current draft text
- keystroke timing data
- local session id
- pause/life state if needed
- unfinished sessions

Do not send short writing to the backend through v2.

If a short session ends:

- it may remain visible locally if product wants recovery
- it must not appear in server history
- it must not unlock cloud features

After the first real anky:

- send it through `/swift/v2/write`
- trust `persisted`
- unlock the full shell only when persistence succeeds

## Unlock Model

### Locked Mode

Allowed:

- onboarding
- backup ceremony
- `NOW` writing
- local-only unfinished writing

Not allowed:

- conversation UI
- persisted anky history implying cloud sync
- broad account/profile UI

### Unlocked Mode

After the first real anky:

- show persisted history
- show reflections and conversation surfaces
- allow threaded conversation below ankys the user actually wrote

If thread APIs are not in the app/backend yet, keep those surfaces hidden rather than faking them.

## Localization

The app should follow device language.

Default language set:

- `en`
- `es`
- `pt`
- `fr`
- `de`
- `it`
- `nl`
- `pl`
- `tr`
- `ru`
- `ja`
- `ko`
- `zh`
- `ar`
- `hi`
- `id`

At minimum localize:

- backup ceremony copy
- `WRITE NOW`
- `8 minutes`
- `CONTINUE`
- `try again`
- lives / words labels
- unlocked reflection and conversation labels

## Security Requirements

The only acceptable signing model is local signing with the seed-derived EVM key.

Do not:

- send the 24 words to the backend
- send the raw seed to the backend
- use Privy as the primary identity model for the new app
- let short writing persist server-side
- let account UI dilute the feeling of `NOW`

Preferred next hardening step:

- use a vetted BIP39 + EVM derivation library if possible instead of custom crypto code

If any local implementation remains custom, require deterministic test vectors for:

- mnemonic checksum
- derivation path
- private key derivation
- public address derivation
- challenge signing
- backend verification compatibility

## QA Checklist

- fresh install creates a new identity
- app derives a valid `0x...` address
- challenge/verify works silently
- reinstall + phrase restore returns the same address
- reboot identity produces a different address
- short `/swift/v2/write` returns `persisted: false`
- short sessions do not appear in `/swift/v2/writings`
- real ankys return `persisted: true`
- real ankys unlock the full shell
- continue flow does not drop the first resumed character
- failed end state contains no loading state
- device language localizes the core copy

## Current Backend Reality

As of March 16, 2026:

- `POST /swift/v2/auth/challenge` is live
- `POST /swift/v2/auth/verify` is live
- `GET /swift/v2/me` is live
- `GET /swift/v2/writings` is live
- `POST /swift/v2/write` is live
- `DELETE /swift/v2/auth/session` is live

The current backend model is Base/EVM wallet address identity with EIP-191-style challenge signing.

## Direct Instruction To The Mobile Agent

Build the updated app against `/swift/v2/*`.

Implement in this order:

1. local seed identity generation
2. Keychain persistence and restore
3. challenge/verify auth
4. locked vs unlocked shell
5. `NOW` writing UI
6. local-only short-session handling
7. unlock after first persisted real anky
8. persisted history and later conversation layer
9. localization pass

Do not build the new app around Privy.
Do not send the 24-word phrase to the backend.
Do not persist short sessions to the server.
Do not let account UI dilute the feeling of `NOW`.
