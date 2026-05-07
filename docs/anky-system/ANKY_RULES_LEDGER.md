# Anky Rules Ledger

This ledger states the rules, whether code enforces them, and where the evidence lives. If a rule is doctrine only, say that before promising it.

## Protocol Rules

| Rule | Label | Enforced By | Evidence | Notes |
|---|---|---|---|---|
| `.anky` hash is SHA-256 over exact UTF-8 bytes. | CURRENT | Mobile parser, backend validator, SP1 library. | `apps/anky-mobile/src/lib/ankyProtocol.ts`, `solana/anky-zk-proof/src/lib.rs`, `src/routes/mobile_sojourn.rs` | Do not hash reconstructed prose or JSON. |
| LF-only line endings. | CURRENT | Mobile tests, backend validator, SP1 library. | `apps/anky-mobile/src/lib/ankyProtocol.test.ts`, `solana/anky-zk-proof/src/lib.rs`, `src/routes/mobile_sojourn.rs` | CRLF must be rejected. |
| No BOM. | CURRENT | Mobile tests, backend validator, SP1 library. | Same as above. | A BOM changes bytes and invalidates canonical hashing. |
| Terminal line is exactly `8000`. | CURRENT | Mobile close/parse, backend validator, SP1 parser. | `apps/anky-mobile/src/lib/ankyProtocol.ts`, `src/routes/mobile_sojourn.rs` | No trailing newline after terminal in canonical mobile tests. |
| Spaces are `SPACE`. | CURRENT | Mobile protocol and SP1 parser. | `apps/anky-mobile/src/lib/ankyProtocol.ts`, `solana/anky-zk-proof/src/lib.rs` | Literal trailing-space payloads are rejected in current protocol. |
| Forward-only writing. | CURRENT | Mobile input policy. | `apps/anky-mobile/src/lib/inputPolicy.ts` | Not cryptographically enforced against modified clients. |
| The protocol proves humanness. | CONFLICT | None. | `static/protocol.md` | Explicitly false. Do not claim it. |

## Mobile Rite Rules

| Rule | Label | Enforced By | Evidence | Notes |
|---|---|---|---|---|
| Rite lasts 8 minutes. | CURRENT | Mobile constants and SP1 receipt duration rule. | `apps/anky-mobile/src/screens/WriteScreen.tsx`, `solana/anky-zk-proof/src/lib.rs` | Timing model depends on client capture. |
| 8 seconds of silence closes the session. | CURRENT | Mobile write screen and chamber constants. | `apps/anky-mobile/src/screens/WriteScreen.tsx`, `apps/anky-mobile/src/components/ritual/AnkyWritingChamber.tsx` | Terminal `8000` encodes closure. |
| Local storage is primary. | CURRENT | Mobile storage and local-first docs. | `apps/anky-mobile/src/lib/ankyStorage.ts`, `docs/local-first-protocol.md` | iCloud backup details in docs should be verified against app config before public policy. |
| Local protocol validity is not SP1 proof verification. | CURRENT | State types and proof sidecar checks. | `apps/anky-mobile/src/lib/ankyState.ts`, `apps/anky-mobile/src/lib/solana/types.ts` | UI copy must keep these separate. |

## Solana Rules

