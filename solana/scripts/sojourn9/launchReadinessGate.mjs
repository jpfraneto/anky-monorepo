#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { redactSecretValues } from "./redactSecrets.mjs";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_REPO_ROOT = path.resolve(SCRIPT_DIR, "../../..");
const BOOLEAN_FLAGS = new Set([]);
const VALUE_FLAGS = new Set(["--repo-root"]);
const LOCAL_FAILURE_STATUSES = new Set([
  "failed",
  "forbidden_match",
  "missing",
  "unmatched",
]);

const LOCAL_ARTIFACTS = [
  ["Anchor seal program", "solana/anky-seal-program/programs/anky-seal-program/src/lib.rs"],
  ["Opt-in Core seal integration test", "solana/anky-seal-program/tests/anky-seal-program.ts"],
  ["Same-day HashSeal operator", "solana/anky-seal-program/scripts/sealAnky.mjs"],
  ["VerifiedSeal operator", "solana/anky-seal-program/scripts/recordVerifiedAnky.mjs"],
  ["Launch config checker", "solana/anky-seal-program/scripts/checkLaunchConfig.mjs"],
  ["SP1 proof library", "solana/anky-zk-proof/src/lib.rs"],
  ["SP1 wrapper", "solana/scripts/sojourn9/proveAndRecordVerified.mjs"],
  ["Demo witness generator", "solana/scripts/sojourn9/makeDemoAnky.mjs"],
  ["Public launch evidence builder", "solana/scripts/sojourn9/makeLaunchEvidence.mjs"],
  ["Public launch evidence auditor", "solana/scripts/sojourn9/auditLaunchEvidence.mjs"],
  ["Live E2E checklist helper", "solana/scripts/sojourn9/liveE2eChecklist.mjs"],
  ["Current-day proof handoff helper", "solana/scripts/sojourn9/prepareCurrentDayProof.mjs"],
  ["Proof handoff status checker", "solana/scripts/sojourn9/checkProofHandoff.mjs"],
  ["Secret redaction utility", "solana/scripts/sojourn9/redactSecrets.mjs"],
  ["Sojourn 9 privacy guard", "solana/scripts/sojourn9/privacyGuard.mjs"],
  ["Verified receipt migration", "migrations/019_mobile_verified_seal_receipts.sql"],
  ["Helius webhook receipt migration", "migrations/020_mobile_helius_webhook_events.sql"],
  [
    "Helius webhook signature dedupe migration",
    "migrations/021_mobile_helius_webhook_signature_dedupe.sql",
  ],
  ["Credit ledger migration", "migrations/022_credit_ledger_entries.sql"],
  ["Verified receipt migration smoke", "solana/scripts/sojourn9/smokeVerifiedSealMigration.mjs"],
  ["Helius event indexer", "solana/scripts/indexer/ankySealIndexer.mjs"],
  ["Helius score snapshot auditor", "solana/scripts/indexer/auditScoreSnapshot.mjs"],
  ["Helius webhook manifest", "solana/scripts/indexer/heliusWebhookManifest.mjs"],
  ["Mobile proof-state types", "apps/anky-mobile/src/lib/solana/types.ts"],
  ["Mobile seal storage", "apps/anky-mobile/src/lib/ankyStorage.ts"],
  ["Mobile backup manifest privacy filter", "apps/anky-mobile/src/lib/ankyBackupManifest.ts"],
  ["Mobile reveal proof state", "apps/anky-mobile/src/screens/RevealScreen.tsx"],
  ["Backend mobile routes", "src/routes/mobile_sojourn.rs"],
  ["SP1 runbook", "runbooks/sojourn9-sp1-verifiedseal.md"],
  ["Helius indexing runbook", "runbooks/sojourn9-helius-indexing.md"],
  ["Backend VerifiedSeal runbook", "runbooks/sojourn9-backend-verifiedseal.md"],
  ["Core seal integration runbook", "runbooks/sojourn9-core-seal-integration.md"],
  ["Launch readiness audit", "runbooks/sojourn9-launch-readiness-audit.md"],
  ["Live devnet 0xx1 evidence artifact", "runbooks/devnet-0xx1-live-e2e-evidence.json"],
  ["Mainnet launch checklist", "runbooks/sojourn9-mainnet-launch-checklist.md"],
];

