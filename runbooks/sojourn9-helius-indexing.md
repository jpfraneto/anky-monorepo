# Sojourn 9 Helius Indexing and Score Snapshot

The current indexer reconstructs public `AnkySealed` and `AnkyVerified` state from finalized transaction data. It writes deterministic score snapshots and never accepts `.anky` plaintext.
It prefers Anchor `Program data:` logs emitted while the active Solana invocation stack is the configured Anky Seal Program, and falls back to public Anchor instruction data in Helius enhanced transaction/webhook payloads when logs are absent.
Decoded fixture/webhook events are also validated for public-key, hash, protocol, and UTC-day shape before scoring.
The indexer can also ingest public operator-style verified metadata, but only when it includes `utcDay`; backend rows that omit UTC day are ignored for scoring until reconciled with finalized chain events.
Verified events only count for scoring and backend posts when `protocolVersion` is `1` and `verifier` matches `ANKY_PROOF_VERIFIER_AUTHORITY` or the current Sojourn 9 verifier default.
Failed transactions are never scored, even if their public logs or instructions are decodable.
The indexer decodes configured program and verifier public keys and rejects values that are not 32-byte Solana public keys.
The CLI rejects secret-shaped `--input`/`--out` paths, direct `.anky` witness paths, credentialed backend URLs, non-local plaintext backend URLs, invalid cluster names, and mainnet indexing that would fall back to devnet program/verifier defaults.

## Fixture Smoke Test

```bash
node solana/scripts/indexer/ankySealIndexer.mjs \
  --input solana/scripts/indexer/fixtures/anky-seal-events.json
```

Repeatable parser/scoring tests:

```bash
node --test solana/scripts/indexer/ankySealIndexer.test.mjs
node --test solana/scripts/indexer/auditScoreSnapshot.test.mjs
node --test solana/scripts/indexer/heliusWebhookManifest.test.mjs
```

Supported input shapes:

- Helius enhanced webhook arrays or saved transaction JSON that includes Anchor `Program data:` logs or public Anky Seal instruction data.
- Known finalized transaction signatures via `--signature`; the indexer fetches each signature with Helius `getTransaction` at finalized commitment and annotates finality explicitly.
- Backend-exported `mobile_helius_webhook_events` rows that include `payload_json` or `payloadJson`; the indexer parses the stored public webhook payload without an ad hoc transform.
- `{"decodedEvents":[...]}` fixtures for deterministic local tests.
- Public operator metadata for landed `VerifiedSeal` receipts, using `wallet`, `sessionHash`, `proofHash`, `verifier`, `protocolVersion`, `utcDay`, and `txSignature` or `proofTxSignature`.

The public operator metadata path is a convenience for local reconciliation. Final reward snapshots should still be regenerated from finalized chain data.

To reconcile stored webhook delivery receipts through the same parser, export only public receipt rows and feed the JSON to the indexer:

```bash
psql "$DATABASE_URL" -Atc \
  "SELECT COALESCE(json_agg(row_to_json(t)), '[]'::json) FROM (
     SELECT payload_json
     FROM mobile_helius_webhook_events
     WHERE network = 'devnet'
     ORDER BY created_at
   ) t" \
  > /tmp/anky-helius-webhook-events.json

node solana/scripts/indexer/ankySealIndexer.mjs \
  --input /tmp/anky-helius-webhook-events.json \
  --include-non-finalized \
  --out sojourn9/devnet-webhook-live-snapshot.json
```

Use `--include-non-finalized` only for live UI reconciliation. Final reward snapshots must use finalized backfill.

Expected score rule:

```text
score = unique_seal_days + verified_days + 2 * floor(each_consecutive_day_run / 7)
```

Reward participant selection is capped at 3,456 wallets by default. The indexer sorts by `score` descending, then wallet address ascending for deterministic ties, and only capped participant rows receive allocation. Use `--max-participants <n>` only for tests or a future published season rule.

## Devnet Backfill

```bash
HELIUS_API_KEY=<configured_in_shell> \
ANKY_SOLANA_CLUSTER=devnet \
ANKY_PROOF_VERIFIER_AUTHORITY=<devnet_verifier_authority> \
node solana/scripts/indexer/ankySealIndexer.mjs \
  --backfill \
  --limit 100 \
  --out sojourn9/devnet-score-snapshot.json
```