| Rule | Label | Enforced By | Evidence | Notes |
|---|---|---|---|---|
| Only current UTC day can be sealed. | CURRENT | Anchor `seal_anky`. | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs` | Uses Solana clock. |
| One seal per writer per UTC day. | CURRENT | `DailySeal` PDA seeds. | Same. | Current active program is writer/day keyed. |
| One seal per writer per session hash. | CURRENT | `HashSeal` PDA seeds. | Same. | Prevents same wallet from reusing a hash. |
| Loom must be owned by writer and in official Core collection. | CURRENT | Anchor Core parser. | Same. | Parser is hand-rolled and must be live-tested before mainnet confidence. |
| `VerifiedSeal` requires verifier authority and protocol version 1. | CURRENT | Anchor `record_verified_anky`. | Same. | Trust rests on verifier authority today. |
| Direct on-chain SP1 proof verification exists. | CONFLICT | None. | Anchor program lacks Groth16/SP1 verifier. | Future hardening only. |
| Mainnet deployment exists. | UNKNOWN | Not proven. | `Anchor.toml` config only. | Needs explorer and deployment signature verification. |

## Backend And Privacy Rules

| Rule | Label | Enforced By | Evidence | Notes |
|---|---|---|---|---|
| Mobile seal receipt rows never store `.anky` plaintext. | CURRENT | Schema and route design. | `migrations/017_mobile_solana_integration.sql`, `src/routes/mobile_sojourn.rs` | Public metadata only. |
| Verified receipt rows never store private witness. | CURRENT | Schema. | `migrations/019_mobile_verified_seal_receipts.sql` | Public proof metadata only. |
| Helius webhook rows reject private `.anky` payloads. | CURRENT | Backend validator. | `src/routes/mobile_sojourn.rs`, `migrations/020_mobile_helius_webhook_events.sql` | Payload itself is public transaction/webhook data. |
| Proof route must not persist raw witness. | CURRENT / CONFIGURED-BUT-UNVERIFIED | Temp file outside repo, cleanup, proof job schema. | `src/routes/mobile_sojourn.rs`, `migrations/023_mobile_proof_jobs.sql` | This pass did not run a job to observe cleanup. |
| No backend path stores writing plaintext anywhere. | CONFLICT | Legacy schema contradicts it. | `migrations/001_init.sql`, `CURRENT_IMPLEMENTATION_REPORT.md` | Scope privacy claims to current mobile proof path. |
| Error logs do not leak `.anky` plaintext in proof path. | CURRENT | Error redaction in proof runner. | `src/routes/mobile_sojourn.rs` | Need operational log audit before public privacy certification. |

## Sojourn Rules

| Rule | Label | Enforced By | Evidence | Notes |
|---|---|---|---|---|
| Sojourn 9 has 96 days. | CURRENT / FOUNDER-DOCTRINE | Mobile calendar constants and doctrine docs. | `apps/anky-mobile/src/lib/sojourn.ts`, `static/sojourn9.md` | Enforced in UI state, not in active Anchor program season bounds. |
| Sojourn 9 has 3,456 vessels. | CURRENT / FOUNDER-DOCTRINE | Backend max Loom index, doctrine docs, indexer cap. | `src/routes/mobile_sojourn.rs`, `static/sojourn9.md`, `solana/scripts/indexer/ankySealIndexer.mjs` | Active on-chain hard cap was not found in the Core seal program. |
| One wallet may steward at most one vessel. | PLANNED / CONFIGURED-BUT-UNVERIFIED | Backend Loom authorization/record logic may assist, older docs say not fully implemented. | `src/routes/mobile_sojourn.rs`, `sojourn9/docs/soulbound_frontier.md` | Needs final implementation audit against active Core path. |
| No retroactive sealing. | CURRENT | Active Anchor current-day check. | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs` | Also doctrine. |
| Great Slumber lasts 21 days and is enforced. | PLANNED / UNKNOWN | None found. | `apps/anky-mobile/src/lib/sojourn.ts` TODO, supplied goal. | Do not claim current enforcement. |

## AI And Credits Rules

| Rule | Label | Enforced By | Evidence | Notes |
|---|---|---|---|---|
| Reflections are optional derived artifacts, not canonical proof data. | CURRENT | Mobile/backend separation. | `apps/anky-mobile/src/screens/RevealScreen.tsx`, `src/routes/mobile_sojourn.rs` | Reflections may use transient plaintext. |
| Credit costs are typed in mobile. | CURRENT | Mobile API types. | `apps/anky-mobile/src/lib/api/types.ts` | Reflection 1, image 3, full_anky 5, deep_mirror 8, full_sojourn_archive 88. |
| Production mobile credits use RevenueCat CREDITS. | CURRENT | Mobile docs/code, backend ledger. | `apps/anky-mobile/docs/native-credit-products.md`, `apps/anky-mobile/src/lib/credits/revenueCatCredits.ts`, `migrations/022_credit_ledger_entries.sql` | Product setup needs external verification. |
| x402/Base is the current mobile purchase path. | CONFLICT | Older docs only. | `docs/concepts/ankycoin.mdx` | Do not use for current mobile launch claims. |

## `$ANKY` Rules

| Rule | Label | Enforced By | Evidence | Notes |
|---|---|---|---|---|
| `$ANKY` is a Solana memetic token page in the app/site repo. | CURRENT | Public template. | `templates/ankycoin.html` | External chain facts need verification. |
| `$ANKY` unlocks features or access. | CONFLICT | None. | `templates/ankycoin.html` says it does not unlock features or grant access. | Keep token separate from app access. |
| 8 percent token distribution uses practice score. | CONFIGURED-BUT-UNVERIFIED / FOUNDER-DOCTRINE | Indexer allocation math, supplied launch doctrine. | `solana/scripts/indexer/ankySealIndexer.mjs`, `runbooks/sojourn9-helius-indexing.md` | Not ready for public claim until external token/snapshot/custody facts are verified. |

## Non-Violable Launch Rules

- Do not store `.anky` plaintext in scoring tables.
- Do not log `.anky` plaintext.
- Do not call hash sealing "encryption."
- Do not say mainnet deployment happened without explorer evidence.
- Do not say Solana verifies SP1 directly until implemented and tested.
- Do not publish token allocation terms without a reproducible finalized snapshot and external token/custody verification.

