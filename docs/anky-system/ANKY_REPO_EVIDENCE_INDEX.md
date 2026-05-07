# Anky Repo Evidence Index

This index maps important system claims to repo evidence and labels. It is the first file to update when implementation changes.

## Evidence Labels

- CURRENT: exists in code/docs now.
- DEVNET-PROVEN: repo evidence shows a devnet proof by script, tx, log, runbook, or artifact.
- LOCAL-PROVEN: repo evidence shows local proof, test, or runnable fixture.
- CONFIGURED-BUT-UNVERIFIED: wired/configured, but no end-to-end proof was found in this pass.
- PLANNED: described as intended but not implemented.
- FOUNDER-DOCTRINE: founder-provided doctrine or meaning, not necessarily code-enforced.
- UNKNOWN: cannot be determined from repo.
- CONFLICT: repo contains inconsistent claims.
- NEEDS-EXTERNAL-VERIFICATION: requires an external service or explorer check.

## Core Product

| Claim | Label | Evidence |
|---|---|---|
| Anky's current launch product is a mobile-first private writing ritual with optional Solana sealing. | CURRENT | `apps/anky-mobile/src/screens/WriteScreen.tsx`, `apps/anky-mobile/src/screens/RevealScreen.tsx`, `apps/anky-mobile/src/lib/solana/sealAnky.ts`, `ANKY_SOURCE_OF_TRUTH.md` |
| The public protocol statement is wallet + UTC day + hash, not plaintext publication. | CURRENT | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs`, `docs/local-first-protocol.md`, `static/protocol.md`, `runbooks/sojourn9-sp1-verifiedseal.md` |
| Anky is not a general journaling app or social writing feed in Sojourn 9 doctrine. | FOUNDER-DOCTRINE | `static/sojourn9.md`, `sojourn9/constitution/SOJOURN_9.md` |
| Some older docs still describe a broader AI marketplace, facilitator system, EVM/x402 payments, and public web product. | CONFLICT | `README.md`, `CURRENT_STATE.md`, `docs/concepts/ankycoin.mdx`, `src/routes/mod.rs`, `templates/*` |

## `.anky` Protocol

| Claim | Label | Evidence |
|---|---|---|
| Canonical `.anky` files are UTF-8 plain text with one capture record per line and terminal `8000`. | CURRENT | `apps/anky-mobile/src/lib/ankyProtocol.ts`, `static/protocol.md`, `solana/anky-zk-proof/src/lib.rs` |
| The first line is `{epoch_ms} {character_or_SPACE}`. | CURRENT | `apps/anky-mobile/src/lib/ankyProtocol.ts`, `apps/anky-mobile/src/lib/ankyProtocol.test.ts`, `static/protocol.md` |
| Delta lines use four-digit milliseconds capped at `7999`. | CURRENT | `apps/anky-mobile/src/lib/ankyProtocol.ts`, `apps/anky-mobile/src/lib/ankyProtocol.test.ts`, `solana/anky-zk-proof/src/lib.rs` |
| Typed spaces are encoded as the exact `SPACE` token. | CURRENT | `apps/anky-mobile/src/lib/ankyProtocol.ts`, `apps/anky-mobile/src/lib/ankyProtocol.test.ts`, `solana/anky-zk-proof/src/lib.rs` |
| Hashing is SHA-256 over exact raw UTF-8 `.anky` bytes. | CURRENT | `apps/anky-mobile/src/lib/ankyProtocol.ts`, `apps/anky-mobile/src/lib/ankyProtocol.test.ts`, `static/protocol.md`, `solana/anky-zk-proof/src/lib.rs` |
| The protocol proves humanness. | CONFLICT | `static/protocol.md` explicitly says the protocol does not prove humanness; do not claim otherwise. |

## Mobile App

| Claim | Label | Evidence |
|---|---|---|
| The mobile app enforces an 8-minute rite and closes after 8 seconds of silence. | CURRENT | `apps/anky-mobile/src/screens/WriteScreen.tsx`, `apps/anky-mobile/src/components/ritual/AnkyWritingChamber.tsx` |
| The mobile app blocks deletion, paste/substitution, enter, and unsupported edits for canonical capture. | CURRENT | `apps/anky-mobile/src/lib/inputPolicy.ts`, `apps/anky-mobile/src/screens/WriteScreen.tsx` |
| Closed `.anky` files are stored locally by session hash and cannot be overwritten by a different byte payload. | CURRENT | `apps/anky-mobile/src/lib/ankyStorage.ts` |
| Mobile proof state distinguishes local validity from proof verified status. | CURRENT | `apps/anky-mobile/src/lib/ankyState.ts`, `apps/anky-mobile/src/lib/solana/types.ts`, `apps/anky-mobile/src/screens/RevealScreen.tsx` |
| The app can request SP1 proving by sending opt-in raw `.anky` to `/api/mobile/seals/prove`. | CURRENT | `apps/anky-mobile/src/screens/RevealScreen.tsx`, `apps/anky-mobile/src/lib/api/ankyApi.ts`, `apps/anky-mobile/src/lib/api/types.ts` |
| Production App Store / Google Play builds are live. | NEEDS-EXTERNAL-VERIFICATION | `apps/anky-mobile/eas.json` shows build profiles, not store status. |

## Solana And Looms

| Claim | Label | Evidence |
|---|---|---|
| The active seal program declares program ID `4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX`. | CURRENT | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs`, `solana/anky-seal-program/Anchor.toml` |
| The current active Loom model uses Metaplex Core assets and collection membership, not metadata strings. | CURRENT | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs`, `apps/anky-mobile/src/lib/solana/mintLoom.ts`, `apps/anky-mobile/src/lib/solana/mobileLoomMint.ts` |
| `seal_anky` records `LoomState`, `DailySeal`, and `HashSeal`, enforces current UTC day, one writer/day, one writer/hash, Core owner/collection, and emits `AnkySealed`. | CURRENT | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs` |
| `record_verified_anky` writes `VerifiedSeal` for a matching `HashSeal` with hardcoded verifier authority and protocol version 1. | CURRENT | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs` |
| Mainnet deployment of the active seal program is proven. | UNKNOWN | `Anchor.toml` has a mainnet section, but no explorer proof or deployment signature was verified in this pass. |
| Mainnet Core collection is finalized. | UNKNOWN | Mainnet metadata files exist under `static/mainnet/metadata`, but no collection account or authority proof was verified. |
| Older Sojourn 9 `sojourn9/` program surface is the active launch program. | CONFLICT | `sojourn9/README.md`, `sojourn9/docs/protocol_surface.md`, and `sojourn9/docs/seal_v1_design.md` now mark Bubblegum/cNFT V1 as historical/transitional; active launch path is `solana/anky-seal-program`. |

## SP1 / ZK

| Claim | Label | Evidence |
|---|---|---|
| The SP1 proof library proves private `.anky` validity and public receipt values. | CURRENT | `solana/anky-zk-proof/src/lib.rs`, `solana/anky-zk-proof/sp1/program/src/main.rs`, `solana/anky-zk-proof/sp1/script/src/bin/main.rs` |
| Public receipt values include version, protocol, writer, session hash, UTC day, duration fields, event count, validity flags, and proof hash. | CURRENT | `solana/anky-zk-proof/src/lib.rs` |
| The SP1 script can execute, prove, verify, and write public receipt/proof artifacts. | CURRENT | `solana/anky-zk-proof/sp1/script/src/bin/main.rs` |
| SP1 proof generation and local verification have been demonstrated from this path. | LOCAL-PROVEN | `runbooks/sojourn9-sp1-verifiedseal.md` and `AGENTS.md` context record the current SP1 prove path. |
| Solana verifies the SP1 proof directly today. | CONFLICT | Current Anchor program records verifier-authority-attested receipts; no direct on-chain SP1/Groth16 verifier is implemented. |

## Backend, Database, And Privacy

| Claim | Label | Evidence |
|---|---|---|
| Backend exposes mobile Solana config, Loom, reflection, seal, proof, score, verified-record, and Helius webhook routes. | CURRENT | `src/routes/mobile_sojourn.rs` |
| Proof jobs are designed to use a temp witness file outside the repo and remove it after the SP1 script returns. | CURRENT | `src/routes/mobile_sojourn.rs`, `migrations/023_mobile_proof_jobs.sql` |
| `mobile_seal_receipts`, `mobile_verified_seal_receipts`, and `mobile_helius_webhook_events` are public metadata tables and should not store `.anky` plaintext. | CURRENT | `migrations/017_mobile_solana_integration.sql`, `migrations/019_mobile_verified_seal_receipts.sql`, `migrations/020_mobile_helius_webhook_events.sql` |
| Legacy database rows can contain writing plaintext. | CURRENT | `migrations/001_init.sql`, `CURRENT_IMPLEMENTATION_REPORT.md` |
| The legacy web backend is fully local-first/private under the same model as the mobile proof path. | CONFLICT | `migrations/001_init.sql`, `src/routes/mod.rs`, and older docs show legacy plaintext/static archive surfaces. |

## Helius, Indexing, And Scoring

| Claim | Label | Evidence |
|---|---|---|
| A Helius/RPC indexer exists to parse `AnkySealed` and `AnkyVerified` events/instructions and build score snapshots. | CURRENT | `solana/scripts/indexer/ankySealIndexer.mjs`, `solana/scripts/indexer/ankySealIndexer.test.mjs`, `runbooks/sojourn9-helius-indexing.md` |
| Score V1 is `unique_seal_days + (2 * verified_seal_days) + streak_bonus`. | CURRENT | `solana/scripts/indexer/ankySealIndexer.mjs`, `runbooks/sojourn9-helius-indexing.md`, `src/routes/mobile_sojourn.rs` |
| The reward participant cap is 3,456 and reward basis points default to 800, or 8 percent. | CURRENT | `solana/scripts/indexer/ankySealIndexer.mjs`, `runbooks/sojourn9-helius-indexing.md` |
| One live devnet finalized event pair produced score `3` for one wallet. | DEVNET-PROVEN | `runbooks/sojourn9-helius-indexing.md`, `runbooks/devnet-0xx1-live-e2e-evidence.json` |
| A production Helius webhook is active. | NEEDS-EXTERNAL-VERIFICATION | The repo has manifest tooling and runbooks, but this pass did not query Helius account state. |

## Credits And Purchases

| Claim | Label | Evidence |
|---|---|---|
| Mobile credits use RevenueCat CREDITS for production iOS/Android purchases. | CURRENT | `apps/anky-mobile/docs/native-credit-products.md`, `apps/anky-mobile/src/lib/credits/revenueCatCredits.ts`, `apps/anky-mobile/src/lib/credits/products.ts` |
| Credit packages grant 22, 99, and 421 credits. | CURRENT | `apps/anky-mobile/docs/native-credit-products.md`, `src/routes/mobile_sojourn.rs`, `apps/anky-mobile/src/lib/credits/products.ts` |
| Backend `credit_ledger_entries` stores UI history, while RevenueCat is the balance source of truth. | CURRENT | `apps/anky-mobile/docs/native-credit-products.md`, `migrations/022_credit_ledger_entries.sql`, `src/routes/mobile_sojourn.rs` |
| x402/USDC on Base is the current mobile credits path. | CONFLICT | `docs/concepts/ankycoin.mdx` describes x402/Base; current mobile docs and code use RevenueCat CREDITS. |

## `$ANKY`

| Claim | Label | Evidence |
|---|---|---|
| The repo includes a public `$ANKY` page claiming a pump.fun Solana launch and showing contract address `6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump`. | CURRENT | `templates/ankycoin.html`, `templates/base.html` |
| The token exists and has current supply/market status. | NEEDS-EXTERNAL-VERIFICATION | Must check pump.fun, DEX, or Solana explorer outside repo. |
| `$ANKY` unlocks app features or Loom access. | CONFLICT | `templates/ankycoin.html` says the token does not unlock features or grant access. |
| 8 percent of token supply is ready for automatic distribution to Sojourn 9 users. | CONFIGURED-BUT-UNVERIFIED | Indexer can compute an 8 percent allocation when raw token supply is provided, but no mint supply, custody, claim contract, legal/dispute process, or mainnet snapshot was verified. |

## Great Slumber

| Claim | Label | Evidence |
|---|---|---|
| Great Slumber exists as a post-sojourn concept. | PLANNED / FOUNDER-DOCTRINE | User-supplied goal mentions 21-day Great Slumbers; code has TODO in `apps/anky-mobile/src/lib/sojourn.ts`. |
| A 21-day Great Slumber is implemented or enforced. | UNKNOWN | No code or migration implementing 21 days was found. |
