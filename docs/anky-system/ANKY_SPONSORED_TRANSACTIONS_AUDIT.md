# Anky Sponsored Transactions Audit

Date: 2026-05-07

Objective audited:

> Update Anky so Solana transactions use the correct sponsored-payer model without deploying: users pay when funded, Anky sponsors eligible mint/seal/prove flows only when needed, the user remains writer/Loom owner/authority, sponsorship is disabled by default, budgeted, idempotent, auditable, abuse-resistant, and mobile surfaces friendly errors.

## Completion Decision

Not complete as stated.

The runtime implementation, tests, migration, and runbooks for the sponsored-payer model are in place locally, and no mainnet command was run. However, the explicit "without deploying" requirement was violated during this pass when the previous `npm test` script for `solana/anky-seal-program` invoked `anchor test` against devnet and deployed. That cannot be undone by code changes. The package test script is now guarded so the same mistake is not repeated, but the original objective cannot be marked fully achieved.

## Prompt-To-Artifact Audit

| Explicit requirement | Concrete evidence inspected | Status |
| --- | --- | --- |
| Follow the current AGENTS.md sponsorship goal | This document plus `docs/anky-system/ANKY_SPONSORED_TRANSACTIONS_STATUS.md` records the required-first-step reconnaissance and implementation audit | Done |
| User pays when wallet has enough SOL | `apps/anky-mobile/src/lib/solana/sponsoredSeal.ts` checks balance and tries user-paid `sealAnky` first; `sponsoredSeal.test.ts` covers funded path | Done |
| Anky sponsors only when user lacks SOL | `sponsoredSeal.ts` falls back on low balance or funding-shaped Solana errors; backend `prepare_sponsorship_event` is called only through eligible prepare routes | Done |
| User remains writer, Loom owner, and authority | Anchor `SealAnky` keeps `writer`; Core mint prepare sets `owner = wallet`; mobile mint tests reject prepared owner changes | Done |
| Sponsor only pays fees and rent | Anchor `payer` is separate from writer and used for `init` rent; prepared mint/seal transactions validate payer separately | Done |
| Sponsor must never become owner, writer, or beneficiary | `mintLoom.test.ts`, `sealAnky.test.ts`, and `sealAnky.test.mjs` cover owner/writer/payer separation | Done |
| Friendly error instead of raw Solana debit error | `needsSolanaFunding` and screen error mappers convert funding errors in Reveal, Entry, and Loom flows | Done |
| Conditional sponsorship | Backend requires eligibility checks and env enablement before `prepare_sponsorship_event` | Done |
| Rate-limited sponsorship | Proof path uses `ANKY_PROOF_MAX_ATTEMPTS_PER_SEAL`; seal and mint uniqueness limits reduce repeated sponsorship | Done |
| Budgeted sponsorship | `ANKY_SPONSOR_DAILY_BUDGET_LAMPORTS` and `mobile_sponsorship_events` budget index are used by `enforce_sponsorship_budget` | Done |
| Idempotent sponsorship | `mobile_sponsorship_events(network, action, idempotency_key)` unique index and upsert behavior | Done |
| Auditable sponsorship | `migrations/025_mobile_sponsorship_events.sql` stores public wallet/action/status/signature metadata | Done |
| Abuse-resistant sponsorship | One Loom per wallet, one daily canonical sponsored seal, Core Loom preflight, current UTC day check, disabled-by-default config | Locally implemented |
| Disabled by default | `sponsorship_enabled()` requires `ANKY_ENABLE_SPONSORSHIP` or legacy aliases; mainnet separately requires `ANKY_ENABLE_MAINNET_SPONSORSHIP` | Done |
| Do not deploy to mainnet | No mainnet deploy or transaction was run; helpers still gate/refuse mainnet send paths | Done |
| Do not deploy to devnet unless explicitly instructed | A devnet deployment occurred accidentally through the old `npm test` script | Failed |
| Do not read or print secrets | No `.env`, keypair JSON, wallet, or API key value was read or printed; migration avoids secret/plaintext columns | Done |
| Do not rotate keys, submit stores, or make reward claims | No commands or code paths for those actions were run | Done |

