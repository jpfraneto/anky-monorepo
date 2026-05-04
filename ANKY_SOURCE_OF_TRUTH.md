# Anky — Source of Truth

_Last updated: 2026-04-29_

This document is the working source of truth for what Anky is, why it exists, and how the current implementation is being shaped.

It should be treated as the spine of the product while the app, Solana integration, and mirror economy are being built.

---

# 1. The essence

Anky is a private writing ritual with optional public witness and optional paid mirrors.

The core act is simple:

> **A person writes forward for 8 minutes.**

No feed.
No followers.
No performance.
No editing.
No escape into revision.

The writing session becomes a `.anky` file.

That file is the trace.
That trace can be hashed.
That hash can be sealed.
That sealed hash can be witnessed publicly.
That writing can later be mirrored by AI if the user chooses.

But the writing itself stays with the writer.

---

# 2. The core sentence

The cleanest summary of the full system is:

> **Write without a wallet. Seal with a Loom. Mirror with credits. Your writing stays with you.**

And the cleanest sentence for the Solana action is:

> **Wallet W used Loom L to anchor `.anky` hash H at Solana time T.**

Everything else is explanation.

---

# 3. The product layers

Anky only makes sense when each layer knows its place.

```text
write        = the human act
.anky        = the private trace
hash         = the identity of the trace
loom         = the public instrument
seal         = the witness
credits      = the mirror economy
reflection   = the fruit
archive      = the writer’s memory
```

The layers should never pretend to be each other.

- The `.anky` file is truth.
- Solana is witness.
- The Loom is the instrument.
- Credits pay for mirrors.
- The backend processes, but should not become the archive.
- The app is the writing instrument.
- The user keeps the memory.

---

# 4. The non-negotiable principle

> **The user can always write.**

No account should block writing.
No wallet should block writing.
No Loom should block writing.
No credits should block writing.
No network failure should block writing.
No Solana issue should block writing.

Writing is the seed.
Everything else is fruit.

---

# 5. What Anky is not

Anky is not primarily:

- a journaling app,
- a productivity app,
- a social network,
- a feed,
- a Solana app,
- an NFT project,
- an AI wrapper,
- a mental health claim,
- a proof-of-humanity protocol.

Anky is an encounter protocol.

It gives a person a container for meeting the page and preserving the trace of that meeting.

---

# 6. Proof of Encounter

The phrase for the deeper category is:

> **PoE — Proof of Encounter**

Proof of Encounter does not prove identity.
It does not prove sincerity.
It does not prove humanness.
It does not prove literary quality.
It does not prove consciousness.

It proves something smaller and more honest:

> **A trace existed. It had this exact hash. It was anchored at this time. The content can remain private.**

This matters because Anky should never overclaim.

The proof is humble.
The ritual is not.

---

# 7. The `.anky` protocol

A writing session is a plain text `.anky` file.

The first accepted character begins the session.
The first line stores the absolute Unix epoch millisecond timestamp and the first character.
Every following line stores the delay since the previous accepted character and the next character.
The session closes with a terminal `8000` line.

Conceptually:

```text
{epoch_ms} {character}
{delta_ms_0000_to_7999} {character}
{delta_ms_0000_to_7999} {character}
...
8000
```

The file is UTF-8 plain text.
The file is immutable after the terminal `8000` is written.

No metadata belongs inside the canonical file.
No reflection belongs inside the canonical file.
No title belongs inside the canonical file.
No wallet belongs inside the canonical file.
No author name belongs inside the canonical file.

The `.anky` file is the writing session itself.

---

# 8. The hash

The session hash is:

```text
session_hash = sha256(raw_utf8_bytes_of_the_file)
```

The hash is computed locally on the writing device.

The hash is the identity of the session.

If the file changes, the hash changes.
If the hash still matches, the file is intact.

A local file can be named:

```text
{session_hash}.anky
```

This means offline verification is always possible.

No server is required to know whether a `.anky` file is intact.

---

# 9. Verification levels

Anky should stay honest about what can and cannot be verified.

## Level 0 — structural validity

The file follows the `.anky` format.

## Level 1 — integrity

The file hash matches the expected hash.

## Level 2 — anchored existence

The hash appears in a public anchor.

This proves the hash existed no later than the anchor timestamp.

## Level 3 — claimed provenance

A wallet, app, or device claims association with the session.

This relies on external systems.

## Level 4 — humanness

The protocol does not prove this.

