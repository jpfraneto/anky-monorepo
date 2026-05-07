# Anky System Documentation Status

Updated: 2026-05-07

## Scope

This document set is a source-of-truth pass for Anky as observed in the monorepo. It prioritizes documentation over runtime changes and does not deploy, transact, rotate keys, or read secrets.

## Output Files

- `docs/anky-system/ANKY_WHITEPAPER.md`
- `docs/anky-system/ANKY_TECHNICAL_SOURCE_OF_TRUTH.md`
- `docs/anky-system/ANKY_REPO_EVIDENCE_INDEX.md`
- `docs/anky-system/ANKY_3_DAY_LAUNCH_GAP_AUDIT.md`
- `docs/anky-system/ANKY_RULES_LEDGER.md`
- `docs/anky-system/ANKY_IF_JP_IS_UNAVAILABLE.md`
- `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md`
- `docs/anky-system/ANKY_3_DAY_CLOSURE_STATUS.md`
- `docs/anky-system/STATUS.md`

## Claim Labels

Every major claim in this set uses one of:

- CURRENT
- DEVNET-PROVEN
- LOCAL-PROVEN
- CONFIGURED-BUT-UNVERIFIED
- PLANNED
- FOUNDER-DOCTRINE
- UNKNOWN
- CONFLICT
- NEEDS-EXTERNAL-VERIFICATION

## Read Pass Completed

Evidence was inspected across:

- Git status, recent commits, and branches.
- Root docs: `README.md`, `ANKY_SOURCE_OF_TRUTH.md`, `CURRENT_STATE.md`, `CURRENT_IMPLEMENTATION_REPORT.md`, `docs/local-first-protocol.md`, `static/protocol.md`.
- Mobile app: `apps/anky-mobile/src/lib/ankyProtocol.ts`, tests, storage/state files, writing/reveal/loom screens, Solana clients, credits/RevenueCat files.
- Backend: `src/routes/mobile_sojourn.rs`, route mounting in `src/routes/mod.rs`, privacy-sensitive older backend paths.
- Database migrations: `001`, `009`, `017` through `023`.
- Solana: `solana/anky-seal-program`, Anchor program source, scripts, runbooks.
- SP1 proof path: `solana/anky-zk-proof`, guest/script/library sources.
- Indexer/scoring: `solana/scripts/indexer`, tests, Helius runbook.
- Sojourn doctrine: `static/sojourn9.md`, `sojourn9/constitution/SOJOURN_9.md`, related Sojourn 9 docs.
- `$ANKY` references: `templates/ankycoin.html`, `templates/base.html`, docs and config references.

## Major Findings

- CURRENT: The active launch path is a React Native mobile `.anky` writing flow, local hash, Metaplex Core Loom ownership, custom Anchor Anky Seal Program, SP1 off-chain proof path, verifier-authority `record_verified_anky`, backend proof/indexing routes, and a Helius/RPC indexer script.
- DEVNET-PROVEN: Runbooks record a live devnet `seal_anky` plus `record_verified_anky` pair on 2026-05-06 and a finalized score snapshot for one wallet with score `3`.
- CURRENT: Older Sojourn 9 docs in `sojourn9/` describe a Bubblegum/cNFT season program with `initialize_season`, `seal_anky`, scaffolded `claim_reward`, no actual iOS app code, and no published program IDs. Those docs are now marked historical/transitional. The active current path is Metaplex Core Looms plus `solana/anky-seal-program`.
- CONFLICT: Root legacy docs describe older web/EVM/x402/facilitator/AI marketplace systems that do not match the current mobile Solana launch path.
- CURRENT: The mobile and SP1 protocol agree on exact `.anky` UTF-8 byte hashing, LF-only, no BOM, terminal `8000`, zero-padded deltas, and `SPACE` as the space token.
- CONFIGURED-BUT-UNVERIFIED: Backend proof routes exist, but this pass did not run an end-to-end proof job or send a chain transaction.
- UNKNOWN: Mainnet deployment status for the active seal program and mainnet Core collection is not proven from repo evidence.
- PLANNED / FOUNDER-DOCTRINE: Great Slumber is mentioned in the supplied goal and appears only as a TODO in mobile code; no 21-day Great Slumber implementation was found.
- NEEDS-EXTERNAL-VERIFICATION: `$ANKY` pump.fun mint/page, RevenueCat product setup, App Store/Google Play setup, Helius webhook existence, mainnet explorer state, and any token supply/custody claims require external checks.

## Commands Run In This Pass

Read-only or documentation setup commands:

- `pwd`
- `rg --files`
- `git status --short`
- `git branch --show-current`
- `git log --oneline -12`
- `find ...`
- `rg -n ...`
- `nl -ba ... | sed -n ...`
- `mkdir -p docs/anky-system`

No test suite, build, deployment, mainnet command, private key command, or Helius live API command was run during this documentation pass.

## Known Limits Of This Pass

- No secrets were read.
- No `.env` files were read.
- No keypair or deployer files were read.
- No mainnet status was externally verified.
- No Helius webhook list was queried.
- No RevenueCat, App Store Connect, Google Play, Stripe, Privy, or pump.fun account was accessed.
- No code was changed outside this documentation set.
