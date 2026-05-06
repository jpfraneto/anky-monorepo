import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const SCRIPT_PATH = path.join(SCRIPT_DIR, "makeLaunchEvidence.mjs");
const INDEXER_PATH = path.resolve(SCRIPT_DIR, "../indexer/ankySealIndexer.mjs");
const FIXTURE_PATH = path.resolve(SCRIPT_DIR, "../indexer/fixtures/anky-seal-events.json");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");
const PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const CORE_COLLECTION = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const PROOF_VERIFIER = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const WRITER = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
const LOOM_ASSET = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
const SESSION_HASH = "881ecaf0685337bdc2c92778d60464d0b00363b5e07995d3bec3c5241d845865";
const PROOF_HASH = "38154c2b641335180ac313c8081f29f0e4f0e394084e901497de3b4690cfa982";
const SP1_VKEY = "0x00399c50f86cb417d0cf0c80485b0f1781590170c6892861a1a55974da6e4758";
const SEAL_SIGNATURE =
  "2hntvJaJzRkFWt3hTa7Q9oiGyVsTpjMwmzY8WcN52UDMsTyMuzKUtcEhupAe7BcZGeq49dFBhhgoYgeZ79m53sNh";
const VERIFIED_SIGNATURE = "5".repeat(88);

test("builds audited public launch evidence from a handoff manifest without reading witness data", async () => {
  const manifestPath = writeManifest();
  const result = await runNode(baseArgs(manifestPath, ["--audit"]), {
    ANKY_INDEXER_WRITE_SECRET: "actual-backend-secret",
    HELIUS_API_KEY: "actual-helius-key",
  });

  assert.equal(result.code, 0, result.stderr);
  const evidence = JSON.parse(result.stdout);
  assert.equal(evidence.cluster, "devnet");
  assert.equal(evidence.programId, PROGRAM_ID);
  assert.equal(evidence.coreCollection, CORE_COLLECTION);
  assert.equal(evidence.proofVerifierAuthority, PROOF_VERIFIER);
  assert.equal(evidence.sp1Vkey, SP1_VKEY);
  assert.equal(evidence.devnetE2E.writer, WRITER);
  assert.equal(evidence.devnetE2E.loomAsset, LOOM_ASSET);
  assert.equal(evidence.devnetE2E.sessionHash, SESSION_HASH);
  assert.equal(evidence.devnetE2E.proofHash, PROOF_HASH);
  assert.deepEqual(evidence.devnetE2E.utcDayStatus, {
    currentUtcDay: 20579,
    receiptUtcDay: 20579,
    isCurrentDay: true,
    sealWindow: "open",
    secondsUntilRollover: 120,
    dayRolloverAt: "2026-05-07T00:00:00.000Z",
  });
  assert.equal(evidence.devnetE2E.sealSignature, SEAL_SIGNATURE);
  assert.equal(evidence.devnetE2E.sealOrbUrl, `https://orbmarkets.io/tx/${SEAL_SIGNATURE}`);
  assert.equal(evidence.devnetE2E.verifiedSignature, VERIFIED_SIGNATURE);
  assert.equal(
    evidence.devnetE2E.verifiedOrbUrl,
    `https://orbmarkets.io/tx/${VERIFIED_SIGNATURE}`,
  );
  assert.deepEqual(evidence.helius.webhookAccountAddresses, [PROGRAM_ID]);
  assert.equal(evidence.helius.receiverPath, "/api/helius/anky-seal");
  assert.equal(evidence.helius.backfillMethod, "getTransactionsForAddress");
  assert.equal(evidence.helius.backfillCommitment, "finalized");
  assert.equal(evidence.helius.backfillAudited, true);
  assert.equal(evidence.scoreSnapshot.audited, true);
  assert.doesNotMatch(result.stdout, /private-witness|actual-backend-secret|actual-helius-key/);
});

test("writes evidence to a public output path and verifies it with the auditor", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-launch-evidence-out-"));
  const manifestPath = writeManifest();
  const outPath = path.join(tempDir, "public-launch-evidence.json");

  const result = await runNode(baseArgs(manifestPath, ["--audit", "--out", outPath]));

  assert.equal(result.code, 0, result.stderr);
  assert.match(result.stdout, /^wrote /);
  const evidence = JSON.parse(fs.readFileSync(outPath, "utf8"));
  assert.equal(evidence.backend.requireVerifiedSealChainProof, true);
  assert.equal(evidence.claims.directOnchainSp1, false);
});

