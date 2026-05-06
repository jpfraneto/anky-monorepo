# Sojourn 9 SP1 to VerifiedSeal Runbook

This is the current non-mainnet operator path. It never stores `.anky` plaintext; the plaintext is read only by the local SP1 process from the operator-provided file path.

## 0. Read-Only Config Check

The operator package exposes the launch helpers as `npm run` aliases from `solana/anky-seal-program`:

```bash
cd solana/anky-seal-program
npm run sojourn9:readiness
npm run sojourn9:privacy
npm run sojourn9:test
npm run sojourn9:demo-witness -- --help
npm run sojourn9:live-checklist -- --help
npm run sojourn9:prepare-proof -- --help
npm run sojourn9:handoff-status -- --help
npm run sojourn9:prove-record -- --help
npm run sojourn9:make-evidence -- --help
```

Before proving or recording receipts, confirm the public devnet program and Core collection are visible:

```bash
cd solana/anky-seal-program
npm run check-config -- --cluster devnet
```

When a real Loom asset and expected owner are known, include them so the checker also validates the Core AssetV1 base layout, collection update authority, configured collection, and owner:

```bash
cd solana/anky-seal-program
npm run check-config -- \
  --cluster devnet \
  --loom-asset <core_asset_v1_loom> \
  --loom-owner <expected_wallet_owner>
```

This only reads public account state. It does not read keypairs, sign transactions, spend SOL, or print API keys.

If the operator already has the public same-day witness metadata, generate a no-secret live handoff checklist before touching keypairs:

```bash
cd solana/anky-seal-program
npm run sojourn9:live-checklist -- \
  --writer <writer_wallet> \
  --loom-asset <core_asset_v1_loom> \
  --session-hash <session_hash_from_demo_witness> \
  --utc-day <utc_day_from_demo_witness> \
  --backend-url <backend_url>
```

The checklist validates public keys, the current UTC day, the session hash shape, backend/webhook URL safety, and prints placeholders for the writer keypair, verifier keypair, backend write secret, and Helius API key. It does not accept keypair paths or secret values.
It prints `utcDayStatus` with `sealWindow`, seconds until UTC-day rollover, and the rollover timestamp so the operator can see when the same-day seal command expires.

To prepare a complete current-day local proof handoff without touching signing keys, run:

```bash
cd solana/anky-seal-program
npm run sojourn9:prepare-proof -- \
  --writer <writer_wallet> \
  --loom-asset <core_asset_v1_loom> \
  --backend-url <backend_url>
```

This command creates a demo witness under `/tmp`, runs SP1 Core prove, re-verifies the saved proof artifact, checks whether the matching public `HashSeal` exists, and writes a no-secret `handoff-manifest.json` next to the temp artifacts. The manifest prints the next human command. If `hashSeal.exists` is `false`, the next command is the writer-keypair `seal_anky` send. If it is `true`, the next command is the verifier-authority `record_verified_anky` send. These next commands are chain-only. When `--backend-url` is supplied, the manifest also prints `backendFollowupCommands` that use a landed transaction signature placeholder and must be run only after the relevant PDA is readable and the backend is reachable.
The manifest includes `utcDayStatus` with the same seal-window and rollover metadata printed by the checklist and handoff status commands.

Do not commit or upload the generated witness. Regenerate the handoff after UTC midnight because `seal_anky` accepts only the current UTC day.

After the handoff exists, re-check public chain/backend readiness without reading the witness:

```bash
cd solana/anky-seal-program
npm run sojourn9:handoff-status -- \
  --manifest /tmp/anky-sojourn9-current-.../handoff-manifest.json
```

The status checker reads only public manifest and receipt metadata. It never reads the witness file, checks the public `HashSeal` and `VerifiedSeal` PDAs on devnet, optionally checks public backend seal/score status, and prints the next safe operator action.
It also prints `utcDayStatus` with the receipt UTC day, current UTC day, seal-window status, seconds until UTC-day rollover, and the exact rollover timestamp so operators can see when a same-day `HashSeal` command will become stale.
If either backend status endpoint fails, it keeps HashSeal, VerifiedSeal, and Helius critical-step commands chain/index-only and prints separate backend metadata commands for after both public backend status reads are healthy.

After the HashSeal and VerifiedSeal have landed, Helius finalized backfill has been audited, and a public webhook ID exists, build the public launch evidence file from the handoff without reading the witness:

```bash
cd solana/anky-seal-program
npm run sojourn9:make-evidence -- \
  --manifest /tmp/anky-sojourn9-current-.../handoff-manifest.json \
  --core-collection <devnet_core_collection> \
  --sp1-vkey <sp1_vkey> \
  --seal-signature <landed_seal_signature> \
  --verified-signature <landed_verified_signature> \
  --backend-url https://<public_backend_host> \
  --helius-webhook-id <public_helius_webhook_id> \
  --score-snapshot sojourn9/devnet-score-snapshot.json \
  --snapshot-time <utc_iso_snapshot_time> \
  --audit-score-snapshot \
  --backfill-audited \
  --audit \
  --out sojourn9/public-launch-evidence.json
```

This evidence helper reads only public manifest and receipt metadata, derives Orb transaction links from real signatures, refuses mainnet, and requires either an explicit `--score-audited` confirmation or `--audit-score-snapshot` to run the public Score V1 auditor directly. It also requires `--backfill-audited`. It accepts no keypair paths, backend secrets, Helius API keys, or `.anky` plaintext.
The public evidence builder copies `manifest.utcDayStatus` into `devnetE2E.utcDayStatus`, and the public evidence auditor rejects missing or inconsistent UTC-day status so final evidence preserves the same-day seal-window context.

## 1. One-Shot Local Prove and Record Wrapper

Use this when the operator has an opt-in `.anky` file path and wants one command to produce the public SP1 receipt and then run the VerifiedSeal preflight/operator path.

For a fresh devnet E2E, first create a same-day demo witness outside the repo:

```bash
node solana/scripts/sojourn9/makeDemoAnky.mjs \
  --out /tmp/anky-sojourn9-demo.anky
```

The generator refuses to write `.anky` plaintext inside the git worktree, writes the temp file with `0600` permissions, and prints only public metadata: `sessionHash`, `utcDay`, start time, duration, event count, and the temp path. Do not commit or upload the generated witness.

Before running SP1, create or verify the matching same-day `HashSeal`. This helper reads only the public hash/day/Loom values. Dry-run with public chain preflight first:

```bash
cd solana/anky-seal-program
npm run seal -- \
  --writer <writer_wallet> \
  --loom-asset <core_asset_v1_loom> \
  --session-hash <session_hash_from_demo_witness> \
  --utc-day <utc_day_from_demo_witness> \
  --cluster devnet \
  --check-chain
```

To send the devnet seal from an operator shell, the sealer keypair must be the writer and current owner of the Loom:

```bash
cd solana/anky-seal-program
ANKY_SEALER_KEYPAIR_PATH=<writer_keypair_path> \
npm run seal -- \
  --loom-asset <core_asset_v1_loom> \
  --session-hash <session_hash_from_demo_witness> \
  --utc-day <utc_day_from_demo_witness> \
  --cluster devnet \
  --check-chain \
  --send
```

The seal helper refuses mainnet, refuses stale UTC days during preflight/send, checks the public Core collection/Loom base account fields, and fails if either the `DailySeal` or `HashSeal` already exists.

If the seal has already landed and Helius indexing is not configured yet, post only the public seal receipt metadata to the backend after a HashSeal chain check:

```bash
cd solana/anky-seal-program
npm run seal -- \
  --writer <writer_wallet> \
  --loom-asset <core_asset_v1_loom> \
  --session-hash <session_hash_from_demo_witness> \
  --utc-day <utc_day_from_demo_witness> \
  --cluster devnet \
  --backend-url <backend_url> \
  --backend-signature <landed_seal_signature> \
  --check-sealed-chain
```

When `--backend-url` is supplied with `--send`, the helper posts the same public seal metadata immediately after `seal_anky` confirms. Backend seal posts do not send `.anky` plaintext and do not require the verified receipt secret; the verified receipt route remains secret-gated.
For launch handoffs, prefer the two-step flow above unless the backend is known reachable before sending: first land the chain transaction, then post public metadata with `--backend-signature` after a `--check-sealed-chain` read. This keeps an HTTP metadata failure from obscuring whether the chain transaction landed.

```bash
node solana/scripts/sojourn9/proveAndRecordVerified.mjs \
  --file /tmp/anky-sojourn9-demo.anky \
  --writer <writer_wallet> \
  --expected-hash <sealed_session_hash> \
  --utc-day <sealed_utc_day> \
  --cluster devnet \
  --check-chain-first \
  --check-chain
```