const SOURCE_CHECKS = [
  {
    name: "Same-day HashSeal operator refuses stale days and mainnet",
    path: "solana/anky-seal-program/scripts/sealAnky.mjs",
    required: [
      "seal_anky preflight requires the current UTC day",
      "Refusing mainnet seal_anky",
      "Core Loom asset owner does not match writer",
      "HashSeal account does not match writer, Loom asset, session hash, and UTC day",
      "--check-sealed-chain is required before posting already-landed seal metadata",
      "postBackendSeal",
      "--send",
    ],
  },
  {
    name: "Operator package exposes Sojourn 9 command aliases",
    path: "solana/anky-seal-program/package.json",
    required: [
      "\"sojourn9:demo-witness\"",
      "\"sojourn9:audit-evidence\"",
      "\"sojourn9:audit-snapshot\"",
      "\"sojourn9:handoff-status\"",
      "\"sojourn9:privacy\"",
      "\"sojourn9:make-evidence\"",
      "\"sojourn9:prove-record\"",
      "\"sojourn9:readiness\"",
      "\"sojourn9:test\"",
      "\"sojourn9:webhook-manifest\"",
      "\"sojourn9:index\"",
      "\"sojourn9:live-checklist\"",
      "\"sojourn9:prepare-proof\"",
    ],
  },
  {
    name: "Public launch evidence builder reads handoff metadata only",
    path: "solana/scripts/sojourn9/makeLaunchEvidence.mjs",
    required: [
      "This reads only public handoff metadata",
      "never reads the private .anky witness",
      "--score-audited is required after running sojourn9:audit-snapshot",
      "SCORE_AUDIT_SCRIPT",
      "deriveLegacyUtcDayStatus",
      "manifest.generatedAt must agree with manifest.currentUtcDay",
      "manifest.utcDayStatus.receiptUtcDay must match manifest.publicReceipt.utcDay",
      "--audit-score-snapshot requires --score-snapshot to point to an existing public JSON file",
      "--require-allocation",
      "--backfill-audited is required after finalized Helius backfill has been checked",
      "https://orbmarkets.io/tx/${sealSignature}",
      "makeLaunchEvidence is devnet-handoff only",
      "manifest.proofVerified must be true",
      "SECRET_PATH_RE",
      "\\.anky$",
      "resolvePublicPath(requiredArg(args, \"manifest\"), \"--manifest\")",
      "resolvePublicPath(args.out, \"--out\")",
      "spawnSync(process.execPath, [AUDIT_SCRIPT, \"--evidence\", evidencePath]",
    ],
  },
  {
    name: "Public launch evidence auditor reads only public receipts",
    path: "solana/scripts/sojourn9/auditLaunchEvidence.mjs",
    required: [
      "The audit reads only public launch evidence JSON",
      "private/plaintext-like field",
      "complete .anky plaintext-like value",
      "devnetE2E.sealSignature must be a real 64-byte Solana signature",
      "devnetE2E.sealOrbUrl must be an Orb transaction link",
      "devnetE2E.utcDayStatus",
      "receiptUtcDay must match devnetE2E.utcDay",
      "scoreSnapshot.requireFinalized must be true",
      "Helius webhook/backfill evidence is required",
      "helius.webhookAccountAddresses must contain only the Anky Seal Program ID",
      "helius.receiverPath must be /api/helius/anky-seal",
      "helius.backfillMethod must be getTransactionsForAddress",
      "helius.backfillCommitment must be finalized",
      "--evidence must point to a public JSON file",
      "SECRET_PATH_RE",
      "\\.anky$",
      "resolvePublicEvidencePath",
      "env/.anky/keypair/wallet/deployer file",
      "--print-template",
      "templateOnly must not be true in final launch evidence",
    ],
  },
  {
    name: "Live E2E checklist keeps human-key steps explicit",
    path: "solana/scripts/sojourn9/liveE2eChecklist.mjs",
    required: [
      "This live E2E checklist is devnet-only",
      "utc day",
      "utcDayStatus",
      "sealWindow",
      "secondsUntilRollover",
      "dayRolloverAt",
      "ANKY_SEALER_KEYPAIR_PATH=<writer_keypair_path>",
      "ANKY_VERIFIER_KEYPAIR_PATH=<verifier_authority_keypair_path>",
      "ANKY_INDEXER_WRITE_SECRET=<backend_write_secret>",
      "HELIUS_API_KEY=<configured_in_shell>",
      "Do not paste or print keypair JSON",
    ],
  },
  {
    name: "Current-day proof handoff prepares public artifacts only",
    path: "solana/scripts/sojourn9/prepareCurrentDayProof.mjs",
    required: [
      "Refusing to write demo witness or SP1 artifacts inside this git worktree",
      "Current-day proof preparation is devnet-only",
      "proofVerified",
      "HashSeal",
      "utcDayStatus",
      "sealWindow",
      "secondsUntilRollover",
      "dayRolloverAt",
      "Do not commit or upload the witness file",
      "ANKY_SEALER_KEYPAIR_PATH",
      "writer_keypair_path",
      "ANKY_VERIFIER_KEYPAIR_PATH",
      "verifier_authority_keypair_path",
    ],
  },
  {
    name: "Proof handoff status checker reads only public manifest metadata",
    path: "solana/scripts/sojourn9/checkProofHandoff.mjs",
    required: [
      "never reads the witness file",
      "witnessRead: false",
      "utcDayStatus",
      "sealWindow",
      "secondsUntilRollover",
      "dayRolloverAt",
      "Proof handoff status is devnet-only",
      "Unknown option",
      "ANKY_VERIFIER_KEYPAIR_PATH",
      "HELIUS_API_KEY",
    ],
  },
  {
    name: "SP1 saved proof verification mode",
    path: "solana/anky-zk-proof/sp1/script/src/bin/main.rs",
    required: ["--verify", "SP1ProofWithPublicValues::load", "client.verify(&proof"],
  },
  {
    name: "SP1 wrapper verifies saved proofs before operator flow",
    path: "solana/scripts/sojourn9/proveAndRecordVerified.mjs",
    required: [
      "--proof",
      "--verify",
      "--sp1-proof-verified",
      "--send is not allowed with raw --receipt",
    ],
  },
  {
    name: "Privacy guard checks proof/indexing plaintext boundaries",
    path: "solana/scripts/sojourn9/privacyGuard.mjs",
    required: [
      "launch receipt migrations must not add private/plaintext column",
      "Helius webhook payload must be privacy-validated before it is serialized for storage",
      "script must not log private .anky/witness variables",
      "operator/indexer script stderr must redact secret-looking values",
      "solana/scripts/sojourn9/launchReadinessGate.mjs",
      "solana/scripts/sojourn9/smokeVerifiedSealMigration.mjs",
      "mobile proof-state resolver must use only public seal/proof metadata",
    ],
  },
  {
    name: "VerifiedSeal send requires local SP1 proof verification",
    path: "solana/anky-seal-program/scripts/recordVerifiedAnky.mjs",
    required: [
      "if (send && args.sp1ProofVerified !== true)",
      "--sp1-proof-verified is required with --send",
    ],
  },
  {
    name: "Mainnet VerifiedSeal send follows Helius Sender policy when explicitly enabled",
    path: "solana/anky-seal-program/scripts/recordVerifiedAnky.mjs",
    required: [
      'const SENDER_ENDPOINT = "https://sender.helius-rpc.com/fast"',
      "const SENDER_TIP_LAMPORTS = 200_000",
      "SystemProgram.transfer",
      "ComputeBudgetProgram.setComputeUnitPrice",
      "getPriorityFeeEstimate",
      "skipPreflight: true",
      "maxRetries: 0",
    ],
  },
  {
    name: "Backend verified receipt upsert is immutable",
    path: "src/routes/mobile_sojourn.rs",
    required: [
      "mobile_verified_seal_receipts.proof_hash = EXCLUDED.proof_hash",
      "mobile_verified_seal_receipts.signature = EXCLUDED.signature",
      "verified seal metadata conflicts with an existing immutable receipt",
    ],
  },
  {
    name: "Backend finalized seal receipts require indexer secret",
    path: "src/routes/mobile_sojourn.rs",
    required: [
      "require_finalized_seal_record_secret",
      "status == \"finalized\"",
      "finalized seal metadata",
      "WHERE mobile_seal_receipts.status <> 'finalized'",
      "OR ($12 AND EXCLUDED.status = 'finalized')",
      "finalized seal metadata is immutable",
    ],
  },
  {
    name: "Backend Helius webhook rejects private .anky payloads",
    path: "src/routes/mobile_sojourn.rs",
    required: [
      "\"/api/helius/anky-seal\"",
      "post(record_helius_anky_seal_webhook)",
      "validate_public_webhook_payload",
      "find_private_webhook_field",
      "contains_anky_plaintext_value",
    ],
  },
  {
    name: "Backend Helius webhook dedupes valid transaction signatures",
    path: "src/routes/mobile_sojourn.rs",
    required: [
      "ON CONFLICT (network, signature) WHERE signature IS NOT NULL",
      "collect_public_webhook_signatures",
      "ON CONFLICT (network, payload_hash)",
    ],
  },
  {
    name: "Backend accepts Helius authHeader Authorization bearer secret",
    path: "src/routes/mobile_sojourn.rs",
    required: [
      "x-anky-indexer-secret",
      "axum::http::header::AUTHORIZATION",
      "authorization == expected || authorization == format!(\"Bearer {expected}\")",
    ],
  },
  {
    name: "Helius score snapshot auditor enforces finalized Score V1 artifacts",
    path: "solana/scripts/indexer/auditScoreSnapshot.mjs",
    required: [
      "snapshot.requireFinalized must be true for launch scoring",
      "scores do not recompute from finalized public events under Score V1",
      "private/plaintext-like field",
      "complete .anky plaintext-like value",
      "SECRET_PATH_RE",
      "\\.anky$",
      "resolvePublicSnapshotPath",
      "env/.anky/keypair/wallet/deployer file",
      "--require-allocation",
      "reward allocation is required",
    ],
  },
  {
    name: "Helius webhook migration has signature retry dedupe",
    path: "migrations/021_mobile_helius_webhook_signature_dedupe.sql",
    required: [
      "idx_mobile_helius_webhook_events_network_signature_unique",
      "ON mobile_helius_webhook_events(network, signature)",
      "WHERE signature IS NOT NULL",
    ],
  },
  {
    name: "Helius backfill requests finalized commitment explicitly",
    path: "solana/scripts/indexer/ankySealIndexer.mjs",
    required: [
      "const requestedCommitment = \"finalized\"",
      "commitment: requestedCommitment",
      "finalizedEventsInferredFromBackfillRequest",
    ],
  },
  {
    name: "Helius indexer rejects unsafe launch inputs",
    path: "solana/scripts/indexer/ankySealIndexer.mjs",
    required: [
      "SECRET_PATH_RE",
      "\\.anky$",
      "resolvePublicPath(args.input, \"--input\")",
      "resolvePublicPath(args.out, \"--out\")",
      "normalizeBackendUrl",
      "HTTPS URL without credentials unless it is localhost HTTP",
      "requireExplicitMainnetConfig",
      "mainnet-beta indexing requires an explicit --program-id",
      "mainnet-beta indexing requires an explicit --proof-verifier",
    ],
  },
  {
    name: "Helius runbook records webhook delivery and dedupe guidance",
    path: "runbooks/sojourn9-helius-indexing.md",
    required: [
      "Helius webhook guidance was checked live on 2026-05-06",
      "deduping by processed transaction signature",
      "retried with exponential backoff for up to 24 hours",
      "valid type set includes `enhanced`, `raw`, `discord`, `enhancedDevnet`, `rawDevnet`, and `discordDevnet`",
      "Helius supports `authHeader` as an optional Authorization header value sent with webhook requests",
      "do not create paid Helius resources or read `HELIUS_API_KEY`",
      "--webhook-url https://<public-backend-domain>/api/helius/anky-seal",
    ],
  },
  {
    name: "Helius webhook manifest prints live delivery caveats",
    path: "solana/scripts/indexer/heliusWebhookManifest.mjs",
    required: [
      "Dry-run only: this script does not call Helius and does not read Helius API keys.",
      "The receiver must return HTTP 200 and dedupe by transaction signature.",
      "Helius retries failed deliveries with exponential backoff for up to 24 hours",
      "monitor webhook logs and re-enable disabled webhooks",
      "Helius cannot deliver to private localhost",
      "authHeader: \"Bearer $ANKY_INDEXER_WRITE_SECRET\"",
      "--webhook-url https://your-domain.example/api/helius/anky-seal",
    ],
  },
  {
    name: "Backend score view uses finalized Score V1 public receipts",
    path: "src/routes/mobile_sojourn.rs",
    required: [
      "/api/mobile/seals/score",
      "status = 'finalized'",
      "score = unique_seal_days + verified_days + 2 * floor(each_consecutive_day_run / 7)",
    ],
  },
  {
    name: "Mobile backup excludes transient proof artifacts",
    path: "apps/anky-mobile/src/lib/ankyBackupManifest.ts",
    required: [
      "isBackupEligibleRelativePath",
      "TRANSIENT_PROOF_ARTIFACT_FILE_NAMES",
      "proof-with-public-values.bin",
      'fileName.endsWith(".anky")',
    ],
  },
  {
    name: "Mobile public env exposes proof verifier authority",
    path: "apps/anky-mobile/src/lib/config/env.ts",
    required: [
      '"EXPO_PUBLIC_ANKY_PROOF_VERIFIER_AUTHORITY"',
      "process.env.EXPO_PUBLIC_ANKY_PROOF_VERIFIER_AUTHORITY",
    ],
  },
  {
    name: "Mobile Loom screen surfaces indexed score",
    path: "apps/anky-mobile/src/screens/LoomScreen.tsx",
    required: [
      "lookupMobileSealScore",
      "indexed score",
      "backend score unavailable",
      "finalized receipts only",
      "hash seals, ownership, and verified receipts when they exist",
      "proof days",
    ],
    forbidden: ["solana records proof, ownership, and the ritual trace"],
  },
  {
    name: "Mobile reveal separates hash seal from SP1 proof state",
    path: "apps/anky-mobile/src/screens/RevealScreen.tsx",
    required: [
      "proof verified",
      "weaving your hash seal into the ankyverse",
      "sp1 receipt pending",
      "sp1 receipt failed",
    ],
    forbidden: ["weaving your proof into the ankyverse"],
  },
  {
    name: "Public Colosseum pitch uses Sojourn 9 truth claims",
    path: "HACKATHON.md",
    required: [
      "hashes the exact UTF-8 bytes",
      "Metaplex Core Loom",
      "verifier-authority-attested",
      "Not claimed yet: mainnet deployment",
    ],
    forbidden: [
      "AWS Nitro Enclave",
      "spl-memo",
      "Mirror cNFT",
      "backend is cryptographically blind",
      "The chain proves",
      "proof you did the practice",
    ],
  },
  {
    name: "Local-first protocol documents exact .anky hash and seal path",
    path: "docs/local-first-protocol.md",
    required: [
      "SHA-256 of exact `.anky` UTF-8 bytes",
      "Anky Seal Program `seal_anky` tx",
      "Reconstructed prose is not a valid hashing input",
    ],
    forbidden: ["SHA-256 of plaintext", "spl-memo"],
  },
  {
    name: "Mainnet launch checklist keeps signing and claims gated",
    path: "runbooks/sojourn9-mainnet-launch-checklist.md",
    required: [
      "Do not run this before the devnet SP1 -> VerifiedSeal -> Helius score loop has landed end-to-end",
      "This checklist does not authorize mainnet signing, deployment, webhook creation, or paid API changes",
      "Mainnet public values to publish before the season begins",
      "mainnet program ID",
      "Metaplex Core collection",
      "proof verifier authority",
      "snapshot time",
      "Do not claim mainnet deployment until the read-only checks and signed transactions have real signatures",
      "Use Orb links for public transaction/account references",
      "ANKY_ALLOW_MAINNET_RECORD_VERIFIED=true",
      "ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true",
      "Forbidden wording unless direct on-chain SP1 verification is implemented and tested",
      "The hash is a commitment to exact `.anky` UTF-8 bytes. It does not encrypt the writing.",
    ],
  },
];

