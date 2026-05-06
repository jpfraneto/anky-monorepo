#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { redactSecretValues } from "./redactSecrets.mjs";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_REPO_ROOT = path.resolve(SCRIPT_DIR, "../../..");
const BOOLEAN_FLAGS = new Set([]);
const VALUE_FLAGS = new Set(["--repo-root"]);

const SQL_FILES = [
  "migrations/019_mobile_verified_seal_receipts.sql",
  "migrations/020_mobile_helius_webhook_events.sql",
  "migrations/021_mobile_helius_webhook_signature_dedupe.sql",
  "migrations/022_credit_ledger_entries.sql",
];

const OPERATOR_SCRIPTS = [
  "solana/scripts/sojourn9/proveAndRecordVerified.mjs",
  "solana/scripts/sojourn9/makeDemoAnky.mjs",
  "solana/scripts/sojourn9/makeLaunchEvidence.mjs",
  "solana/scripts/sojourn9/auditLaunchEvidence.mjs",
  "solana/scripts/sojourn9/liveE2eChecklist.mjs",
  "solana/scripts/sojourn9/prepareCurrentDayProof.mjs",
  "solana/scripts/sojourn9/checkProofHandoff.mjs",
  "solana/scripts/sojourn9/launchReadinessGate.mjs",
  "solana/scripts/sojourn9/privacyGuard.mjs",
  "solana/scripts/sojourn9/smokeVerifiedSealMigration.mjs",
  "solana/scripts/indexer/ankySealIndexer.mjs",
  "solana/scripts/indexer/auditScoreSnapshot.mjs",
  "solana/scripts/indexer/heliusWebhookManifest.mjs",
  "solana/anky-seal-program/scripts/checkLaunchConfig.mjs",
  "solana/anky-seal-program/scripts/sealAnky.mjs",
  "solana/anky-seal-program/scripts/recordVerifiedAnky.mjs",
];

const BACKEND_FILE = "src/routes/mobile_sojourn.rs";
const MOBILE_PROOF_FILES = [
  "apps/anky-mobile/src/lib/solana/types.ts",
  "apps/anky-mobile/src/lib/ankyStorage.ts",
  "apps/anky-mobile/src/lib/ankyBackupManifest.ts",
];
const PUBLIC_EVIDENCE_FILES = ["runbooks/devnet-0xx1-live-e2e-evidence.json"];

const PRIVATE_COLUMN_RE =
  /(?:raw_?anky|anky_?plaintext|plaintext_?anky|writing_?text|reconstructed_?text|sp1_?witness|proof_?witness|private_?input|private_?inputs|witness_?bytes|file_?bytes|file_?contents)/i;
const PRIVATE_FIELD_RE =
  /(?:rawAnky|raw_anky|ankyPlaintext|plaintext|writingText|reconstructedText|sp1Witness|proofWitness|privateInput|privateInputs|witnessBytes|fileBytes|fileContents)/;
const PRIVATE_OPTION_RE =
  /--(?:raw-anky|anky-plaintext|plaintext|writing-text|reconstructed-text|sp1-witness|proof-witness|private-input|witness-bytes|file-contents)\b/;
const PLAINTEXT_LOG_RE =
  /console\.(?:log|error|warn)\([^;\n]*(?:rawAnky|raw_anky|ankyPlaintext|rawWitness|witnessBytes|privateInput|privateInputs|fileContents|rawText)/;
const SECRET_PATH_RE =
  /(^|[/\\])\.env(?:[./\\]|$)|(^|[/\\])id\.json$|\.anky$|keypair|deployer|wallet|\.pem$/i;
const PRIVATE_EVIDENCE_KEY_RE =
  /(?:rawAnky|raw_anky|ankyPlaintext|anky_plaintext|plaintext|writingText|writing_text|reconstructedText|reconstructed_text|sp1Witness|sp1_witness|proofWitness|proof_witness|privateInput|private_input|privateInputs|private_inputs|witnessBytes|witness_bytes|fileBytes|file_bytes|fileContents|file_contents|keypair|privateKey|private_key|secret|apiKey|api_key|accessToken|access_token|bearer|authorization|envFile|env_file|mnemonic|seedPhrase|seed_phrase|deployer)/i;

main();

function main() {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help === true) {
      printUsage();
      return;
    }

    const repoRoot = path.resolve(args.repoRoot ?? DEFAULT_REPO_ROOT);
    const issues = [];

    checkSqlMigrations(repoRoot, issues);
    checkBackendGuards(repoRoot, issues);
    checkOperatorScripts(repoRoot, issues);
    checkMobileProofState(repoRoot, issues);
    checkPublicEvidenceArtifacts(repoRoot, issues);

    const report = {
      checkedAt: new Date().toISOString(),
      ok: issues.length === 0,
      checkedFiles: [
        ...SQL_FILES,
        BACKEND_FILE,
        ...OPERATOR_SCRIPTS,
        ...MOBILE_PROOF_FILES,
        ...PUBLIC_EVIDENCE_FILES,
      ],
      issues,
    };

    console.log(JSON.stringify(report, null, 2));
    if (issues.length > 0) {
      process.exit(1);
    }
  } catch (error) {
    console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
    process.exit(1);
  }
}