Anky must not claim that `.anky` proves unaided human writing.

---

# 10. The user journey

The current product journey is:

```text
Open app
→ Write .anky locally
→ Reveal the session
→ Keep it in archive
→ Optional: mint a Loom
→ Optional: seal the hash on Solana devnet
→ Optional: spend credits to reflect on it
```

The user should feel:

1. I can write immediately.
2. My writing is safe locally.
3. I can choose to seal it.
4. I can choose to mirror it.
5. I am not forced into account/wallet/market complexity before writing.

---

# 11. The app contract

The app should speak clearly:

- **Write now. Seal later.**
- **Your writing stays with you.**
- **Nothing is posted.**
- **The moment passes. The trace remains.**
- **The seal is a public witness.**
- **Credits pay for mirrors, not for truth.**

The app should avoid false or misleading statements like:

- “Nothing is saved.”
- “Your words disappear.”
- “You will receive a reflection.”

Better versions:

- “Nothing is posted.”
- “The writing stays with you.”
- “You can ask for a reflection.”

---

# 12. Core screens

The app should be understandable through a few core places.

## Onboarding

Introduces the world and the ritual.

Important principle:

> Onboarding must end in permission to write, not mandatory account creation.

The final onboarding actions should be:

- Write now
- Create account
- I already have an account

## Home / Loom

Shows the user’s current state and always offers writing.

Primary action:

- Write 8 minutes

Secondary actions:

- Archive
- Loom
- Credits

## Write

The sacred room.

It should remain almost silent.

It should not contain wallet logic, Loom logic, credits, or Solana explanations.

It only captures the `.anky` session.

## Reveal

The ritual hub after writing.

It should show:

- reconstructed writing,
- saved locally,
- hash verified,
- seal action,
- mirror action,
- keep/archive action.

## Archive / Past

The local memory of `.anky` traces.

Entries can have badges:

- local
- sealed
- mirrored

## Entry

Detailed view of one `.anky`.

It can show:

- reconstructed text,
- raw `.anky`,
- hash,
- seal sidecar,
- reflection sidecar,
- processing receipt.

## Credits

The mirror economy surface.

It should explain that credits are for processing, not truth.

---

# 13. State families

The app should not be modeled as one giant state machine.

It should be modeled as several related state families.

## Onboarding state

```ts
type OnboardingState =
  | "unseen"
  | "in_progress"
  | "complete";
```

## Auth state

```ts
type AuthState =
  | "guest"
  | "signing_up"
  | "logging_in"
  | "authenticated"
  | "error";
```

## Wallet state

```ts
type WalletState =
  | "disconnected"
  | "connected";
```

## Loom state

```ts
type LoomState =
  | "none"
  | "redeeming_code"
  | "minting"
  | "ready"
  | "error";
```

## Writing state

```ts
type WritingState =
  | "ready"
  | "focused"
  | "active"
  | "silent"
  | "closing"
  | "revealed"
  | "recoverable"
  | "error";
```

## Seal state

```ts
type SealState =
  | "unsealed"
  | "pending"
  | "sealed"
  | "error";
```

## Processing state

```ts
type ProcessingState =
  | "idle"
  | "insufficient_credits"
  | "pending"
  | "done"
  | "error";
```

---

# 14. Error language

Error handling must protect the user’s trust.

The app should always reassure the user that the `.anky` is safe when that is true.

Use these patterns:

## Storage error

> We could not save this session. Keep the app open. Your writing is still here.

## Wallet error

> Wallet connection failed. You can still write.

## Mint error

> Mint failed. You can still write and keep this `.anky`.

## Seal error

> Seal failed. Your `.anky` is still safe locally.

## Reflection error

> Reflection failed. Your `.anky` is unchanged.

## Insufficient credits

> Not enough credits. Credits pay for mirrors, not for truth.

---

# 15. Solana’s role

Solana is not the storage layer.
Solana is not the writing layer.
Solana is not the mirror layer.

Solana is the public witness layer.

It receives a hash and records that a wallet used a Loom to anchor that hash at a time.

It should never receive the writing text.

---

# 16. The Loom

The Loom is the public instrument.

Current language:

> **Anky Sojourn 9 Loom**

A Loom is a transferable Metaplex Core asset that gives its current holder the ability to seal `.anky` hashes.

The Loom does not contain the writing.
The Loom does not own the writing.
The Loom carries public knots in its lineage.

