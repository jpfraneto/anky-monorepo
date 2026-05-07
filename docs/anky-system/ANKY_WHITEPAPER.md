# Anky: Private Practice, Public Proof

Status: evidence-backed whitepaper draft, 2026-05-07.

## Abstract

**CURRENT:** Anky is a mobile writing ritual that lets a person prove daily private practice without publishing the writing. The writer creates a canonical `.anky` file locally, hashes the exact UTF-8 bytes, seals the hash on Solana through a Metaplex Core Loom, and can attach an SP1 proof receipt that verifies the private file's structure and duration without revealing plaintext.

**CURRENT:** The proof statement is narrow:

```text
Wallet W privately completed one valid .anky rite for UTC day D,
producing hash H, without revealing the writing.
```

**CURRENT:** For Sojourn 9, the system combines a React Native mobile app, Metaplex Core Looms, the Anky Seal Program, SP1 off-chain proving, verifier-authority-attested `VerifiedSeal` accounts, Helius-backed indexing, and deterministic practice scoring.

## Why Anky Exists

**FOUNDER-DOCTRINE:** Anky exists because the most important human writing is often the least publishable. Ordinary apps optimize for editing, performance, metrics, and audience. Anky optimizes for a private daily encounter with the raw mind.

The core product asks for one thing: write for 8 minutes, without backspace, without composing for an audience, and let the session close when silence arrives.

## The Private Human Growth Problem

**FOUNDER-DOCTRINE:** The internet rewards visible output. Human growth often happens in private, before it is legible, polished, or socially useful.

**CURRENT:** Anky's current protocol does not attempt to judge meaning or quality. It preserves timing, characters, and the fact of completion.

**CURRENT:** The protocol does not prove humanness or spiritual progress. It proves byte integrity and supports public anchoring of a private practice commitment.

## The Solution: Prove Practice, Not Content

**CURRENT:** Anky separates private writing from public proof.

Private:

- exact `.anky` bytes,
- reconstructed text,
- local archive,
- optional AI reflection inputs.

Public:

- wallet,
- UTC day,
- session hash,
- Loom/Core collection identity,
- Solana transaction signature,
- optional SP1 receipt metadata,
- score derived from finalized public events.

This lets a user demonstrate consistency without turning the writing itself into public content.

## The Writing Ritual

**CURRENT:** The mobile rite is 8 minutes of forward-only writing. The app rejects canonical edit operations such as backspace, delete, paste/substitution, and unsupported text changes. A final 8 seconds of silence closes the session with the terminal `8000` marker.

Evidence: `apps/anky-mobile/src/screens/WriteScreen.tsx`, `apps/anky-mobile/src/lib/inputPolicy.ts`, `apps/anky-mobile/src/components/ritual/AnkyWritingChamber.tsx`.

## The `.anky` Protocol

**CURRENT:** A canonical `.anky` is plain UTF-8 text:

```text
{epoch_ms} {character_or_SPACE}
{delta_ms_0000_to_7999} {character_or_SPACE}
...
8000
```

Rules:

- first line is absolute Unix epoch milliseconds plus first accepted character,
- later lines are four-digit deltas plus accepted character,
- typed spaces are `SPACE`,
- line endings are LF-only,
- no BOM,
- terminal line is exactly `8000`,
- hash is `sha256(raw_utf8_bytes_of_the_file)`.

**CURRENT:** The mobile app, backend validator, and SP1 proof library all implement these rules.

## The Mobile App

**CURRENT:** The active app lives in `apps/anky-mobile`. It supports:

- local `.anky` drafting and closed-session storage,
- parsing and local verification,
- reveal flow,
- Loom selection/minting surfaces,
- Solana `seal_anky`,
- optional backend record of public seal metadata,
- optional reflection processing,
- optional proof request and polling,
- proof state separate from local validity.

**CONFIGURED-BUT-UNVERIFIED:** This pass did not run the app or test production builds.

## The 96-Day Sojourn