const HUMAN_GATES = [
  {
    gate: "devnet_current_day_hashseal",
    reason:
      "A fresh same-day devnet HashSeal -> VerifiedSeal evidence bundle must be produced or refreshed for the target demo before final launch evidence is published. Operators can use `npm run seal -- --check-chain --send` with a writer-owned Core Loom; Codex must not sign with keypairs.",
  },
  {
    gate: "verifier_authority_custody",
    reason: "The configured verifier authority keypair/custody must be supplied by the human; Codex must not read keypairs.",
  },
  {
    gate: "target_backend_migration",
    reason:
      "Migrations 019_mobile_verified_seal_receipts, 020_mobile_helius_webhook_events, 021_mobile_helius_webhook_signature_dedupe, and 022_credit_ledger_entries are prepared and smoke-tested locally, but they must be applied to the target backend database by an operator.",
  },
  {
    gate: "backend_verified_seal_chain_proof",
    reason:
      "The launch backend must set ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true with private ANKY_SOLANA_RPC_URL, plus a separate public ANKY_PUBLIC_SOLANA_RPC_URL/EXPO_PUBLIC_SOLANA_RPC_URL for mobile config.",
  },
  {
    gate: "helius_live_indexing",
    reason: "A Helius webhook or credentialed Helius backfill must be configured by an operator; Codex must not create paid webhooks or print API keys.",
  },
  {
    gate: "live_mobile_demo",
    reason: "The phone flow must be run against the chosen devnet config: write, seal, prove, record verified, index, and display proof state.",
  },
  {
    gate: "real_core_seal_integration",
    reason:
      "The opt-in Anchor integration test must be run against an owned real Core Loom before mainnet confidence.",
  },
  {
    gate: "mainnet_launch_values",
    reason: "Mainnet program ID, Core collection, verifier authority, funding, snapshot time, and deployment status are not confirmed.",
  },
];

