# Anky Sponsored Transactions Status

Updated: 2026-05-08

This is the status note for the sponsored-payer pass. It records the current model before runtime changes, and it treats the existing devnet E2E evidence as evidence for the previous account model only.

## Current Payer Model

### Mint Loom

- Mobile calls `/api/mobile/looms/mint-authorizations`, then `/api/mobile/looms/prepare-mint`, then asks the connected wallet to sign and send the prepared Metaplex Core transaction.
- The backend builds the Core `CreateV2` transaction with `owner = wallet` and `payer = payer`.
- The mobile caller currently sends `payer = wallet`, so normal Loom mints are wallet-paid.
- `mobile_mint_authorizations` already stores `sponsor` and `sponsor_payer`, but the mobile prepared transaction builder does not switch the transaction payer to `sponsor_payer`.
- The backend currently signs with the generated asset keypair and the Core collection authority. It does not sign with a separate sponsor payer.
- One-Loom-per-wallet is enforced by mobile UI state and backend lookup conventions, not by a strict sponsorship budget table.

### Seal

- The Anchor `seal_anky` instruction currently has `writer: Signer` as the payer for `LoomState`, `DailySeal`, and `HashSeal`.
- Mobile `sealAnky.ts` sets `transaction.feePayer = writer` and sends through the user wallet.
- The operator `sealAnky.mjs` also loads the writer/sealer keypair for sends and rejects a `--writer` that differs from that keypair.
- Therefore the writer currently pays both transaction fees and account rent for `seal_anky`.
- There is no separate fee/rent sponsor account in the Anchor account model today.

### Prove / VerifiedSeal

- `record_verified_anky` already separates payer authority from writer identity.
- The `verifier` signer pays for the `VerifiedSeal` account and transaction fees.
- The writer account is unchecked public identity data and is not a signer in `record_verified_anky`.
- Backend proof jobs run SP1 off-chain, then use the configured verifier authority to submit `record_verified_anky`.
- This is verifier-authority-attested after off-chain SP1 verification, not direct on-chain SP1 verification.

## Gaps To Close

- Add explicit payer separation to `seal_anky`: writer remains the signer/identity/Loom owner; payer pays account rent and fees and may equal writer or a sponsor.
- Update mobile and operator clients to support user-paid and sponsored seal transactions without changing the writer PDA seeds.
- Update Loom mint preparation so sponsorship uses a backend-controlled sponsor payer only when enabled and eligible, while the user wallet remains the owner.
- Add backend sponsorship policy guards: disabled by default, one sponsored Loom mint per wallet, one sponsored canonical daily seal per wallet/day, budget/rate checks, and idempotent audit rows.
- Add mobile SOL balance checks and friendly fallback messages before raw Solana debit errors reach users.
- Keep proof sponsorship as backend/verifier-paid, but add proof retry/rate guards and sponsorship audit metadata.
- Add tests and docs proving the sponsor never becomes writer, Loom owner, or beneficiary.

## Evidence Caveat

Changing the Anchor account model or IDL invalidates the latest devnet launch evidence for the new version. After this pass, an operator must rebuild, redeploy to devnet, rerun local readiness, and rerun fresh devnet `HashSeal -> SP1 -> VerifiedSeal -> index -> evidence` before using any evidence for launch or mainnet readiness.

The UTC day 20580 evidence has now been explicitly marked stale for the sponsored-payer account model in:

- `runbooks/devnet-20580-live-e2e-summary.md`
- `runbooks/devnet-20580-live-e2e-evidence.json`

New sponsored-payer devnet seal/index evidence exists for UTC day 20581:

- `runbooks/devnet-20581-sponsored-payer-validation-evidence.json`
- `runbooks/devnet-20581-sponsored-payer-score-snapshot.json`

This new evidence validates the payer-split seal model and indexer parsing. It does not replace the need for a fresh SP1 -> VerifiedSeal rerun if the launch evidence needs the full proof loop.

The prompt-to-artifact completion audit is recorded in
`docs/anky-system/ANKY_SPONSORED_TRANSACTIONS_COMPLETION_AUDIT.md`.

## Implemented In This Pass

### Seal payer separation

- `seal_anky` now has both `writer` and `payer` signer accounts.
- `writer` remains the identity used for Loom ownership checks, `DailySeal` PDA seeds, `HashSeal` PDA seeds, and emitted writer data.
- `payer` pays transaction fees and rent for `LoomState`, `DailySeal`, and `HashSeal`.
- If `payer == writer`, the seal is user-paid.
- If `payer != writer`, the sponsor still cannot seal without the writer wallet signature.

### Mobile seal policy

- Mobile checks the writer wallet SOL balance before sealing.
- If the wallet appears funded, mobile sends a normal writer-paid seal.
- If the writer-paid path fails with a funding-shaped Solana error, mobile asks the backend to prepare a sponsored seal.
- Backend-prepared sponsored seal transactions are partially signed by the sponsor payer and must still be signed by the writer wallet on device.
- Local seal sidecars now preserve `payer` and `sponsored` metadata.

### Loom mint sponsorship

- Mint authorization now ignores caller-selected external payers for policy and starts with `payer = wallet`.
- If the wallet already has a recorded Loom, the backend refuses a new authorization.
- If the wallet lacks enough SOL and sponsorship is enabled, the backend switches `payer` to the configured sponsor payer.
- Backend-prepared Core mint transactions keep `owner = wallet`.
- Sponsored Core mint transactions include an owner-signed memo instruction, so a sponsor/collection authority cannot mint a Loom to a user without that user's wallet signature.