The wrapper does not read the `.anky` file itself. It passes the path to the SP1 script, requires `--expected-hash`, and then invokes the lower-level VerifiedSeal operator using the generated public `receipt.json`.
For `--send`, the wrapper refuses `--sp1-mode execute` and passes the lower-level `--sp1-proof-verified` guard only after `--sp1-mode prove`.
`--check-chain-first` verifies the public `HashSeal` exists before SP1 runs; because the wrapper does not parse plaintext, it also requires the expected `--utc-day`.

If a saved SP1 proof artifact already exists, use `--proof` instead of `--receipt` so the wrapper verifies the proof locally, extracts the public receipt values from the proof, and only then invokes the VerifiedSeal operator:

```bash
node solana/scripts/sojourn9/proveAndRecordVerified.mjs \
  --proof /tmp/anky-sp1-proof-with-public-values.bin \
  --writer <writer_wallet> \
  --cluster devnet \
  --check-chain
```

Use raw `--receipt` only for dry-runs or after an operator has independently verified the matching SP1 proof artifact.
The one-shot wrapper refuses `--send` with raw `--receipt`; use `--file` so SP1 `--prove` runs now, or `--proof` so a saved proof artifact is verified locally before the chain write. The lower-level `npm run record-verified` command remains available for explicit manual operator attestations.

Wrapper smoke tests:

```bash
node --test solana/scripts/sojourn9/makeDemoAnky.test.mjs
node --test solana/scripts/sojourn9/proveAndRecordVerified.test.mjs
```

## 2. Generate and Verify the SP1 Receipt Manually

```bash
cd solana/anky-zk-proof/sp1/program
cargo prove build

cd ../script
PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc \
RUST_LOG=info \
cargo run --release -- \
  --prove \
  --file ../../fixtures/full.anky \
  --writer <writer_wallet> \
  --expected-hash <sealed_session_hash> \
  --receipt-out /tmp/anky-sp1-prove-receipt.json \
  --proof-out /tmp/anky-sp1-proof-with-public-values.bin
```

The SP1 script verifies the proof locally before it writes `receipt.json`. It prints/writes public receipt metadata and proof artifacts only; it must not print or persist the private `.anky` witness.
Do not upload or persist the private `.anky` file outside the operator machine.
Use `--expected-hash` when verifying a real sealed rite so the private witness must hash to the existing `HashSeal`.
For a new devnet E2E run, use a same-day `.anky` file. The Anchor program only allows `seal_anky` for the current UTC day, so a historical SP1 receipt can only be verified if its matching historical `HashSeal` already exists.

To re-verify a saved proof artifact without reading the `.anky` witness again:

```bash
cd solana/anky-zk-proof/sp1/script
PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc \
RUST_LOG=info \
cargo run --release -- \
  --verify \
  --proof /tmp/anky-sp1-proof-with-public-values.bin \
  --receipt-out /tmp/anky-sp1-verified-receipt-from-proof.json
```

Current local proof check, run on 2026-05-06:

```bash
cd solana/anky-zk-proof/sp1/script
PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc \
RUST_LOG=info \
cargo run --release -- \
  --prove \
  --file ../../fixtures/full.anky \
  --writer 11111111111111111111111111111111 \
  --receipt-out /tmp/anky-sp1-prove-receipt.json \
  --proof-out /tmp/anky-sp1-proof-with-public-values.bin
```

Observed verification key:

```text
0x00399c50f86cb417d0cf0c80485b0f1781590170c6892861a1a55974da6e4758
```

Observed result: the script printed `proof verified` and the public receipt contained `valid: true`, `duration_ok: true`, session hash `c4d8d04ee62d4c6080df750ee5a742b71bcf74d8f4e29f84a4966b1eef26d824`, UTC day `19675`, and proof hash `a655d1183d5503baa8e32eb14e96253173ba697d9fef695447b5e0e0922bd1dd`.

## 3. Dry-Run the VerifiedSeal Transaction

```bash
cd solana/anky-seal-program
npm run record-verified -- \
  --receipt ../anky-zk-proof/sp1/script/receipt.json \
  --writer <writer_wallet> \
  --cluster devnet
```

This validates receipt fields and prints the `HashSeal` and `VerifiedSeal` PDAs. It does not send a transaction.
It also recomputes the public `proof_hash` from receipt fields and rejects mismatches.