const KNOWN_LIMITATIONS = [
  {
    id: "direct_onchain_sp1",
    reason: "Direct on-chain SP1/Groth16 verification is not implemented; launch wording must remain verifier-authority-attested after off-chain SP1 verification.",
  },
];

main();

function main() {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help === true) {
      printUsage();
      return;
    }

    const repoRoot = path.resolve(args.repoRoot ?? DEFAULT_REPO_ROOT);
    const localArtifacts = LOCAL_ARTIFACTS.map(([name, relativePath]) => {
      const exists = fs.existsSync(path.join(repoRoot, relativePath));

      return {
        name,
        path: relativePath,
        status: exists ? "present" : "missing",
      };
    });
    const sourceChecks = SOURCE_CHECKS.map((check) => sourceCheckStatus(repoRoot, check));
    const privacyGuardCheck = runPrivacyGuard(repoRoot);
    const localChecks = [
      {
        name: "mobile node_modules is not required for readiness evidence",
        path: "apps/anky-mobile/node_modules",
        status: fs.existsSync(path.join(repoRoot, "apps/anky-mobile/node_modules"))
          ? "present_ignored"
          : "absent_not_required",
      },
      {
        name: "secret files not inspected by this readiness gate",
        status: "not_read",
      },
      privacyGuardCheck,
    ].concat(sourceChecks);
    const missingArtifacts = localArtifacts.filter((artifact) => artifact.status !== "present");
    const failedLocalChecks = localChecks.filter((check) =>
      LOCAL_FAILURE_STATUSES.has(check.status),
    );
    const localReady = missingArtifacts.length === 0 && failedLocalChecks.length === 0;

    console.log(
      JSON.stringify(
        {
          generatedAt: new Date().toISOString(),
          localReady,
          launchReady: false,
          localArtifacts,
          localChecks,
          humanGatedBlockers: HUMAN_GATES.map((gate) => ({
            ...gate,
            status: "blocked",
          })),
          knownLimitations: KNOWN_LIMITATIONS.map((limitation) => ({
            ...limitation,
            status: "documented",
          })),
          nextRequiredInputs: [
            "operator-run or attach a public audited fresh same-day devnet HashSeal -> VerifiedSeal evidence bundle for the target demo, using the writer-owned Core Loom and npm run seal/prove helpers",
            "verifier authority signing path approved by the human",
            "target backend DATABASE_URL migrations 019_mobile_verified_seal_receipts, 020_mobile_helius_webhook_events, 021_mobile_helius_webhook_signature_dedupe, and 022_credit_ledger_entries applied by operator",
            "launch backend configured with ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true, private ANKY_SOLANA_RPC_URL, and public ANKY_PUBLIC_SOLANA_RPC_URL/EXPO_PUBLIC_SOLANA_RPC_URL",
            "HELIUS_API_KEY or Helius RPC URL configured outside Codex",
            "owned devnet Core Loom asset for ANKY_CORE_INTEGRATION_LOOM_ASSET integration test",
            "mainnet program/collection/verifier/funding/snapshot values confirmed only after devnet E2E",
          ],
        },
        null,
        2,
      ),
    );
  } catch (error) {
    console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
    process.exit(1);
  }
}

