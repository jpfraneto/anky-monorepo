# Anky Public Launch Claims

Updated: 2026-05-07

This is the launch-safe public claim set for the 3-day Sojourn 9 closure. Use it for the website, pitch, runbooks, and operator-facing launch notes until mainnet evidence changes.

## One Sentence

Anky is a mobile writing ritual that lets users prove daily private practice on Solana by sealing a hash of their local `.anky` file and attaching an SP1-enabled, verifier-authority-attested receipt without revealing the writing.

## Allowed Claims

### Product

- Anky is a mobile-first private daily writing practice.
- The user writes for 8 minutes in a canonical `.anky` file.
- The app computes SHA-256 over the exact `.anky` UTF-8 bytes.
- The public proof statement is: wallet W completed one valid `.anky` rite for UTC day D, producing hash H, without revealing the writing.
- Public proof/scoring metadata can include wallet, Loom, UTC day, session hash, signatures, proof hash, verifier, protocol version, slot/block time, and status.

### Sojourn 9

- Sojourn 9 is 96 days.
- Public season structure is 12 regions of 8 days.
- The 8 kingdoms/chakras/colors are an inner symbolic cycle.
- Great Slumber is the planned 21-day post-sojourn rest/integration phase.
- Great Slumber is not currently proven as enforced by code.

### Solana

- Sojourn 9 uses Metaplex Core Loom ownership as the season access artifact.
- The Anky Seal Program supports `DailySeal`, `HashSeal`, `LoomState`, and `VerifiedSeal`.
- The active program enforces one writer/day seal, one writer/hash seal, current UTC day, and Core Loom owner/collection checks in the current parser model.
- Mainnet deployment must not be claimed until explorer evidence and signed transaction evidence are published.

### SP1 / ZK

- Anky has an SP1-enabled proof pipeline.
- SP1 proves private `.anky` validity off-chain.
- The current on-chain verified badge is verifier-authority-attested after off-chain SP1 verification.
- Future hardening is direct on-chain SP1/Groth16 verification or another trust-minimized verifier.

### Privacy

- The current mobile proof path is designed so plaintext writing is local-first.
- The proof/indexing/scoring path persists public metadata, not plaintext.
- Reflection and proof processing may handle plaintext only as explicit opt-in transient process input.
- Legacy web/backend paths are not privacy-equivalent to the current mobile proof path unless separately audited.

### Helius / Scoring

- Helius/RPC indexing can reconstruct finalized public seal and verified receipt metadata.
- Score V1 is deterministic: `unique_seal_days + (2 * verified_seal_days) + streak_bonus`.
- A score snapshot must use finalized public data before it is used for launch claims.

### `$ANKY`

- `$ANKY` is the memetic distribution layer.
- `$ANKY` does not unlock the practice.
- `$ANKY` points people back to the practice.
- A proof-of-practice reward distribution is planned only if rules, custody, snapshot, dispute, and claim/transfer mechanics are finalized and published.

## Forbidden Claims Until Proven

- Fully trustless ZK on Solana.
- Solana directly verifies SP1 proofs today.
- No trusted verifier authority.
- The hash encrypts the writing.
- Anonymous writing.
- The chain proves the user wrote for 8 minutes, unless the client timing model and its limits are explained.
- Anky never stores writing.
- No server ever sees plaintext.
- All legacy systems are privacy-equivalent to the current mobile proof path.
- Mainnet deployment is live.
- Mainnet Core collection is final.
- Production Helius webhook is active.
- `$ANKY` rewards are claimable.
- 8 percent of `$ANKY` is ready for automatic distribution.
- Token supply, reward custody, snapshot, dispute window, or claim mechanics are final unless external evidence is published.

## Current Launch Readiness Statement

Use this wording before mainnet evidence is complete:

```text
Anky has a real mobile proof-of-practice path and a devnet-proven Solana/SP1/indexing loop, but the current launch remains gated on a fresh devnet rerun, live Core parser verification, backend proof worker verification, Helius webhook/backfill confirmation, and mainnet program/collection/verifier evidence. Mainnet deployment and `$ANKY` distribution are not public-ready claims.
```

## Mainnet Evidence Required Before Changing This File

- Mainnet program ID and explorer account evidence.
- Mainnet Core collection and authority evidence.
- Verifier authority public key and custody policy.
- SP1 vkey from the current build.
- Fresh non-mainnet `seal_anky -> SP1 prove/verify -> record_verified_anky -> finalized index -> Score V1` evidence.
- Helius webhook/backfill evidence without exposing API keys.
- Backend migration and proof worker validation.
- Token supply, reward custody, snapshot time, allocation export, dispute window, and claim/transfer process for any `$ANKY` distribution.