### Flow Requirements

| Flow requirement | Evidence inspected | Status |
| --- | --- | --- |
| Mint: if wallet already owns a Loom, do not mint another | Backend authorization checks recorded Looms; Loom screen blocks when wallet has a recorded Loom | Done |
| Mint: user pays when funded and path supports it | Mobile starts with `payer = wallet`; default builder requires `payer == owner` | Done |
| Mint: sponsor only when unfunded and enabled | Backend authorization can switch payer to configured sponsor only after balance/policy checks | Done |
| Mint: sponsorship unavailable gives clear error | Loom screen maps funding/sponsorship failures to wallet-friendly copy | Done |
| Mint: one sponsored Loom per wallet lifetime | Recorded Loom check plus `mint_loom:{wallet}` sponsorship idempotency | Done |
| Mint: Loom ownership tied to authenticated wallet | Backend prepared mint keeps `owner = wallet`; local state is wallet-scoped | Done |
| Mint: logout clears cached Loom state | `AuthScreen.tsx` calls `clearSelectedLoom()` before logout | Done |
| Mint: login restores from wallet | `LoomScreen.tsx` uses `getSelectedLoomForWallet` and backend `restoreRecordedLoomSelection` for current wallet | Done |
| Seal: writer wallet authorizes seal | Anchor `writer: Signer`; mobile/user must still sign prepared sponsored transaction | Done |
| Seal: writer must own Loom | Anchor checks Core owner; backend sponsored prepare also preflights Core owner/collection | Done |
| Seal: writer pays when funded | `sealAnkyWithPayerPolicy` funded path calls `sealAnky` without external payer | Done |
| Seal: sponsor pays when writer lacks SOL and eligible | `/api/mobile/seals/prepare` returns sponsor payer prepared transaction; mobile signs it with writer wallet | Done |
| Seal: sponsor cannot seal alone | Transaction has writer signer account; backend only partial-signs sponsor payer | Done |
| Seal: one sponsored canonical daily seal per wallet/day | Migration partial unique index plus backend duplicate check | Done |
| Seal: extra fragments not sponsored by default | `/api/mobile/seals/prepare` rejects noncanonical unless `ANKY_SPONSOR_EXTRA_SEALS` is enabled | Done |
| Seal: session hash must be 64 hex | `normalize_hash` and DB check constraints enforce hex hash | Done |
| Seal: UTC day must be current for same-day sponsored path | `/api/mobile/seals/prepare` compares requested day to `current_utc_day()` | Done |
| Seal: no `.anky` plaintext in sponsorship code | Prepare seal request contains wallet, Loom, collection, hash, day, canonical flag only | Done |
| Prove: backend/verifier pays VerifiedSeal | `record_verified_anky` remains verifier-paid; proof sponsorship row uses verifier authority as payer identity | Done |
| Prove: proof retries rate-limited | `ANKY_PROOF_MAX_ATTEMPTS_PER_SEAL` default 3/day | Done |
| Prove: no VerifiedSeal before local SP1 verification | Proof send/recovery path validates receipt public values and local verification flags before recording | Locally implemented |
| Prove: honest wording, not direct on-chain SP1 | Docs preserve verifier-authority-attested wording and direct on-chain SP1 as future hardening | Done |

### Named Files And Deliverables

All required first-step files named in the prompt are present in the worktree. This was rechecked with a local `test -e` loop on 2026-05-07. Present files:

- `docs/anky-system/ANKY_MAINNET_READINESS_GATE.md`
- `docs/anky-system/ANKY_LOCAL_READINESS_FIX.md`
- `docs/anky-system/ANKY_3_DAY_CLOSURE_STATUS.md`
- `docs/anky-system/ANKY_TECHNICAL_SOURCE_OF_TRUTH.md`
- `docs/anky-system/ANKY_RULES_LEDGER.md`
- `runbooks/devnet-20580-live-e2e-summary.md`
- `runbooks/devnet-20580-live-e2e-evidence.json`
- `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs`
- `solana/anky-seal-program/scripts/sealAnky.mjs`
- `solana/anky-seal-program/scripts/recordVerifiedAnky.mjs`
- `solana/anky-seal-program/scripts/checkLaunchConfig.mjs`
- `solana/anky-seal-program/tests/anky-seal-program.ts`
- `solana/anky-seal-program/package.json`
- `src/routes/mobile_sojourn.rs`
- `apps/anky-mobile/src/lib/solana/types.ts`
- `apps/anky-mobile/src/lib/api/ankyApi.ts`
- `apps/anky-mobile/src/lib/api/types.ts`
- `apps/anky-mobile/src/screens/LoomScreen.tsx`
- `apps/anky-mobile/src/screens/RevealScreen.tsx`
- `apps/anky-mobile/src/screens/you/YouDetailScreens.tsx`
- Privy wallet integration files under `apps/anky-mobile/src/lib/privy/`
- Loom minting files under `apps/anky-mobile/src/lib/solana/`

Required created deliverable:

- `docs/anky-system/ANKY_SPONSORED_TRANSACTIONS_STATUS.md` exists and records the pre-change payer model plus implemented status.

## Checklist

