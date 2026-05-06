import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "privacyGuard.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");

test("passes on the current Sojourn 9 launch privacy surface", async () => {
  const result = await runNode([SCRIPT_PATH], {
    DATABASE_URL: "postgres://secret",
    HELIUS_API_KEY: "secret-api-key",
  });

  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.equal(report.issues.length, 0);
  assert.ok(report.checkedFiles.includes("migrations/019_credit_ledger_entries.sql"));
  assert.ok(report.checkedFiles.includes("migrations/020_mobile_verified_seal_receipts.sql"));
  assert.ok(report.checkedFiles.includes("runbooks/devnet-0xx1-live-e2e-evidence.json"));
  assert.doesNotMatch(result.stdout, /secret-api-key|postgres:\/\/secret/);
});

test("rejects private columns in launch receipt migrations", async () => {
  const repoRoot = createMinimalRepo();
  fs.writeFileSync(
    path.join(repoRoot, "migrations/019_credit_ledger_entries.sql"),
    "CREATE TABLE credit_ledger_entries (id TEXT PRIMARY KEY, private_input TEXT NOT NULL);",
    "utf8",
  );
  fs.writeFileSync(
    path.join(repoRoot, "migrations/020_mobile_verified_seal_receipts.sql"),
    "CREATE TABLE mobile_verified_seal_receipts (id TEXT PRIMARY KEY, session_hash TEXT NOT NULL);",
    "utf8",
  );

  const result = await runNode([SCRIPT_PATH, "--repo-root", repoRoot]);

  assert.notEqual(result.code, 0);
  assert.match(result.stdout, /private_input/);
  assert.match(result.stdout, /must not add private\/plaintext column/);
});

test("rejects plaintext logging in operator scripts", async () => {
  const repoRoot = createMinimalRepo();
  fs.writeFileSync(
    path.join(repoRoot, "solana/scripts/sojourn9/makeDemoAnky.mjs"),
    "const rawText = process.argv[2];\nconsole.log(rawText);\n",
    "utf8",
  );

  const result = await runNode([SCRIPT_PATH, "--repo-root", repoRoot]);

  assert.notEqual(result.code, 0);
  assert.match(result.stdout, /must not log private \.anky\/witness variables/);
});

test("rejects mobile backup manifests without proof artifact filters", async () => {
  const repoRoot = createMinimalRepo();
  fs.writeFileSync(
    path.join(repoRoot, "apps/anky-mobile/src/lib/ankyBackupManifest.ts"),
    "export function createAnkyBackupManifest(files) { return files; }\n",
    "utf8",
  );

  const result = await runNode([SCRIPT_PATH, "--repo-root", repoRoot]);

  assert.notEqual(result.code, 0);
  assert.match(result.stdout, /mobile backups must use an explicit eligibility filter/);
  assert.match(result.stdout, /mobile backups must exclude transient SP1 proof handoff artifacts/);
});

test("rejects private fields in public live evidence artifacts", async () => {
  const repoRoot = createMinimalRepo();
  fs.writeFileSync(
    path.join(repoRoot, "runbooks/devnet-0xx1-live-e2e-evidence.json"),
    JSON.stringify(
      {
        artifactKind: "anky_sojourn9_devnet_live_e2e_evidence",
        isFinalLaunchEvidence: false,
        privateInput: "not allowed",
        notFinalBecause: ["fixture"],
      },
      null,
      2,
    ),
    "utf8",
  );

  const result = await runNode([SCRIPT_PATH, "--repo-root", repoRoot]);

  assert.notEqual(result.code, 0);
  assert.match(result.stdout, /public evidence must not contain private\/plaintext-like field/);
  assert.match(result.stdout, /privateInput/);
});

