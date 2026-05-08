# Anky Mainnet Readiness Gate

Updated: 2026-05-07

This is a no-deploy, no-signing, no-secret readiness gate for moving the Anky proof-of-practice system to a controlled mainnet launch before App Store submission.

Main question:

```text
FAIL
```

Anky cannot safely move the current proof-of-practice worktree to mainnet yet. The local proof/indexing surface is real, but mainnet values, live Core parser confidence, production backend proof operation, Helius configuration, mobile production config, and App Store crypto posture are not launch-ready.

## Executive Recommendation

| Decision | Answer | Status | Reason |
|---|---:|---|---|
| MAINNET READY | no | FAIL | Mainnet deployment, collection, verifier custody, Helius, backend proof worker, and fresh devnet E2E are not proven. |
| APP STORE READY AFTER MAINNET | no | FAIL | Mobile production config is still devnet/mock, Loom minting/wallet/proof-points copy needs App Store strategy, and `$ANKY` must stay out of iOS. |
| `$ANKY` DISTRIBUTION READY | no | FAIL | Scoring math exists, but token supply, custody, snapshot, dispute, legal/tax, and claim/transfer mechanics are not proven. |

## Required Mainnet Values

| Value | Current Repo Evidence | Gate |
|---|---|---|
| Seal program ID | `4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX` is declared in Anchor source and `Anchor.toml`, including under `[programs.mainnet]`. Mainnet deployment is not proven. | NEEDS HUMAN |
| Core program ID | `CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d` is configured as Metaplex Core. | PASS |
| Core collection | Repo defaults to devnet-looking `F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u`; `mainnetConfig.json` is missing and the example has placeholders. | FAIL |
| Verifier authority | `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP` is configured. Custody and mainnet use are not proven. | NEEDS HUMAN |
| RPC URL strategy | Backend must use private mainnet RPC; mobile must use public/mobile-safe RPC. Current production EAS config uses devnet public RPC. | FAIL |
| Helius webhook receiver | Backend route `/api/helius/anky-seal` exists and manifest tooling exists. No webhook ID/account state is confirmed. | NEEDS HUMAN |
| Backend URL | Mobile points at `https://anky.app`, but target migrations, proof worker, webhook secret, and mainnet env are not proven. | NEEDS HUMAN |
| Mobile env vars | Required names exist. Production EAS profile currently sets `EXPO_PUBLIC_SOLANA_CLUSTER=devnet` and `EXPO_PUBLIC_SOLANA_SEAL_ADAPTER=mock`. | FAIL |
| RevenueCat/IAP state | Code and docs define RevenueCat `CREDITS` products and API key env names. App Store Connect/RevenueCat production setup is external. | NEEDS HUMAN |

Required mobile public env names before a production mainnet build:

```text
EXPO_PUBLIC_ANKY_API_URL
EXPO_PUBLIC_APP_URL
EXPO_PUBLIC_SOLANA_CLUSTER=mainnet-beta
EXPO_PUBLIC_SOLANA_SEAL_ADAPTER=program
EXPO_PUBLIC_SOLANA_RPC_URL=<mobile-safe public RPC URL>
EXPO_PUBLIC_ANKY_CORE_PROGRAM_ID
EXPO_PUBLIC_ANKY_CORE_COLLECTION
EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID
EXPO_PUBLIC_ANKY_PROOF_VERIFIER_AUTHORITY
EXPO_PUBLIC_REVENUECAT_IOS_API_KEY
EXPO_PUBLIC_REVENUECAT_ANDROID_API_KEY
```

Backend-only env names that must not be exposed through mobile config:

```text
ANKY_SOLANA_RPC_URL
ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF
ANKY_MOBILE_PROVER_ENABLED
ANKY_PROVER_VERIFIER_KEYPAIR_PATH
ANKY_PROVER_WORK_DIR
ANKY_PROVER_PROTOC
ANKY_INDEXER_WRITE_SECRET
ANKY_VERIFIED_SEAL_RECORD_SECRET
HELIUS_API_KEY
ANKY_REVENUECAT_PROJECT_ID
ANKY_REVENUECAT_SECRET_KEY
```

