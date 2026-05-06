# Sojourn 9 Backend VerifiedSeal Runbook

The backend stores public seal and verified receipt metadata for mobile state. It must not store `.anky` plaintext, SP1 witness bytes, or private proof inputs.

## Migration

Apply the current backend migration chain through the mobile verified receipt and Helius webhook receipt migrations before enabling the operator or indexer backend posts:

```bash
psql "$DATABASE_URL" -f migrations/019_credit_ledger_entries.sql
psql "$DATABASE_URL" -f migrations/020_mobile_verified_seal_receipts.sql
psql "$DATABASE_URL" -f migrations/021_mobile_helius_webhook_events.sql
psql "$DATABASE_URL" -f migrations/022_mobile_helius_webhook_signature_dedupe.sql
```

The migrations create or harden `mobile_verified_seal_receipts`, add `utc_day` columns for public seal and verified receipt metadata, add check constraints for hash/protocol/status shape, add unique guards for seal identity and verified transaction signatures, and add a foreign-key guard so verified rows require the matching public seal receipt. They also create `mobile_helius_webhook_events` for public enhanced webhook delivery receipts. If a check constraint, unique index, or foreign key fails, stop and inspect invalid, duplicate, or orphaned receipt rows before launch.

Before applying to a shared database, run the disposable local Postgres smoke:

```bash
node solana/scripts/sojourn9/smokeVerifiedSealMigration.mjs
```

The smoke applies `017_mobile_solana_integration.sql`, `019_credit_ledger_entries.sql`, `020_mobile_verified_seal_receipts.sql`, `021_mobile_helius_webhook_events.sql`, and `022_mobile_helius_webhook_signature_dedupe.sql` twice in a clean database and in a partial pre-existing verified-table database, then verifies public-only columns, constraints, unique indexes, the matching-seal foreign key, Helius webhook receipt guards, signature dedupe, and basic insert guards.

## Required Env

```bash
ANKY_SOLANA_CLUSTER=devnet
ANKY_SOLANA_RPC_URL=<devnet_rpc_url>
ANKY_PUBLIC_SOLANA_RPC_URL=<public_mobile_devnet_rpc_url>
ANKY_CORE_COLLECTION=<devnet_core_collection>
ANKY_PROOF_VERIFIER_AUTHORITY=<devnet_verifier_authority>
ANKY_INDEXER_WRITE_SECRET=<operator_indexer_secret>
ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true
```

`ANKY_VERIFIED_SEAL_RECORD_SECRET` can be used instead of `ANKY_INDEXER_WRITE_SECRET` for the verified metadata route. Do not print either value.
If `ANKY_PROOF_VERIFIER_AUTHORITY` is unset, the backend falls back to the current Sojourn 9 verifier authority compiled into the Anchor program. Set it explicitly before launch so config drift is visible.
`ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true` makes `/api/mobile/seals/verified/record` fetch the public `VerifiedSeal` PDA with finalized `getAccountInfo` and reject metadata that does not match the on-chain writer, session hash, UTC day, proof hash, verifier, and protocol version. Enable it for the launch backend after `ANKY_SOLANA_RPC_URL` is configured.
`ANKY_SOLANA_RPC_URL` is private server-side RPC for chain proof and may point at Helius. The public config routes return `ANKY_PUBLIC_SOLANA_RPC_URL` or `EXPO_PUBLIC_SOLANA_RPC_URL` instead, so do not put a private Helius API key in public RPC config.

## Routes

Public mobile seal receipt:

```text
POST /api/mobile/seals/record
```

This route stores public seal metadata only. It accepts mobile receipt lifecycle statuses (`pending`, `processed`, `confirmed`, `failed`) without a secret because the mobile client may record local/backend state before finalized indexing catches up. `finalized` seal receipts affect Score V1 and therefore require the indexer/operator secret via `x-anky-indexer-secret` or `Authorization`; mobile clients should not post finalized status directly. Once a seal receipt is finalized, later public/mobile writes for the same wallet/hash cannot downgrade or overwrite it; the conflict update only allows a finalized indexer/operator write to touch an existing finalized row. Verified metadata still cannot be recorded until the matching seal row is `confirmed` or `finalized`.

