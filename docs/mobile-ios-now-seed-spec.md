# iOS NOW + Seed Identity Implementation Spec

This document is the implementation brief for the native iOS app.

It combines two product shifts:

1. the new `NOW` writing experience
2. the new seed-phrase-based identity model

The goal is to make identity invisible, sovereign, and calm, while making writing feel immediate, stripped-down, and centered on `NOW`.

## Product Intent

The app should feel like this:

- identity is born quietly on first open
- the user does not create an account with email, phone, or social login
- the app generates a recovery phrase locally and carries the identity for them
- the user experiences writing first, not account setup
- the writing UI collapses attention into the present moment
- incomplete writing remains local and private
- only a true anky unlocks persistence, wallet-backed identity continuity, and the full reflective interface

The product rule is simple:

- before the first real anky, the experience is private, local, and minimal
- after a real anky, the user steps into the fuller Anky system

## Canonical Rules

These are the product rules the app should treat as canonical.

- A real anky is `>= 8 minutes` and `>= 300 words`.
- Anything below that threshold is not a real anky.
- Non-anky writing stays local-only on device.
- Non-anky writing should not unlock chat, reflection, threads, or expanded account surfaces.
- A real anky is persisted to the backend.
- A real anky unlocks the full experience.
- The full experience includes persisted ankys, identity continuity, and threaded conversation with Anky below ankys the user actually wrote.

## Identity Model

### Core Principle

The recovery phrase is the root identity secret.

- It is generated on-device.
- It never reaches the backend.
- The backend never stores the phrase.
- The backend never needs the phrase again.
- The backend only knows the derived public key and signed proofs.

### Identity Lifecycle

#### First app open

- Generate a 24-word BIP39-compatible recovery phrase locally.
- Derive a Solana-compatible Ed25519 keypair locally from that phrase.
- Store the private key in iOS Keychain.
- Prefer iCloud Keychain sync if product wants identity continuity across devices under the user's Apple account.
- Show the recovery phrase to the user once in a dedicated backup ceremony.

#### Normal daily use

- Do not ask the user for the phrase.
- Load the private key from Keychain.
- Authenticate silently by signing a backend challenge.
- Cache the returned backend session token in Keychain.

#### Recovery on new device or after reinstall

- Ask the user to enter the 24 words locally.
- Re-derive the same keypair locally.
- Sign a backend challenge.
- The backend restores the same identity because the public key matches.

#### Reboot identity

- Explicit user action only.
- Delete local private key, local phrase backup state, and local cached unfinished writing.
- Generate a new phrase and new keypair.
- Treat this as a fully new identity.

### Security Warnings

The user-facing copy should be direct:

- if you lose this phrase, you lose your writings forever
- Anky cannot recover it for you
- the phrase never leaves your device

This should be treated as a product truth, not marketing copy.

## Key Management Requirements

The iOS app should implement the following locally:

- 24-word phrase generation
- deterministic Ed25519 key derivation
- public key derivation
- raw message signing
- secure Keychain persistence

Use one stable derivation path and never change it after shipping.

Recommended product choice:

- BIP39 mnemonic
- Solana/Ed25519 derivation
- one canonical derivation path for all users, for example `m/44'/501'/0'/0'`

If a different path is chosen, it must be frozen before public rollout.

## Backend Contract

The backend seed-identity flow is live now.

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
  "expires_at": "2026-03-16 20:07:23"
}
```

Behavior:

- validates public key shape
- creates a one-time challenge
- challenge expires after 10 minutes
- app must sign the exact `message` bytes returned

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

- verifies Ed25519 signature over the stored challenge message
- finds or creates the user for that public key
- consumes the challenge so it cannot be replayed
- returns a normal bearer session token

### `GET /swift/v2/me`

Use this after login or recovery to confirm identity.

Response includes:

- `user_id`
- `wallet_address`
- `total_writings`
- `total_ankys`

### `GET /swift/v2/writings`

Use this for the persisted history view after unlock.

Important:

- because short sessions are local-only in v2, this list should effectively represent persisted ankys and any older server-side writings

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
  "wallet_address": "Base58PublicKey"
}
```

Behavior:

