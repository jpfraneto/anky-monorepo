import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "auditLaunchEvidence.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");
const PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const CORE_COLLECTION = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const PROOF_VERIFIER = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const WRITER = "4vJ9JU1bJJE96FWS5zNtVM6DfHyWixJjx5KJ4LJh5S7K";
const LOOM_ASSET = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
const VALID_SIGNATURE =
  "2hntvJaJzRkFWt3hTa7Q9oiGyVsTpjMwmzY8WcN52UDMsTyMuzKUtcEhupAe7BcZGeq49dFBhhgoYgeZ79m53sNh";
const SECOND_VALID_SIGNATURE = "5".repeat(88);
const SCORE_FORMULA =
  "score = unique_seal_days + verified_days + 2 * floor(each_consecutive_day_run / 7)";

test("accepts complete public devnet launch evidence without reading secrets", async () => {
  const evidencePath = writeEvidence(publicEvidence());
  const result = await runNode([SCRIPT_PATH, "--evidence", evidencePath], {
    HELIUS_API_KEY: "actual-helius-key",
    ANKY_INDEXER_WRITE_SECRET: "actual-indexer-secret",
  });

  assert.equal(result.code, 0, result.stderr);
  assert.equal(result.stderr, "");
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.deepEqual(report.issues, []);
  assert.equal(report.summary.cluster, "devnet");
  assert.equal(report.summary.programId, PROGRAM_ID);
  assert.equal(report.summary.coreCollection, CORE_COLLECTION);
  assert.equal(report.summary.proofVerifierAuthority, PROOF_VERIFIER);
  assert.equal(report.summary.protocolVersion, 1);
  assert.equal(report.summary.devnetUtcDay, 20579);
  assert.equal(report.summary.devnetSealWindow, "open");
  assert.equal(report.summary.devnetDayRolloverAt, "2026-05-07T00:00:00.000Z");
  assert.doesNotMatch(result.stdout, /actual-helius-key|actual-indexer-secret/);
});

test("prints a no-secret public evidence template that is not valid final evidence", async () => {
  const template = await runNode([SCRIPT_PATH, "--print-template"], {
    HELIUS_API_KEY: "actual-helius-key",
  });
  assert.equal(template.code, 0, template.stderr);
  const evidence = JSON.parse(template.stdout);
  assert.equal(evidence.templateOnly, true);
  assert.equal(evidence.protocolVersion, 1);
  assert.equal(evidence.scoreSnapshot.rewardBps, 800);
  assert.equal(evidence.scoreSnapshot.participantCap, 3456);
  assert.equal(evidence.helius.receiverPath, "/api/helius/anky-seal");
  assert.equal(evidence.helius.backfillMethod, "getTransactionsForAddress");
  assert.equal(evidence.helius.backfillCommitment, "finalized");
  assert.equal(evidence.devnetE2E.utcDayStatus.sealWindow, "open");
  assert.match(evidence.devnetE2E.sealOrbUrl, /^https:\/\/orbmarkets\.io\/tx\//);
  assert.doesNotMatch(template.stdout, /actual-helius-key|api-key=|Bearer /);

  const evidencePath = writeEvidence(evidence);
  const audit = await runNode([SCRIPT_PATH, "--evidence", evidencePath]);
  assert.equal(audit.code, 1);
  const report = JSON.parse(audit.stdout);
  assert.ok(report.issues.includes("templateOnly must not be true in final launch evidence"));
});

test("rejects private fields and complete .anky plaintext-like values", async () => {
  const evidence = publicEvidence({
    operatorNotes: "1704067200000 a\n0001 SPACE\n8000",
    sp1Witness: {
      witnessPath: "/tmp/private-demo.anky",
    },
  });
  const evidencePath = writeEvidence(evidence);
  const result = await runNode([SCRIPT_PATH, "--evidence", evidencePath]);

  assert.equal(result.code, 1);
  assert.equal(result.stderr, "");
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, false);
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("complete .anky plaintext-like value is present at operatorNotes"),
    ),
  );
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("private/plaintext-like field is present at sp1Witness"),
    ),
  );
  assert.doesNotMatch(result.stdout, /1704067200000 a|private-demo/);
});

