import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "launchReadinessGate.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");

test("reports local artifacts present but keeps launch blocked on live gates", async () => {
  const result = await runNode([SCRIPT_PATH], {
    DATABASE_URL: "postgres://secret",
    HELIUS_API_KEY: "secret-api-key",
  });

  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.localReady, true);
  assert.equal(report.launchReady, false);
  assert.ok(report.localArtifacts.length > 10);
  assert.ok(report.localArtifacts.every((artifact) => artifact.status === "present"));
  assert.ok(report.humanGatedBlockers.length >= 6);
  assert.ok(report.humanGatedBlockers.every((blocker) => blocker.status === "blocked"));
  assert.ok(
    report.humanGatedBlockers.every((blocker) => blocker.gate !== "direct_onchain_sp1"),
  );
  assert.ok(
    report.knownLimitations.some(
      (limitation) =>
        limitation.id === "direct_onchain_sp1" && limitation.status === "documented",
    ),
  );
  assert.ok(
    report.humanGatedBlockers.some(
      (blocker) => blocker.gate === "backend_verified_seal_chain_proof",
    ),
  );
  assert.ok(
    report.nextRequiredInputs.some((input) =>
      input.includes("fresh same-day devnet HashSeal -> VerifiedSeal evidence bundle"),
    ),
  );
  assert.ok(
    report.nextRequiredInputs.some((input) =>
      input.includes("ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true"),
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Same-day HashSeal operator refuses stale days and mainnet" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Operator package exposes Sojourn 9 command aliases" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localArtifacts.some(
      (artifact) =>
        artifact.name === "Public launch evidence auditor" &&
        artifact.status === "present",
    ),
  );
  assert.ok(
    report.localArtifacts.some(
      (artifact) =>
        artifact.name === "Public launch evidence builder" &&
        artifact.status === "present",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Public launch evidence builder reads handoff metadata only" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Public launch evidence auditor reads only public receipts" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Live E2E checklist keeps human-key steps explicit" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Current-day proof handoff prepares public artifacts only" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Proof handoff status checker reads only public manifest metadata" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) => check.name === "SP1 saved proof verification mode" && check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Privacy guard checks proof/indexing plaintext boundaries" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "privacy guard execution" &&
        check.status === "ok" &&
        check.issueCount === 0,
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "VerifiedSeal send requires local SP1 proof verification" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Mainnet VerifiedSeal send follows Helius Sender policy when explicitly enabled" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) => check.name === "Backend verified receipt upsert is immutable" && check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Backend finalized seal receipts require indexer secret" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Backend Helius webhook rejects private .anky payloads" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Helius backfill requests finalized commitment explicitly" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Helius indexer rejects unsafe launch inputs" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Backend accepts Helius authHeader Authorization bearer secret" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Helius runbook records webhook delivery and dedupe guidance" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Helius webhook manifest prints live delivery caveats" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Backend score view uses finalized Score V1 public receipts" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Mobile Loom screen surfaces indexed points" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Mobile reveal separates hash seal from SP1 proof state" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Public Colosseum pitch uses Sojourn 9 truth claims" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Local-first protocol documents exact .anky hash and seal path" &&
        check.status === "ok",
    ),
  );
  assert.ok(
    report.localChecks.some(
      (check) =>
        check.name === "Mainnet launch checklist keeps signing and claims gated" &&
        check.status === "ok",
    ),
  );
  assert.doesNotMatch(result.stdout, /secret-api-key|postgres:\/\/secret/);
});

test("reports missing local artifacts for alternate repo roots", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-readiness-gate-"));
  const result = await runNode([SCRIPT_PATH, "--repo-root", tempDir]);

  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.localReady, false);
  assert.equal(report.launchReady, false);
  assert.ok(report.localArtifacts.some((artifact) => artifact.status === "missing"));
});

test("treats a failed privacy guard execution as not locally ready", async () => {
  const tempDir = createRepoFixtureWithFailingPrivacyGuard();
  const result = await runNode([SCRIPT_PATH, "--repo-root", tempDir]);

  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.localReady, false);
  assert.ok(report.localArtifacts.every((artifact) => artifact.status === "present"));
  assert.ok(
    report.localChecks.some(
      (check) => check.name === "privacy guard execution" && check.status === "failed",
    ),
  );
});

test("rejects unknown readiness gate options", async () => {
  const result = await runNode([SCRIPT_PATH, "--read-env-file", ".env"]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Unknown option: --read-env-file/);
  assert.equal(result.stdout, "");
});

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

function createRepoFixtureWithFailingPrivacyGuard() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-readiness-gate-"));

  symlinkFromRepo(tempDir, "AGENTS.md");
  symlinkFromRepo(tempDir, "HACKATHON.md");
  symlinkFromRepo(tempDir, "apps");
  symlinkFromRepo(tempDir, "docs");
  symlinkFromRepo(tempDir, "migrations");
  symlinkFromRepo(tempDir, "runbooks");
  symlinkFromRepo(tempDir, "src");

  fs.mkdirSync(path.join(tempDir, "solana", "scripts"), { recursive: true });
  symlinkFromRepo(tempDir, "solana/anky-seal-program");
  symlinkFromRepo(tempDir, "solana/anky-zk-proof");
  symlinkFromRepo(tempDir, "solana/scripts/indexer");

  const realSojourn9Dir = path.join(REPO_ROOT, "solana", "scripts", "sojourn9");
  const fixtureSojourn9Dir = path.join(tempDir, "solana", "scripts", "sojourn9");
  fs.mkdirSync(fixtureSojourn9Dir, { recursive: true });
  for (const entry of fs.readdirSync(realSojourn9Dir)) {
    if (entry === "privacyGuard.mjs") {
      continue;
    }
    fs.symlinkSync(path.join(realSojourn9Dir, entry), path.join(fixtureSojourn9Dir, entry));
  }
  fs.writeFileSync(
    path.join(fixtureSojourn9Dir, "privacyGuard.mjs"),
    `#!/usr/bin/env node
// launch receipt migrations must not add private/plaintext column
// Helius webhook payload must be privacy-validated before it is serialized for storage
// script must not log private .anky/witness variables
// operator/indexer script stderr must redact secret-looking values
// solana/scripts/sojourn9/launchReadinessGate.mjs
// solana/scripts/sojourn9/smokeVerifiedSealMigration.mjs
// mobile proof-state resolver must use only public seal/proof metadata
console.error("intentional privacy guard failure");
process.exit(1);
`,
  );

  return tempDir;
}

function symlinkFromRepo(tempDir, relativePath) {
  const target = path.join(REPO_ROOT, relativePath);
  const link = path.join(tempDir, relativePath);
  fs.mkdirSync(path.dirname(link), { recursive: true });
  fs.symlinkSync(target, link);
}