You can also set private server-side `ANKY_SOLANA_RPC_URL` to a full Helius RPC URL. Do not print the API key, and do not reuse that private URL as `ANKY_PUBLIC_SOLANA_RPC_URL` or `EXPO_PUBLIC_SOLANA_RPC_URL`.
Backfill requires `HELIUS_API_KEY` or `ANKY_SOLANA_RPC_URL` pointing at a Helius RPC endpoint and uses Helius `getTransactionsForAddress` with full transaction details. Without Helius configuration, use `--input` fixtures or saved webhook payloads only.
Backfill requests `commitment: finalized`. If Helius omits commitment fields in the response, the indexer marks those events with `finalitySource: "requested_finalized_commitment"` and increments `summary.finalizedEventsInferredFromBackfillRequest`; ordinary `--input` files with missing finality are not treated as finalized by default.
Backfill retries transient HTTP 429/5xx and common RPC node-busy errors. Tune with `ANKY_INDEXER_RPC_RETRIES` and `ANKY_INDEXER_RETRY_BASE_MS` if a snapshot run is rate-limited.
For `--cluster mainnet-beta`, pass explicit published mainnet values through `--program-id` or `ANKY_SEAL_PROGRAM_ID` and `--proof-verifier` or `ANKY_PROOF_VERIFIER_AUTHORITY`. The devnet defaults are intentionally refused for mainnet indexing.

If a fresh devnet transaction pair is finalized but the program-address backfill has not caught up yet, reconcile those known signatures without weakening finalized-data scoring:

```bash
HELIUS_API_KEY=<configured_in_shell> \
node solana/scripts/indexer/ankySealIndexer.mjs \
  --signature <seal_tx_signature>,<record_verified_anky_signature> \
  --cluster devnet \
  --program-id 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX \
  --out sojourn9/devnet-score-snapshot.json

node solana/scripts/indexer/auditScoreSnapshot.mjs \
  --snapshot sojourn9/devnet-score-snapshot.json
```

This path still reads only public finalized transaction data from Helius. It is useful for live demo evidence and debugging index lag; final season snapshots should prefer the program-address backfill or stored finalized webhook receipts across the full snapshot window.

### Live 0xx1 Devnet Evidence Replay

The live `0xx1` devnet run landed this finalized transaction pair:

```text
seal_anky:             5EvmetB1HBsRJR4ErvTbvvamq63fQzEEAhxoiaUobVxUybFeMRBYVVur3CX5mfJxRCAubFkjib2QodK1E8avTEJU
record_verified_anky:  2pyxGzZeYzmd3r5ctTqhB73RX3QPCAeE7VRQYMRPD16nCv1t7Gj3U7NXhgWJkcWiwaSyYZP72sNtoDRbcUNTQ91
```

Rebuild and audit the no-secret known-signature score snapshot:

```bash
HELIUS_API_KEY=<configured_in_shell> \
node solana/scripts/indexer/ankySealIndexer.mjs \
  --signature 5EvmetB1HBsRJR4ErvTbvvamq63fQzEEAhxoiaUobVxUybFeMRBYVVur3CX5mfJxRCAubFkjib2QodK1E8avTEJU,2pyxGzZeYzmd3r5ctTqhB73RX3QPCAeE7VRQYMRPD16nCv1t7Gj3U7NXhgWJkcWiwaSyYZP72sNtoDRbcUNTQ91 \
  --cluster devnet \
  --program-id 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX \
  --proof-verifier FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP \
  --out /tmp/anky-live-devnet-signature-score-snapshot-20579.json

node solana/scripts/indexer/auditScoreSnapshot.mjs \
  --snapshot /tmp/anky-live-devnet-signature-score-snapshot-20579.json
```

Observed on 2026-05-06: the known-signature snapshot indexed `2` finalized events, one sealed day, one verified day, and score `2` for wallet `5xf7VcURsgiy3SvkBUirAYSPu3SYhto9qX6AFrLTvN1Q`; the score auditor returned `ok: true`.

After the target backend has `020_mobile_verified_seal_receipts`, `021_mobile_helius_webhook_events`, and `022_mobile_helius_webhook_signature_dedupe` applied, plus `019_credit_ledger_entries` if applying the full backend chain, and the indexer secret configured outside Codex, post only public seal/verified metadata from the same finalized transaction pair:

```bash
HELIUS_API_KEY=<configured_in_shell> \
ANKY_INDEXER_WRITE_SECRET=<configured_in_shell> \
node solana/scripts/indexer/ankySealIndexer.mjs \
  --signature 5EvmetB1HBsRJR4ErvTbvvamq63fQzEEAhxoiaUobVxUybFeMRBYVVur3CX5mfJxRCAubFkjib2QodK1E8avTEJU,2pyxGzZeYzmd3r5ctTqhB73RX3QPCAeE7VRQYMRPD16nCv1t7Gj3U7NXhgWJkcWiwaSyYZP72sNtoDRbcUNTQ91 \
  --cluster devnet \
  --program-id 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX \
  --proof-verifier FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP \
  --core-collection F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u \
  --backend-url https://<public_backend_host> \
  --out /tmp/anky-live-devnet-signature-score-snapshot-20579.json
```