### Backend sponsorship controls

- Added `mobile_sponsorship_events` as the public audit and idempotency table.
- Sponsorship is disabled unless `ANKY_ENABLE_SPONSORSHIP=true` (or the legacy aliases `ANKY_SPONSORED_TRANSACTIONS_ENABLED=true` / `ANKY_SPONSORSHIP_ENABLED=true`).
- Mainnet sponsorship is separately disabled unless `ANKY_ENABLE_MAINNET_SPONSORSHIP=true`.
- Budgeting uses `ANKY_SPONSOR_DAILY_BUDGET_LAMPORTS`.
- User-funded thresholds use `ANKY_USER_MINT_MIN_LAMPORTS` and `ANKY_USER_SEAL_MIN_LAMPORTS`.
- Sponsorship estimates use `ANKY_SPONSORED_LOOM_MINT_ESTIMATED_LAMPORTS`, `ANKY_SPONSORED_SEAL_ESTIMATED_LAMPORTS`, and `ANKY_SPONSORED_PROOF_ESTIMATED_LAMPORTS`.
- One sponsored Loom mint per wallet is enforced against recorded Looms.
- One sponsored canonical seal per wallet per UTC day is enforced by the sponsorship table.
- Repeated prepares for the same idempotency key do not double-count against budget and do not downgrade submitted/confirmed/finalized sponsorship rows.
- Sponsored seal preparation now preflights the supplied Loom asset and Core collection on-chain before the sponsor signs: account owners must be the configured Metaplex Core program, the Loom owner must be the writer wallet, and the Loom collection must match the configured collection.

### Proof retries

- VerifiedSeal remains backend/verifier-paid after local SP1 verification.
- Proof retry pressure is capped by `ANKY_PROOF_MAX_ATTEMPTS_PER_SEAL` before a new proof job is accepted.
- Proof jobs now reserve a `mobile_sponsorship_events` budget row with action `proof`; the public verifier authority is recorded as the payer identity.
- This remains verifier-authority-attested after off-chain SP1 verification, not direct on-chain SP1 verification.

## Operational Notes

- Apply `migrations/025_mobile_sponsorship_events.sql` before enabling sponsorship in a backend environment.
- Configure sponsor payer custody with `ANKY_SPONSOR_PAYER_KEYPAIR` or `ANKY_SPONSOR_PAYER_KEYPAIR_PATH`; do not print or commit either value.
- Configure Core collection authority custody for prepared Loom mints with `ANKY_CORE_COLLECTION_AUTHORITY_KEYPAIR` or `ANKY_CORE_COLLECTION_AUTHORITY_KEYPAIR_PATH`; do not print or commit either value.
- The broad `anchor test` command is unsafe for this repo unless `Anchor.toml` is pointed at localnet, because it can deploy to the configured provider cluster. The package `npm test` script now runs local-only typecheck/operator tests; live Anchor integration is behind `npm run test:anchor:live` and `ANKY_ALLOW_LIVE_ANCHOR_TEST=true`.
- During this implementation pass, `cd solana/anky-seal-program && npm test` was mistakenly run and deployed the current program to devnet. Public deployment signature observed: `4EvACDAMXe83heHsW28V8A7T2BAtN2zoZJh8Lcn6aZZjvngzGKLNwvLLoMcxE5uZHnrsrU93v7ULjRmDn56rHm8G`. Do not treat that as launch evidence.
- Use `runbooks/sojourn9-sponsored-transactions-devnet-validation.md` for the next controlled devnet run after explicit deployment/transaction approval.

## Devnet Validation Update

Date: 2026-05-08, UTC day 20581.

Controlled devnet validation was run with explicit human approval. No mainnet command, App Store action, paid API mutation, key rotation, `.env` value print, or keypair JSON print was performed.

Public results:

- Devnet deploy: pass, program `4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX`, signature `2UEhpCCu2tGAxzY2c2gZEXkDShsVPkwTAujLSrSPpWaeygeacjLCPjDVdFWVXVUenfbZq7Rt8DSHQejM1BZfBFWA`.
- Writer-pays seal: pass, writer/payer `6yS2xjgYeBn6HSeMm5zwyYCWQhwFGfw6Sf9fvb8f1NX`, Loom `DHVdX41WRKmHFW2q8MUoJDRYPCmowpC5VJEvdjyviU1g`, signature `5XTh9SmXvkNcFD1ZXyDsLf6aFjqL9hCubWRKp3kBw2osyijg9U718ezutsCbY9EGdbbLw8L1kH4H1FXjXkEukRNE`.
- Sponsor-pays seal: pass, writer `HK52v7KLU7TxPM3RnTYHjEmeg5kgERk1bpzHUmmuBkmR`, payer `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP`, Loom `L5woSyRnN2P1G4v13BH95AVAVZftWN513axR59d9VGy`, signature `4jPvBKCt81SgQxA2vPBkgeVPCU4xsAkg1RN17W1WaHrYzgnDCDjWWC1aokai875hcCBqdqZVEWPNaCAzUUtRtSJJ`.
- Sponsor without writer signature: pass, strict serialization failed with missing writer signature and the bypass-preflight raw transaction remained not found.
- Wrong Loom owner: pass, seal helper rejected with `Core Loom asset owner does not match writer`.
- Indexer: pass, known-signature finalized indexer parsed both new seal signatures and produced two finalized score rows.
- Readiness gate: pass for expected state, `localReady: true`, `launchReady: false`.

Remaining gap: the full SP1 -> VerifiedSeal loop was not rerun for the sponsored-payer model in this validation pass.