test("can run the public score snapshot auditor instead of trusting a score-audited flag", async () => {
  const manifestPath = writeManifest();
  const scoreSnapshotPath = await writeFixtureSnapshot(["--token-supply", "1000"]);
  const args = replaceArg(
    baseArgs(manifestPath, ["--audit", "--audit-score-snapshot"]).filter(
      (arg) => arg !== "--score-audited",
    ),
    "--score-snapshot",
    scoreSnapshotPath,
  );

  const result = await runNode(args);

  assert.equal(result.code, 0, result.stderr);
  const evidence = JSON.parse(result.stdout);
  assert.equal(evidence.scoreSnapshot.path, scoreSnapshotPath);
  assert.equal(evidence.scoreSnapshot.audited, true);
});

test("derives UTC-day status from legacy public handoff metadata", async () => {
  const manifestPath = writeManifest({
    currentUtcDay: 20579,
    generatedAt: "2026-05-06T08:48:51.406Z",
    utcDayStatus: null,
  });
  const result = await runNode(baseArgs(manifestPath, ["--audit"]));

  assert.equal(result.code, 0, result.stderr);
  const evidence = JSON.parse(result.stdout);
  assert.deepEqual(evidence.devnetE2E.utcDayStatus, {
    currentUtcDay: 20579,
    receiptUtcDay: 20579,
    isCurrentDay: true,
    sealWindow: "open",
    secondsUntilRollover: 54668,
    dayRolloverAt: "2026-05-07T00:00:00.000Z",
  });
});

test("requires explicit score and Helius audit confirmations", async () => {
  const manifestPath = writeManifest();
  const withoutScore = await runNode(baseArgs(manifestPath).filter((arg) => arg !== "--score-audited"));
  assert.equal(withoutScore.code, 1);
  assert.match(withoutScore.stderr, /--score-audited is required/);

  const withoutBackfill = await runNode(
    baseArgs(manifestPath).filter((arg) => arg !== "--backfill-audited"),
  );
  assert.equal(withoutBackfill.code, 1);
  assert.match(withoutBackfill.stderr, /--backfill-audited is required/);

  const missingUtcDayStatus = await runNode(baseArgs(writeManifest({ utcDayStatus: null })));
  assert.equal(missingUtcDayStatus.code, 1);
  assert.match(missingUtcDayStatus.stderr, /manifest\.currentUtcDay must be a non-negative safe integer/);

  const inconsistentUtcDayStatus = await runNode(
    baseArgs(
      writeManifest({
        utcDayStatus: {
          currentUtcDay: 20579,
          receiptUtcDay: 20578,
          isCurrentDay: true,
          sealWindow: "open",
          secondsUntilRollover: 120,
          dayRolloverAt: "2026-05-07T00:00:00.000Z",
        },
      }),
    ),
  );
  assert.equal(inconsistentUtcDayStatus.code, 1);
  assert.match(
    inconsistentUtcDayStatus.stderr,
    /manifest\.utcDayStatus\.receiptUtcDay must match manifest\.publicReceipt\.utcDay/,
  );

  const missingSnapshot = await runNode(
    replaceArg(
      baseArgs(manifestPath, ["--audit-score-snapshot"]).filter(
        (arg) => arg !== "--score-audited",
      ),
      "--score-snapshot",
      "sojourn9/missing-score-snapshot.json",
    ),
  );
  assert.equal(missingSnapshot.code, 1);
  assert.match(missingSnapshot.stderr, /requires --score-snapshot to point to an existing public JSON file/);
});