Operator/indexer verified receipt:

```text
POST /api/mobile/seals/verified/record
```

The verified route requires `x-anky-indexer-secret`, rejects unsupported protocol versions, requires the configured proof verifier authority, requires a matching seal receipt whose status is `confirmed` or `finalized`, rejects a mismatched `utcDay` when the matching seal has one, rejects non-landed verified receipt statuses (`pending`, `processed`, `failed`), optionally verifies the landed on-chain `VerifiedSeal` account when `ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true`, and stores only public receipt and transaction metadata.

Helius enhanced webhook receipt:

```text
POST /api/helius/anky-seal
```

The Helius route also requires the indexer secret. Helius should send it via webhook `authHeader` as `Authorization: Bearer <ANKY_INDEXER_WRITE_SECRET>`; operator scripts may still use `x-anky-indexer-secret`. The route rejects private-looking `.anky` field names such as `rawAnky`, rejects string values that are complete valid `.anky` plaintext, caps payload size, and stores only public webhook JSON plus a payload hash, optional signature, source, network, and item count for later parser/indexer reconciliation.

## Verification

```bash
CARGO_TARGET_DIR=/tmp/anky-root-target cargo check
CARGO_TARGET_DIR=/tmp/anky-root-target cargo test proof_verifier_authority -- --nocapture
CARGO_TARGET_DIR=/tmp/anky-root-target cargo test verified_seal_record_secret -- --nocapture
CARGO_TARGET_DIR=/tmp/anky-root-target cargo test verified_seal_record_requires_configured_verifier_authority -- --nocapture
CARGO_TARGET_DIR=/tmp/anky-root-target cargo test verified_seal_account_data -- --nocapture
```

This migration set was smoke-tested against disposable local Postgres clusters by applying `017_mobile_solana_integration.sql`, `019_credit_ledger_entries.sql`, `020_mobile_verified_seal_receipts.sql`, `021_mobile_helius_webhook_events.sql`, and `022_mobile_helius_webhook_signature_dedupe.sql` twice, and confirming the `utc_day` columns, UTC-day constraints, verified receipt hash/protocol/status constraints, unique indexes, Helius webhook receipt constraints/indexes, Helius signature dedupe index, and `mobile_verified_seal_receipts_matching_seal` foreign key exist. A second smoke simulated a partial pre-existing verified receipt table and confirmed the migration adds the missing constraints and indexes.

Reusable smoke command:

```bash
node --test solana/scripts/sojourn9/smokeVerifiedSealMigration.test.mjs
```

For backend-accepted VerifiedSeal metadata, only `confirmed` and `finalized` are valid because this table represents a landed verified receipt. Mobile can still render `proving` or `proof_failed` from local/prover state, but those states should not be inserted into `mobile_verified_seal_receipts`.

## Live 0xx1 Metadata Follow-Up

After the target backend is migrated and configured, replay the live devnet `0xx1` public transactions through the Helius/indexer path documented in `runbooks/sojourn9-helius-indexing.md`.

Use the known finalized signatures:

```text
seal_anky:             5EvmetB1HBsRJR4ErvTbvvamq63fQzEEAhxoiaUobVxUybFeMRBYVVur3CX5mfJxRCAubFkjib2QodK1E8avTEJU
record_verified_anky:  2pyxGzZeYzmd3r5ctTqhB73RX3QPCAeE7VRQYMRPD16nCv1t7Gj3U7NXhgWJkcWiwaSyYZP72sNtoDRbcUNTQ91
```

The backend replay must use `ANKY_INDEXER_WRITE_SECRET` from the operator shell and must not paste the secret into the command history. The replay posts only public seal and verified receipt metadata; it does not send `.anky` plaintext or SP1 witness data.