- requires bearer token from the seed auth flow
- sessions under the anky threshold are not persisted
- real ankys are persisted
- real ankys trigger the normal generation pipeline
- response makes persistence explicit through `persisted`

### `DELETE /swift/v2/auth/session`

- invalidates the current session token

## Login Flow

The app should use this exact order:

1. load private key from Keychain
2. derive public key
3. call `POST /swift/v2/auth/challenge`
4. sign the returned `message` locally
5. call `POST /swift/v2/auth/verify`
6. store `session_token` in Keychain
7. call `GET /swift/v2/me`
8. route the user based on unlock state and local session state

If Keychain has no identity:

1. generate phrase
2. derive keypair
3. save key locally
4. show backup ceremony
5. then perform the same challenge/verify flow

## Writing Experience Spec

This is the core interaction model the app should match.

### Visual Principle

The screen must focus on `NOW`.

- the dominant thing on screen is the current glyph
- the UI should not feel like a traditional text editor
- previous text is secondary and ambient
- everything should reinforce immediacy and impermanence

### Initial state

The landing state is minimal:

- `WRITE NOW`
- `8 minutes`

No heavy chrome.
No account ceremony before the writing invitation.

### Active writing state

- show only the latest glyph very large in the center
- do not show a conventional multiline text field
- preserve a sense of invisibility around the typed history

### Bottom ribbon

The bottom ribbon contains previously typed characters flowing endlessly to the left.

Rules:

- base font size is `16pt`
- apply the existing rhythm/inter-keystroke multiplier on top of that base
- letters should be larger than before
- letters should be mostly white
- chakra colors should be visible only as a faint tint
- the effect should stay subtle and atmospheric

### Idle / life mechanic

There are 2 lives.

Rules:

- after 3 seconds of inactivity, the active heart starts draining
- by 8 seconds of inactivity, that life is lost
- the main glyph must stay visible and fade during that window
- the fade should happen in sync with the heart drain
- over the same 8-second idle window, the glyph should crack or fracture open beautifully

Important correction:

- the current glyph must not disappear on a fixed 3-second timer
- it should fade gradually from 3 seconds to 8 seconds

### Pause / continue state

If the user loses one life but still has another left:

- freeze the current writing state
- show a `CONTINUE` action
- pressing a key should resume immediately
- tapping `CONTINUE` should resume immediately
- the first resumed character must not be dropped

### Failed / unfinished end state

If the user does not complete a real anky:

- no loading dots
- no processing state
- no fake waiting state
- no chat
- no reflection

Only show:

- one subtle `try again` button
- a small `(8 minutes)` label below it

This state should feel clean, quiet, and final.

### Successful anky end state

If the user completes a real anky:

- transition into the unlocked Anky experience
- persisted writing now exists on the backend
- full UI can open
- the conversation interface with Anky becomes available

## Local Persistence Rules

Before the first real anky, local storage matters.

Store locally:

- current in-progress writing text
- keystroke timing data if used for flow score
- local draft/session id
- whether the writing was interrupted
- whether the backup ceremony was completed

Do not send short writing to the backend through the v2 path.

If a short session ends:

- keep it available locally if product still wants local reflection or recovery of unfinished text
- but do not treat it as persisted identity content
- do not show it inside the unlocked cloud history

Once the user completes a real anky:

- send it through `POST /swift/v2/write`
- let the backend persist it
- clear any obsolete local-only draft fragments that should not remain part of the long-term record

## Unlock Model

The full UI should stay locked until the first real anky.

### Locked mode

Allowed:

- onboarding
- backup ceremony
- `NOW` writing flow
- local-only unfinished writing

Not allowed:

- threaded conversation UI
- expanded history UI that implies full sync
- account/settings surfaces that conflict with the identity-less feel

### Unlocked mode

After first real anky:

- show persisted anky history
- show reflection / conversation surfaces
- allow threaded conversation below ankys the user actually wrote
- preserve the seed-identity model as the hidden account layer

## Conversation Rules

The user can thread conversations below ankys they have written.

Interpretation for iOS:

- no conversation surface for unfinished or local-only writing
- conversation unlock is tied to real ankys
- the conversation should feel like a continuation of a completed piece, not a generic chatbot

## Localization

The app should follow device language.

Current default language set:

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

