# Sojourn 9 Sponsored Transactions Devnet Validation

This runbook is the controlled validation path for the sponsored-payer changes. It must be run only after a human explicitly approves devnet deployment and devnet transactions.

This runbook is not mainnet approval.

## Purpose

Prove the current code path after the sponsored-payer account model change:

- Funded users pay their own Loom mint and seal transactions.
- Unfunded users can receive eligible sponsorship when sponsorship is enabled and budget remains.
- The user wallet remains Loom owner and seal writer.
- The sponsor only pays transaction fees and rent.
- Proof requests are backend/verifier-paid, retry-limited, and represented in the sponsorship budget ledger.
- Helius/RPC indexing can reconstruct payer, writer, seal, verified receipt, and score state.

## Stop Rules

Stop immediately if any step would:

- Use mainnet.
- Print, paste, or commit a keypair, API key, webhook secret, verifier secret, or private key.
- Store `.anky` plaintext outside transient prover input.
- Send a transaction before devnet deployment/transaction approval is explicit.
- Claim fresh launch evidence without collecting the evidence artifacts listed below.

## Required Public Inputs

Record these public values in the final evidence summary:

- Devnet Anky Seal Program ID.
- Devnet Metaplex Core collection address.
- Devnet proof verifier authority.
- Devnet sponsor payer public key.
- Devnet Core collection authority public key.
- Backend base URL used for the run.
- Helius/indexer source used for scoring, or `fixture-only`.

Do not record secret values or keypair paths in public evidence.

## Local Preflight

Run before any deployment or transaction:

```bash
git diff --check
cargo check
cargo test mobile_sponsorship_migration_has_no_private_input_columns
cargo test mobile_sponsorship_migration_tracks_proof_budget_metadata
cargo test sponsored_core_loom_parser

cd solana/anky-seal-program
npm test
npm run build
cargo test --manifest-path Cargo.toml --package anky_seal_program

cd ../../apps/anky-mobile
npm run typecheck
npm run test:protocol
npm run test:sojourn
```

The live Anchor integration guard should refuse without explicit approval:

```bash
cd solana/anky-seal-program
npm run test:anchor:live
```

Expected result: it exits before `anchor test` and says `ANKY_ALLOW_LIVE_ANCHOR_TEST=true` is required.

## Backend Setup

Apply the sponsorship migration in the devnet backend database:

```bash
migrations/025_mobile_sponsorship_events.sql
```

Configure these env var names in the backend environment. Do not print values:

```text
ANKY_SOLANA_CLUSTER=devnet
ANKY_SOLANA_RPC_URL
ANKY_SEAL_PROGRAM_ID
ANKY_CORE_PROGRAM_ID
ANKY_CORE_COLLECTION
ANKY_PROOF_VERIFIER_AUTHORITY
ANKY_ENABLE_SPONSORSHIP=true
ANKY_SPONSOR_DAILY_BUDGET_LAMPORTS
ANKY_USER_MINT_MIN_LAMPORTS
ANKY_USER_SEAL_MIN_LAMPORTS
ANKY_SPONSORED_LOOM_MINT_ESTIMATED_LAMPORTS
ANKY_SPONSORED_SEAL_ESTIMATED_LAMPORTS
ANKY_SPONSORED_PROOF_ESTIMATED_LAMPORTS
ANKY_SPONSOR_PAYER_KEYPAIR_PATH
ANKY_CORE_COLLECTION_AUTHORITY_KEYPAIR_PATH
ANKY_MOBILE_PROVER_ENABLED=true
ANKY_PROVER_VERIFIER_KEYPAIR_PATH
ANKY_PROVER_WORK_DIR
ANKY_PROVER_PROTOC
ANKY_PROOF_MAX_ATTEMPTS_PER_SEAL
ANKY_INDEXER_WRITE_SECRET
```

Keep `ANKY_ENABLE_MAINNET_SPONSORSHIP` unset.

## Controlled Devnet Deployment

Only after explicit human approval, rebuild and deploy the Anchor program to devnet using the operator’s normal deploy procedure.

After deployment, record:

- Deployment command used.
- Program ID.
- Deployment signature.
- Deployer public key.
- Cluster URL.