test("rejects final-looking public demo evidence artifacts", async () => {
  const repoRoot = createMinimalRepo();
  fs.writeFileSync(
    path.join(repoRoot, "runbooks/devnet-0xx1-live-e2e-evidence.json"),
    JSON.stringify(
      {
        artifactKind: "anky_sojourn9_devnet_live_e2e_evidence",
        isFinalLaunchEvidence: true,
        notFinalBecause: [],
      },
      null,
      2,
    ),
    "utf8",
  );

  const result = await runNode([SCRIPT_PATH, "--repo-root", repoRoot]);

  assert.notEqual(result.code, 0);
  assert.match(result.stdout, /must be explicitly marked as non-final launch evidence/);
  assert.match(result.stdout, /must list why they are not final launch evidence/);
});

test("rejects unknown privacy guard options", async () => {
  const result = await runNode([SCRIPT_PATH, "--read-env-file", ".env"]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Unknown option: --read-env-file/);
  assert.equal(result.stdout, "");
});

function createMinimalRepo() {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "anky-privacy-guard-"));
  writeFile(
    repoRoot,
    "migrations/019_credit_ledger_entries.sql",
    "CREATE TABLE credit_ledger_entries (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, amount INTEGER NOT NULL, metadata_json TEXT);",
  );
  writeFile(
    repoRoot,
    "migrations/020_mobile_verified_seal_receipts.sql",
    "CREATE TABLE mobile_verified_seal_receipts (id TEXT PRIMARY KEY, session_hash TEXT NOT NULL, proof_hash TEXT NOT NULL);",
  );
  writeFile(
    repoRoot,
    "migrations/021_mobile_helius_webhook_events.sql",
    "CREATE TABLE mobile_helius_webhook_events (id TEXT PRIMARY KEY, payload_hash TEXT NOT NULL, payload_json TEXT NOT NULL);",
  );
  writeFile(
    repoRoot,
    "migrations/022_mobile_helius_webhook_signature_dedupe.sql",
    "CREATE UNIQUE INDEX idx_mobile_helius_webhook_events_network_signature_unique ON mobile_helius_webhook_events(network, signature) WHERE signature IS NOT NULL;",
  );
  writeFile(
    repoRoot,
    "src/routes/mobile_sojourn.rs",
    `
fn record() {
    validate_public_webhook_payload(&payload)?;
    let payload_json = serde_json::to_string(&payload)?;
}
fn contains_anky_plaintext_value() {}
fn find_private_webhook_field() {}
fn require_finalized_seal_record_secret() {}
const ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF: &str = "ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF";
const UPSERT: &str = "mobile_verified_seal_receipts.proof_hash = EXCLUDED.proof_hash";
pub struct RecordMobileVerifiedSealRequest {
    wallet: String,
    session_hash: String,
    proof_hash: String,
    verifier: String,
    protocol_version: u16,
}
`,
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/proveAndRecordVerified.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconst expectedHash = requiredArg(args, "expectedHash");\nthrow new Error("Refusing to write SP1 receipt/proof artifacts inside this git worktree");\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/makeDemoAnky.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconsole.log(JSON.stringify({ sessionHash: "a".repeat(64) }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/makeLaunchEvidence.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconst SECRET_PATH_RE = /\\.anky$/;\nconst manifestPath = resolvePublicPath(requiredArg(args, "manifest"), "--manifest");\nconst outPath = resolvePublicPath(args.out, "--out");\nconsole.log(JSON.stringify({ ok: true, manifestPath, outPath }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/auditLaunchEvidence.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconst SECRET_PATH_RE = /\\.anky$/;\nconst evidencePath = resolvePublicEvidencePath(args.evidence);\nconsole.log(JSON.stringify({ ok: true, evidencePath }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/liveE2eChecklist.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconsole.log(JSON.stringify({ ok: true }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/prepareCurrentDayProof.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconsole.log(JSON.stringify({ ok: true }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/checkProofHandoff.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconsole.log(JSON.stringify({ ok: true }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/launchReadinessGate.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconsole.log(JSON.stringify({ ok: true }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/privacyGuard.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconsole.log(JSON.stringify({ ok: true }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/sojourn9/smokeVerifiedSealMigration.mjs",
    'import { redactSecretValues } from "./redactSecrets.mjs";\nconsole.log(JSON.stringify({ ok: true }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/indexer/ankySealIndexer.mjs",
    'import { redactSecretValues } from "../sojourn9/redactSecrets.mjs";\nconsole.log(JSON.stringify({ events: [] }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/indexer/auditScoreSnapshot.mjs",
    'import { redactSecretValues } from "../sojourn9/redactSecrets.mjs";\nconst SECRET_PATH_RE = /\\.anky$/;\nconst snapshotPath = resolvePublicSnapshotPath(args.snapshot);\nconsole.log(JSON.stringify({ ok: true, snapshotPath }));\n',
  );
  writeFile(
    repoRoot,
    "solana/scripts/indexer/heliusWebhookManifest.mjs",
    'import { redactSecretValues } from "../sojourn9/redactSecrets.mjs";\nconsole.log(JSON.stringify({ ok: true }));\n',
  );
  writeFile(
    repoRoot,
    "solana/anky-seal-program/scripts/checkLaunchConfig.mjs",
    'import { redactSecretValues } from "../../scripts/sojourn9/redactSecrets.mjs";\nconsole.log(JSON.stringify({ ok: true }));\n',
  );
  writeFile(
    repoRoot,
    "solana/anky-seal-program/scripts/sealAnky.mjs",
    'import { redactSecretValues } from "../../scripts/sojourn9/redactSecrets.mjs";\nconsole.log(JSON.stringify({ dryRun: true }));\n',
  );
  writeFile(
    repoRoot,
    "solana/anky-seal-program/scripts/recordVerifiedAnky.mjs",
    'import { redactSecretValues } from "../../scripts/sojourn9/redactSecrets.mjs";\nconsole.log(JSON.stringify({ dryRun: true }));\n',
  );
  writeFile(
    repoRoot,
    "apps/anky-mobile/src/lib/solana/types.ts",
    `
export function getLoomSealProofState(seal, expectedProofVerifier) {
  if (seal.proofProtocolVersion !== 1) return "failed";
  if (Number.isSafeInteger(seal.utcDay)) return "verified";
  return "none";
}
`,
  );
  writeFile(
    repoRoot,
    "apps/anky-mobile/src/lib/ankyStorage.ts",
    "const state = getLoomSealProofState(latestSeal, verifier);\n",
  );
  writeFile(
    repoRoot,
    "apps/anky-mobile/src/lib/ankyBackupManifest.ts",
    `
const TRANSIENT_PROOF_ARTIFACT_FILE_NAMES = new Set(["proof-with-public-values.bin"]);
export function isBackupEligibleRelativePath(path) {
  const fileName = path.split("/").at(-1) ?? path;
  if (fileName.endsWith(".anky") && fileName !== "pending.anky") return false;
  return !TRANSIENT_PROOF_ARTIFACT_FILE_NAMES.has(fileName);
}
`,
  );
  writeFile(
    repoRoot,
    "runbooks/devnet-0xx1-live-e2e-evidence.json",
    JSON.stringify(
      {
        artifactKind: "anky_sojourn9_devnet_live_e2e_evidence",
        artifactVersion: 1,
        isFinalLaunchEvidence: false,
        cluster: "devnet",
        programId: "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX",
        proofSubject: {
          sessionHash: "f6c922b2b87fec532aa3d24cb2bafcc237043fa28de168820c2326e0b18955b3",
          utcDay: 20579,
        },
        notFinalBecause: ["fixture is not final launch evidence"],
      },
      null,
      2,
    ),
  );

  return repoRoot;
}

function writeFile(repoRoot, relativePath, contents) {
  const fullPath = path.join(repoRoot, relativePath);
  fs.mkdirSync(path.dirname(fullPath), { recursive: true });
  fs.writeFileSync(fullPath, contents, "utf8");
}

function runNode(args, env = {}) {
  return new Promise((resolve) => {
    execFile(
      process.execPath,
      args,
      {
        cwd: REPO_ROOT,
        env: {
          ...process.env,
          ...env,
        },
      },
      (error, stdout, stderr) => {
        resolve({
          code: error?.code ?? 0,
          stderr,
          stdout,
        });
      },
    );
  });
}
