# Anky Sponsored Transactions Completion Audit

Date: 2026-05-08

This audit maps the sponsored-transactions objective to concrete artifacts in the
current worktree. It is intentionally separate from launch readiness: the
implementation is present and locally/devnet validated, but the active recorded
goal still contains a historical "without deploying" condition that is no
longer literally true after the later approved devnet exception.

## Objective Restatement

Update Anky so Solana transactions use the correct payer model:

- A funded user pays for minting and sealing.
- Anky sponsors only when the user does not have enough SOL and the action is
  eligible.
- The user remains the writer, Loom owner, and authority.
- A sponsor only pays transaction fees and rent.
- Sponsorship is conditional, disabled by default unless configured,
  budgeted, idempotent, auditable, and abuse-resistant.
- Proving remains backend/verifier-sponsored after SP1 verification.
- Mobile shows friendly gas/sponsorship errors instead of raw Solana debit
  failures.
- Tests, docs, and evidence must cover the model.
- Mainnet must not be touched.

## Prompt-To-Artifact Checklist

| Requirement | Evidence | Current status |
| --- | --- | --- |
| Follow repo AGENTS privacy, secrets, and mainnet rules | `AGENTS.md`; no `.env` values or keypair JSON values are documented in artifacts | Pass |
| Program payer separation for `seal_anky` | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs` has separate `writer: Signer` and `payer: Signer`; account init uses `payer = payer`; PDA seeds still use `writer` | Pass |
| Writer remains Loom owner/authority | `verify_loom_owner` still checks Core asset owner against `writer`; `sealAnky.mjs` and mobile builders keep writer first and signer | Pass |
| Sponsor only pays fees/rent | Anchor payer account is only the funding signer; docs and tests cover payer different from writer | Pass |
| User-paid seal still works | Devnet UTC day 20581 writer-paid seal signature in `runbooks/devnet-20581-sponsored-payer-validation-evidence.json` | Pass |
| Sponsor-paid seal works | Devnet UTC day 20581 sponsor-paid seal signature in `runbooks/devnet-20581-sponsored-payer-validation-evidence.json` | Pass |
| Sponsor cannot seal without writer authority | Devnet validation records missing-writer-signature failure in `docs/anky-system/ANKY_SPONSORED_TRANSACTIONS_DEVNET_VALIDATION.md` | Pass |
| Wrong Loom owner fails | Devnet validation records `Core Loom asset owner does not match writer`; local tests cover same case | Pass |
| Payer may equal writer | `sealAnky.test.mjs`; devnet writer-paid seal evidence | Pass |
| Payer may differ from writer | `sealAnky.test.mjs`; devnet sponsor-paid seal evidence | Pass |
| Funded users pay first | `apps/anky-mobile/src/lib/solana/sponsoredSeal.ts`; `sponsoredSeal.test.ts` asserts no backend prepare call when normal seal succeeds | Pass |
| Fallback only on funding-shaped errors | `sponsoredSeal.ts`; `sponsoredSeal.test.ts` covers funding fallback and non-funding non-fallback | Pass |
| Friendly mobile errors | `sponsoredSeal.ts`, `RevealScreen.tsx`, `LoomScreen.tsx`; tests cover unavailable sponsorship message | Pass |
| Mint Loom sponsorship policy | `src/routes/mobile_sojourn.rs` mobile mint authorization policy; `apps/anky-mobile/src/lib/solana/mintLoom.ts`; `mintLoom.test.ts` | Implemented, live backend route not exercised |
| One sponsored Loom mint per wallet | `mobile_mint_authorization_policy` checks existing Loom; backend sponsorship uniqueness table | Pass in policy tests |
| Sponsored Loom keeps user as owner | Backend `build_core_loom_mint_transaction` uses owner wallet and payer separately; mobile test rejects owner/payer/collection mismatch | Pass in tests |
| Seal sponsorship endpoint | `POST /api/mobile/seals/prepare` in `src/routes/mobile_sojourn.rs`; partial sponsor signature requires writer wallet completion | Pass |
| One sponsored canonical seal per wallet/day | `mobile_sponsorship_events` unique index and backend `enforce_sponsorship_uniqueness`; `prepare_mobile_seal_eligibility` tests | Pass |
| Session hash must be 64 hex | `validate_prepare_mobile_seal_request`; tests cover bad hash rejection | Pass |
| Same-day seal path only | `validate_prepare_mobile_seal_request`; tests cover wrong UTC day rejection | Pass |
| Extra/noncanonical seals not sponsored by default | `validate_prepare_mobile_seal_request`; tests cover noncanonical rejection unless explicitly enabled | Pass |
| No `.anky` plaintext in sponsorship ledger | `migrations/025_mobile_sponsorship_events.sql`; backend test `mobile_sponsorship_migration_has_no_private_input_columns` | Pass |
| Proof sponsorship and retry guard | `src/routes/mobile_sojourn.rs` proof jobs reserve action `proof`; `ANKY_PROOF_MAX_ATTEMPTS_PER_SEAL`; verifier authority remains payer identity | Implemented, fresh SP1 -> VerifiedSeal not rerun |
| Backend sponsorship disabled by default | `sponsorship_enabled()` requires explicit env enablement; docs list env names only | Pass |
| Backend daily budget | `sponsorship_daily_budget_lamports()` and `enforce_sponsorship_budget()`; migration has budget index | Pass in focused tests |
| Idempotent audit rows | `mobile_sponsorship_events` idempotency key and upsert logic; `sponsorship_idempotency` test | Pass |
| Indexer still parses events after account model change | `solana/scripts/indexer/ankySealIndexer.test.mjs`; devnet known-signature snapshot for UTC day 20581 | Pass |
| Readiness gate remains local-ready but not launch-ready | `npm run sojourn9:readiness` output: `localReady: true`, `launchReady: false` | Pass |
| Previous 20580 evidence marked stale | `runbooks/devnet-20580-live-e2e-summary.md`; `runbooks/devnet-20580-live-e2e-evidence.json` | Pass |
| New sponsored-payer evidence bundle | `runbooks/devnet-20581-sponsored-payer-validation-evidence.json`; `runbooks/devnet-20581-sponsored-payer-score-snapshot.json` | Pass |
| Mainnet untouched during final audit | Only local reads/tests and devnet evidence inspection were run in final audit; docs continue to state mainnet ready: no | Pass |
| Original no-deploy condition | Later user approved controlled devnet redeploy; public devnet deploy signature exists | Not satisfiable as originally worded |

## Current Verification Commands

The following checks were rerun after implementation:

- `git diff --check`: pass
- `cd solana/anky-seal-program && npm test`: pass, 40 tests
- `cd solana/anky-seal-program && npm run sojourn9:readiness`: pass, `localReady: true`, `launchReady: false`
- `node --test solana/scripts/indexer/ankySealIndexer.test.mjs`: pass, 37 tests
- `cd apps/anky-mobile && npm run test -- src/lib/api/ankyApi.test.ts src/lib/solana/mintLoom.test.ts src/lib/solana/sealAnky.test.ts src/lib/solana/sponsoredSeal.test.ts`: pass, 23 tests
- `cargo test -q mobile_mint_authorization_policy`: pass, 5 tests
- `cargo test -q sponsorship_idempotency`: pass, 1 test
- `cargo test -q prepare_mobile_seal_eligibility`: pass, 2 tests
- `cargo test -q prover_error_redaction`: pass, 1 test

## Remaining Weak Spots

- Live authenticated backend Loom mint sponsorship was not exercised against a
  running backend and wallet session.
- Full SP1 -> VerifiedSeal was not rerun after the sponsored-payer deploy.
- Target database migrations were not applied by Codex.
- Live mobile device UX was not manually exercised by Codex.
- Production sponsor payer custody, verifier custody, Helius configuration,
  and mainnet values remain human/operator gates.

## Completion Verdict

Implementation status: complete enough for the sponsored-payer program model,
backend guards, mobile policy, local tests, docs, and devnet seal/index
validation.

Recorded active goal status: not complete, because the active goal still says
"without deploying" and a devnet deployment occurred under later explicit human
approval. The later devnet exception is documented, but it does not make the
older no-deploy wording true.

Mainnet ready: no.
