# If JP Is Unavailable

This is the bus-factor-zero guide for a serious builder inheriting the Anky repo.

## First Principle

Do not guess. Do not deploy. Do not publish new claims. Read the evidence and rerun safe checks.

The system is valuable because its boundaries are narrow:

```text
private .anky bytes -> SHA-256 hash -> Solana seal -> optional SP1 receipt -> public score
```

## Where To Start

Repo root:

```bash
cd /home/kithkui/anky
git status
```

Read in this order:

1. `docs/anky-system/STATUS.md`
2. `docs/anky-system/ANKY_TECHNICAL_SOURCE_OF_TRUTH.md`
3. `docs/anky-system/ANKY_RULES_LEDGER.md`
4. `docs/anky-system/ANKY_3_DAY_LAUNCH_GAP_AUDIT.md`
5. `runbooks/sojourn9-sp1-verifiedseal.md`
6. `runbooks/sojourn9-helius-indexing.md`
7. `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs`
8. `solana/anky-zk-proof/src/lib.rs`
9. `src/routes/mobile_sojourn.rs`
10. `apps/anky-mobile/src/lib/ankyProtocol.ts`

## Do Not Touch

Never read, print, copy, or commit:

- `.env`
- `.env.*`
- keypair JSON files
- deployer/wallet files
- private keys
- API keys
- Helius key values
- RevenueCat secrets
- Privy secrets
- Apple/Google credentials
- Stripe credentials

Env var names are okay. Values are not.

## Current Safe Local Checks

These are local/read-oriented or test commands. They should not spend funds or deploy.

```bash
cd solana/anky-seal-program
npm run sojourn9:privacy
npm run sojourn9:test
npm run sojourn9:readiness
```

```bash
cd apps/anky-mobile
npm run typecheck
npm run test:protocol
npm run test:sojourn
```

```bash
cd solana/anky-zk-proof
cargo test
```

```bash
node solana/scripts/indexer/ankySealIndexer.mjs --input solana/scripts/indexer/fixtures/anky-seal-events.json
node --test solana/scripts/indexer/ankySealIndexer.test.mjs
node --test solana/scripts/indexer/auditScoreSnapshot.test.mjs
```

Stop if a command unexpectedly requests a keypair, mainnet RPC, mainnet SOL, or secret value.

## Devnet Proof Demo Path

Read `runbooks/sojourn9-sp1-verifiedseal.md` before running.

Safe starting point:

```bash
cd solana/anky-seal-program
npm run check-config -- --cluster devnet
npm run sojourn9:prepare-proof -- --writer <writer_wallet> --loom-asset <core_asset_v1_loom> --backend-url <backend_url>
npm run sojourn9:handoff-status -- --manifest /tmp/anky-sojourn9-current-.../handoff-manifest.json
```

This may produce next commands that require keypairs. Do not run send commands unless you control the relevant devnet keys and intend to send devnet transactions.

Never use this as a mainnet command.

## Backend Proof Path

The backend proof route is:

```text
POST /api/mobile/seals/prove
GET  /api/mobile/seals/prove/{job_id}
```

Required env var names include:

- `ANKY_MOBILE_PROVER_ENABLED`
- `ANKY_PROVER_VERIFIER_KEYPAIR_PATH`
- `ANKY_PROVER_WORK_DIR`
- `ANKY_PROVER_PROTOC`
- `ANKY_INDEXER_WRITE_SECRET`
- `ANKY_VERIFIED_SEAL_RECORD_SECRET`
- `ANKY_SOLANA_CLUSTER`
- `ANKY_SOLANA_RPC_URL`
- `ANKY_SEAL_PROGRAM_ID`
- `ANKY_CORE_COLLECTION`
- `ANKY_PROOF_VERIFIER_AUTHORITY`

Do not print values.

Before enabling:

1. Confirm `ANKY_SOLANA_CLUSTER` is not mainnet unless launch is explicitly approved.
2. Confirm prover workdir is outside the repo.
3. Confirm migrations through `023_mobile_proof_jobs.sql`.
4. Confirm logs redact raw `.anky`.
5. Confirm witness files are deleted after proof runs.

## Helius / Indexer

The indexer is:

```text
solana/scripts/indexer/ankySealIndexer.mjs
```

For fixtures:

```bash
node solana/scripts/indexer/ankySealIndexer.mjs --input solana/scripts/indexer/fixtures/anky-seal-events.json
```

For live backfill, you need Helius configuration in the operator shell. Do not print it.

```bash
HELIUS_API_KEY=<configured_in_shell> \
ANKY_SOLANA_CLUSTER=devnet \
node solana/scripts/indexer/ankySealIndexer.mjs \
  --backfill \
  --limit 100 \
  --out sojourn9/devnet-score-snapshot.json
```

Audit before publishing:

```bash
node solana/scripts/indexer/auditScoreSnapshot.mjs --snapshot sojourn9/devnet-score-snapshot.json
```

## Mobile

Active app:

```text
apps/anky-mobile
```

Important files:

- `src/screens/WriteScreen.tsx`
- `src/screens/RevealScreen.tsx`
- `src/screens/LoomScreen.tsx`
- `src/lib/ankyProtocol.ts`
- `src/lib/ankyStorage.ts`
- `src/lib/ankyState.ts`
- `src/lib/solana/sealAnky.ts`
- `src/lib/api/ankyApi.ts`
- `src/lib/credits/revenueCatCredits.ts`

Do not call local `.anky` validity "proof verified." In the app, proof verified means public proof metadata exists and passes verifier/protocol/session/day checks.

## Mainnet Procedure

Mainnet is blocked until external verification is complete.

Before any mainnet transaction:

1. Confirm active program ID on explorer.
2. Confirm upgrade authority policy.
3. Confirm Core collection and update authority.
4. Confirm verifier authority custody.
5. Confirm Helius mainnet webhook.
6. Confirm one devnet E2E from current worktree.
7. Confirm score snapshot auditor passes.
8. Confirm public docs do not imply direct on-chain SP1.

If any value is unknown, stop.

## `$ANKY`

Repo page:

```text
templates/ankycoin.html
```

Public address shown in repo:

```text
6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump
```

Treat all token facts as requiring external verification:

- mint exists,
- current supply,
- holder distribution,
- liquidity,
- reward custody,
- snapshot time,
- allocation export,
- claim or transfer process.

Do not promise the 8 percent distribution until an audited finalized snapshot and custody plan exist.

## If Something Breaks

Use this order:

1. Protocol mismatch: compare mobile `ankyProtocol.ts`, SP1 `src/lib.rs`, backend `validate_closed_anky`.
2. Seal failure: check UTC day, Loom owner, Core collection, `DailySeal`/`HashSeal` PDA existence.
3. Proof failure: verify expected session hash, UTC day, SP1 `PROTOC`, prover workdir, and fixture duration.
4. VerifiedSeal failure: check matching `HashSeal`, verifier pubkey, protocol version `1`, PDA seeds.
5. Indexer failure: use fixture input first, then known signatures, then Helius backfill.
6. Score mismatch: run `auditScoreSnapshot.mjs`.

## Public Wording Rules

Safe:

- "Private writing, public hash seal."
- "SP1 proves `.anky` validity off-chain."
- "VerifiedSeal is verifier-authority-attested today."
- "Helius/RPC indexing reconstructs public practice score."

Unsafe:

- "Fully trustless ZK on Solana."
- "The hash encrypts the writing."
- "The chain proves the user wrote for 8 minutes."
- "Mainnet deployed" without explorer proof.
- "`$ANKY` rewards are claimable" without final snapshot/custody proof.

