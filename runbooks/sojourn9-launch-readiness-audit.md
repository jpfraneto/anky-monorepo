# Sojourn 9 Launch Readiness Audit

This audit maps the launch objective to local artifacts and remaining blockers. It is intentionally blunt: green local checks do not mean mainnet readiness.

## Objective

Make Anky Sojourn 9 launch-ready as a mobile proof-of-practice loop:

```text
write privately -> hash exact .anky bytes -> seal on Solana -> prove privately with SP1 -> record VerifiedSeal -> index score -> show mobile proof state
```

Concrete success criteria for this objective:

1. Harden the active Core-based Anchor seal path for `DailySeal`, `HashSeal`, `LoomState`, and `VerifiedSeal`.
2. Provide a runnable SP1 -> VerifiedSeal path that proves and verifies private `.anky` validity locally, then records only public receipt metadata on Solana after verifier-authority attestation.
3. Show mobile proof state that clearly separates local protocol validity from `Sealed`, `Proving`, `Verified`, and failed proof states.
4. Index finalized sealed and verified events with Helius-backed tooling and compute deterministic Score V1 rewards from public data only.
5. Preserve privacy and launch honesty: no canonical plaintext persistence, no secret exposure, no invented mainnet deployment, and no direct on-chain SP1 claim.

## Completion Audit

Date: 2026-05-06

This is not a completion certificate. It is the current prompt-to-artifact audit for the AGENTS.md launch objective.

Completion verdict from this audit: **not complete**. The local implementation, no-secret tooling, and one live devnet CLI loop are in place. The `0xx1` devnet run proved the current edited `.anky` bytes with SP1, sealed the hash to a writer-owned Core Loom, recorded a `VerifiedSeal`, and produced a one-wallet finalized Score V1 snapshot through the known-signature Helius path. The launch objective still requires human-owned release steps before it is actually achieved: target backend migrations/config, public backend metadata/status for the live run, live Helius webhook or full finalized backfill coverage, a live mobile-device demo, the opt-in real Core integration test, and confirmed mainnet launch values.

Machine-readable local readiness report:

```bash
node solana/scripts/sojourn9/launchReadinessGate.mjs
```

The readiness gate checks local launch artifacts and prints live/human-gated blockers. It does not read `.env` files, keypairs, wallet files, private `.anky` contents, or API key values.

Latest continuation audit window ending at `2026-05-06T11:58:41Z`:

- `node solana/scripts/sojourn9/launchReadinessGate.mjs` reported `localReady: true` and `launchReady: false` at `2026-05-06T11:58:40Z`, including the no-secret public launch evidence builder/auditor artifact/source gates, the score snapshot auditor unsafe-path source gate, the live checklist UTC-day rollover source gate, the proof-prep manifest UTC-day rollover source gate, the proof handoff UTC-day rollover source gate, the legacy public handoff UTC-day derivation source gate, the Helius webhook manifest delivery-caveat source gate, the Helius webhook receiver route consistency source gate, the Helius `authHeader` bearer-secret compatibility source gate, the concrete Helius launch evidence reproduction source gate, the Helius indexer unsafe-launch-input source gate, and the mobile hash-seal-vs-SP1-proof wording source gate.
- `node solana/scripts/sojourn9/privacyGuard.mjs` passed with `ok: true`, 23 checked files, and no issues at `2026-05-06T11:58:40Z`. The guard now also source-checks that the public launch evidence builder, public launch evidence auditor, Helius score snapshot auditor, and Helius indexer reject secret-shaped paths and direct `.anky` witness paths before reading; the indexer additionally rejects credentialed or non-local plaintext backend URLs and devnet default IDs on mainnet indexing.
- `node --test solana/scripts/sojourn9/makeLaunchEvidence.test.mjs solana/scripts/sojourn9/auditLaunchEvidence.test.mjs solana/scripts/indexer/auditScoreSnapshot.test.mjs solana/scripts/sojourn9/launchReadinessGate.test.mjs solana/scripts/sojourn9/privacyGuard.test.mjs` passed 28 focused tests after extending public audit-tool `.anky` path refusal coverage.
- `node --test solana/scripts/sojourn9/auditLaunchEvidence.test.mjs` passed 7 tests, including no-secret audit summary fields for `devnetUtcDay`, `devnetSealWindow`, and `devnetDayRolloverAt`.
- `node --test solana/scripts/sojourn9/makeLaunchEvidence.test.mjs solana/scripts/sojourn9/launchReadinessGate.test.mjs` passed 11 tests, including public evidence UTC-day status preservation, legacy public handoff metadata UTC-day derivation, builder rejection of inconsistent manifest `utcDayStatus`, and readiness-gate source checks for the no-secret evidence builder/auditor.
- `node --test solana/scripts/sojourn9/makeLaunchEvidence.test.mjs solana/scripts/sojourn9/auditLaunchEvidence.test.mjs solana/scripts/sojourn9/launchReadinessGate.test.mjs` passed 18 tests after the public evidence schema was tightened to include concrete Helius reproduction fields.
- `node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs solana/scripts/indexer/ankySealIndexer.test.mjs solana/scripts/sojourn9/privacyGuard.test.mjs` passed 45 tests after adding a readiness-gate source check for unsafe Helius indexer launch inputs and direct `.anky` witness path refusal.
- `node solana/scripts/sojourn9/auditLaunchEvidence.mjs --print-template` prints a no-secret Helius evidence shape with `webhookAccountAddresses: ["<anky_seal_program_public_key>"]`, `receiverPath: "/api/helius/anky-seal"`, `backfillMethod: "getTransactionsForAddress"`, and `backfillCommitment: "finalized"`.
- `node solana/scripts/sojourn9/makeLaunchEvidence.mjs --manifest /tmp/anky-sojourn9-current-DXfFDY/handoff-manifest.json ...` succeeded against the current public handoff manifest with public test signatures, derived `devnetE2E.utcDayStatus.sealWindow: open`, and did not leak the private witness.
- Helius official webhook guidance was checked through the Helius docs MCP at `2026-05-06T11:21:03Z`; the generated no-secret manifest now includes retry/backoff, localhost tunnel, and high-delivery-failure auto-disable/re-enable caveats without reading `HELIUS_API_KEY`.
- `node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs` passed 4 tests, including the readiness-gate checks that the backend webhook receiver route is covered and that the backend accepts Helius `Authorization: Bearer <secret>` from webhook `authHeader`.
- The mobile copy now keeps hash sealing separate from SP1 verification: sealing says "hash seal", verified proof state says "proof verified", the Loom intro says Solana records hash seals plus verified receipts when they exist, and the indexed score metric says "proof days".
- `node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs solana/scripts/indexer/heliusWebhookManifest.test.mjs` passed 9 tests, including the no-secret manifest, live delivery caveats, and the readiness-gate check that the backend webhook receiver route is covered.
- `node solana/scripts/indexer/heliusWebhookManifest.mjs --webhook-url https://example.com/api/helius/anky-seal --program-id 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX --cluster devnet` printed only public placeholders and selected `webhookType: "enhancedDevnet"`. The readiness gate now checks that the backend receiver route, manifest usage, and runbook all use `/api/helius/anky-seal`.
- `node solana/scripts/sojourn9/liveE2eChecklist.mjs --writer 9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp --loom-asset 4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9 --session-hash 881ecaf0685337bdc2c92778d60464d0b00363b5e07995d3bec3c5241d845865 --utc-day 20579 --backend-url http://127.0.0.1:3000` printed public `utcDayStatus.sealWindow: open`, `utcDayStatus.dayRolloverAt: 2026-05-07T00:00:00.000Z`, and placeholder-only secret fields at `2026-05-06T11:04:05Z`; the readiness gate source-checks this live-checklist UTC-day rollover surface.
- `node solana/scripts/sojourn9/checkProofHandoff.mjs --manifest /tmp/anky-sojourn9-current-DXfFDY/handoff-manifest.json` reported `proofExists: true`, `receiptExists: true`, `verifiedReceiptExists: true`, `witnessRead: false`, `utcDayStatus.sealWindow: open`, `utcDayStatus.dayRolloverAt: 2026-05-07T00:00:00.000Z`, `hashSealReady.ok: false`, `verifiedSealLanded.ok: false`, backend fetch failed, and `nextAction: send_hashseal` at `2026-05-06T11:58:41Z`.
- `git diff --check` passed.
- `cd solana/anky-seal-program && npm run sojourn9:test` passed: 156 tests in the final pass ending after `2026-05-06T11:52:46Z`.
- `cd solana/anky-seal-program && npm --silent run sojourn9:audit-evidence -- --print-template` printed a no-secret public evidence template; the focused test verifies the template is rejected as final evidence until `templateOnly` is removed and real public values replace placeholders.
- `node --test solana/scripts/sojourn9/makeLaunchEvidence.test.mjs` is covered by the 152-test suite. The builder reads only public handoff metadata, derives Orb links from landed signatures, derives UTC-day status from legacy public handoff metadata when explicit `utcDayStatus` is absent, rejects inconsistent explicit UTC-day status, can run the public Score V1 snapshot auditor with `--require-allocation`, requires explicit score/backfill audit confirmation, refuses mainnet, and can run the evidence auditor before writing output.
- `apps/anky-mobile/node_modules` was absent.