test("rejects fixture signatures and non-Orb transaction links", async () => {
  const evidence = publicEvidence({
    devnetE2E: {
      ...publicEvidence().devnetE2E,
      sealSignature: "fixture_signature",
      sealOrbUrl: `https://solscan.io/tx/${VALID_SIGNATURE}`,
    },
  });
  const evidencePath = writeEvidence(evidence);
  const result = await runNode([SCRIPT_PATH, "--evidence", evidencePath]);

  assert.equal(result.code, 1);
  const report = JSON.parse(result.stdout);
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("devnetE2E.sealSignature must be a real 64-byte Solana signature"),
    ),
  );
  assert.ok(
    report.issues.some((issue) =>
      issue.includes("devnetE2E.sealOrbUrl must be an Orb transaction link"),
    ),
  );
});

test("rejects missing or inconsistent UTC-day status evidence", async () => {
  const missing = publicEvidence({
    devnetE2E: {
      ...publicEvidence().devnetE2E,
      utcDayStatus: null,
    },
  });
  const missingPath = writeEvidence(missing);
  const missingResult = await runNode([SCRIPT_PATH, "--evidence", missingPath]);

  assert.equal(missingResult.code, 1);
  const missingReport = JSON.parse(missingResult.stdout);
  assert.ok(missingReport.issues.includes("devnetE2E.utcDayStatus is required"));

  const inconsistent = publicEvidence({
    devnetE2E: {
      ...publicEvidence().devnetE2E,
      utcDayStatus: {
        ...publicEvidence().devnetE2E.utcDayStatus,
        receiptUtcDay: 20578,
        sealWindow: "open",
      },
    },
  });
  const inconsistentPath = writeEvidence(inconsistent);
  const inconsistentResult = await runNode([SCRIPT_PATH, "--evidence", inconsistentPath]);

  assert.equal(inconsistentResult.code, 1);
  const inconsistentReport = JSON.parse(inconsistentResult.stdout);
  assert.ok(
    inconsistentReport.issues.includes("devnetE2E.utcDayStatus.receiptUtcDay must match devnetE2E.utcDay"),
  );
  assert.ok(
    inconsistentReport.issues.includes("devnetE2E.utcDayStatus.isCurrentDay is inconsistent with UTC day values"),
  );
});

test("rejects missing finalized Score V1 and Helius audit markers", async () => {
  const evidence = publicEvidence({
    helius: {
      ...publicEvidence().helius,
      backfillCommitment: "confirmed",
      backfillAudited: false,
      backfillMethod: "getSignaturesForAddress",
      dedupeBySignature: false,
      receiverPath: "/api/mobile/helius/webhook",
      webhookAccountAddresses: ["11111111111111111111111111111111"],
    },
    scoreSnapshot: {
      ...publicEvidence().scoreSnapshot,
      requireFinalized: false,
      participantCap: 5000,
    },
  });
  const evidencePath = writeEvidence(evidence);
  const result = await runNode([SCRIPT_PATH, "--evidence", evidencePath]);

  assert.equal(result.code, 1);
  const report = JSON.parse(result.stdout);
  assert.ok(report.issues.includes("helius.backfillAudited must be true"));
  assert.ok(
    report.issues.includes("helius.webhookAccountAddresses must contain only the Anky Seal Program ID"),
  );
  assert.ok(report.issues.includes("helius.receiverPath must be /api/helius/anky-seal"));
  assert.ok(report.issues.includes("helius.backfillMethod must be getTransactionsForAddress"));
  assert.ok(report.issues.includes("helius.backfillCommitment must be finalized"));
  assert.ok(report.issues.includes("helius.dedupeBySignature must be true"));
  assert.ok(report.issues.includes("scoreSnapshot.requireFinalized must be true"));
  assert.ok(report.issues.includes("scoreSnapshot.participantCap must be 3456"));
});