## Gate Table

| Gate | Status | Evidence | Required To Pass |
|---|---|---|---|
| Solana program | NEEDS HUMAN | Source exists; mainnet deployment/signature not proven. | Publish verified mainnet program account and deployment signature. |
| Anchor tests | NEEDS HUMAN | Prior closure doc records Anchor tests passed; this pass did not rerun them. Live Core integration test skips without env. | Rerun Rust/Anchor tests and owned real Core Loom integration. |
| Core parser | NEEDS HUMAN | Parser is hand-rolled; fixtures exist. | Run read-only config check and live owned Loom seal gate before mainnet. |
| Loom collection | FAIL | Mainnet config is missing; only placeholder example exists. | Create/verify mainnet Core collection and authority. |
| Loom mint path | NEEDS HUMAN | Mobile/backend mint path exists; mainnet Loom mints require explicit backend enablement and collection authority. | Prove mainnet mint prep and custody in operator environment. |
| SP1 proof generation | PASS | SP1 source and wrapper exist; prior closure records local proof pass. | Fresh proof rerun from current worktree before launch. |
| VerifiedSeal | NEEDS HUMAN | Program supports `record_verified_anky`; current badge is verifier-authority-attested. | Fresh devnet HashSeal -> SP1 verify -> VerifiedSeal -> index evidence. |
| Backend proof worker | FAIL | Backend proof requests explicitly return disabled on `mainnet-beta`. | Decide mainnet proof posture: operator CLI only, or enable hardened backend worker after legal/security review. |
| Migrations | NEEDS HUMAN | Migrations 019-025 exist and store public metadata only, including sponsorship ledger migration 025. | Apply and verify on target database. |
| Helius webhook | NEEDS HUMAN | Manifest tooling and receiver exist; no live webhook confirmed. | Human creates/validates enhanced mainnet webhook with secret auth. |
| Indexer backfill | NEEDS HUMAN | Indexer requests finalized commitment and has tests/runbooks. | Credentialed finalized mainnet backfill with audited output. |
| Score snapshot | NEEDS HUMAN | Score V1 and allocation math exist. No mainnet snapshot/token supply/custody. | Finalized snapshot, audit, token supply, custody, dispute policy. |
| Mobile production config | FAIL | `apps/anky-mobile/eas.json` production profile is devnet + mock. | Switch to mainnet/program only after proof system is verified. |
| App Store crypto compliance risk | NEEDS HUMAN | Wallet, Loom mint, proof points, and optional sealing are in-app. Apple policy is sensitive around wallets, NFTs, and task-based currency. | Rewrite/hide risky copy and get legal/App Review strategy. |
| `$ANKY` distribution visibility | FAIL | Docs correctly say distribution is not public-ready. | Keep `$ANKY` distribution outside iOS and outside launch claims. |

## Commands Run

For this mainnet-readiness gate pass, only safe local/read-only or no-secret inspection commands were run. No mainnet mutation, signing, deployment, keypair read, `.env` read, Helius webhook creation, or App Store submission was performed.

Later sponsored-transaction work accidentally deployed to devnet through the old `solana/anky-seal-program` `npm test` script. That incident is documented in `docs/anky-system/ANKY_SPONSORED_TRANSACTIONS_STATUS.md` and `docs/anky-system/ANKY_SPONSORED_TRANSACTIONS_AUDIT.md`; it is not launch evidence and it does not change this document's mainnet status.