function checkPublicEvidenceArtifacts(repoRoot, issues) {
  for (const relativePath of PUBLIC_EVIDENCE_FILES) {
    const source = readRequiredFile(repoRoot, relativePath, issues);
    if (source == null) {
      continue;
    }

    let artifact;
    try {
      artifact = JSON.parse(source);
    } catch (error) {
      issues.push({
        path: relativePath,
        reason: `public live evidence artifact must be valid JSON: ${
          error instanceof Error ? error.message : String(error)
        }`,
      });
      continue;
    }

    if (artifact?.isFinalLaunchEvidence !== false) {
      issues.push({
        path: relativePath,
        reason: "demo evidence artifacts must be explicitly marked as non-final launch evidence",
      });
    }

    if (!Array.isArray(artifact?.notFinalBecause) || artifact.notFinalBecause.length === 0) {
      issues.push({
        path: relativePath,
        reason: "demo evidence artifacts must list why they are not final launch evidence",
      });
    }

    scanPublicEvidenceArtifact(artifact, [], relativePath, issues);
  }
}

function scanPublicEvidenceArtifact(value, fieldPath, relativePath, issues) {
  if (Array.isArray(value)) {
    value.forEach((item, index) =>
      scanPublicEvidenceArtifact(item, [...fieldPath, String(index)], relativePath, issues),
    );
    return;
  }

  if (value == null || typeof value !== "object") {
    if (typeof value !== "string") {
      return;
    }
    const joinedPath = fieldPath.join(".") || "<root>";
    if (looksLikeCompleteAnkyPlaintext(value)) {
      issues.push({
        path: relativePath,
        reason: `public evidence must not contain complete .anky plaintext at ${joinedPath}`,
      });
    }
    if (SECRET_PATH_RE.test(value) || redactSecretValues(value) !== value) {
      issues.push({
        path: relativePath,
        reason: `public evidence must not contain secret-shaped values at ${joinedPath}`,
      });
    }
    return;
  }

  for (const [key, nested] of Object.entries(value)) {
    const nestedPath = [...fieldPath, key];
    if (PRIVATE_EVIDENCE_KEY_RE.test(key)) {
      issues.push({
        path: relativePath,
        reason: `public evidence must not contain private/plaintext-like field ${nestedPath.join(".")}`,
      });
    }
    scanPublicEvidenceArtifact(nested, nestedPath, relativePath, issues);
  }
}

function checkSqlMigrations(repoRoot, issues) {
  for (const relativePath of SQL_FILES) {
    const source = readRequiredFile(repoRoot, relativePath, issues);
    if (source == null) {
      continue;
    }

    const columns = extractSqlColumnNames(source);
    for (const column of columns) {
      if (PRIVATE_COLUMN_RE.test(column)) {
        issues.push({
          path: relativePath,
          reason: `launch receipt migrations must not add private/plaintext column \`${column}\``,
        });
      }
    }
  }
}