| Requirement | Evidence | Status |
| --- | --- | --- |
| Required reconnaissance and pre-change payer notes | `docs/anky-system/ANKY_SPONSORED_TRANSACTIONS_STATUS.md` | Done |
| Anchor `seal_anky` separates writer from payer | `SealAnky` has `writer` and mutable `payer`; `LoomState`, `DailySeal`, and `HashSeal` use `payer = payer` | Done |
| Writer remains authority for seal | `DailySeal` and `HashSeal` PDA seeds still use writer; instruction requires writer signature | Done |
| Mobile seal instruction matches payer-split account order | `apps/anky-mobile/src/lib/solana/sealAnky.test.ts` checks writer, payer, Loom, collection, PDA, and system accounts | Done |
| VerifiedSeal remains verifier-paid/attested | `RecordVerifiedAnky` still uses verifier signer as payer; proof path records after local verification | Done |
| Mint uses user as owner and sponsor only as payer | Backend prepared Core mint sets `owner = wallet`, `payer = payer`, and includes owner authorization memo | Done |
| Mint payer policy is unit-tested | `mobile_mint_authorization_policy_*` covers funded self-payment, unfunded sponsorship, existing Loom rejection, invite rejection, sponsorship-unavailable rejection, and unknown balance fallback | Done |
| Sponsored Loom mint requires user wallet authorization | `owner_authorization_memo_instruction` requires the owner as signer; `owner_authorization_memo_requires_user_wallet_signature` covers this locally | Done |
| One sponsored Loom mint per wallet lifetime | `mobile_loom_mints` check plus `mint_loom:{wallet}` idempotency key | Done |
| Loom local state is wallet-scoped | Mobile clears selected Loom when wallet is missing or mismatched | Done |
| Seal sponsors only current canonical day by default | `/api/mobile/seals/prepare` validates current UTC day and rejects noncanonical seals unless explicitly enabled | Done |
| Sponsored seal eligibility is unit-tested | `prepare_mobile_seal_eligibility_*` covers current UTC day, canonical-only default, optional extra-seal override, and 64-hex hash normalization/rejection | Done |
| One sponsored daily seal per wallet/day | DB partial unique index and backend duplicate check | Done |
| Sponsor cannot seal alone | Prepared seal transaction partial-signs sponsor payer only; user wallet must still sign writer | Done |
| Sponsored seal avoids obvious invalid-Loom fee drain | Backend preflights Loom asset and collection owner/data before preparing sponsor-paid seal | Done |
| Backend Core preflight parser is fixture-tested | Synthetic and observed public devnet Core AssetV1/CollectionV1 byte fixtures | Done |
| Sponsorship disabled by default | `ANKY_ENABLE_SPONSORSHIP` and aliases default false; mainnet sponsorship separately disabled | Done |
| Sponsorship is budgeted/idempotent/auditable | `mobile_sponsorship_events` with daily budget check, idempotency key, status/signature tracking | Done |
| Sponsorship idempotency keys are deterministic and action-scoped | `sponsorship_idempotency_key` is shared by prepare and landed updates; unit test covers mint, seal, proof, and missing required fields | Done |
| Sponsorship ledger tracks failed public receipts | `sponsorship_status_mapping_tracks_submitted_landed_and_failed_receipts` covers pending/processed -> submitted, confirmed/finalized, and failed status mapping | Done |
| Proof sponsorship reservations are failed after prover failure | `run_mobile_proof_job` marks the proof job failed with a redacted error and then calls `mark_mobile_proof_sponsorship_failed`; failed rows are excluded from the daily budget query | Done |
| Proof sponsorship failure reasons are redacted before persistence | `prover_error_redaction_removes_private_paths_and_raw_anky` covers verifier keypair path, prover workdir, protoc path, and raw `.anky` redaction | Done |
| Proof retries are rate-limited and budgeted | `ANKY_PROOF_MAX_ATTEMPTS_PER_SEAL`, default 3/day; proof jobs reserve `mobile_sponsorship_events` action `proof` | Done |
| No `.anky` plaintext in sponsorship ledger | Migration stores only public metadata and has a regression test for forbidden column names | Done |
| Mobile chooses user-paid first | `sealAnkyWithPayerPolicy` checks wallet SOL and tries user-paid before sponsored fallback | Done |
| Mobile sponsorship fallback is unit-tested | `apps/anky-mobile/src/lib/solana/sponsoredSeal.test.ts` covers funded, unfunded, funding-error fallback, non-funding errors, and friendly unavailable copy | Done |
| Mobile prepare-seal API contract is tested | `apps/anky-mobile/src/lib/api/ankyApi.test.ts` covers `POST /api/mobile/seals/prepare` payload and sponsored response parsing | Done |
| Mobile Loom mint prepared transaction contract is tested | `apps/anky-mobile/src/lib/solana/mintLoom.test.ts` covers owner/payer/collection checks and self-funded builder rejection of sponsorship | Done |
| Friendly gas/sponsorship errors | Reveal, Entry, and Loom screens map funding failures to user copy | Done |
| Indexer understands new payer account | `decodeSealAnkyInstruction` handles old and new account order and emits `payer` | Done |
| Mainnet not touched | No mainnet deploy or transaction was run | Done |
| No deployment during this work | `npm test` for the Anchor package ran `anchor test` against devnet and deployed | Failed |
| Default Anchor package tests are non-deploying | `npm test` now runs local typecheck/operator tests; live Anchor integration requires `npm run test:anchor:live` and `ANKY_ALLOW_LIVE_ANCHOR_TEST=true` | Done after incident |
| Controlled devnet validation is documented | `runbooks/sojourn9-sponsored-transactions-devnet-validation.md` | Done |
| Readiness docs use guarded live-test and current migration range | `ANKY_MAINNET_READINESS_GATE.md` uses `ANKY_ALLOW_LIVE_ANCHOR_TEST=true npm run test:anchor:live` and migrations 019-025; `rg` found no stale old command/range references | Done |

## Verification Evidence

Passed:

- `cargo check`
- `cargo test -q mobile_sponsorship`
- `cargo test -q mobile_mint_authorization_policy`
- `cargo test -q sponsorship_status_mapping`
- `cargo test -q sponsorship_idempotency`
- `cargo test -q prover_error_redaction`
- `cargo test -q prepare_mobile_seal_eligibility`
- `cargo test -q owner_authorization`
- `cargo test sponsored_core`
- `cargo test mobile_sponsorship_migration_has_no_private_input_columns`
- `cargo test mobile_sponsorship_migration_tracks_proof_budget_metadata`
- `cargo test sponsored_core_loom_parser`
- `cargo test sponsored_core_collection_parser`
- `cd apps/anky-mobile && npm run typecheck`
- `cd apps/anky-mobile && npm run test -- src/lib/api/ankyApi.test.ts`
- `cd apps/anky-mobile && npm run test -- src/lib/solana/mintLoom.test.ts`
- `cd apps/anky-mobile && npm run test -- src/lib/solana/sealAnky.test.ts src/lib/solana/sponsoredSeal.test.ts`
- `cd apps/anky-mobile && npm run test -- src/lib/solana/sponsoredSeal.test.ts`
- `cd apps/anky-mobile && npm test`
- `cd solana/anky-seal-program && npm run typecheck`
- `cd apps/anky-mobile && npm run test:protocol`
- `cd apps/anky-mobile && npm run test:sojourn`
- `cd solana/anky-seal-program && cargo test --manifest-path Cargo.toml --package anky_seal_program`
- `cd solana/anky-seal-program && npm run build`
- `node --test solana/scripts/indexer/ankySealIndexer.test.mjs`
- `cd solana/anky-seal-program && npm run test:seal`
- `cd solana/anky-seal-program && npm run sojourn9:test`
- `cd solana/anky-seal-program && npm test`
- `cd solana/anky-seal-program && npm run test:anchor:live` refuses to run without `ANKY_ALLOW_LIVE_ANCHOR_TEST=true`.
- `test -e` presence check for every required first-step file listed above
- `rg -n "019-024|npm test -- --skip-local-validator --skip-deploy" docs runbooks` returned no matches
- `git diff --check`
- Readiness gate check with placeholder secret values reported `localReady=true`, `launchReady=false`, and no failed local checks.

Failed or blocked:

- `cargo test mobile_sponsorship_migration_has_no_private_input_columns mobile_sponsorship_migration_tracks_proof_budget_metadata sponsored_core_loom_parser sponsored_core_collection_parser` failed because `cargo test` accepts one test filter argument. The relevant filters were rerun correctly and passed.
- `cargo test -q sponsorship_idempotency owner_authorization` failed for the same reason: Cargo accepts one test filter. Each focused test was rerun separately and passed. This invalid command was accidentally re-run during the 2026-05-07 completion audit; the separate focused reruns passed again.
- `cargo test -q prepare_mobile_seal_eligibility` initially failed because the helper result type did not implement `Debug`, which `unwrap_err` requires. `PrepareMobileSealEligibility` now derives `Debug`, and the focused test rerun passed.
- Earlier in this pass, `cd solana/anky-seal-program && npm test` was unsafe because it ran `anchor test`, deployed to devnet, and only executed a skipped integration test. The package script has since been changed to local-only tests, and live Anchor integration is gated behind `npm run test:anchor:live` plus `ANKY_ALLOW_LIVE_ANCHOR_TEST=true`.

## Residual Gaps

- This is not fresh launch evidence. The Anchor account model changed, and the accidental devnet deploy is not a controlled readiness run.
- No fresh devnet HashSeal -> SP1 -> VerifiedSeal -> index -> score evidence has been produced after these changes.
- The controlled devnet validation steps are documented in `runbooks/sojourn9-sponsored-transactions-devnet-validation.md`, but they have not been executed in this pass because deployment/transactions require explicit human approval.
- Sponsored seal preparation now has a backend Core account preflight with observed devnet Core byte fixture coverage, but it is still a hand-rolled parser and needs fresh live integration confidence after controlled devnet deployment.
- Proof transactions are now included in the `mobile_sponsorship_events` daily SOL budget ledger, but no fresh live proof run has verified the budget row lifecycle.
- Mainnet remains blocked until program ID, Core collection, verifier authority custody, sponsor payer funding, budget policy, and fresh devnet evidence are confirmed.