test("rejects mainnet, fake signatures, secret paths, and credentialed backend URLs", async () => {
  const manifestPath = writeManifest();
  const mainnet = await runNode([...baseArgs(manifestPath), "--cluster", "mainnet-beta"]);
  assert.equal(mainnet.code, 1);
  assert.match(mainnet.stderr, /devnet-handoff only/);

  const fakeSignature = await runNode(
    replaceArg(baseArgs(manifestPath), "--seal-signature", "fixture_signature"),
  );
  assert.equal(fakeSignature.code, 1);
  assert.match(fakeSignature.stderr, /seal signature must be a real 64-byte Solana signature/);

  const keypair = await runNode([...baseArgs(manifestPath), "--keypair", "/tmp/id.json"]);
  assert.equal(keypair.code, 1);
  assert.match(keypair.stderr, /Unknown option: --keypair/);
  assert.doesNotMatch(keypair.stderr, /id\.json/);

  const secretOut = await runNode([...baseArgs(manifestPath), "--out", "/tmp/verifier-keypair.json"]);
  assert.equal(secretOut.code, 1);
  assert.match(secretOut.stderr, /--out must be a public non-secret path/);

  const ankyManifest = await runNode(replaceArg(baseArgs(manifestPath), "--manifest", "/tmp/private.anky"));
  assert.equal(ankyManifest.code, 1);
  assert.match(ankyManifest.stderr, /--manifest must be a public non-secret path/);

  const backend = await runNode(
    replaceArg(baseArgs(manifestPath), "--backend-url", "https://user:pass@anky.example"),
  );
  assert.equal(backend.code, 1);
  assert.match(backend.stderr, /backend URL must be an HTTPS URL without credentials/);
});

test("prints usage without accepting secrets", async () => {
  const result = await runNode([SCRIPT_PATH, "--help"], {
    HELIUS_API_KEY: "actual-helius-key",
  });

  assert.equal(result.code, 0, result.stderr);
  assert.match(result.stdout, /Builds a public Sojourn 9 launch evidence JSON file/);
  assert.doesNotMatch(result.stdout, /actual-helius-key/);
});

function baseArgs(manifestPath, extra = []) {
  return [
    SCRIPT_PATH,
    "--manifest",
    manifestPath,
    "--core-collection",
    CORE_COLLECTION,
    "--sp1-vkey",
    SP1_VKEY,
    "--seal-signature",
    SEAL_SIGNATURE,
    "--verified-signature",
    VERIFIED_SIGNATURE,
    "--backend-url",
    "https://anky.example",
    "--helius-webhook-id",
    "wh_public_123456",
    "--score-snapshot",
    "sojourn9/devnet-score-snapshot.json",
    "--snapshot-time",
    "2026-05-06T00:00:00.000Z",
    "--score-audited",
    "--backfill-audited",
    ...extra,
  ];
}

function replaceArg(args, flag, value) {
  const copy = [...args];
  const index = copy.indexOf(flag);
  assert.notEqual(index, -1);
  copy[index + 1] = value;
  return copy;
}

function writeManifest(overrides = {}) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-make-evidence-"));
  const manifestPath = path.join(tempDir, "handoff-manifest.json");
  const manifest = {
    cluster: "devnet",
    files: {
      witness: "/tmp/private-witness.anky",
    },
    programId: PROGRAM_ID,
    proofVerified: true,
    utcDayStatus: {
      currentUtcDay: 20579,
      receiptUtcDay: 20579,
      isCurrentDay: true,
      sealWindow: "open",
      secondsUntilRollover: 120,
      dayRolloverAt: "2026-05-07T00:00:00.000Z",
    },
    publicInputs: {
      loomAsset: LOOM_ASSET,
    },
    publicReceipt: {
      acceptedDurationMs: 472000,
      eventCount: 60,
      proofHash: PROOF_HASH,
      riteDurationMs: 480000,
      sessionHash: SESSION_HASH,
      utcDay: 20579,
      valid: true,
      writer: WRITER,
    },
    verifiedSeal: {
      verifier: PROOF_VERIFIER,
    },
    ...overrides,
  };
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  return manifestPath;
}

async function writeFixtureSnapshot(extraArgs) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-make-evidence-snapshot-"));
  const snapshotPath = path.join(tempDir, "devnet-score-snapshot.json");
  const result = await runNode([
    INDEXER_PATH,
    "--input",
    FIXTURE_PATH,
    "--out",
    snapshotPath,
    ...extraArgs,
  ]);
  assert.equal(result.code, 0, result.stderr);
  return snapshotPath;
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