function checkBackendGuards(repoRoot, issues) {
  const source = readRequiredFile(repoRoot, BACKEND_FILE, issues);
  if (source == null) {
    return;
  }

  requireNeedle(source, "validate_public_webhook_payload(&payload)?", BACKEND_FILE, issues);
  requireNeedle(source, "contains_anky_plaintext_value", BACKEND_FILE, issues);
  requireNeedle(source, "find_private_webhook_field", BACKEND_FILE, issues);
  requireNeedle(source, "require_finalized_seal_record_secret", BACKEND_FILE, issues);
  requireNeedle(source, "ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF", BACKEND_FILE, issues);
  requireNeedle(source, "mobile_verified_seal_receipts.proof_hash = EXCLUDED.proof_hash", BACKEND_FILE, issues);

  const validateIndex = source.indexOf("validate_public_webhook_payload(&payload)?");
  const stringifyIndex = source.indexOf("let payload_json = serde_json::to_string(&payload)?");
  if (validateIndex < 0 || stringifyIndex < 0 || validateIndex > stringifyIndex) {
    issues.push({
      path: BACKEND_FILE,
      reason: "Helius webhook payload must be privacy-validated before it is serialized for storage",
    });
  }

  const verifiedRequest = extractRustStruct(source, "RecordMobileVerifiedSealRequest");
  if (verifiedRequest == null) {
    issues.push({
      path: BACKEND_FILE,
      reason: "RecordMobileVerifiedSealRequest struct was not found",
    });
  } else if (PRIVATE_FIELD_RE.test(verifiedRequest)) {
    issues.push({
      path: BACKEND_FILE,
      reason: "RecordMobileVerifiedSealRequest must contain only public proof receipt metadata",
    });
  }
}

function checkOperatorScripts(repoRoot, issues) {
  for (const relativePath of OPERATOR_SCRIPTS) {
    const source = readRequiredFile(repoRoot, relativePath, issues);
    if (source == null) {
      continue;
    }

    if (PRIVATE_OPTION_RE.test(source)) {
      issues.push({
        path: relativePath,
        reason: "operator/indexer scripts must not add CLI options that accept .anky plaintext or witness payloads directly",
      });
    }

    requireNeedle(source, "redactSecretValues", relativePath, issues, {
      reason: "operator/indexer script stderr must redact secret-looking values",
    });

    const leakingLine = source
      .split(/\r?\n/)
      .find((line) => PLAINTEXT_LOG_RE.test(line));
    if (leakingLine != null) {
      issues.push({
        path: relativePath,
        reason: `script must not log private .anky/witness variables: ${leakingLine.trim()}`,
      });
    }
  }

  const wrapper = readRequiredFile(repoRoot, "solana/scripts/sojourn9/proveAndRecordVerified.mjs", issues);
  if (wrapper != null) {
    requireNeedle(wrapper, 'const expectedHash = requiredArg(args, "expectedHash")', "solana/scripts/sojourn9/proveAndRecordVerified.mjs", issues, {
      reason: "SP1 wrapper must require an expected public hash when a private .anky path is supplied",
    });
    requireNeedle(wrapper, "Refusing to write SP1 receipt/proof artifacts inside this git worktree", "solana/scripts/sojourn9/proveAndRecordVerified.mjs", issues);
  }

  const indexer = readRequiredFile(repoRoot, "solana/scripts/indexer/ankySealIndexer.mjs", issues);
  if (indexer != null) {
    requireNeedle(indexer, "SECRET_PATH_RE", "solana/scripts/indexer/ankySealIndexer.mjs", issues, {
      reason: "Helius indexer must reject secret-shaped input/output paths",
    });
    requireNeedle(indexer, "\\.anky$", "solana/scripts/indexer/ankySealIndexer.mjs", issues, {
      reason: "Helius indexer must reject private .anky witness file paths before reading input",
    });
    requireNeedle(indexer, "normalizeBackendUrl", "solana/scripts/indexer/ankySealIndexer.mjs", issues, {
      reason: "Helius indexer must reject credentialed backend URLs",
    });
    requireNeedle(indexer, "requireExplicitMainnetConfig", "solana/scripts/indexer/ankySealIndexer.mjs", issues, {
      reason: "Helius indexer must not use devnet defaults for mainnet indexing",
    });
  }

  const evidenceBuilder = readRequiredFile(repoRoot, "solana/scripts/sojourn9/makeLaunchEvidence.mjs", issues);
  if (evidenceBuilder != null) {
    requireNeedle(evidenceBuilder, "SECRET_PATH_RE", "solana/scripts/sojourn9/makeLaunchEvidence.mjs", issues, {
      reason: "public launch evidence builder must reject secret-shaped input/output paths",
    });
    requireNeedle(evidenceBuilder, "\\.anky$", "solana/scripts/sojourn9/makeLaunchEvidence.mjs", issues, {
      reason: "public launch evidence builder must reject private .anky witness file paths before reading input",
    });
    requireNeedle(
      evidenceBuilder,
      "resolvePublicPath(requiredArg(args, \"manifest\"), \"--manifest\")",
      "solana/scripts/sojourn9/makeLaunchEvidence.mjs",
      issues,
      {
        reason: "public launch evidence builder must sanitize handoff manifest paths before reading",
      },
    );
    requireNeedle(evidenceBuilder, "resolvePublicPath(args.out, \"--out\")", "solana/scripts/sojourn9/makeLaunchEvidence.mjs", issues, {
      reason: "public launch evidence builder must sanitize output paths before writing",
    });
  }

  const evidenceAuditor = readRequiredFile(repoRoot, "solana/scripts/sojourn9/auditLaunchEvidence.mjs", issues);
  if (evidenceAuditor != null) {
    requireNeedle(evidenceAuditor, "SECRET_PATH_RE", "solana/scripts/sojourn9/auditLaunchEvidence.mjs", issues, {
      reason: "public launch evidence auditor must reject secret-shaped evidence paths",
    });
    requireNeedle(evidenceAuditor, "\\.anky$", "solana/scripts/sojourn9/auditLaunchEvidence.mjs", issues, {
      reason: "public launch evidence auditor must reject private .anky witness file paths before reading input",
    });
    requireNeedle(evidenceAuditor, "resolvePublicEvidencePath", "solana/scripts/sojourn9/auditLaunchEvidence.mjs", issues, {
      reason: "public launch evidence auditor must sanitize evidence paths before reading",
    });
  }

  const snapshotAuditor = readRequiredFile(repoRoot, "solana/scripts/indexer/auditScoreSnapshot.mjs", issues);
  if (snapshotAuditor != null) {
    requireNeedle(snapshotAuditor, "SECRET_PATH_RE", "solana/scripts/indexer/auditScoreSnapshot.mjs", issues, {
      reason: "score snapshot auditor must reject secret-shaped snapshot paths",
    });
    requireNeedle(snapshotAuditor, "\\.anky$", "solana/scripts/indexer/auditScoreSnapshot.mjs", issues, {
      reason: "score snapshot auditor must reject private .anky witness file paths before reading input",
    });
    requireNeedle(snapshotAuditor, "resolvePublicSnapshotPath", "solana/scripts/indexer/auditScoreSnapshot.mjs", issues, {
      reason: "score snapshot auditor must sanitize snapshot paths before reading",
    });
  }
}