```bash
git rev-parse --show-toplevel
git status --short
rg --files /home/kithkui/anky/docs/anky-system
test -f docs/anky-system/ANKY_MAINNET_GO_NO_GO.md && printf present || printf missing
test -f docs/anky-system/ANKY_MAINNET_READINESS_GATE.md && printf present || printf missing
sed -n '1,240p' docs/anky-system/ANKY_3_DAY_CLOSURE_STATUS.md
sed -n '1,260p' docs/anky-system/ANKY_3_DAY_LAUNCH_GAP_AUDIT.md
sed -n '1,140p' docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md
sed -n '1,240p' docs/anky-system/ANKY_RULES_LEDGER.md
sed -n '1,560p' docs/anky-system/ANKY_TECHNICAL_SOURCE_OF_TRUTH.md
sed -n '1,180p' docs/anky-system/ANKY_REPO_EVIDENCE_INDEX.md
sed -n '1,220p' solana/anky-seal-program/Anchor.toml
sed -n '1,260p' solana/scripts/sojourn9/launchReadinessGate.mjs
npm run sojourn9:readiness
node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs
node solana/scripts/sojourn9/prepareCurrentDayProof.mjs --help
node solana/scripts/sojourn9/proveAndRecordVerified.mjs --help
node solana/anky-seal-program/scripts/sealAnky.mjs --help
node solana/anky-seal-program/scripts/recordVerifiedAnky.mjs --help
```

External policy/doc checks:

- Apple App Review Guidelines, current official page, sections 3.1.1 and 3.1.5: https://developer.apple.com/app-store/review/guidelines/
- Helius webhook guide fetched through the Helius MCP docs tool; no webhook/account mutation was performed.

Command results that matter:

| Command | Result | Meaning |
|---|---|---|
| `npm run sojourn9:readiness` | FAIL for local readiness: `localReady: false`, `launchReady: false`. | Current worktree is not even locally green. Missing readiness marker: `RevealScreen.tsx` no longer contains expected `verified +2` copy. |
| `node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs` | FAIL: 3 pass, 1 fail. | Test expects `localReady: true`; actual current gate returns false. |
| Required source docs | PASS | All required present docs were read. `ANKY_MAINNET_GO_NO_GO.md` is missing. |
| Helius docs lookup | PASS | Official docs confirm webhook retries, 200 response requirement, dedupe-by-signature guidance, and auto-disable behavior. |

## Commands JP Must Run Manually

These commands require human-owned signers, external environments, mainnet values, paid API credentials, or store access. Codex must not run them.

