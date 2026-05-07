# Anky 3-Day Closure Status

Updated: 2026-05-07

This is the live status log for the launch-closure pass. It tracks what is being closed in this pass, what stays explicitly out of scope, what requires human or external verification, and which safe commands were planned or run.

## Initial Read

Required source files read first:

- `docs/anky-system/ANKY_3_DAY_LAUNCH_GAP_AUDIT.md`
- `docs/anky-system/ANKY_RULES_LEDGER.md`
- `docs/anky-system/ANKY_TECHNICAL_SOURCE_OF_TRUTH.md`
- `docs/anky-system/ANKY_REPO_EVIDENCE_INDEX.md`

Repo root detected from nested Codex working directory:

```text
/home/kithkui/anky
```

## Blockers Found

1. Mainnet truth is still unknown. The repo does not prove active mainnet deployment, final mainnet Core collection, final verifier authority custody, production Helius webhook, or audited mainnet score snapshot.
2. Core parser risk remains. The active Anchor program hand-parses Metaplex Core account data and still needs a live Core gate before mainnet confidence.
3. SP1 to VerifiedSeal is historically devnet-proven but not freshly rerun from this dirty worktree in this pass.
4. Backend proof worker is configured but operationally unverified. A real proof job, witness cleanup, target migrations, and public-metadata-only persistence need environment validation.
5. Helius webhook and finalized backfill are implemented in code/runbooks but not externally confirmed active.
6. `$ANKY` distribution is not public-ready. Scoring/allocation math exists, but token supply, reward custody, snapshot time, dispute policy, eligibility source, and claim/transfer mechanics are not proven.
7. Sojourn language remains conceptually conflicted where some docs/code say 8 kingdoms x 12 days while launch canon requires 12 regions x 8 days with the 8 kingdoms as an inner symbolic cycle.
8. Legacy plaintext and old architecture claims still need fencing so public copy does not imply the whole repo has the same privacy model as the current mobile proof path.
9. Great Slumber is doctrine/planned only. The repo does not show current 21-day enforcement.

## What This Pass Will Close

- Canonicalize public Sojourn wording as 96 days, 12 regions of 8 days, with 8 kingdoms/chakras/colors as the inner symbolic cycle.
- Fence or annotate stale public-facing claims that imply direct on-chain SP1 verification, global plaintext-free storage, active mainnet deployment, current Bubblegum/cNFT Sojourn launch architecture, or ready `$ANKY` claims.
- Produce or update a launch-safe public claim set using only evidence-backed language.
- Rerun safe local verification gates that do not require secrets, private keys, mainnet funds, paid API changes, or deployment.
- Update closure status with command results and residual launch blockers.

## What This Pass Will Not Close

- No mainnet deployment.
- No mainnet transaction.
- No key rotation or keypair inspection.
- No paid Helius account mutation.
- No production token, custody, legal, tax, dispute, or claim process.
- No direct on-chain SP1/Groth16 verifier implementation.
- No broad product rewrite or new mythology.
- No claim that legacy backend/web paths are privacy-equivalent to the mobile proof path.

## Human Or External Verification Needed

- Mainnet program account and deployment signature.
- Mainnet Metaplex Core collection account and authority proof.
- Verifier authority custody policy and operational signer ownership.
- Live Core parser check against real devnet and then mainnet Core accounts.
- Backend target environment migrations and proof worker env vars.
- Helius webhook account state, API key configuration, and production receiver secret.
- Token mint, supply, reward pool custody, snapshot time, dispute window, and claim/transfer policy for any `$ANKY` distribution.
- App Store / Google Play status and RevenueCat product setup.

## Planned Safe Commands

These commands are planned only if dependencies are present and they do not request secrets, private keys, mainnet funds, or paid API access:

```bash
git status --short
rg -n "<launch claim patterns>" docs static templates README.md runbooks sojourn9 apps/anky-mobile/src src solana
cd solana/anky-seal-program && npm run sojourn9:privacy
cd solana/anky-seal-program && npm run sojourn9:test
cd solana/anky-seal-program && npm run build
cd solana/anky-seal-program && cargo test --manifest-path Cargo.toml --package anky_seal_program
cd solana/anky-zk-proof && cargo test
cd solana/anky-zk-proof && cargo run -- --file fixtures/full.anky --writer <public-test-wallet>
node --test solana/scripts/indexer/ankySealIndexer.test.mjs
node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs
node solana/scripts/sojourn9/launchReadinessGate.mjs
```