function checkMobileProofState(repoRoot, issues) {
  const types = readRequiredFile(repoRoot, "apps/anky-mobile/src/lib/solana/types.ts", issues);
  if (types != null) {
    requireNeedle(types, "expectedProofVerifier", "apps/anky-mobile/src/lib/solana/types.ts", issues);
    requireNeedle(types, "proofProtocolVersion !== 1", "apps/anky-mobile/src/lib/solana/types.ts", issues);
    requireNeedle(types, "Number.isSafeInteger(seal.utcDay)", "apps/anky-mobile/src/lib/solana/types.ts", issues);
    const proofState = extractTypeScriptFunction(types, "getLoomSealProofState");
    if (proofState != null && PRIVATE_FIELD_RE.test(proofState)) {
      issues.push({
        path: "apps/anky-mobile/src/lib/solana/types.ts",
        reason: "mobile proof-state resolver must use only public seal/proof metadata",
      });
    }
  }

  const storage = readRequiredFile(repoRoot, "apps/anky-mobile/src/lib/ankyStorage.ts", issues);
  if (storage != null) {
    requireNeedle(storage, "getLoomSealProofState(latestSeal", "apps/anky-mobile/src/lib/ankyStorage.ts", issues);
  }

  const backupManifest = readRequiredFile(
    repoRoot,
    "apps/anky-mobile/src/lib/ankyBackupManifest.ts",
    issues,
  );
  if (backupManifest != null) {
    requireNeedle(
      backupManifest,
      "isBackupEligibleRelativePath",
      "apps/anky-mobile/src/lib/ankyBackupManifest.ts",
      issues,
      {
        reason: "mobile backups must use an explicit eligibility filter",
      },
    );
    requireNeedle(
      backupManifest,
      "TRANSIENT_PROOF_ARTIFACT_FILE_NAMES",
      "apps/anky-mobile/src/lib/ankyBackupManifest.ts",
      issues,
      {
        reason: "mobile backups must exclude transient SP1 proof handoff artifacts",
      },
    );
    requireNeedle(
      backupManifest,
      'fileName.endsWith(".anky")',
      "apps/anky-mobile/src/lib/ankyBackupManifest.ts",
      issues,
      {
        reason: "mobile backups must exclude generic .anky witness files",
      },
    );
  }
}