| Step | Exact Command | Required Signer/Key/Env | Expected Output | Stop Condition |
|---|---|---|---|---|
| Fix and rerun local readiness | `cd /home/kithkui/anky/solana/anky-seal-program && npm run sojourn9:readiness` | No signer. | JSON with `localReady: true`, `launchReady: false` until human gates are filled. | Stop if any local check is `missing`, `unmatched`, `failed`, or `forbidden_match`. |
| Rerun readiness test | `cd /home/kithkui/anky && node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs` | No signer. | 4 passing tests. | Stop if first test still fails on `localReady`. |
| Devnet public config check | `cd /home/kithkui/anky/solana/anky-seal-program && npm run check-config -- --cluster devnet --program-id 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX --core-collection F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u --loom-asset <owned_devnet_core_loom_asset> --loom-owner <writer_wallet>` | Public devnet Loom and owner. No signer. | JSON with every check true and `ok: true`. | Stop if collection/account ownership/layout/owner mismatches. |
| Devnet real Core seal integration | `cd /home/kithkui/anky/solana/anky-seal-program && ANCHOR_PROVIDER_URL=https://api.devnet.solana.com ANCHOR_WALLET=<provider_wallet_keypair_path> ANKY_CORE_INTEGRATION_LOOM_ASSET=<owned_devnet_core_loom_asset> ANKY_CORE_INTEGRATION_COLLECTION=F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u ANKY_ALLOW_LIVE_ANCHOR_TEST=true npm run test:anchor:live -- --skip-local-validator --skip-deploy` | Writer/provider keypair that owns the Loom; devnet SOL; explicit live-test approval. | One current-day `seal_anky` succeeds and `LoomState` updates. | Stop if test skips, asks for mainnet, prints key material, deploys unexpectedly, or owner/collection fails. |
| Devnet proof handoff | `cd /home/kithkui/anky && node solana/scripts/sojourn9/prepareCurrentDayProof.mjs --writer <writer_wallet> --loom-asset <owned_devnet_core_loom_asset>` | Public writer and Loom; SP1/protoc installed. | Handoff manifest outside repo, SP1 proof verified locally, public receipt metadata only. | Stop if witness would be written inside repo or proof verification fails. |
| Devnet seal send | `cd /home/kithkui/anky/solana/anky-seal-program && ANKY_SEALER_KEYPAIR_PATH=<writer_keypair_path> npm run seal -- --loom-asset <owned_devnet_core_loom_asset> --session-hash <sha256_hex> --utc-day <current_utc_day> --cluster devnet --check-chain --send` | Writer keypair that owns Loom; devnet SOL. | Landed `seal_anky` signature and matching HashSeal. | Stop if UTC day is stale, Loom owner mismatches, or command attempts mainnet. |
| Devnet VerifiedSeal send | `cd /home/kithkui/anky/solana/anky-seal-program && npm run record-verified -- --receipt <public_receipt_json> --writer <writer_wallet> --cluster devnet --check-chain --sp1-proof-verified --send --keypair <verifier_authority_keypair_path>` | Verifier authority keypair; devnet SOL; locally verified SP1 proof. | Landed `record_verified_anky` signature and matching VerifiedSeal. | Stop if `--sp1-proof-verified` is not justified by local proof verification or verifier custody is unclear. |
| Backend migrations | `DATABASE_URL=<target_database_url> sqlx migrate run` | Target DB URL held by operator. | Migrations 019-025 applied in order. | Stop if target is wrong, migration history diverges, or DB contains plaintext proof/sponsorship fields. |
| Backend proof worker env | `ANKY_MOBILE_PROVER_ENABLED=true ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true ANKY_PROVER_VERIFIER_KEYPAIR_PATH=<verifier_keypair_path> ANKY_PROVER_WORK_DIR=<outside_repo_work_dir> ANKY_PROVER_PROTOC=<protoc_path> cargo test` | Backend env, verifier keypair path, outside-repo workdir. | Tests pass and workdir/witness cleanup policy is confirmed. | Stop if workdir is inside repo or mainnet proof endpoint is being enabled without explicit strategy. |
| Helius webhook manifest | `cd /home/kithkui/anky/solana/anky-seal-program && npm run sojourn9:webhook-manifest -- --cluster mainnet-beta --program-id <mainnet_program_id> --webhook-url https://<backend-domain>/api/helius/anky-seal --out /tmp/anky-helius-webhook-mainnet.json` | Public mainnet program ID and backend domain. | Dry-run JSON only; no Helius call. | Stop if URL is not HTTPS, has credentials/query strings, or account address is not only the seal program. |
| Create Helius webhook | `curl -X POST "https://api-mainnet.helius-rpc.com/v0/webhooks?api-key=$HELIUS_API_KEY" -H "Content-Type: application/json" --data @/tmp/anky-helius-webhook-mainnet.json` | `HELIUS_API_KEY`, indexer write secret configured outside Codex. | Helius returns webhook ID/type/account addresses. | Stop if API key would be printed, receiver fails 200, or webhook monitors wrong accounts. |
| Mainnet read-only config | `cd /home/kithkui/anky/solana/anky-seal-program && npm run check-config -- --cluster mainnet-beta --program-id <mainnet_program_id> --core-collection <mainnet_core_collection> --allow-mainnet-read` | Final public mainnet values. No signer. | JSON with `ok: true`, executable program, Core-owned collection. | Stop if any placeholder/devnet value remains. |
| Mainnet deployment | `cd /home/kithkui/anky/solana/anky-seal-program && ANCHOR_PROVIDER_URL=<mainnet_rpc_url> ANCHOR_WALLET=<deployer_keypair_path> anchor deploy --provider.cluster mainnet-beta` | Deployer/upgrade authority, mainnet SOL, final deployment approval. | Mainnet program ID and deployment signature. | Stop unless devnet E2E, Core parser, backend, Helius, and App Store strategy are already green. |
| Final score snapshot | `cd /home/kithkui/anky/solana/anky-seal-program && HELIUS_API_KEY=<configured_in_shell> npm run sojourn9:index -- --backfill --cluster mainnet-beta --program-id <mainnet_program_id> --proof-verifier <proof_verifier_authority> --token-supply <token_supply_raw_units> --out sojourn9/mainnet-score-snapshot.json && npm run sojourn9:audit-snapshot -- --snapshot sojourn9/mainnet-score-snapshot.json --proof-verifier <proof_verifier_authority> --reward-bps 800 --require-allocation` | Helius key, token supply, finalized data policy. | Audited finalized Score V1 snapshot. | Stop if finality is inferred without documentation, supply/custody is unverified, or allocation cannot be reproduced. |

