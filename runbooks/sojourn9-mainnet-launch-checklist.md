# Sojourn 9 Mainnet Launch Checklist

This is a no-secret operator checklist for the eventual mainnet launch. It is intentionally separate from the devnet SP1 and Helius runbooks.

Do not run this before the devnet SP1 -> VerifiedSeal -> Helius score loop has landed end-to-end:

```text
write on phone -> hash exact .anky bytes -> seal_anky -> SP1 prove/verify -> record_verified_anky -> finalized Helius backfill -> Score V1 snapshot -> mobile verified state
```

This checklist does not authorize mainnet signing, deployment, webhook creation, or paid API changes. It is a gate list for the human operator. Do not paste `.env` values, keypair JSON, private keys, Helius API keys, backend write secrets, or wallet file contents into Codex or into committed files.

## Required Public Values

Mainnet public values to publish before the season begins:

- `mainnet program ID`: Anky Seal Program ID after confirmed mainnet deployment.
- `Metaplex Core collection`: Sojourn 9 Loom collection address on mainnet.
- `proof verifier authority`: verifier authority public key used by `record_verified_anky`.
- `protocol version`: `1` unless the Anchor program and SP1 receipt format are intentionally versioned.
- `SP1 vkey`: verification key printed by the current SP1 build.
- `snapshot time`: UTC timestamp and slot policy for Score V1 reward export.
- `Helius webhook ID`: public operational reference, not an API key.
- `backend URL`: public API base URL for mobile config and status checks.
- `public Solana RPC URL`: mobile-safe URL without embedded API-key secrets.
- `token supply and reward pool`: total token supply and the 8 percent reward pool used by the snapshot auditor.
- `export format`: fields included in the final score and allocation export.

Do not claim mainnet deployment until the read-only checks and signed transactions have real signatures. Use Orb links for public transaction/account references:

```text
https://orbmarkets.io/address/<program_or_account>
https://orbmarkets.io/tx/<signature>
```

## Preflight Gates

Run these only after the devnet loop above is complete.

1. Confirm the repo still passes the no-secret readiness gate:

```bash
node solana/scripts/sojourn9/launchReadinessGate.mjs
```

Expected before final launch: `localReady: true` and `launchReady: false` until every human gate below is completed and documented. This gate does not read secret files.

2. Confirm mainnet public account configuration with explicit read-only approval:

```bash
cd solana/anky-seal-program
npm run check-config -- \
  --cluster mainnet-beta \
  --program-id <mainnet_program_id> \
  --core-collection <mainnet_core_collection> \
  --allow-mainnet-read
```

If a real mainnet Loom asset exists for an operator wallet, add:

```bash
  --loom-asset <mainnet_core_loom_asset> \
  --loom-owner <expected_wallet_owner>
```

This is read-only. Stop if the program is not executable, the collection is not owned by Metaplex Core, the Core account layout is unexpected, the Loom owner is wrong, or any value is still a devnet placeholder.

3. Confirm backend configuration without exposing secrets:

```text
ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true
ANKY_SOLANA_RPC_URL=<private_mainnet_rpc_url_configured_only_on_backend>
ANKY_PUBLIC_SOLANA_RPC_URL=<mobile_safe_public_rpc_url>
EXPO_PUBLIC_SOLANA_RPC_URL=<mobile_safe_public_rpc_url>
EXPO_PUBLIC_SOLANA_CLUSTER=mainnet-beta
EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID=<mainnet_program_id>
EXPO_PUBLIC_ANKY_CORE_COLLECTION=<mainnet_core_collection>
EXPO_PUBLIC_ANKY_PROOF_VERIFIER_AUTHORITY=<proof_verifier_authority>
```

Do not expose a private Helius API-key RPC URL through public mobile config.

4. Generate a mainnet Helius webhook manifest without creating the webhook:

```bash
cd solana/anky-seal-program
npm run sojourn9:webhook-manifest -- \
  --cluster mainnet-beta \
  --program-id <mainnet_program_id> \
  --webhook-url <public_backend_webhook_url>
```

Create or update the real webhook only from an operator shell or Helius dashboard with `HELIUS_API_KEY` configured outside Codex. Use `enhanced` for mainnet. The backend webhook route must require the indexer secret and must reject private `.anky` payloads.

5. Prepare a finalized Helius backfill and reward snapshot plan:

```bash
cd solana/anky-seal-program
HELIUS_API_KEY=<configured_in_shell> \
npm run sojourn9:index -- \
  --backfill \
  --cluster mainnet-beta \
  --program-id <mainnet_program_id> \
  --proof-verifier <proof_verifier_authority> \
  --token-supply <token_supply_raw_units> \
  --out sojourn9/mainnet-score-snapshot.json

npm run sojourn9:audit-snapshot -- \
  --snapshot sojourn9/mainnet-score-snapshot.json \
  --proof-verifier <proof_verifier_authority> \
  --reward-bps 800 \
  --require-allocation
```

Final reward snapshots must use finalized data. If Helius omits per-transaction commitment in a finalized backfill response, document that condition next to the snapshot before using `--allow-inferred-finality`.

6. Audit the public launch evidence file before publishing or using it for claims:

```bash
cd solana/anky-seal-program
npm --silent run sojourn9:audit-evidence -- --print-template > sojourn9/public-launch-evidence.json
# Fill every placeholder with real public values and remove `templateOnly`.
npm run sojourn9:audit-evidence -- \
  --evidence sojourn9/public-launch-evidence.json
```

The evidence file must contain public values only: program ID, Core collection, verifier authority, SP1 vkey, snapshot time, backend URL, Helius webhook ID/type, finalized Score V1 audit markers, and real Orb links for the landed seal and verified transactions. Do not include `.env` paths, keypair paths, API keys, backend write secrets, `.anky` plaintext, witness bytes, or private proof inputs. The printed template intentionally does not pass final audit until `templateOnly` is removed and every placeholder is replaced with real public evidence.

## Signing Gates

Mainnet signing is a separate human approval step. Stop before any signed transaction unless all previous gates are documented.

- `seal_anky` mainnet sending is not handled by the devnet helper; use a dedicated launch procedure after final program and collection values are confirmed.
- `record_verified_anky` refuses mainnet unless `ANKY_ALLOW_MAINNET_RECORD_VERIFIED=true` is set.
- If `ANKY_ALLOW_MAINNET_RECORD_VERIFIED=true` is ever set, the operator must still use Helius Sender policy: live priority fee estimate, compute unit price, Sender endpoint, `skipPreflight: true`, and the required Sender tip.
- The on-chain verified badge remains verifier-authority-attested after off-chain SP1 verification. Direct on-chain SP1/Groth16 verification is future hardening.

## Public Claim Rules

Allowed wording after devnet E2E but before mainnet signatures:

```text
ZK-enabled proof-of-practice with off-chain SP1 verification and verifier-authority-attested on-chain receipts.
```

Allowed wording after mainnet deployment is actually confirmed:

```text
Mainnet program: <mainnet_program_id>
Mainnet Core collection: <mainnet_core_collection>
Verifier authority: <proof_verifier_authority>
Snapshot time: <utc_timestamp>
```

Forbidden wording unless direct on-chain SP1 verification is implemented and tested:

- fully trustless ZK on Solana
- Solana verifies the SP1 proof directly today
- anonymous writing
- hash encrypts the writing

The hash is a commitment to exact `.anky` UTF-8 bytes. It does not encrypt the writing.