function extractSqlColumnNames(source) {
  const columns = [];
  const columnPattern =
    /\b([a-z_][a-z0-9_]*)\s+(?:TEXT|BIGINT|INTEGER|TIMESTAMPTZ|JSONB|BYTEA|VARCHAR|CHARACTER VARYING)\b/gim;

  for (const match of source.matchAll(columnPattern)) {
    columns.push(match[1]);
  }

  return columns;
}

function extractRustStruct(source, name) {
  const start = source.indexOf(`struct ${name}`);
  if (start < 0) {
    return null;
  }
  const open = source.indexOf("{", start);
  if (open < 0) {
    return null;
  }
  const close = source.indexOf("\n}", open);
  if (close < 0) {
    return null;
  }
  return source.slice(open + 1, close);
}

function extractTypeScriptFunction(source, name) {
  const start = source.indexOf(`function ${name}`);
  if (start < 0) {
    return null;
  }
  const nextExport = source.indexOf("\nexport ", start + 1);
  return source.slice(start, nextExport < 0 ? source.length : nextExport);
}

function requireNeedle(source, needle, relativePath, issues, options = {}) {
  if (!source.includes(needle)) {
    issues.push({
      path: options.sourcePath ?? relativePath,
      reason: options.reason ?? `required privacy guard text not found: ${needle}`,
    });
  }
}

function looksLikeCompleteAnkyPlaintext(value) {
  return (
    typeof value === "string" &&
    value.includes("\n") &&
    value.includes("8000") &&
    isClosedAnky(value)
  );
}

function isClosedAnky(value) {
  if (
    value.length === 0 ||
    value.charCodeAt(0) === 0xfeff ||
    value.includes("\r") ||
    !value.endsWith("\n8000") ||
    countOccurrences(value, "\n8000") !== 1
  ) {
    return false;
  }

  const lines = value.split("\n");
  const first = lines.shift();
  if (!captureLineHasValidTimeAndCharacter(first, { firstLine: true })) {
    return false;
  }

  for (const line of lines) {
    if (line === "8000") {
      return true;
    }
    if (!captureLineHasValidTimeAndCharacter(line, { firstLine: false })) {
      return false;
    }
  }
  return false;
}

function captureLineHasValidTimeAndCharacter(line, { firstLine }) {
  if (typeof line !== "string") {
    return false;
  }
  const separator = line.indexOf(" ");
  if (separator < 0) {
    return false;
  }
  const time = line.slice(0, separator);
  const token = line.slice(separator + 1);
  if (firstLine) {
    if (!/^\d+$/.test(time)) {
      return false;
    }
  } else if (!/^\d{4}$/.test(time) || Number(time) > 7_999) {
    return false;
  }
  return token === "SPACE" || token === " " || [...token].length === 1;
}

function countOccurrences(value, pattern) {
  let count = 0;
  let index = value.indexOf(pattern);
  while (index >= 0) {
    count += 1;
    index = value.indexOf(pattern, index + pattern.length);
  }
  return count;
}

function readRequiredFile(repoRoot, relativePath, issues) {
  const absolutePath = path.join(repoRoot, relativePath);
  if (!fs.existsSync(absolutePath)) {
    issues.push({
      path: relativePath,
      reason: "required launch privacy file is missing",
    });
    return null;
  }
  return fs.readFileSync(absolutePath, "utf8");
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
  node solana/scripts/sojourn9/privacyGuard.mjs

Options:
  --repo-root <path>  Optional repository root for tests or alternate worktrees.

This no-secret guard checks that the Sojourn 9 proof, receipt, Helius, and
mobile proof-state surfaces keep .anky plaintext and witness data out of
persistent launch metadata.`);
}
