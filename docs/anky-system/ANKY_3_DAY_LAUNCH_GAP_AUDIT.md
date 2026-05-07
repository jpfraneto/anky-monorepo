# Anky 3-Day Launch Gap Audit

Updated: 2026-05-07

This is blunt by design. The project has a real demoable proof-of-practice path, but it is not mainnet/public-distribution ready until the gaps below are closed.

## Executive Status

| Area | Status | Label | Reason |
|---|---|---|---|
| Mobile `.anky` capture | Strong | CURRENT | Protocol, storage, write/reveal/seal/proof state exist. |
| Devnet seal/proof/index demo | Real but must be rerun | DEVNET-PROVEN | Runbooks record a 2026-05-06 devnet seal + verified seal + score `3`; this pass did not rerun it. |
| Mainnet launch | Not ready | UNKNOWN | Mainnet program/collection/verifier/deployment not externally proven. |
| Direct ZK trustlessness | Not ready | CONFLICT | Current model is verifier-authority-attested after off-chain SP1. |
| Helius scoring | Runnable, not externally confirmed live | CURRENT / NEEDS-EXTERNAL-VERIFICATION | Indexer exists; Helius account/webhook status not checked. |
| `$ANKY` public reward distribution | Not ready | CONFIGURED-BUT-UNVERIFIED | Allocation math exists, but token supply/custody/claim/dispute/snapshot are missing or external. |
| Great Slumber | Not implemented | PLANNED / UNKNOWN | Only a TODO/reference was found. |

## Critical Blockers

### 1. Mainnet Truth Is Unknown

**Status:** UNKNOWN

Repo evidence does not prove:

- active seal program deployed on mainnet,
- final mainnet Core collection,
- final verifier authority custody,
- mainnet Loom minting,
- Helius mainnet webhook,
- audited mainnet score snapshot.

Required action:

1. Publish and verify mainnet program ID.
2. Verify executable program account on explorer.
3. Publish Core collection address and authority proof.
4. Publish verifier authority pubkey and custody policy.
5. Run one full non-mainnet E2E again before any mainnet send.

### 2. Core Parser Risk

**Status:** CURRENT / RISK

The active Anchor program parses Metaplex Core account data by hand. Unit tests include devnet layout fixtures, but this must be treated as a mainnet risk until checked against real Core assets and collection accounts.

Required action:

1. Run `npm run check-config -- --cluster devnet --loom-asset <real_loom> --loom-owner <wallet>`.
2. Add or run a live Core integration gate against public devnet accounts.
3. Do not deploy or promote mainnet until this gate is green.

### 3. SP1 To VerifiedSeal Must Be Rerun From Current Worktree

**Status:** DEVNET-PROVEN historically, CONFIGURED-BUT-UNVERIFIED in this pass

Runbooks record a working devnet pair, but the current dirty worktree needs a fresh run before launch.

Required action:

```bash
cd solana/anky-seal-program
npm run sojourn9:privacy
npm run sojourn9:test
npm run sojourn9:prepare-proof -- --writer <wallet> --loom-asset <core_asset> --backend-url <backend_url>
npm run sojourn9:handoff-status -- --manifest /tmp/anky-sojourn9-current-.../handoff-manifest.json
```

Stop if any command asks for a private key or mainnet funds without explicit operator intent.

### 4. Backend Proof Worker Needs Operational Verification

**Status:** CONFIGURED-BUT-UNVERIFIED

The route exists and includes strong privacy shaping, but this pass did not verify the runtime environment:

- `ANKY_MOBILE_PROVER_ENABLED`
- `ANKY_PROVER_VERIFIER_KEYPAIR_PATH`
- `ANKY_PROVER_WORK_DIR`
- `ANKY_PROVER_PROTOC`
- `ANKY_INDEXER_WRITE_SECRET` or `ANKY_VERIFIED_SEAL_RECORD_SECRET`
- database migration `023_mobile_proof_jobs.sql`

Required action:

1. Apply migrations in order on the target environment.
2. Confirm prover workdir is outside repo and not synced.
3. Run one proof job on devnet with an opt-in fixture.
4. Confirm witness cleanup.
5. Confirm only public metadata persisted.

### 5. Helius Webhook And Backfill Are Not Externally Confirmed

**Status:** NEEDS-EXTERNAL-VERIFICATION

The indexer and manifest tooling exist. This pass did not query Helius.