function sourceCheckStatus(repoRoot, check) {
  const absolutePath = path.join(repoRoot, check.path);
  if (!fs.existsSync(absolutePath)) {
    return {
      name: check.name,
      path: check.path,
      status: "missing",
    };
  }

  const source = fs.readFileSync(absolutePath, "utf8");
  const missing = check.required.filter((needle) => !source.includes(needle));
  const forbiddenMatches = (check.forbidden ?? []).filter((needle) => source.includes(needle));
  const status =
    missing.length > 0 ? "unmatched" : forbiddenMatches.length > 0 ? "forbidden_match" : "ok";

  return {
    name: check.name,
    path: check.path,
    status,
    missing,
    forbiddenMatches,
  };
}

function runPrivacyGuard(repoRoot) {
  const guardPath = path.join(repoRoot, "solana/scripts/sojourn9/privacyGuard.mjs");
  if (!fs.existsSync(guardPath)) {
    return {
      name: "privacy guard execution",
      path: "solana/scripts/sojourn9/privacyGuard.mjs",
      status: "missing",
    };
  }

  const result = spawnSync(process.execPath, [guardPath, "--repo-root", repoRoot], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      PATH: process.env.PATH ?? "",
    },
    maxBuffer: 1024 * 1024,
  });

  if (result.status !== 0) {
    return {
      name: "privacy guard execution",
      path: "solana/scripts/sojourn9/privacyGuard.mjs",
      status: "failed",
      stderr: sanitizeToolOutput(result.stderr),
      stdout: sanitizeToolOutput(result.stdout),
    };
  }

  try {
    const report = JSON.parse(result.stdout);
    return {
      name: "privacy guard execution",
      path: "solana/scripts/sojourn9/privacyGuard.mjs",
      status: report.ok === true ? "ok" : "failed",
      checkedFiles: Array.isArray(report.checkedFiles) ? report.checkedFiles.length : 0,
      issueCount: Array.isArray(report.issues) ? report.issues.length : 0,
    };
  } catch (error) {
    return {
      name: "privacy guard execution",
      path: "solana/scripts/sojourn9/privacyGuard.mjs",
      status: "failed",
      stderr: error instanceof Error ? error.message : String(error),
    };
  }
}

function sanitizeToolOutput(value) {
  return redactSecretValues(value).slice(0, 4_000);
}

function parseArgs(argv) {
  const args = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (!BOOLEAN_FLAGS.has(arg) && !VALUE_FLAGS.has(arg)) {
      if (!arg.startsWith("--")) {
        throw new Error(`Unexpected argument: ${arg}`);
      }
      throw new Error(`Unknown option: ${arg}`);
    }

    const value = argv[index + 1];
    if (value == null || value.startsWith("--")) {
      throw new Error(`${arg} requires a value.`);
    }
    const key = arg.slice(2).replace(/-([a-z])/g, (_match, letter) => letter.toUpperCase());
    args[key] = value;
    index += 1;
  }

  return args;
}

function printUsage() {
  console.log(`Usage:
  node solana/scripts/sojourn9/launchReadinessGate.mjs

Options:
  --repo-root <path>  Optional repository root for tests or alternate worktrees.

This is a no-secret readiness report. It checks local launch artifacts and
prints the remaining live/human-gated blockers. It does not read .env files,
keypairs, wallet files, private .anky contents, or API key values.`);
}