Latest live 0xx1 devnet E2E evidence from the continuation ending at `2026-05-06T15:29:21Z`:

- Public handoff manifest: `/tmp/anky-0xx1-live-handoff-manifest.json`.
- Public handoff status: `/tmp/anky-0xx1-live-handoff-status.json`.
- Durable no-secret repo artifact: `runbooks/devnet-0xx1-live-e2e-evidence.json`. This is demo evidence, not final launch evidence.
- The status checker read only public manifest/receipt metadata and reported `files.witnessRead: false`, `files.proofExists: true`, `files.receiptExists: true`, `files.verifiedReceiptExists: true`, and `nextAction: backfill_or_post_verified_metadata` at `2026-05-06T15:26:29Z`.
- Devnet program ID: `4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX`.
- Devnet Core collection: `F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u`.
- Verifier authority: `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP`.
- SP1 vkey: `0x00399c50f86cb417d0cf0c80485b0f1781590170c6892861a1a55974da6e4758`.
- Writer: `5xf7VcURsgiy3SvkBUirAYSPu3SYhto9qX6AFrLTvN1Q`.
- Loom asset: `6oEyFPQPksvKyCtdjsSEzL6JMxAPPwBPkMBBAMvUnNLJ`.
- Session hash: `f6c922b2b87fec532aa3d24cb2bafcc237043fa28de168820c2326e0b18955b3`.
- Proof hash: `b228eea80545194b3e39b7ac9dcf5f04443c9bf18ef94ff1f3e0e0fe0d17fd1f`.
- UTC day: `20579`.
- HashSeal PDA: `ABiRs6Mqf3SyFWB5D2bKLy5fD5yRnYbT9hqeXHbzKd9g`.
- VerifiedSeal PDA: `6FhuWEqMUcpuePVgywdFwWJ295mE8g9nwK7oQz7x7J19`.
- Seal transaction: `5EvmetB1HBsRJR4ErvTbvvamq63fQzEEAhxoiaUobVxUybFeMRBYVVur3CX5mfJxRCAubFkjib2QodK1E8avTEJU`, Orb `https://orbmarkets.io/tx/5EvmetB1HBsRJR4ErvTbvvamq63fQzEEAhxoiaUobVxUybFeMRBYVVur3CX5mfJxRCAubFkjib2QodK1E8avTEJU`.
- VerifiedSeal transaction: `2pyxGzZeYzmd3r5ctTqhB73RX3QPCAeE7VRQYMRPD16nCv1t7Gj3U7NXhgWJkcWiwaSyYZP72sNtoDRbcUNTQ91`, Orb `https://orbmarkets.io/tx/2pyxGzZeYzmd3r5ctTqhB73RX3QPCAeE7VRQYMRPD16nCv1t7Gj3U7NXhgWJkcWiwaSyYZP72sNtoDRbcUNTQ91`.
- Public chain status confirmed `hashSealReady.ok: true`, `hashSealReady.postVerified: true`, `verifiedSealLanded.ok: true`, matching writer, Loom asset, session hash, UTC day, proof hash, verifier, and protocol version `1`.
- The SP1 receipt public values reported `valid: true`, `duration_ok: true`, `accepted_duration_ms: 479011`, `rite_duration_ms: 487011`, and `event_count: 3817`.
- `node solana/scripts/indexer/ankySealIndexer.mjs --signature 5EvmetB1HBsRJR4ErvTbvvamq63fQzEEAhxoiaUobVxUybFeMRBYVVur3CX5mfJxRCAubFkjib2QodK1E8avTEJU,2pyxGzZeYzmd3r5ctTqhB73RX3QPCAeE7VRQYMRPD16nCv1t7Gj3U7NXhgWJkcWiwaSyYZP72sNtoDRbcUNTQ91 --cluster devnet --program-id 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX --out /tmp/anky-live-devnet-signature-score-snapshot-20579.json` produced a finalized one-wallet snapshot with `indexedEvents: 2`, `sealedEvents: 1`, `verifiedEvents: 1`, `scoreRows: 1`, `totalScore: 2`, and score `2` for writer `5xf7VcURsgiy3SvkBUirAYSPu3SYhto9qX6AFrLTvN1Q`.
- `node solana/scripts/indexer/auditScoreSnapshot.mjs --snapshot /tmp/anky-live-devnet-signature-score-snapshot-20579.json` passed.
- `cd solana/anky-seal-program && npm run sojourn9:test` passed 160 tests after recording the live evidence, updating readiness-gate wording, and adding privacy-guard coverage for `runbooks/devnet-0xx1-live-e2e-evidence.json`.
- `node solana/scripts/sojourn9/launchReadinessGate.mjs` reported `localReady: true` and `launchReady: false` at `2026-05-06T15:35:07Z`. The first human-gated blocker now requires a public audited fresh same-day devnet `HashSeal -> VerifiedSeal` evidence bundle for the target demo rather than implying no devnet proof loop has ever landed.
- `node solana/scripts/sojourn9/privacyGuard.mjs` passed with `ok: true` and no issues at `2026-05-06T15:38:27Z`; it now checks `runbooks/devnet-0xx1-live-e2e-evidence.json`, rejects private/plaintext-like fields, rejects complete `.anky` plaintext-looking values, rejects secret-shaped values, and requires demo evidence to remain explicitly non-final with `notFinalBecause`.
- The readiness gate now lists `runbooks/devnet-0xx1-live-e2e-evidence.json` as the `Live devnet 0xx1 evidence artifact`; the artifact was present when checked at `2026-05-06T15:39:32Z`.
- `runbooks/sojourn9-helius-indexing.md` now includes an exact `Live 0xx1 Devnet Evidence Replay` command for the landed seal/verified transaction pair, plus a backend metadata replay command with placeholder-only `HELIUS_API_KEY` and `ANKY_INDEXER_WRITE_SECRET` values.
- `runbooks/sojourn9-backend-verifiedseal.md` now cross-references the same live `0xx1` transaction pair for target-backend metadata follow-up after migrations/config are applied.
- `git diff --check` passed after the live-evidence audit update.
- Program-address Helius backfill remained empty for the same devnet program at `/tmp/anky-live-devnet-program-backfill-score-snapshot-20579-rerun.json`: `indexedEvents: 0`, `scoreRows: 0`. This keeps full backfill/webhook coverage as a launch blocker even though known-signature finalized reconciliation worked for the demo pair.
- Caveat: the proof is for the edited current bytes of the `0xx1` witness after the terminal `8000` and timestamp fix. It is valid proof-of-current-bytes, not proof of the pre-edit file.