## App Store Risk Review

Apple policy checked on the official App Review Guidelines page. The relevant risk points are:

- Guideline 3.1.1 says app features unlocked by cryptocurrencies or crypto wallets are not allowed outside IAP; NFT minting/listing/transferring services may use IAP, and viewing owned NFTs is allowed only if NFT ownership does not unlock app features.
- Guideline 3.1.5 says wallet apps may facilitate storage only if offered by an organization, exchange/transmission has licensing constraints, and cryptocurrency apps may not offer currency for completing tasks.

Current user-facing risk findings:

| Question | Finding | Status |
|---|---|---|
| User-facing token/wallet/reward claims | Mobile does not appear to expose `$ANKY` token distribution. It does expose wallet, Loom minting, Solana sealing, indexed score, `proof +2`, and `to earn the proof points`. Public web/docs include `$ANKY` and reward-distribution language but are now fenced. | NEEDS HUMAN |
| Does the app offer cryptocurrency for completing tasks? | Not explicitly in mobile. However Score V1 and planned `$ANKY` distribution could make proof points look like task rewards if any token/reward copy leaks into iOS. | NEEDS HUMAN |
| Does NFT/token ownership unlock app features? | Writing works without a Loom, but sealing/proof score requires a Loom access artifact. That can be read as NFT ownership enabling proof functionality. | NEEDS HUMAN |
| Is wallet functionality optional or core? | Wallet is optional for writing/reflection, core for Loom minting/sealing/proof. Do not submit as a wallet app posture unless Apple developer enrollment, review notes, and compliance are ready. | NEEDS HUMAN |
| RevenueCat/IAP | Credits use RevenueCat consumables, which is the right direction. Store product setup is external and unverified. | NEEDS HUMAN |

Recommended iOS posture before App Store submission:

- Hide `$ANKY`, airdrop, reward distribution, claim, token supply, and 8 percent allocation from the iOS app and App Store metadata.
- Replace `to earn the proof points` with neutral copy such as `to attach the optional proof receipt`.
- Consider hiding in-app Loom minting for App Review, or document it as optional proof infrastructure with no feature/token unlock and no external purchase CTA.
- Keep the first-run experience as writing-first: `Anky is an 8-minute writing ritual with optional proof-of-practice sealing.`
- Keep wallet creation/connect behind an optional proof path, not as the app's primary identity or reviewed business model.
- Do not show `$ANKY` links from the app until explicit legal/App Review strategy exists.

## Final Operator Sequence

1. Devnet rerun: fix the current `sojourn9:readiness` local failure, rerun readiness tests, rerun devnet Core check, seal, SP1 prove/verify, VerifiedSeal, backend record, Helius backfill, score snapshot, and mobile proof-state display.
2. Mainnet deploy/config: only after devnet E2E is green, create/verify mainnet Core collection, deploy/verify seal program, publish verifier authority custody policy, and run mainnet read-only config checks.
3. Backend migration/config: apply migrations 019-025 to target DB, configure private RPC, prover workdir outside repo, verifier custody, webhook/indexer secrets, sponsorship policy, and `ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true`.
4. Helius config: generate manifest, human creates enhanced mainnet webhook, verify receiver 200s, dedupe by signature, and run finalized backfill.
5. Mobile env switch: set production EAS to `mainnet-beta` and `program`, replace collection/program/verifier/RPC with final values, keep private RPC/API keys out of public env.
6. Production build: run mobile typecheck/tests, build EAS production, verify RevenueCat products in sandbox.
7. TestFlight verification: phone flow must show write -> local hash -> optional seal -> optional proof -> verified/indexed state without token reward claims.
8. App Store submission: submit as an 8-minute writing ritual with optional proof-of-practice sealing; keep `$ANKY` distribution outside iOS unless explicit legal/App Review approval exists.

## Final Call