Do not treat the earlier accidental devnet deployment as evidence for this run.

## Live Devnet Checks

Run the Core integration test with a real devnet Loom owned by the provider wallet:

```bash
cd solana/anky-seal-program
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
ANCHOR_WALLET=<provider_wallet_keypair_path> \
ANKY_CORE_INTEGRATION_LOOM_ASSET=<owned_core_loom_asset> \
ANKY_CORE_INTEGRATION_COLLECTION=<devnet_core_collection> \
ANKY_ALLOW_LIVE_ANCHOR_TEST=true \
npm run test:anchor:live -- --skip-local-validator --skip-deploy
```

Expected evidence:

- `seal_anky` succeeds for the current UTC day.
- `writer` is the Loom owner.
- `payer` is either writer or sponsor, depending on case.
- `LoomState`, `DailySeal`, and `HashSeal` are created or updated as expected.

## Sponsored Flow Matrix

Use distinct test wallets where possible.

| Case | Wallet SOL | Expected payer | Expected result |
| --- | --- | --- | --- |
| Mint funded | Above mint threshold | User wallet | Loom owner is user wallet |
| Mint unfunded | Below mint threshold | Sponsor payer | Loom owner is still user wallet |
| Seal funded | Above seal threshold | User wallet | Writer and payer are user wallet |
| Seal unfunded | Below seal threshold | Sponsor payer | Writer is user wallet; sponsor only pays |
| Seal invalid Loom | Below seal threshold | No prepared sponsor tx | Backend rejects before sponsor signs |
| Proof retry | Any | Verifier authority | Proof jobs capped by retry limit |

For each sponsored case, query `mobile_sponsorship_events` and confirm:

- `network = devnet`
- `action` is `mint_loom`, `seal`, or `proof`
- `wallet` is the user wallet
- `sponsor_payer` is the sponsor payer for mint/seal or verifier authority for proof
- `estimated_lamports` is nonzero
- repeated prepare does not create a second budget row for the same idempotency key

## SP1 To VerifiedSeal

Using an explicit opt-in `.anky` fixture or user test file:

1. Validate the exact SHA-256 hash of the raw UTF-8 `.anky` bytes.
2. Submit `POST /api/mobile/seals/prove`.
3. Confirm the backend creates a `mobile_proof_jobs` row without plaintext.
4. Confirm `mobile_sponsorship_events` contains action `proof`.
5. Confirm local SP1 verification passes before chain submission.
6. Confirm `record_verified_anky` lands on devnet.
7. Confirm `mobile_verified_seal_receipts` reaches `finalized` or a documented syncing/recovery state.

Evidence must include only public metadata: wallet, session hash, UTC day, proof hash, verifier, signatures, slots, statuses, and timestamps.

## Index And Score Evidence

Run the active indexer/backfill path against devnet data and produce:

- Parsed `AnkySealed` event with writer, payer, Loom, hash, UTC day, signature, finalized status.
- Parsed `AnkyVerified` event with writer, hash, proof hash, verifier, UTC day, signature, finalized status.
- Score row showing unique sealed days, verified days, streak bonus, and final score.
- Snapshot JSON or summary path.

If Helius is unavailable, mark the result `fixture-only`; do not call it live evidence.

## Pass Criteria

This run passes only if all are true:

- No mainnet transaction occurred.
- No secret or `.anky` plaintext was logged or persisted.
- Funded wallet paths are user-paid.
- Unfunded eligible mint and seal paths are sponsor-paid.
- User remains Loom owner and seal writer in every case.
- Sponsor never becomes writer, owner, or beneficiary.
- Proof is verifier-authority-attested after local SP1 verification.
- Sponsorship rows prove budget/idempotency behavior for mint, seal, and proof.
- Indexer/score output reflects finalized devnet state.

## Launch Status After Run

Even after this run passes, mainnet remains blocked until:

- Mainnet program ID is known and intentionally deployed.
- Mainnet Core collection is known or created.
- Verifier authority custody is confirmed.
- Sponsor payer custody and budget are confirmed.
- Mainnet launch checklist is completed.