To confirm the matching `HashSeal` is already on-chain before sending:

```bash
cd solana/anky-seal-program
npm run record-verified -- \
  --receipt ../anky-zk-proof/sp1/script/receipt.json \
  --writer <writer_wallet> \
  --cluster devnet \
  --check-chain
```

To check only the public sealed hash before spending time on SP1:

```bash
cd solana/anky-seal-program
npm run record-verified -- \
  --writer <writer_wallet> \
  --session-hash <sealed_session_hash> \
  --utc-day <sealed_utc_day> \
  --cluster devnet \
  --check-hashseal-only
```

This fails if the matching `HashSeal` is missing, if the `HashSeal` data does not match the writer/hash/day, or if the `VerifiedSeal` already exists.

Operator-script tests:

```bash
cd solana/anky-seal-program
npm run test:operator
```

## 4. Submit on Devnet Only

```bash
cd solana/anky-seal-program
ANKY_SOLANA_CLUSTER=devnet \
ANKY_VERIFIER_KEYPAIR_PATH=<verifier_authority_keypair_path> \
npm run record-verified -- \
  --receipt ../anky-zk-proof/sp1/script/receipt.json \
  --writer <writer_wallet> \
  --cluster devnet \
  --sp1-proof-verified \
  --send
```

`--send` always performs the same chain preflight: matching `HashSeal` must exist, its writer/hash/day must match the receipt, and `VerifiedSeal` must not already exist.
The lower-level operator requires `--sp1-proof-verified` with `--send`; this is an explicit operator attestation that the receipt came from a local SP1 `--prove` run that printed `proof verified`. Do not use this flag for `--sp1-mode execute` outputs.
The operator also supports `--backend-url` with `--send`, but launch handoffs should normally keep this as a chain-only command. Post backend metadata in the next section after the `VerifiedSeal` account is readable. This keeps an HTTP metadata failure from obscuring whether `record_verified_anky` landed.
If `--backend-url` is supplied, the operator re-reads the landed `VerifiedSeal` PDA and posts only public verified receipt metadata to `/api/mobile/seals/verified/record` after the on-chain account matches the public receipt.
The operator refuses any `--backend-url` run unless `ANKY_INDEXER_WRITE_SECRET` or `ANKY_VERIFIED_SEAL_RECORD_SECRET` is configured before chain checks, keypair loading, or transaction submission.
For the launch backend, also set `ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true` and private `ANKY_SOLANA_RPC_URL=<devnet_rpc_url>` so the backend independently fetches the finalized public `VerifiedSeal` PDA before accepting the posted metadata. Set `ANKY_PUBLIC_SOLANA_RPC_URL` or `EXPO_PUBLIC_SOLANA_RPC_URL` separately for mobile config; do not expose a private Helius API-key URL through public config.

Stop before mainnet. Mainnet requires confirmed program ID, Core collection, verifier custody, funding, Helius Sender policy, and launch approval.

The operator script refuses mainnet unless `ANKY_ALLOW_MAINNET_RECORD_VERIFIED=true` is set. If that gate is ever opened, it requires `HELIUS_API_KEY`, estimates a live priority fee, adds the Helius Sender tip, submits through Sender, and still records only the public receipt metadata.

## 5. Record Backend Metadata

Persist only public metadata after `record_verified_anky` lands. The operator requires `--check-verified-chain` here so an already-landed backend write cannot happen without reading the matching `VerifiedSeal` PDA:

```bash
cd solana/anky-seal-program
ANKY_INDEXER_WRITE_SECRET=<backend_write_secret> \
npm run record-verified -- \
  --receipt ../anky-zk-proof/sp1/script/receipt.json \
  --writer <writer_wallet> \
  --cluster devnet \
  --check-verified-chain \
  --backend-url <backend_url> \
  --backend-signature <record_verified_anky_signature>
```

Body:

```json
{
  "wallet": "<writer_wallet>",
  "sessionHash": "<receipt.session_hash>",
  "proofHash": "<receipt.proof_hash>",
  "verifier": "<verifier_authority>",
  "protocolVersion": 1,
  "utcDay": <receipt.utc_day>,
  "signature": "<record_verified_anky_signature>",
  "status": "confirmed"
}
```

The backend rejects this if the matching seal receipt is not already known.
It also rejects the write unless the operator/indexer secret header matches backend configuration.