Commands that require operator-provided devnet wallets, backend URLs, Helius keys, verifier keypairs, or mainnet values will be documented as manual and not run in this pass unless explicitly authorized.

## Closure Pass Update

Updated: 2026-05-07 15:59 UTC

### Closed In This Pass

- Added the launch-safe public claim set at `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md`.
- Scoped the root `README.md` as broad historical repo context, not the Sojourn 9 launch claim source.
- Scoped `docs/local-first-protocol.md` to the current mobile proof/seal/indexing path and removed global plaintext-free wording.
- Canonicalized public Sojourn language in launch docs as `12 regions x 8 days = 96 days`, with 8 kingdoms/chakras/colors as the inner symbolic cycle.
- Fenced historical Bubblegum/cNFT Sojourn scaffold docs as noncanonical for the active Metaplex Core Loom launch path.
- Fenced older x402/Base documentation and help copy as legacy web/payment surfaces, not current mobile launch claims.
- Added an explicit `$ANKY` page note that no Sojourn 9 reward claim, 8 percent distribution, snapshot, custody plan, or claim/transfer process is public-ready.

### Commands Run

```bash
git status --short
rg --files docs/anky-system
rg -n "<launch claim patterns>" README.md HACKATHON.md docs static templates runbooks sojourn9
npm run sojourn9:privacy
npm run sojourn9:readiness
cd solana/anky-zk-proof && cargo test
cd solana/anky-zk-proof && cargo run -- --file fixtures/full.anky --writer 8qznzSWh7vzM2G1JrDUhEYrPpZK2ehDUmydQiFpU8Q19
cd solana/anky-seal-program && npm run sojourn9:test
cd solana/anky-seal-program && npm run check-config -- --cluster devnet
cd solana/anky-seal-program && cargo test --manifest-path Cargo.toml --package anky_seal_program
cd solana/anky-seal-program && npm run build
cd solana/anky-seal-program && npm run typecheck
cargo check
cd apps/anky-mobile && npm run typecheck
```

### Passed

- `npm run sojourn9:privacy`: passed with `ok: true`, 25 files checked, 0 issues.
- `npm run sojourn9:readiness`: passed local gate with `localReady: true`; correctly kept `launchReady: false`.
- `solana/anky-zk-proof cargo test`: passed 7 tests.
- `solana/anky-zk-proof cargo run` fixture receipt: produced a public valid receipt with `valid: true`, `duration_ok: true`, session hash `c4d8d04ee62d4c6080df750ee5a742b71bcf74d8f4e29f84a4966b1eef26d824`, proof hash `246086712f4467279b8dc6877a0899981a98ca1c55633f69b74208ed6f81a8d2`, and no plaintext output.
- `npm run sojourn9:test`: passed 161 tests.
- `npm run check-config -- --cluster devnet`: passed read-only public devnet config checks. Confirmed executable devnet program `4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX`, Core collection `F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u`, Core program `CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d`, and proof verifier `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP`.
- `cargo test --manifest-path Cargo.toml --package anky_seal_program`: passed 10 tests with one Anchor deprecation warning.
- `npm run build`: passed with one Anchor deprecation warning.
- `npm run typecheck` in `solana/anky-seal-program`: passed.
- Root `cargo check`: passed with warnings only.
- Mobile `npm run typecheck`: passed.

### Still Manual Or Externally Gated

- Fresh same-day devnet `HashSeal -> VerifiedSeal` evidence bundle remains human/operator-gated because it requires a writer-owned Core Loom and signing path.
- Verifier authority custody remains human-gated; no keypair files were read.
- Target backend migrations and proof worker environment remain unverified in the live environment.
- Helius webhook/backfill remains externally unverified; no API key was read and no webhook was created.
- Real Core parser integration against an owned live Core Loom remains required before mainnet confidence.
- Mainnet program ID, Core collection, verifier authority, Helius webhook, token supply, reward custody, snapshot, dispute, and claim/transfer process remain unverified.

### Current Readiness

- Local documentation and no-secret verification gates are green.
- The launch is still not mainnet-ready.
- SP1 to VerifiedSeal is not freshly end-to-end proven in this pass because the chain send requires human-controlled devnet signing and verifier authority custody.
- Helius/indexer scoring is locally tested and historically devnet-proven, but production Helius is not externally confirmed.

## Completion Audit

Objective restated as concrete deliverables:

1. Read the required source-of-truth docs first.
2. Create/update `docs/anky-system/ANKY_3_DAY_CLOSURE_STATUS.md` before other edits.
3. Canonicalize public Sojourn language as 96 days, 12 regions of 8 days, with 8 kingdoms/chakras/colors as the inner symbolic cycle.
4. Fence stale legacy claims around Bubblegum/cNFT, x402/Base current-mobile payments, global plaintext-free privacy, direct on-chain SP1, mainnet readiness, and `$ANKY` distribution readiness.
5. Produce a launch-safe public claim set.
6. Rerun safe local/devnet gates without deploying, signing, reading secrets, spending funds, or mutating paid services.
7. Identify what remains manual or externally verified.

Prompt-to-artifact checklist:

| Requirement | Evidence | Status |
|---|---|---|
| Required docs read first | Initial read section in this file lists `ANKY_3_DAY_LAUNCH_GAP_AUDIT.md`, `ANKY_RULES_LEDGER.md`, `ANKY_TECHNICAL_SOURCE_OF_TRUTH.md`, and `ANKY_REPO_EVIDENCE_INDEX.md`. | Done |
| Closure status log created before other edits | This file was added before the launch copy patches. | Done |
| Sojourn canon is 12 regions x 8 days | `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md`, `HACKATHON.md`, `static/sojourn9.md`, `docs/anky-system/ANKY_TECHNICAL_SOURCE_OF_TRUTH.md`, and `docs/anky-system/ANKY_WHITEPAPER.md`. | Done |
| 8 kingdoms/chakras/colors scoped as inner symbolic cycle | `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md`, `HACKATHON.md`, `docs/concepts/ankyverse.mdx`, `docs/anky-system/ANKY_TECHNICAL_SOURCE_OF_TRUTH.md`, and `docs/anky-system/ANKY_WHITEPAPER.md`. | Done |
| Great Slumber scoped as planned, not enforced | `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md`, existing source-of-truth docs. | Done |
| ZK language scoped to SP1-enabled/off-chain/verifier-authority attested | `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md`, `HACKATHON.md`, `README.md`, `runbooks/sojourn9-mainnet-launch-checklist.md`. | Done |
| Direct on-chain SP1 and fully trustless wording blocked | `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md` forbidden claims and `runbooks/sojourn9-mainnet-launch-checklist.md` forbidden wording. | Done |
| Privacy scoped to current mobile proof path | `docs/local-first-protocol.md`, `README.md`, `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md`. | Done |
| Legacy plaintext paths fenced | `docs/local-first-protocol.md`, `README.md`, `templates/help.html`, source-of-truth docs. | Done |
| `$ANKY` scoped as memetic/distribution layer, not access or ready rewards | `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md`, `templates/ankycoin.html`, `docs/concepts/ankycoin.mdx`. | Done |
| Historical Bubblegum/cNFT path fenced | `sojourn9/README.md`, `sojourn9/constitution/SOJOURN_9.md`, `sojourn9/constitution/DECISIONS.md`, `sojourn9/docs/*`, `sojourn9/clients/*`, `static/sojourn9.md`, `templates/ankycoin_landing.html`. | Done |
| Older x402/Base docs fenced | `README.md`, `docs/concepts/ankycoin.mdx`, `docs/introduction/overview.mdx`, `docs/introduction/philosophy.mdx`, `templates/help.html`. | Done |
| Launch-safe claim set produced | `docs/anky-system/ANKY_PUBLIC_LAUNCH_CLAIMS.md`. | Done |
| Safe no-secret gate rerun | `npm run sojourn9:privacy`, `npm run sojourn9:readiness`, `npm run sojourn9:test`, `npm run check-config -- --cluster devnet`, Anchor Rust tests/build/typecheck, SP1 tests/fixture, root `cargo check`, mobile typecheck. | Done |
| No mainnet deploy or transaction | No mainnet send/deploy command was run; readiness gate still reports mainnet human gates blocked. | Done |
| No secrets/keypairs read | Commands were no-secret/read-only; no `.env`, keypair, deployer, wallet, or API key values were opened. | Done |
| SP1 to VerifiedSeal fresh live E2E | Not run because it requires human-controlled devnet signer/verifier custody. | Manual |
| Helius production webhook/backfill | Not queried or mutated because it requires external Helius credentials. | Manual |
| Mainnet readiness | Not ready; requires external account, collection, verifier, webhook, token/custody/snapshot evidence. | Manual |

Audit conclusion:

The documentation/claim closure and safe no-secret validation scope is complete. The launch remains blocked on human-owned signing, verifier custody, live backend environment, Helius configuration, real Core Loom integration, and mainnet public evidence.