A Loom can be transferred.
Past seals remain historically attached to the wallet and Loom that created them.
Future sealing rights follow the current Loom owner.

The strongest sentence:

> **The Loom is transferable. The writing is not. The seal is the public knot between them.**

---

# 17. Current Solana devnet state

## Metaplex Core collection

```text
Core collection: F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u
Collection name: Anky Sojourn 9 Looms
Collection URI: https://anky.app/devnet/metadata/sojourn-9-looms.json
Metaplex Core program: CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d
```

## Anky Seal Program

```text
Seal program ID: 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX
Upgrade authority: FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP
```

The local Anchor program has been updated so `declare_id!` and `OFFICIAL_COLLECTION` match those deployed devnet IDs.

---

# 18. Backend metadata status

The backend server currently runs on the machine via:

```text
systemd user service: anky.service
binary: /home/kithkui/anky/target/release/anky
port: 8889
public tunnel: anky.app
```

These endpoints return `200`:

```text
https://anky.app/devnet/metadata/sojourn-9-looms.json
https://anky.app/devnet/metadata/looms/0001.json
```

Only `0001.json` exists so far as test Loom metadata.

More Loom metadata files still need to be generated and served when minting more indexes.

---

# 19. Seal program details

The Anchor program lives at:

```text
solana/anky-seal-program/
```

Instruction:

```text
seal_anky(session_hash: [u8; 32])
```

Accounts:

```text
writer: signer
loom_asset: unchecked, intended Metaplex Core asset
loom_collection: unchecked, must equal official collection
loom_state: PDA ["loom_state", loom_asset]
system_program
```

`LoomState` stores:

```text
loom_asset
total_seals
latest_session_hash
rolling_root
created_at
updated_at
```

Each seal means:

> **Wallet W used Loom L to anchor `.anky` hash H at Solana time T.**

The rolling root uses:

```text
ANKY_LOOM_ROOT_V1
previous_root
writer
loom_asset
session_hash
total_seals
timestamp
```

---

# 20. Important devnet security status

The current `verify_core_loom` now performs minimal Metaplex Core base-account verification.

It checks:

- `loom_asset` account owner is the Metaplex Core program,
- `loom_collection` account owner is the Metaplex Core program,
- `loom_collection` equals `OFFICIAL_COLLECTION`,
- Core asset discriminator is `AssetV1`,
- `asset owner == writer`,
- Core asset update authority is the official collection,
- Core collection discriminator is `CollectionV1`.

Therefore:

> **The current seal program is not mainnet-safe.**

It is good enough to build and test the app flow on devnet.

Before mainnet, the hand-rolled parser must be audited against the exact mpl-core account layout and integration-tested against real Core assets.

---

# 21. Current React Native app state

The Expo app lives at:

```text
apps/anky-mobile/
```

Solana scaffolding exists under:

```text
apps/anky-mobile/src/lib/solana/
```

Files:

```text
ankySolanaConfig.ts
walletTypes.ts
mintLoom.ts
sealAnky.ts
```

## ankySolanaConfig.ts

Contains devnet fallback IDs for the deployed collection and program.

`.env.example` includes:

```text
EXPO_PUBLIC_SOLANA_RPC_URL=https://api.devnet.solana.com
EXPO_PUBLIC_ANKY_CORE_COLLECTION=F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u
EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID=4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX
```

## walletTypes.ts

Defines the minimal wallet interface:

```ts
publicKey: string
signTransaction(transaction)
signAndSendTransaction?(transaction)
```

## sealAnky.ts

Functional scaffolding:

- validates 64-character hash,
- converts to 32 bytes,
- derives LoomState PDA,
- builds Anchor instruction manually with `@solana/web3.js`,
- signs/sends with wallet,
- returns receipt shape.

## mintLoom.ts

Not fully implemented yet.

It currently:

- shapes self-funded and invite-code minting,
- validates invite flow,
- names Looms like `Anky Sojourn 9 Loom #0001`,
- uses metadata URI fallback,
- requires a future `buildCoreLoomMintTransaction` hook,
- does not fake mint success.

---

# 22. Current implementation goal

The current implementation target is:

```text
mobile app mints one Loom
→ user writes .anky locally
→ app hashes file
→ app seals hash through Loom
→ user can spend credits for a mirror
```

No bulk minting exists.
No `.anky` text goes on-chain.

---

# 23. Minting model

The mobile app should mint Looms.