### Prompt-To-Artifact Checklist

| Requirement / Gate | Concrete evidence | Verification status |
| --- | --- | --- |
| React Native mobile write/seal UX | `apps/anky-mobile/src/screens/RevealScreen.tsx`, `apps/anky-mobile/src/components/seal/SwipeToSealAction.tsx`, `apps/anky-mobile/src/lib/solana/sealAnky.ts` | Locally typechecked and unit-tested for seal ABI; live phone UX still needs human demo |
| SHA-256 over exact canonical `.anky` UTF-8 bytes | `apps/anky-mobile/src/lib/solana/sealAnky.test.ts`, `solana/anky-zk-proof/src/lib.rs`, `solana/anky-zk-proof` tests | Locally covered; tests check exact hash bytes and no line-ending/UTC-day transformation |
| Canonical space encoding | `apps/anky-mobile/src/lib/ankyProtocol.ts`, `solana/anky-zk-proof/src/lib.rs`, `src/routes/mobile_sojourn.rs`, `AGENTS.md`, `docs/local-first-protocol.md` | Current launch protocol encodes typed spaces as the exact `SPACE` token; legacy literal-space capture records are rejected by mobile/SP1/backend validation |
| No plaintext persistence or logging in proof path | `solana/scripts/sojourn9/proveAndRecordVerified.mjs`, `solana/anky-seal-program/scripts/recordVerifiedAnky.mjs`, `solana/anky-zk-proof/src/lib.rs` receipt privacy test | Locally covered for new SP1/operator path; existing opt-in reflection endpoint remains transient-process-memory only |
| Executable privacy guard for launch metadata | `solana/scripts/sojourn9/privacyGuard.mjs`, `privacyGuard.test.mjs`, `launchReadinessGate.mjs` | Locally covered; guard checks migrations `019_credit_ledger_entries` through `022_mobile_helius_webhook_signature_dedupe`, backend verified/webhook guards, operator/indexer script logging/options, stderr secret redaction, and mobile proof-state plus backup manifest code for private `.anky`/witness persistence regressions; it does not inspect secret files or private `.anky` contents |
| Operator stderr secret redaction | `solana/scripts/sojourn9/redactSecrets.mjs`, `redactSecrets.test.mjs`, operator/indexer/readiness imports, `privacyGuard.mjs` | Locally covered; runnable operator, handoff, indexer, webhook-manifest, readiness, privacy, and migration-smoke scripts redact credential-bearing URLs, bearer tokens, connection URLs, and path-shaped keypair/wallet/deployer references from top-level error output while preserving public placeholder names; privacy guard now checks the expanded runnable-script surface |
| Mobile backup excludes transient proof artifacts | `apps/anky-mobile/src/lib/ankyBackupManifest.ts`, `apps/anky-mobile/src/lib/ankyBackup.ts`, `apps/anky-mobile/src/lib/ankyBackupManifest.test.ts` | Locally covered; backup eligibility keeps canonical hash-named `.anky`, pending/active drafts, session index, images, and public sidecars, while excluding generic `.anky` witness files, dotfiles, `receipt.json`, `verified-receipt.json`, `proof-with-public-values.bin`, `handoff-manifest.json`, and proof/witness/handoff-shaped artifact names |
| Metaplex Core Loom ownership access artifact | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs`, mobile Loom mint/config files, `checkLaunchConfig.mjs --loom-asset`, `solana/anky-seal-program/tests/anky-seal-program.ts`, `runbooks/sojourn9-core-seal-integration.md` | Parser hardened against public devnet Core AssetV1 and CollectionV1 account bytes; read-only live Core asset check passed; opt-in Anchor integration test now requires a real owned Core Loom instead of fake zero-data Core accounts; still needs live Core seal E2E before mainnet |
| Public devnet launch config visibility | `solana/anky-seal-program/scripts/checkLaunchConfig.mjs` | Read-only devnet check passed for seal program executable, Core collection ownership/layout, proof verifier pubkey, and optional Core Loom asset base fields; unknown CLI flags are rejected |
| Fresh same-day HashSeal operator path | `solana/anky-seal-program/scripts/sealAnky.mjs`, `runbooks/sojourn9-sp1-verifiedseal.md` | Locally covered by mocked RPC tests; helper reads only public hash/day/Loom values, refuses mainnet, refuses stale UTC-day preflight/send, checks Core collection/Loom base fields, fails if `DailySeal` or `HashSeal` already exists, sends devnet only when a writer/sealer keypair is supplied by the operator, and can post public seal receipt metadata to the backend after a send or landed-signature HashSeal chain check |
| Operator command discoverability | `solana/anky-seal-program/package.json`, `runbooks/sojourn9-sp1-verifiedseal.md`, `runbooks/sojourn9-helius-indexing.md` | Locally covered; package aliases expose `sojourn9:readiness`, `sojourn9:privacy`, `sojourn9:test`, `sojourn9:demo-witness`, `sojourn9:live-checklist`, `sojourn9:prepare-proof`, `sojourn9:handoff-status`, `sojourn9:prove-record`, `sojourn9:index`, `sojourn9:audit-snapshot`, `sojourn9:audit-evidence`, `sojourn9:make-evidence`, and `sojourn9:webhook-manifest` from the active Solana package |
| Custom Anchor `DailySeal`, `HashSeal`, `LoomState`, `VerifiedSeal` | `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs` | Anchor build/Rust tests passed locally |
| `seal_anky` enforces one wallet/day and one wallet/hash | Anchor source and tests in `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs` | Locally covered by program logic; opt-in TS Core integration now skips unless a real Loom is supplied, so a live run is still required for confidence |
| SP1 proof generation and local verification | `solana/anky-zk-proof`, SP1 script path, runbook commands | Core execute and prove paths passed locally on 2026-05-06 using `/tmp` outputs; generated repo artifacts are not treated as source of truth; current vkey is `0x00399c50f86cb417d0cf0c80485b0f1781590170c6892861a1a55974da6e4758` |
| Receipt public values match writer, session hash, UTC day, protocol, and timing rules | `recordVerifiedAnky.mjs` and `proveAndRecordVerified.mjs` tests | Locally covered; operator rejects bad proof hash, invalid timing, bad UTC-day derivation, mismatched writer/hash/day, and unknown CLI flags |
| Same-day devnet proof witness preparation | `solana/scripts/sojourn9/makeDemoAnky.mjs`, `makeDemoAnky.test.mjs`, SP1 receipt-builder smoke | Locally covered for demo witnesses; generator refuses to write `.anky` plaintext inside the repo, rejects unknown CLI flags, does not overwrite without `--force`, and prints only public hash/day/timing metadata |
| No-secret live E2E handoff checklist | `solana/scripts/sojourn9/liveE2eChecklist.mjs`, `liveE2eChecklist.test.mjs`, package alias `sojourn9:live-checklist` | Locally covered; validates public writer, Loom, hash, current UTC day, backend/webhook URL safety, refuses mainnet and secret-shaped CLI options, and prints placeholders for keypair paths, backend secret, and Helius API key instead of accepting secret values |
| Current-day proof handoff preparation | `solana/scripts/sojourn9/prepareCurrentDayProof.mjs`, `prepareCurrentDayProof.test.mjs`, package alias `sojourn9:prepare-proof` | Locally covered; creates a same-day demo witness outside the repo, runs SP1 prove, re-verifies the saved proof, checks public HashSeal existence, writes a no-secret handoff manifest, refuses mainnet, refuses repo-local output, and prints only placeholder keypair/secret/API-key values |
| Proof handoff status checking | `solana/scripts/sojourn9/checkProofHandoff.mjs`, `checkProofHandoff.test.mjs`, package alias `sojourn9:handoff-status` | Locally covered; reads only public manifest/receipt metadata, never reads or prints the witness path, refuses mainnet and secret-shaped CLI options, checks public `HashSeal` and `VerifiedSeal` PDAs, optionally reads public backend seal/score status, and prints the next safe operator action; if either backend status endpoint fails, generated HashSeal, VerifiedSeal, and Helius commands avoid backend posting during the critical chain/indexing step and print separate post-after-reachable commands |
| Deterministic `proof_hash` derivation | `solana/anky-zk-proof/src/lib.rs`, `recordVerifiedAnky.mjs` recomputation | Locally covered |
| Authority-gated `record_verified_anky` submission | Anchor program plus `recordVerifiedAnky.mjs` | Runnable dry-run/preflight only; live send requires verifier authority, matching `HashSeal`, and explicit `--sp1-proof-verified`; backend metadata writes require a backend write secret before chain/keypair work and require a matching landed `VerifiedSeal` account check |
| SP1 proof verification before `record_verified_anky` | `solana/anky-zk-proof/sp1/script/src/bin/main.rs`, `proveAndRecordVerified.mjs`, `recordVerifiedAnky.mjs`, focused tests | Locally covered for proof artifacts: the SP1 CLI supports `--verify --proof <proof-with-public-values.bin>` and the wrapper can use `--proof` to verify saved public proof artifacts before invoking the operator; `--send` is refused without `--sp1-proof-verified`; the wrapper refuses `--send` with `--sp1-mode execute`, refuses `--send` with raw `--receipt`, and auto-passes the guard only after `--sp1-mode prove` or saved-proof verification; raw existing-receipt sends remain possible only through the lower-level operator as explicit manual attestations with public chain preflight |
| Helius Sender policy for any future mainnet verified receipt send | `solana/anky-seal-program/scripts/recordVerifiedAnky.mjs`, `launchReadinessGate.mjs` | Locally covered as a dormant-path source gate only; the operator still refuses mainnet unless `ANKY_ALLOW_MAINNET_RECORD_VERIFIED=true` is set, but if that gate is explicitly opened the script uses Helius Sender, adds the 0.0002 SOL Sender tip, requests a live Helius priority fee estimate, includes `ComputeBudgetProgram.setComputeUnitPrice`, and submits with `skipPreflight: true` |
| Backend public VerifiedSeal persistence | `src/routes/mobile_sojourn.rs`, `migrations/019_credit_ledger_entries.sql`, `migrations/020_mobile_verified_seal_receipts.sql`, `migrations/021_mobile_helius_webhook_events.sql`, `migrations/022_mobile_helius_webhook_signature_dedupe.sql`, `solana/scripts/sojourn9/smokeVerifiedSealMigration.mjs`, backend runbook | Locally covered; migrations not applied to target backend here; route and DB require matching seal identity; route requires configured proof verifier authority; matching seal receipt must be `confirmed` or `finalized`; public seal/verified rows preserve UTC day when provided; public/mobile seal receipts can record non-finalized lifecycle states without a secret, but `finalized` seal writes are indexer/operator-secret-gated before they can affect Score V1; finalized seal rows are sticky and cannot be downgraded or overwritten by later public/mobile conflict writes; verified receipt metadata accepts only landed statuses (`confirmed` or `finalized`); verified receipt upserts are idempotent only and reject conflicting proof hash, verifier, UTC day, protocol, or signature for an existing wallet/hash; launch backend can require finalized on-chain `VerifiedSeal` account matching via `ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true`; reusable disposable Postgres smoke verifies clean and partial-table migration paths plus Helius webhook receipt guards and route-style immutable upsert behavior |
| Mobile `Sealed -> Proving -> Verified/Failed` state | `apps/anky-mobile/src/lib/ankyState.ts`, `apps/anky-mobile/src/lib/ankyStorage.ts`, `apps/anky-mobile/src/lib/solana/types.ts`, `RevealScreen.tsx`, tests | Locally covered; focused mobile tests passed for protocol, seal ABI, storage proof states, and verifier/UTC-day proof validation; proof-verified UI state requires expected verifier, protocol version 1, proof hash, proof transaction signature, proof UTC day, and matching seal UTC day; readiness gate checks that the visible mobile copy says `proof verified` only for proof state and describes the seal send as a `hash seal`, not SP1 proof |
| Mobile/backend verifier authority config visibility | `apps/anky-mobile/src/lib/config/env.ts`, `apps/anky-mobile/src/lib/solana/sojourn9Program.ts`, `apps/anky-mobile/src/lib/solana/mobileSolanaConfig.ts`, `src/routes/mobile_sojourn.rs` | Locally covered; mobile public env bundling now includes `EXPO_PUBLIC_ANKY_PROOF_VERIFIER_AUTHORITY`; mobile typecheck, Sojourn 9 config tests, and backend config serialization tests include the verifier authority; storage/reveal proof state rejects unexpected proof verifiers using runtime config where available; public config routes use `ANKY_PUBLIC_SOLANA_RPC_URL`/`EXPO_PUBLIC_SOLANA_RPC_URL`, not private server `ANKY_SOLANA_RPC_URL` |
| Helius/RPC event indexing | `solana/scripts/indexer/ankySealIndexer.mjs`, `solana/scripts/indexer/heliusWebhookManifest.mjs`, `src/routes/mobile_sojourn.rs`, fixture/test suite, Helius runbook | Fixture runnable; live backfill now requires `HELIUS_API_KEY` or a Helius `ANKY_SOLANA_RPC_URL` and uses `getTransactionsForAddress` with explicit `commitment: finalized`; no public RPC history fallback; no Helius webhook created here; webhook manifest can be generated without reading API keys and now prints live-doc operational warnings for 24-hour retry/backoff monitoring, localhost tunnel limits, and high-delivery-failure auto-disable/re-enable behavior; readiness gate checks that the backend receiver route, manifest usage, and runbook all use `/api/helius/anky-seal`, that the backend accepts Helius `Authorization: Bearer <secret>` from webhook `authHeader`, and that public launch evidence records the monitored program account, receiver path, Helius method, and finalized backfill commitment; backend receiver stores public webhook payloads only behind the indexer secret, rejects private-looking `.anky` fields plus complete valid `.anky` plaintext string values, dedupes valid Helius transaction retries by partial unique `(network, signature)`, and falls back to `(network, payload_hash)` when no valid signature is present; stored `payload_json`/`payloadJson` rows can be exported directly into the indexer; program/verifier config is decoded as 32-byte public keys; Anchor log events, Helius enhanced instruction payloads, decoded event fixtures, and public operator metadata are validated; failed transactions are excluded; scored/backend-posted events require real 64-byte Solana signatures; non-backfill input with missing commitment/finality is not scored by default; backfill events whose response omits commitment are annotated as inferred from the finalized request; verified events are filtered by configured verifier authority and protocol version; unknown CLI flags, secret-shaped `--input`/`--out` paths, direct `.anky` witness paths, invalid clusters, credentialed or non-local plaintext backend URLs, and mainnet runs that would use devnet defaults are rejected |
| Score snapshot artifact audit | `solana/scripts/indexer/auditScoreSnapshot.mjs`, `auditScoreSnapshot.test.mjs`, package alias `sojourn9:audit-snapshot`, Helius runbook | Locally covered; auditor reads only public snapshot JSON, rejects private/plaintext-like fields and complete `.anky` plaintext-looking string values under generic fields, enforces finalized launch snapshots by default, recomputes Score V1 from events, verifies summary counts and deterministic ordering, checks the configured verifier/protocol policy, enforces the 3,456 participant cap, and verifies allocation totals when `--require-allocation` is set |
| Public launch evidence audit | `solana/scripts/sojourn9/auditLaunchEvidence.mjs`, `auditLaunchEvidence.test.mjs`, package alias `sojourn9:audit-evidence`, `launchReadinessGate.mjs` | Locally covered; auditor reads only a public evidence JSON file, can print a no-secret evidence template, rejects template files as final evidence until `templateOnly` is removed, rejects `.env`/keypair/wallet/deployer evidence paths, rejects private/plaintext-like fields and complete `.anky` plaintext-looking values, requires real 64-byte Solana signatures with Orb transaction links for landed devnet seal and verified transactions, requires finalized Score V1 audit markers, requires concrete Helius webhook/backfill evidence (`webhookAccountAddresses`, `receiverPath`, `backfillMethod`, and `backfillCommitment`), and keeps devnet evidence from claiming mainnet deployment or direct on-chain SP1 |
| Public launch evidence build | `solana/scripts/sojourn9/makeLaunchEvidence.mjs`, `makeLaunchEvidence.test.mjs`, package alias `sojourn9:make-evidence`, `launchReadinessGate.mjs` | Locally covered; builder reads only public handoff manifest metadata, never reads the private witness, requires real landed seal and verified signatures, derives Orb links, copies valid explicit UTC-day status, derives UTC-day status from legacy public handoff metadata when explicit `utcDayStatus` is absent, refuses mainnet, requires explicit `--score-audited` or runs `auditScoreSnapshot.mjs --require-allocation` through `--audit-score-snapshot`, requires `--backfill-audited`, emits concrete public Helius reproduction fields for the monitored program account, receiver path, Helius method, and finalized backfill commitment, rejects secret-shaped paths and credentialed backend URLs, and can run `auditLaunchEvidence.mjs` before writing a public evidence file |
| Finalized-data scoring | `ankySealIndexer.mjs` default behavior and tests, `GET /api/mobile/seals/score`, `LoomScreen.tsx` | Locally covered; snapshot must be regenerated from finalized launch data and published verifier authority; backend live score view counts only finalized seal rows plus finalized matching verified rows for the configured verifier and protocol version 1; mobile Loom screen surfaces the backend indexed score when a wallet and backend are configured |
| Score V1 allocation | `score = unique_seal_days + verified_days + 2 * floor(each_consecutive_day_run / 7)` in indexer/runbook | Locally covered; allocation defaults to an auditable 3,456 participant cap with deterministic score-desc/wallet-asc tie handling; token supply/snapshot time still launch inputs |
| Machine-readable launch gate | `solana/scripts/sojourn9/launchReadinessGate.mjs` | Locally covered; reports artifact presence, executes the no-secret privacy guard, keeps launch blocked until live devnet/mainnet/operator gates are satisfied, and separates documented future-hardening limitations such as direct on-chain SP1/Groth16 verification from human-owned launch gates |
| Mainnet honesty | Mainnet gates in operator/backend/mobile config and runbooks | Covered locally; mainnet deployment/status is not claimed |
| Separate mainnet launch checklist | `runbooks/sojourn9-mainnet-launch-checklist.md`, `launchReadinessGate.mjs`, `launchReadinessGate.test.mjs` | Locally covered; checklist is no-secret and explicitly does not authorize mainnet signing, deployment, paid Helius changes, or public mainnet claims before real signatures and published public values exist |
| Public claim hygiene | `HACKATHON.md`, `docs/local-first-protocol.md`, `launchReadinessGate.mjs` | Locally covered; readiness gate rejects reintroduced legacy-pipeline wording and stale hash/seal overclaims in public docs |
| Stop rules | No secrets read, no keypairs printed, no mainnet tx/deploy, no paid API operations | Respected in this worktree session |

## Verified Commands

Latest final sanity pass run from this worktree on 2026-05-06:

```bash
git diff --check
node solana/scripts/sojourn9/privacyGuard.mjs
node solana/scripts/sojourn9/launchReadinessGate.mjs
cd solana/anky-seal-program && npm run sojourn9:test
cargo check
cargo test
cd solana/anky-seal-program && cargo test --manifest-path Cargo.toml --package anky_seal_program
cd solana/anky-seal-program && npm run typecheck
cd solana/anky-seal-program && npm run build
cd solana/anky-zk-proof && cargo test
cd apps/anky-mobile && npm run typecheck
cd apps/anky-mobile && npm test
```

Observed latest results at `2026-05-06T11:58:41Z`:

- `git diff --check` passed.
- Public launch evidence auditor tests passed: 7 tests. The audit report summary now exposes public UTC-day status fields: `devnetUtcDay`, `devnetSealWindow`, and `devnetDayRolloverAt`.
- Focused public evidence/gate tests passed: 18 tests across `makeLaunchEvidence`, `auditLaunchEvidence`, and `launchReadinessGate`. The public evidence builder copies valid manifest `utcDayStatus`, derives UTC-day status from legacy public handoff metadata when explicit `utcDayStatus` is absent, rejects manifest UTC-day status that disagrees with the public receipt day, emits concrete Helius reproduction fields, and the readiness gate source-checks the no-secret public evidence builder/auditor surfaces.
- Privacy guard passed with `ok: true`, 23 checked files, and no issues; it now source-checks the public launch evidence builder/auditor and score snapshot auditor direct `.anky` path refusal in addition to the Helius indexer secret-path, credentialed-backend, and explicit-mainnet-config guards.
- `launchReadinessGate` reported `localReady: true` and `launchReady: false`; it now has source checks that the public launch evidence builder, public launch evidence auditor, Helius score snapshot auditor, and Helius indexer reject secret-shaped paths and direct `.anky` witness paths, and that the indexer rejects unsafe backend URLs plus explicit mainnet program/verifier config.
- Focused public audit/privacy/readiness tests passed: 28 tests across `makeLaunchEvidence`, `auditLaunchEvidence`, `auditScoreSnapshot`, `launchReadinessGate`, and `privacyGuard`.
- Full Sojourn 9 JS operator/indexer/script suite passed: 156 tests. The suite includes a public launch evidence auditor regression that prints a no-secret template, rejects that template as final evidence until `templateOnly` is removed, rejects secret-shaped evidence paths, direct `.anky` evidence paths, private/plaintext-looking fields, complete `.anky` plaintext values, fixture signatures, non-Orb transaction links, missing or inconsistent UTC-day status evidence, missing finalized Score V1 markers, and missing or vague Helius audit markers; evidence-builder regressions that derive Orb links from signatures, derive UTC-day status from legacy public handoff metadata, copy valid explicit UTC-day status, emit concrete Helius reproduction fields, reject inconsistent explicit UTC-day status, and reject secret or direct `.anky` manifest paths; score snapshot auditor regressions that reject direct `.anky` snapshot paths before reading; indexer CLI regressions that reject secret-shaped input/output paths, direct `.anky` witness paths, invalid clusters, credentialed or non-local plaintext backend URLs, and mainnet runs that would use devnet defaults; a readiness-gate regression that forces `localReady: false` when privacy guard execution fails; readiness-gate checks for the no-secret mainnet launch checklist, public launch evidence builder/auditor, Helius score snapshot auditor, Helius webhook manifest delivery caveats, Helius webhook receiver route consistency, Helius `authHeader` bearer-secret compatibility, concrete Helius launch evidence reproduction fields, mobile hash-seal-vs-SP1-proof wording, and UTC-day rollover source surfaces; a webhook-manifest regression that prints Helius retry/backoff, high-delivery-failure auto-disable, and localhost-delivery warnings without reading `HELIUS_API_KEY`; a handoff command regression that requires Helius backfill mode and backend public config while preventing unsupported indexer flags; a stale-proof regeneration regression that preserves known public Loom/backend/program inputs; a VerifiedSeal handoff command regression that pins the manifest program ID; HashSeal/proof-prep regressions that pin the proved program ID in generated handoff commands; a proof-prep regression that keeps chain sends separate from backend metadata follow-up commands; and handoff regressions that avoid adding `--backend-url` to HashSeal, VerifiedSeal, and Helius critical-step commands unless both public backend status reads pass.
- Backend `cargo check` passed, with pre-existing unrelated warning noise.
- Full backend `cargo test` passed: 54 tests, including verified seal, Helius webhook privacy/dedupe, finalized seal receipt, Score V1, and private/public Solana RPC config tests.
- Anchor Rust tests passed: 9 tests, including public devnet Core asset/collection parser cases.
- Anchor TypeScript typecheck passed.
- Anchor build passed with the existing Anchor `AccountInfo::realloc` deprecation warning.
- SP1 proof library tests passed: 7 tests, including receipt serialization not including the private witness.
- Mobile TypeScript typecheck passed.
- Mobile Vitest passed: 15 files, 94 tests.
- Temporary `apps/anky-mobile/node_modules` symlink was removed after mobile checks.
- A later continuation audit at `2026-05-06T10:16:44Z` reran the machine-readable readiness gate and current proof handoff status. The readiness gate still reported `localReady: true` and `launchReady: false`; the handoff status still reported `proofExists: true`, `receiptExists: true`, `verifiedReceiptExists: true`, `witnessRead: false`, and `nextAction: send_hashseal`. Because the backend status checks failed, the generated HashSeal command is chain-only and pinned to the manifest program ID, with a separate backend metadata post command for after both public backend status reads are healthy; the same split is covered for later VerifiedSeal and Helius handoff branches.

Earlier local sanity pass from this worktree on 2026-05-06:

```bash
node --test solana/scripts/sojourn9/liveE2eChecklist.test.mjs solana/scripts/sojourn9/launchReadinessGate.test.mjs
node --test solana/scripts/indexer/*.test.mjs solana/scripts/sojourn9/*.test.mjs solana/anky-seal-program/scripts/*.test.mjs
node solana/scripts/sojourn9/privacyGuard.mjs
cd solana/anky-seal-program && npm run sojourn9:test
CARGO_TARGET_DIR=/tmp/anky-root-target cargo test mobile_sojourn -- --nocapture
CARGO_TARGET_DIR=/tmp/anky-root-target cargo test finalized_public_seal -- --nocapture
cd solana/anky-seal-program && npm run check-config -- --cluster devnet
cd apps/anky-mobile && npm test
cd apps/anky-mobile && npm run typecheck
node solana/scripts/sojourn9/launchReadinessGate.mjs
git diff --check
```

Observed earlier results:

- Focused proof-prep/readiness Node tests passed: 6 tests.
- Full local JS operator/indexer/sojourn9 script suite passed: 125 tests, including through the `npm run sojourn9:test` package alias.
- Privacy guard passed with `ok: true`, 13 checked files, and no issues.
- Backend `mobile_sojourn` Rust tests passed: 34 tests, with pre-existing unrelated warning noise.
- Focused finalized public seal metadata regressions passed: 2 tests.
- Public devnet config check passed: seal program executable, Core collection owned by Metaplex Core with CollectionV1 discriminator, proof verifier public key valid.
- Mobile Vitest passed: 15 files, 93 tests.
- Mobile TypeScript typecheck passed.
- `launchReadinessGate` reported `localReady: true` and `launchReady: false`.
- `git diff --check` passed.
- Temporary `apps/anky-mobile/node_modules` symlink was removed after mobile checks.

Broader safe local checks run earlier from this worktree on 2026-05-06:

```bash
cd solana/anky-seal-program && npm run typecheck
cd solana/anky-seal-program && npm run build
CARGO_TARGET_DIR=/tmp/anky-anchor-target cargo test --manifest-path solana/anky-seal-program/Cargo.toml --package anky_seal_program
CARGO_TARGET_DIR=/tmp/anky-root-target cargo test
CARGO_TARGET_DIR=/tmp/anky-sp1-target cargo test --manifest-path solana/anky-zk-proof/Cargo.toml
PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc CARGO_TARGET_DIR=/tmp/anky-sp1-script-target cargo check --manifest-path solana/anky-zk-proof/sp1/script/Cargo.toml
node --test solana/anky-seal-program/scripts/checkLaunchConfig.test.mjs solana/anky-seal-program/scripts/sealAnky.test.mjs solana/anky-seal-program/scripts/recordVerifiedAnky.test.mjs solana/scripts/indexer/ankySealIndexer.test.mjs solana/scripts/indexer/heliusWebhookManifest.test.mjs solana/scripts/sojourn9/launchReadinessGate.test.mjs solana/scripts/sojourn9/makeDemoAnky.test.mjs solana/scripts/sojourn9/proveAndRecordVerified.test.mjs solana/scripts/sojourn9/smokeVerifiedSealMigration.test.mjs
cd apps/anky-mobile && npm test -- --run src/lib/ankyStorage.test.ts src/lib/solana/types.test.ts src/lib/solana/sealAnky.test.ts src/lib/solana/sojourn9Program.test.ts
git diff --check
node solana/scripts/sojourn9/launchReadinessGate.mjs
```

Observed broader results:

- Anchor TypeScript typecheck passed.
- Anchor build passed with the existing Anchor `AccountInfo::realloc` deprecation warning.
- Anchor Rust tests passed: 9 tests.
- Full backend Rust tests passed: 54 tests, with pre-existing warning noise in unrelated modules.
- SP1 proof library tests passed: 7 tests.
- SP1 script compile passed.
- Node launch/seal/operator/indexer/webhook/migration/readiness tests passed locally.
- Focused mobile tests passed: 4 files, 22 tests.
- `git diff --check` passed.
- `launchReadinessGate` reported `localReady: true` and `launchReady: false`.

Latest public devnet read-only checks, refreshed after the final sanity pass:

```bash
cd solana/anky-seal-program && npm run build
cd solana/anky-seal-program && npm run check-config -- --cluster devnet
cd solana/anky-seal-program && npm run check-config -- --cluster devnet --loom-asset 4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9 --loom-owner 9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp
cd solana/anky-seal-program && npm run sojourn9:privacy
cd solana/anky-seal-program && npm run sojourn9:readiness
cd solana/anky-seal-program && npm run sojourn9:test
cd solana/anky-seal-program && npm run sojourn9:demo-witness -- --help
cd solana/anky-seal-program && npm run sojourn9:live-checklist -- --help
cd solana/anky-seal-program && npm run sojourn9:live-checklist -- --writer 9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp --loom-asset 4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9 --session-hash 28a7b5c28dbed9f0047321860dd6b060fe3fd7fce15480621e1eb65276a659e1 --utc-day 20579 --backend-url http://127.0.0.1:3000
cd solana/anky-seal-program && npm run sojourn9:prepare-proof -- --help
cd solana/anky-seal-program && npm run sojourn9:prepare-proof -- --writer 9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp --loom-asset 4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9 --backend-url http://127.0.0.1:3000
cd solana/anky-seal-program && npm run sojourn9:handoff-status -- --help
cd solana/anky-seal-program && npm run sojourn9:handoff-status -- --manifest /tmp/anky-sojourn9-current-DXfFDY/handoff-manifest.json
cd solana/anky-seal-program && npm run sojourn9:prove-record -- --help
cd solana/anky-seal-program && npm run sojourn9:index -- --help
cd solana/anky-seal-program && npm run sojourn9:audit-snapshot -- --help
cd solana/anky-seal-program && npm run sojourn9:audit-evidence -- --help
cd solana/anky-seal-program && npm run sojourn9:make-evidence -- --help
cd solana/anky-seal-program && npm run sojourn9:webhook-manifest -- --help
```

Observed devnet results:

- Seal program `4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX` exists and is executable on devnet.
- Core collection `F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u` exists, is owned by Metaplex Core, and has the expected CollectionV1 discriminator.
- Proof verifier `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP` is a valid public key.
- Known devnet Loom asset `4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9` exists, is owned by Metaplex Core, has the expected AssetV1 discriminator, belongs to the configured collection, uses collection update authority, and is owned by `9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp`.
- Operator package aliases for readiness, privacy, demo witness generation, live checklist generation, proof handoff status, proof recording, indexing, and webhook manifest help ran successfully from `solana/anky-seal-program`.
- The snapshot auditor passed against a fixture-generated finalized Score V1 snapshot with `--require-allocation`, and the full JS suite now covers score-recompute failure, non-finalized snapshot rejection, private-field rejection, and unknown secret-shaped options.
- A current-day no-secret live checklist smoke for UTC day `20579` printed the expected devnet command sequence for the known devnet Loom owner/asset, with placeholders for writer keypair, verifier keypair, backend write secret, and Helius API key.
- A current-day no-secret proof-prep smoke wrote `/tmp/anky-sojourn9-current-DXfFDY/handoff-manifest.json` and confirmed `proofVerified: true` while `hashSeal.exists: false`; because public `--loom-asset` was provided, the manifest printed an exact devnet `seal_anky` command with placeholders only for the writer keypair path. The latest source regression keeps regenerated proof-prep chain sends separate from backend metadata follow-up commands even when `--backend-url` is supplied.
- A no-secret proof handoff status check against `/tmp/anky-sojourn9-current-DXfFDY/handoff-manifest.json` read only public manifest/receipt metadata, reported `witnessRead: false`, reported `utcDayStatus.sealWindow: open` with UTC rollover at `2026-05-07T00:00:00.000Z`, confirmed both public chain checks are still blocked by the missing `HashSeal`, and returned `nextAction: send_hashseal`.

SP1 proof commands verified locally:

```bash
cd solana/anky-zk-proof/sp1/script && PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc RUST_LOG=info cargo run --release -- --execute --file ../../fixtures/full.anky --writer 11111111111111111111111111111111 --receipt-out /tmp/anky-sp1-execute-receipt.json
cd solana/anky-zk-proof/sp1/script && PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc RUST_LOG=info cargo run --release -- --prove --file ../../fixtures/full.anky --writer 11111111111111111111111111111111 --receipt-out /tmp/anky-sp1-prove-receipt.json --proof-out /tmp/anky-sp1-proof-with-public-values.bin
cd solana/anky-zk-proof/sp1/script && PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc RUST_LOG=info cargo run --release -- --verify --proof /tmp/anky-sp1-proof-with-public-values.bin --receipt-out /tmp/anky-sp1-verified-receipt-from-proof-latest.json
cd solana/anky-zk-proof/sp1/script && PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc cargo run --release --bin vkey
cd solana/anky-seal-program && npm run record-verified -- --receipt /tmp/anky-sp1-prove-receipt.json --writer 11111111111111111111111111111111 --cluster devnet
```

Current-day SP1 proof handoff refreshed on 2026-05-06:

```bash
cd solana/anky-seal-program && npm run sojourn9:prepare-proof -- --writer 9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp --loom-asset 4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9 --backend-url http://127.0.0.1:3000
```

Observed current-day proof result: the no-secret proof-prep manifest at `/tmp/anky-sojourn9-current-DXfFDY/handoff-manifest.json` reported `proofVerified: true` for writer `9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp`, session hash `881ecaf0685337bdc2c92778d60464d0b00363b5e07995d3bec3c5241d845865`, UTC day `20579`, and proof hash `38154c2b641335180ac313c8081f29f0e4f0e394084e901497de3b4690cfa982`. It derived HashSeal PDA `5yXopHUUp881bvWxzr2HTRgnrZHPHQVVNxtydcLQUD7c` and VerifiedSeal PDA `2fLbHJh1eWpoYgAQ2qUdP4ZKDerwqyi4YeThSXZajMsg`. The public devnet preflight reported `hashSeal.exists: false` with `HashSeal preflight failed: matching HashSeal account does not exist`, so the next human-owned step is still the matching same-day `seal_anky` transaction.

Continuation handoff status at `2026-05-06T10:22:04Z` confirmed the same blocker: the matching `HashSeal` and `VerifiedSeal` chain checks both fail because the `HashSeal` account does not exist. The backend checks against `http://127.0.0.1:3000` also failed because no backend was running in this shell. The status checker did not read the witness and printed the exact next command as a chain-only send, pinned to the manifest program ID:

```bash
cd solana/anky-seal-program && ANKY_SEALER_KEYPAIR_PATH='<writer_keypair_path>' npm run seal -- --loom-asset 4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9 --session-hash 881ecaf0685337bdc2c92778d60464d0b00363b5e07995d3bec3c5241d845865 --utc-day 20579 --cluster devnet --program-id 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX --check-chain --send
```

After the transaction lands and the backend is reachable, post public seal metadata separately:

```bash
cd solana/anky-seal-program && npm run seal -- --writer 9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp --loom-asset 4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9 --session-hash 881ecaf0685337bdc2c92778d60464d0b00363b5e07995d3bec3c5241d845865 --utc-day 20579 --cluster devnet --program-id 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX --check-sealed-chain --backend-signature '<landed_seal_signature>' --backend-url http://127.0.0.1:3000
```

The verified receipt and Helius webhook migrations were also smoke-tested against disposable local Postgres clusters: migrations `017`, `019`, `020`, and `021` applied cleanly, repeated `019`, `020`, and `021` applied cleanly, `utc_day` columns were present, verified receipt hash/protocol/status constraints existed, pending verified receipt inserts were rejected, verified receipt unique indexes existed, Helius webhook constraints existed, Helius signature retry dedupe used a partial unique `(network, signature)` index, finalized-only backend score SQL counted only matching verifier/protocol verified days, and the matching-seal FK existed. A partial pre-existing verified table smoke confirmed missing constraints and indexes are added.

Expected failing preflight:

```bash
cd solana/anky-seal-program && npm run record-verified -- --receipt /tmp/anky-sp1-prove-receipt.json --writer 11111111111111111111111111111111 --cluster devnet --check-chain
```

This failed correctly because the historical fixture receipt has no matching devnet `HashSeal` and cannot be newly sealed on the current UTC day.

## Not Yet Achieved

- One live devnet CLI `seal_anky` -> SP1 prove/verify -> `record_verified_anky` loop has landed for the edited `0xx1` witness, but this is not a launch completion certificate.
- No target backend public metadata/status has been posted or verified for the live `0xx1` seal and verified receipt.
- No target backend migrations/config were applied from this shell.
- No verifier-authority keypair was accessed by Codex. The human-operated send succeeded, but custody and operational runbooks still need explicit launch approval.
- No Helius webhook was created.
- Helius known-signature finalized reconciliation worked for the live transaction pair, but program-address backfill was still empty on devnet, so full backfill/webhook coverage remains unresolved.
- The DB migration was prepared but not applied to a running backend here.
- Mainnet program ID, Core collection, verifier authority custody, funding, and snapshot time are not confirmed.
- Direct on-chain SP1 verification is not implemented.
- A live mobile-device demo has not been run from this shell.
- A live passing Core Loom integration test result against the active launch collection is still missing.
  The harness now exists as an opt-in Anchor test and must be run with `ANKY_CORE_INTEGRATION_LOOM_ASSET` set to an owned devnet Loom before mainnet confidence.
- The same-day `HashSeal` CLI helper is prepared and mocked-RPC tested, but Codex did not run `--send` because that requires a writer keypair and devnet funds.

## Exact Next Human Actions

1. Apply `migrations/019_credit_ledger_entries.sql`, `migrations/020_mobile_verified_seal_receipts.sql`, `migrations/021_mobile_helius_webhook_events.sql`, and `migrations/022_mobile_helius_webhook_signature_dedupe.sql` to the target backend database. VerifiedSeal backend readiness specifically requires `020`, `021`, and `022`; `019_credit_ledger_entries` is part of the full backend migration chain.
2. Configure the launch backend with `ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true`, private `ANKY_SOLANA_RPC_URL`, public mobile RPC vars, and the proof verifier authority, then post or index the live `0xx1` public seal/verified metadata.
3. Configure live Helius delivery or finalized backfill outside Codex. The known-signature snapshot proves the parser/scorer can reconcile the demo pair, but the program-address backfill must be made reliable for launch scoring.
4. Run the opt-in Core integration test in `runbooks/sojourn9-core-seal-integration.md` against an owned real devnet Loom.
5. Run the live phone/mobile flow against the chosen devnet config and verify the UI shows `Sealed`, `Proving`, indexed score, and `Verified` from public backend/indexer state.
6. Build and audit public launch evidence with real backend URL, public Helius webhook ID or audited finalized backfill evidence, the live seal/verified signatures, and the score snapshot.
7. Only after the devnet loop, backend status, live indexing, mobile demo, and evidence audit are complete, fill mainnet program/collection/verifier/snapshot values and run the separate mainnet launch checklist.