Controlled mainnet proof launch is allowed by policy, but this worktree is not ready to do it today.

```text
FAIL
```

## Local Readiness Fix Update

Updated: 2026-05-07

The local readiness failure was reproduced and fixed without weakening the mainnet gate.

Root cause:

- `RevealScreen.tsx` still separated hash seal, proving, syncing, failed, unavailable, and verified proof states.
- The verified success label had drifted from the readiness marker `verified +2` to `proof +2`.
- The gate correctly kept local readiness false because the reveal UI marker no longer made the verified receipt distinction explicit.

Fix:

- Updated `apps/anky-mobile/src/screens/RevealScreen.tsx` so the verified state now shows `sealed +1 · verified +2 · 3 pts`.
- Added `docs/anky-system/ANKY_LOCAL_READINESS_FIX.md`.

Post-fix command results:

| Command | Result |
|---|---|
| `cd solana/anky-seal-program && npm run sojourn9:readiness` | PASS: `localReady: true`, `launchReady: false`. |
| `node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs` | PASS: 4 tests. |
| `cd solana/anky-seal-program && npm run sojourn9:test` | PASS: 161 tests. |
| `cd apps/anky-mobile && npm run typecheck` | PASS. |

What did not change:

- Mainnet readiness is still false.
- In the local readiness fix pass, no deployment, signing, keypair read, secret read, mainnet transaction, paid Helius mutation, or App Store action was performed.
- Later sponsored-transaction work accidentally deployed to devnet through the old `npm test` script. Do not treat that deployment as launch evidence.
- Fresh devnet `HashSeal -> SP1 verify -> VerifiedSeal -> index`, verifier custody, target backend migration, Helius production indexing, real Core Loom integration, and final mainnet values remain manual gates.

## Sponsored-Payer Devnet Validation Update

Updated: 2026-05-08

Controlled devnet validation for the sponsored-payer `seal_anky` account model has been run after explicit human approval.

Evidence:

- Validation doc: `docs/anky-system/ANKY_SPONSORED_TRANSACTIONS_DEVNET_VALIDATION.md`
- Evidence bundle: `runbooks/devnet-20581-sponsored-payer-validation-evidence.json`
- Index snapshot: `runbooks/devnet-20581-sponsored-payer-score-snapshot.json`

Result:

| Gate | Status | Evidence |
| --- | --- | --- |
| Devnet deploy after payer split | PASS | Program `4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX`, signature `2UEhpCCu2tGAxzY2c2gZEXkDShsVPkwTAujLSrSPpWaeygeacjLCPjDVdFWVXVUenfbZq7Rt8DSHQejM1BZfBFWA` |
| Writer-pays seal | PASS | Writer/payer `6yS2xjgYeBn6HSeMm5zwyYCWQhwFGfw6Sf9fvb8f1NX`, signature `5XTh9SmXvkNcFD1ZXyDsLf6aFjqL9hCubWRKp3kBw2osyijg9U718ezutsCbY9EGdbbLw8L1kH4H1FXjXkEukRNE` |
| Sponsor-pays seal | PASS | Writer `HK52v7KLU7TxPM3RnTYHjEmeg5kgERk1bpzHUmmuBkmR`, payer `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP`, signature `4jPvBKCt81SgQxA2vPBkgeVPCU4xsAkg1RN17W1WaHrYzgnDCDjWWC1aokai875hcCBqdqZVEWPNaCAzUUtRtSJJ` |
| Sponsor cannot seal alone | PASS | Strict serialization failed with missing writer signature; bypass-preflight raw signature remained not found |
| Wrong Loom owner fails | PASS | Helper rejected `Core Loom asset owner does not match writer` |
| Indexer after payer split | PASS | Known-signature finalized indexer parsed both new seal signatures and produced two score rows |
| Readiness state | PASS | `localReady: true`, `launchReady: false` |

The previous UTC day 20580 evidence is stale for the sponsored-payer account model and has been marked as such in its summary and JSON artifact.

Mainnet remains blocked. This validation did not rerun SP1 -> VerifiedSeal, did not apply target backend migrations, did not validate live mobile, did not create a Helius webhook, and did not touch mainnet.