test("rejects secret-shaped evidence paths and unknown secret options", async () => {
  const secretPath = await runNode([SCRIPT_PATH, "--evidence", "/tmp/.env"]);
  assert.equal(secretPath.code, 1);
  assert.match(secretPath.stderr, /public JSON file/);

  const ankyPath = await runNode([SCRIPT_PATH, "--evidence", "/tmp/private.anky"]);
  assert.equal(ankyPath.code, 1);
  assert.match(ankyPath.stderr, /public JSON file/);

  const unknown = await runNode([SCRIPT_PATH, "--evidence", writeEvidence(publicEvidence()), "--api-key", "secret-api-key"]);
  assert.equal(unknown.code, 1);
  assert.match(unknown.stderr, /Unknown option: --api-key/);
  assert.doesNotMatch(unknown.stderr, /secret-api-key/);

  const conflicting = await runNode([
    SCRIPT_PATH,
    "--print-template",
    "--evidence",
    writeEvidence(publicEvidence()),
  ]);
  assert.equal(conflicting.code, 1);
  assert.match(conflicting.stderr, /cannot be combined/);
});

function publicEvidence(overrides = {}) {
  const base = {
    backend: {
      migrationsApplied: ["019_mobile_verified_seal_receipts", "020", "021"],
      requireVerifiedSealChainProof: true,
      url: "https://anky.example",
    },
    claims: {
      directOnchainSp1: false,
      mainnetDeployment: false,
    },
    cluster: "devnet",
    coreCollection: CORE_COLLECTION,
    devnetE2E: {
      hashSealLanded: true,
      loomAsset: LOOM_ASSET,
      proofHash: "b".repeat(64),
      sealOrbUrl: `https://orbmarkets.io/tx/${VALID_SIGNATURE}`,
      sealSignature: VALID_SIGNATURE,
      sessionHash: "a".repeat(64),
      sp1ProofVerified: true,
      utcDay: 20579,
      utcDayStatus: {
        currentUtcDay: 20579,
        receiptUtcDay: 20579,
        isCurrentDay: true,
        sealWindow: "open",
        secondsUntilRollover: 120,
        dayRolloverAt: "2026-05-07T00:00:00.000Z",
      },
      verifiedOrbUrl: `https://orbmarkets.io/tx/${SECOND_VALID_SIGNATURE}`,
      verifiedSealLanded: true,
      verifiedSignature: SECOND_VALID_SIGNATURE,
      writer: WRITER,
    },
    helius: {
      backfillAudited: true,
      backfillCommitment: "finalized",
      backfillMethod: "getTransactionsForAddress",
      dedupeBySignature: true,
      requireFinalized: true,
      receiverPath: "/api/helius/anky-seal",
      webhookAccountAddresses: [PROGRAM_ID],
      webhookId: "wh_public_123456",
      webhookType: "enhancedDevnet",
    },
    programId: PROGRAM_ID,
    proofVerifierAuthority: PROOF_VERIFIER,
    protocolVersion: 1,
    scoreSnapshot: {
      audited: true,
      formula: SCORE_FORMULA,
      participantCap: 3456,
      path: "sojourn9/devnet-score-snapshot.json",
      requireFinalized: true,
      rewardBps: 800,
    },
    snapshotTime: "2026-05-06T00:00:00.000Z",
    sp1Vkey: "0x00399c50f86cb417d0cf0c80485b0f1781590170c6892861a1a55974da6e4758",
  };

  return deepMerge(base, overrides);
}

function deepMerge(base, overrides) {
  if (overrides == null || typeof overrides !== "object" || Array.isArray(overrides)) {
    return overrides ?? base;
  }
  const merged = { ...base };
  for (const [key, value] of Object.entries(overrides)) {
    if (
      value != null &&
      typeof value === "object" &&
      !Array.isArray(value) &&
      base[key] != null &&
      typeof base[key] === "object" &&
      !Array.isArray(base[key])
    ) {
      merged[key] = deepMerge(base[key], value);
    } else {
      merged[key] = value;
    }
  }
  return merged;
}

function writeEvidence(evidence) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-launch-evidence-"));
  const evidencePath = path.join(tempDir, "public-launch-evidence.json");
  fs.writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  return evidencePath;
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