- onboarding backup copy
- `WRITE NOW`
- `8 minutes`
- `CONTINUE`
- `try again`
- any resume hint
- words / lives labels
- unlocked reflection and conversation labels

## Suggested App Architecture

The native app should separate these concerns:

### IdentityManager

Responsibilities:

- phrase generation
- key derivation
- signing
- Keychain persistence
- wipe / reboot identity
- recovery import

### AuthService

Responsibilities:

- request challenge
- verify signature
- store session token
- refresh authenticated app state on launch

### WritingSessionStore

Responsibilities:

- local in-progress writing state
- local short-session history if product wants it
- keystroke timing capture
- life/pause state
- recovery after interruption

### WritingAPI

Responsibilities:

- submit real ankys to `/swift/v2/write`
- fetch persisted writings from `/swift/v2/writings`
- fetch `/swift/v2/me`

### AppState

Recommended top-level flags:

- `hasLocalIdentity`
- `hasBackedUpPhrase`
- `isAuthenticated`
- `hasUnlockedFullExperience`
- `hasInProgressWriting`

## Onboarding Flow

Recommended sequence:

1. first open
2. brief statement of what Anky is
3. generate identity silently
4. show recovery phrase ceremony
5. require user acknowledgment that Anky cannot recover it
6. enter the `WRITE NOW` landing state

Do not front-load the app with settings, profile creation, or social login choices.

## Recovery Phrase Ceremony

The backup ceremony is a critical product moment.

It should feel:

- slow
- serious
- beautiful
- minimal

The user should understand:

- this phrase is their identity
- this phrase restores their writings
- this phrase never leaves the device
- if they lose it, Anky cannot restore access

Avoid turning this into a generic web3 wallet onboarding flow.

## Migration From Current Mobile App

The current mobile backend path used Privy.

New direction:

- new app versions should move to `/swift/v2/*`
- seed identity should become the primary mobile identity model
- Privy can remain in code temporarily as legacy support, but new product work should target seed identity

Migration posture:

- do not try to merge Privy identity and seed identity automatically in the first iteration
- treat seed identity as the new canonical path
- if legacy account migration is needed later, design it as a separate explicit migration project

## Error Handling

### Challenge expired

- request a new challenge automatically
- do not ask the user to do anything special

### Signature verification failed

- retry once with a fresh challenge
- if still failing, surface a generic identity error
- do not expose cryptographic jargon

### Missing local key

- if expected on cold start, route into recovery or new identity flow

### Backend unavailable

- keep local writing intact
- never lose in-progress text because network auth failed

### Session token invalid

- repeat challenge/verify silently if local key exists

## QA Checklist

The mobile agent should verify all of this:

- fresh install generates a new phrase
- app can authenticate without showing login UI
- reinstall + phrase restore recovers same wallet/public key
- reboot identity produces a different wallet/public key
- no phrase is sent over the network
- short sessions return `persisted: false`
- short sessions do not appear in persisted history
- real ankys return `persisted: true`
- real ankys unlock full experience
- continue flow does not drop the first resumed character
- failed end state has no loading indicators
- copy follows device language
- Keychain persistence survives app relaunch
- iCloud Keychain behavior is understood and tested

## Current Backend Reality

As of March 16, 2026:

- the seed-identity backend path is live
- `POST /swift/v2/auth/challenge` is live
- `POST /swift/v2/auth/verify` is live
- `GET /swift/v2/me` is live
- `GET /swift/v2/writings` is live
- `POST /swift/v2/write` is live
- `DELETE /swift/v2/auth/session` is live

The backend has been verified end-to-end:

- challenge creation
- signature verification
- session issuance
- wallet-backed identity lookup
- short `v2/write` returning `persisted: false`

## Direct Instructions To The Mobile Agent

Build the updated app against `/swift/v2/*`.

Implement this in order:

1. local seed identity generation
2. Keychain persistence and recovery import
3. challenge/verify auth flow
4. `NOW` writing UI
5. local-only short-session handling
6. unlock transition after first real anky
7. full persisted history and conversation surfaces
8. localization pass

Do not build the new app around Privy.
Do not send the 24-word phrase to the backend.
Do not persist short sessions to the server.
Do not let account UI dilute the feeling of `NOW`.