**FOUNDER-DOCTRINE:** Sojourn 9 is a 96-day daily rite bounded by 3,456 vessels. Doctrine says it is daily writing, not trading, spectacle, or social performance.

**CURRENT:** Mobile code defines a Sojourn 9 calendar starting `2026-03-03T00:00:00.000Z` and clamps post-day-96 users to the final day until future routing exists.

**PUBLIC CANON:** Sojourn structure is 12 regions of 8 days, for 96 total days. The 8 kingdoms/chakras/colors remain valid as an inner symbolic cycle. Mobile code still names 8 kingdoms of 12 days internally; that is an implementation/calendar naming detail until deliberately migrated, not the public launch structure.

## The 21-Day Great Slumber

**PLANNED / FOUNDER-DOCTRINE:** The requested system doctrine includes 21-day Great Slumbers as rest/integration phases.

**UNKNOWN:** No code or migration enforcing a 21-day Great Slumber was found. Mobile has only a TODO to route post-day-96 users into Great Slumber or the next sojourn.

Therefore Great Slumber should be described as planned doctrine until implemented.

## Looms

**CURRENT:** Looms are the Sojourn access artifact in the active launch path. They are Metaplex Core assets, not custom NFTs in the active seal program.

**CURRENT:** The Anky Seal Program verifies:

- supplied Loom asset account is owned by the Metaplex Core program,
- supplied collection account is owned by the Metaplex Core program,
- supplied collection equals the configured official collection,
- Loom asset owner equals writer,
- Loom asset update authority points to the official collection.

**UNKNOWN:** Mainnet collection status is not proven by repo evidence.

## Seals

**CURRENT:** A seal is a commitment, not a publication.

`seal_anky` records:

- writer,
- Loom asset,
- session hash,
- UTC day,
- timestamp,
- Loom rolling state.

It enforces current UTC day, one writer/day seal, one writer/hash seal, and Core Loom ownership/collection checks.

## Hashes

**CURRENT:** The session hash is a commitment to exact bytes. It is not encryption. A hash does not hide low-entropy content if an attacker already has candidate plaintext, and it cannot reconstruct the writing.

Safe wording:

- "The chain stores only the SHA-256 commitment."

Unsafe wording:

- "The hash encrypts the writing."

## SP1 / ZK Proof

**CURRENT:** SP1 proves private `.anky` validity off-chain. The guest verifies file structure, exact hash, timing/duration rules, UTC day derivation, and public receipt values.

**CURRENT:** The public receipt includes version, protocol, writer, session hash, UTC day, duration fields, event count, validity flags, and proof hash.

**CONFLICT:** The current Solana program does not directly verify SP1 proofs.

## VerifiedSeal

**CURRENT:** `record_verified_anky` records a `VerifiedSeal` account after off-chain SP1 verification. It is authority-gated by the configured proof verifier and protocol version `1`.

This is a verifier-attested proof receipt, not a fully trustless on-chain ZK verifier.

Future hardening is direct on-chain SP1/Groth16 verification or equivalent trust minimization.

## Helius, Indexing, And Scoring

**CURRENT:** The indexer reconstructs `AnkySealed` and `AnkyVerified` state from finalized public transaction data and computes deterministic Score V1.

Formula:

```text
score = unique_seal_days + (2 * verified_seal_days) + streak_bonus
```

**CURRENT:** Default reward basis points are 800, meaning 8 percent of a supplied raw token supply can be allocated proportionally by score.

**DEVNET-PROVEN:** A runbook records one finalized devnet seal plus verified seal pair producing score `3`.

**NEEDS-EXTERNAL-VERIFICATION:** Production Helius webhook status and mainnet scoring are not proven by this repo pass.

## AI Reflections And Images

**CURRENT:** AI reflections are derived artifacts. They are not canonical proof data.

**CURRENT:** The backend can reconstruct plaintext from `.anky` transiently for reflection generation. This must remain opt-in and must not be confused with the public proof/scoring path.