That command uses Helius only for public finalized transaction reads and posts only public `AnkySealed` and `AnkyVerified` metadata. It does not send `.anky` plaintext. Do not paste or print `HELIUS_API_KEY` or `ANKY_INDEXER_WRITE_SECRET`; keep them in the operator shell.

To include deterministic reward allocations, pass the total token supply in raw units:

```bash
HELIUS_API_KEY=<configured_in_shell> \
ANKY_SOLANA_CLUSTER=devnet \
node solana/scripts/indexer/ankySealIndexer.mjs \
  --backfill \
  --limit 100 \
  --token-supply <raw_token_supply> \
  --reward-bps 800 \
  --max-participants 3456 \
  --out sojourn9/devnet-score-snapshot.json
```

`--reward-bps 800` means 8%. Allocation uses integer raw units and deterministic remainder distribution by remainder, then wallet address. Snapshot output includes `summary.participantCap`, `summary.uncappedScoreRows`, and `summary.excludedByParticipantCap` so the cap is auditable.

Audit the generated snapshot before publishing or using it for a reward export:

```bash
node solana/scripts/indexer/auditScoreSnapshot.mjs \
  --snapshot sojourn9/devnet-score-snapshot.json \
  --require-allocation
```

The auditor reads only public snapshot JSON, rejects secret-shaped and direct `.anky` snapshot paths before reading, recomputes Score V1 from finalized public events, rejects private/plaintext-like fields and complete `.anky` plaintext-looking values under generic fields, verifies the configured proof verifier/protocol policy, verifies the 3,456 participant cap, and checks reward allocation sums when `--require-allocation` is set. If Helius omitted per-transaction commitment in a finalized backfill response, rerun with `--allow-inferred-finality` only when that condition is documented next to the published snapshot.

## Backend Metadata Upsert

To upsert finalized public event metadata into the mobile backend:

```bash
HELIUS_API_KEY=<configured_in_shell> \
ANKY_SOLANA_CLUSTER=devnet \
ANKY_CORE_COLLECTION=<devnet_core_collection> \
ANKY_PROOF_VERIFIER_AUTHORITY=<devnet_verifier_authority> \
ANKY_INDEXER_WRITE_SECRET=<configured_in_shell> \
node solana/scripts/indexer/ankySealIndexer.mjs \
  --backfill \
  --limit 100 \
  --backend-url http://localhost:3000 \
  --out sojourn9/devnet-score-snapshot.json
```

This posts only public `AnkySealed` and `AnkyVerified` metadata. It does not send `.anky` plaintext.
Indexer/operator posts include `utcDay` so the backend keeps day identity alongside wallet, Loom, hash, signature, slot, status, and public proof metadata.
The verified metadata route requires `x-anky-indexer-secret`; the indexer sets it from `ANKY_INDEXER_WRITE_SECRET`.
The indexer refuses `--backend-url` when `ANKY_INDEXER_WRITE_SECRET` is missing, so an operator cannot accidentally run an unauthenticated metadata upsert.
Scoring and backend posts use only events with real 64-byte Solana transaction signatures. Local parser fixtures with fake signatures are useful for parser debugging only; they do not contribute score and are skipped for backend writes.
The indexer skips verified metadata posts whose verifier or protocol version does not match the configured Sojourn 9 proof policy.
Any failed backend metadata post makes the indexer exit nonzero and includes the backend response body in the error.

After finalized indexer metadata has been posted, the backend can expose the wallet's public Score V1 view without reading any `.anky` plaintext:

```bash
curl "http://localhost:3000/api/mobile/seals/score?wallet=<wallet>"
```

The route counts only finalized `mobile_seal_receipts` and finalized matching `mobile_verified_seal_receipts` from the configured proof verifier authority and protocol version 1. It returns `uniqueSealDays`, `verifiedSealDays`, `streakBonus`, `score`, `sealedDays`, `verifiedDays`, and the exact formula. Treat this as a live/backend convenience view; final reward snapshots should still be regenerated from finalized Helius backfill output and published with the snapshot time.

## Helius Webhook Setup

The operator package exposes the indexing helpers as aliases from `solana/anky-seal-program`:

```bash
cd solana/anky-seal-program
npm run sojourn9:index -- --help
npm run sojourn9:audit-snapshot -- --help
npm run sojourn9:make-evidence -- --help
npm run sojourn9:webhook-manifest -- --help
```

Use an enhanced webhook that monitors the active Anky Seal Program. Helius uses `enhancedDevnet` for devnet webhook creation and `enhanced` for mainnet:

```text
webhookType: enhancedDevnet
accountAddresses: [ANKY_SEAL_PROGRAM_ID]
transactionTypes: ["ANY"]
```

Generate the public creation manifest without exposing an API key or creating a paid webhook:

```bash
node solana/scripts/indexer/heliusWebhookManifest.mjs \
  --cluster devnet \
  --webhook-url https://<public-backend-domain>/api/helius/anky-seal \
  --out /tmp/anky-helius-webhook.json
```

The manifest prints the Helius creation endpoint with `$HELIUS_API_KEY` as a placeholder and the JSON body to use from the Helius dashboard or a shell outside Codex. It selects the correct Helius webhook type for the chosen cluster and includes `authHeader: "Bearer $ANKY_INDEXER_WRITE_SECRET"` as a placeholder; replace that with the real bearer value outside Codex. The receiver must return HTTP 200 and dedupe by transaction signature before running the same parser/indexer path used by backfill.

For local smoke testing, use a public HTTPS tunnel URL such as ngrok. `--allow-http-localhost` exists only for local receiver tests; Helius cannot deliver to private localhost directly.

Official Helius webhook guidance was checked live on 2026-05-06 through the Helius docs MCP. The relevant operational points for this launch path are: enhanced webhooks post to a public `webhookURL`, Helius recommends deduping by processed transaction signature, failed deliveries are retried with exponential backoff for up to 24 hours, local testing needs a public tunnel such as ngrok, and webhooks with very high delivery failure rates may be auto-disabled until the receiver is fixed and the webhook is re-enabled from the Helius dashboard or API. The scripts here still do not create paid Helius resources or read `HELIUS_API_KEY`.
A filtered official docs check for `webhookType` on the same date confirmed the valid type set includes `enhanced`, `raw`, `discord`, `enhancedDevnet`, `rawDevnet`, and `discordDevnet`; use `enhancedDevnet` for the Sojourn 9 devnet receiver manifest.
A filtered official docs check for `authHeader` on the same date confirmed Helius supports `authHeader` as an optional Authorization header value sent with webhook requests; use `Bearer <ANKY_INDEXER_WRITE_SECRET>` outside Codex.

The backend receiver path is:

```text
POST /api/helius/anky-seal
```

It requires the indexer secret via Helius `Authorization` authHeader or the operator `x-anky-indexer-secret`, rejects private-looking `.anky` fields, rejects complete valid `.anky` plaintext string values under generic keys, and stores only public webhook JSON plus payload hash/signature summary in `mobile_helius_webhook_events`. Apply `migrations/021_mobile_helius_webhook_events.sql` and `migrations/022_mobile_helius_webhook_signature_dedupe.sql` before enabling this route in a deployed backend. Stored receipt rows can be exported directly into `ankySealIndexer.mjs` because it understands `payload_json` and `payloadJson`.
When Helius includes a valid 64-byte Solana transaction signature, the receiver dedupes by `(network, signature)` with a partial unique index. Payloads without a valid signature fall back to `(network, payload_hash)` dedupe. This keeps webhook retry deliveries from creating duplicate receipt rows while still allowing unsigned local smoke payloads.

Webhook delivery costs credits. Use finalized data for reward snapshots; webhook events are best used for live UI and then reconciled with a finalized backfill.

## Snapshot Readiness Gates

- Program ID is confirmed for the target cluster.
- Core collection is confirmed for the target cluster.
- Indexer output includes only finalized events for scoring.
- `auditScoreSnapshot.mjs --require-allocation` passes for the published snapshot.
- Any `summary.finalizedEventsInferredFromBackfillRequest` value is explained by a finalized Helius backfill request, not an arbitrary input file.
- Scored events have real 64-byte Solana transaction signatures.
- Failed transactions are excluded from scoring.
- Duplicate signatures are deduped.
- One wallet/day contributes at most one base sealed-day point.
- VerifiedSeal bonus only counts when the wallet has a matching sealed day.
- VerifiedSeal bonus only counts for the published verifier authority and protocol version 1.
- Reward allocation is capped at 3,456 participants unless a different published season rule is explicitly configured.
- Mainnet snapshot time, program ID, collection, verifier, protocol version, and export format are published before the season begins.