Required action:

1. Create or confirm an enhanced devnet webhook monitoring `ANKY_SEAL_PROGRAM_ID`.
2. Keep `HELIUS_API_KEY` and indexer write secret out of logs.
3. Backfill finalized signatures.
4. Run the snapshot auditor.

Minimum validation:

```bash
node solana/scripts/indexer/ankySealIndexer.mjs --input solana/scripts/indexer/fixtures/anky-seal-events.json
node --test solana/scripts/indexer/ankySealIndexer.test.mjs
node --test solana/scripts/indexer/auditScoreSnapshot.test.mjs
```

### 6. `$ANKY` Distribution Is Not Public-Ready

**Status:** NOT READY FOR PUBLIC CLAIM

Repo supports deterministic scoring and 8 percent allocation math if raw token supply is provided. That is not enough to promise distribution.

Missing:

- externally verified token mint and supply,
- reward pool custody,
- snapshot time,
- final eligible event source,
- dispute/review window,
- allocation export format,
- claim or transfer mechanism,
- legal/tax stance,
- explicit sybil caveats,
- publishable policy saying SP1 verification is bonus or required.

Classification:

```text
$ANKY Distribution Readiness: NOT READY
```

Safe internal statement:

> The indexer can compute a deterministic practice score and a hypothetical 8 percent allocation from finalized public seal data if a raw token supply is supplied.

Unsafe public statement:

> 8 percent of `$ANKY` is ready to claim by Sojourn 9 participants.

## Required Yes/No Answers

| Question | Answer | Label |
|---|---|---|
| Is there a runnable mobile write/hash/seal path in code? | Yes. | CURRENT |
| Is there a runnable SP1 proof path in code? | Yes. | CURRENT |
| Does Solana directly verify SP1 today? | No. | CONFLICT |
| Does the active program support VerifiedSeal? | Yes. | CURRENT |
| Does a devnet seal plus VerifiedSeal evidence exist in repo docs/runbooks? | Yes. | DEVNET-PROVEN |
| Did this documentation pass rerun the full E2E? | No. | CONFIGURED-BUT-UNVERIFIED |
| Is Helius scoring implemented? | Yes. | CURRENT |
| Is production Helius configured and active? | Not proven. | NEEDS-EXTERNAL-VERIFICATION |
| Is Great Slumber implemented? | Not found. | UNKNOWN |
| Is mainnet ready? | No. | UNKNOWN / NEEDS-EXTERNAL-VERIFICATION |
| Is `$ANKY` reward distribution ready for public launch? | No. | CONFIGURED-BUT-UNVERIFIED |

## Three-Day Priority Plan

### Day 1: Freeze Truth And Rerun Devnet

- Freeze public wording around "ZK-enabled", not "fully trustless".
- Remove or annotate stale Bubblegum/cNFT and x402/current-mobile conflicts.
- Rerun Anchor tests and Sojourn 9 privacy/indexer tests.
- Rerun devnet SP1 to VerifiedSeal with a current-day witness.
- Rebuild a finalized devnet score snapshot and audit it.

### Day 2: Mobile/Backend Demo Closure

- Confirm backend migrations on target environment.
- Run `/api/mobile/seals/prove` end-to-end against devnet.
- Confirm mobile states move `sealed -> proving -> proof_verified`.
- Confirm Helius webhook posts or finalized backfill upserts public metadata.
- Confirm no `.anky` plaintext in DB rows or logs for proof/indexing path.

### Day 3: Mainnet Checklist, Not Blind Mainnet

- Verify program ID, collection, verifier, and Loom mint path on mainnet.
- Dry-run all public config checks.
- Publish mainnet values only after explorer verification.
- Publish score rules and privacy caveats before season start.
- Keep `$ANKY` distribution as "planned reward scoring" until token/custody/snapshot is final.

## Mainnet Go / No-Go

No-go until all are true:

- Mainnet program ID verified externally.
- Mainnet Core collection verified externally.
- Verifier authority custody is documented.
- Core parser tested against live real Core accounts.
- SP1 proof path rerun locally.
- Devnet `record_verified_anky` rerun from current worktree.
- Helius finalized backfill and snapshot audit pass.
- Backend proof path tested with witness cleanup.
- Public docs remove stale claims.
- `$ANKY` distribution terms are either removed or backed by a reproducible audited snapshot and custody plan.