The Anky Seal Program should not mint Looms.

The mint flow should support:

## Self-funded mint

The user pays devnet/mainnet SOL and mints their own Loom.

Because the Loom must belong to Anky's official Core collection, the transaction also needs the collection update authority or a valid collection delegate. The user can pay and own the Loom, but Anky must authorize official collection membership through a prepared transaction or collection policy.

Current devnet path:

1. Mobile requests `/api/mobile/looms/mint-authorizations`.
2. Backend validates self-funded or invite-code eligibility.
3. Mobile requests `/api/mobile/looms/prepare-mint`.
4. Backend returns a Core mint transaction signed by the collection authority and new asset keypair.
5. Mobile wallet signs as payer/owner and sends.
6. Mobile records the mint receipt.

## Invite-code mint

The user enters a code.
The backend validates the code.
The user can mint if allowed.

Later, some codes may sponsor or discount minting.

For now, devnet can use a simple test code.

---

# 24. Credits and mirrors

Credits are not part of Solana v1.
Credits are app/backend state.

The intended pricing language:

```text
1 credit = reflection
3 credits = image
Full Anky = title + reflection + image
8 credits = deep mirror
88 credits = full sojourn archive processing
```

The immediate dev implementation can start with:

- local credits balance,
- 8 starting credits,
- local placeholder reflection,
- no fake AI.

Long term, credits are purchased through Stripe or similar.

The processing server should eventually handle temporary encrypted carpets without becoming the archive.

---

# 25. Local artifacts

The local filesystem should be able to contain:

```text
{hash}.anky
{hash}.seal.json
{hash}.reflection.md
{hash}.processing.json
{hash}.image.webp
{hash}.title.txt
```

The `.anky` file is upstream.

All sidecars are downstream.

A sidecar can be deleted and regenerated.

The `.anky` file cannot.

---

# 26. Onboarding copy direction

The text-only onboarding should be inspired by the current visual direction but remain truthful.

## Screen 1

```text
anky
a journey inward
```

Button:

```text
begin
```

## Screen 2

```text
Welcome, traveler.
Anky is a companion for meeting the page.
```

## Screen 3

```text
A daily practice of presence.
Write for 8 minutes.
Nothing is posted.
The writing stays with you.
```

## Screen 4

```text
Express. Let go.
Move forward.
No backspace.
The moment passes. The trace remains.
```

## Screen 5

```text
Receive a reflection.
After writing, you can ask for a mirror.
```

## Screen 6

```text
This is for you.
No followers. No feed.
Just you and the page.
```

Actions:

- Write now
- Create account
- I already have an account

---

# 27. Near-term engineering priorities

1. Implement text-only onboarding.
2. Add Privy provider and auth/wallet hook.
3. Preserve write-without-wallet flow.
4. Add local dev credits with 8 starting credits.
5. Wire Reveal to real devnet `sealAnky` using selected Loom.
6. Persist `{hash}.seal.json` after confirmation.
7. Show sealed state in Entry.
8. Add local reflection spending 1 credit.
9. Add Loom mint UI.
10. Wire Loom mint UI to the Metaplex Core mobile mint builder.
11. Wire Loom mint UI to backend-prepared collection-authority transactions.
12. Audit Core verification before mainnet.

---

# 28. What must not happen

Do not make account creation mandatory before writing.

Do not make wallet connection mandatory before writing.

Do not make Loom ownership mandatory before writing.

Do not make credits mandatory before writing.

Do not upload `.anky` text to Solana.

Do not say the current devnet verification is mainnet-safe.

Do not fake successful minting or AI reflection.

Do not let the Solana layer take over the writing room.

---

# 29. The final architecture

```text
User writes
  ↓
Device creates .anky
  ↓
Device computes session_hash
  ↓
Optional Loom mint through mobile app
  ↓
Optional seal_anky(session_hash)
  ↓
Solana records public witness
  ↓
App saves seal sidecar
  ↓
Optional credit spend
  ↓
Mirror artifact is created locally
```

---

# 30. The final thesis

Anky is a private writing ritual whose truth lives in a local `.anky` file, whose witness can live on Solana through a Loom, and whose mirror can be purchased with credits.

The app must stay simple enough for the ritual to breathe.

The chain must stay small enough to remain honest.

The mirror must stay optional enough to preserve trust.

The archive must stay with the writer.

**Write without a wallet. Seal with a Loom. Mirror with credits. Your writing stays with you.**