**CURRENT:** Mobile stores local sidecars for reflections, images, metadata, processing, conversation, deep mirror, and full sojourn archive outputs.

## Credits, Purchases, And Revenue

**CURRENT:** Mobile credits use RevenueCat CREDITS in production mobile builds. Packages grant 22, 99, and 421 credits.

**CURRENT:** Backend `credit_ledger_entries` is UI/history state; RevenueCat remains the balance source of truth for mobile credits.

**CONFLICT:** Older docs describe x402/USDC on Base. That is not the current mobile credits source of truth.

## `$ANKY`: The Memetic Distribution Layer

**CURRENT:** The repo includes a `$ANKY` page that says `$ANKY` was launched on pump.fun on Solana and gives contract address `6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump`.

**NEEDS-EXTERNAL-VERIFICATION:** Current token status, supply, liquidity, holders, and canonical mint facts must be checked externally.

**CURRENT:** The `$ANKY` page says the token does not unlock features or grant access.

**CONFIGURED-BUT-UNVERIFIED:** The Sojourn 9 indexer can compute a practice-based 8 percent allocation if given raw token supply, but a public distribution is not ready until supply, custody, snapshot, claim/transfer, and dispute rules are final.

Safe framing:

> `$ANKY` is the memetic layer. The practice does not require it. Future reward distribution can be based on public proof-of-practice scores if and when the snapshot and custody rules are published.

## Privacy Model

**CURRENT:** The canonical proof/scoring path should never require `.anky` plaintext.

**CURRENT:** Plaintext appears only when a user explicitly asks for reflection or proof processing, and then it must be transient.

**CONFLICT:** Older backend tables and web flows can contain plaintext. Public claims must distinguish current mobile proof-of-practice from legacy systems.

## Threat Model

Anky protects:

- byte integrity,
- public hash anchoring,
- same-day seal uniqueness,
- duplicate hash prevention per wallet,
- Loom ownership and collection membership in the current parser model,
- verifier/protocol matching for verified score bonuses,
- finalized-data scoring by default.

Anky does not yet protect:

- unaided human authorship,
- modified clients,
- sybil resistance beyond wallet/Loom mechanics,
- leaked verifier authority,
- direct on-chain ZK trustlessness,
- all legacy plaintext paths.

## What Is Live Now

**CURRENT:** The repo has real mobile capture/seal/proof UI code, Anchor seal program code, SP1 proof code, backend proof/indexing routes, public metadata migrations, and Helius/RPC indexer code.

**DEVNET-PROVEN:** Runbooks record a live devnet seal plus verified seal plus score snapshot.

**CONFIGURED-BUT-UNVERIFIED:** This pass did not rerun tests, SP1 proof generation, backend proof job, Helius backfill, or mobile app flows.

## The 3-Day Ship Window

To ship honestly:

1. Rerun the full devnet E2E from the current worktree.
2. Confirm mobile UI states match chain/indexed proof state.
3. Confirm backend proof witness cleanup and no plaintext persistence.
4. Confirm Helius webhook/backfill and audited score snapshot.
5. Confirm mainnet program/collection/verifier externally.
6. Publish score and privacy rules before the season.
7. Keep `$ANKY` distribution as planned until all token/custody/snapshot facts are final.

## Future Work

- Direct on-chain SP1/Groth16 verification.
- Audited Core parser or safer Core integration.
- Great Slumber implementation.
- Stronger anti-replay and timing integrity.
- Mainnet verified deployment evidence.
- Public score snapshot and allocation exporter.
- Legacy plaintext cleanup or hard isolation.

## Vision

**FOUNDER-DOCTRINE:** Anky's strongest idea is not that writing becomes financialized. It is that private practice can become publicly legible without becoming public content.

The product wins if the user can understand this in minutes:

```text
write privately -> hash exact .anky bytes -> seal on Solana
-> prove privately with SP1 -> attach verified receipt
-> index fair practice score
```
