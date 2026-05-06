import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "checkProofHandoff.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");
const WRITER = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
const LOOM_ASSET = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
const SESSION_HASH = "881ecaf0685337bdc2c92778d60464d0b00363b5e07995d3bec3c5241d845865";
const PROOF_HASH = "38154c2b641335180ac313c8081f29f0e4f0e394084e901497de3b4690cfa982";
const START_2024_MS = "1704067200000";
const START_2024_UTC_DAY = 19723;

test("prints proof handoff status usage", async () => {
  const result = await runNode([SCRIPT_PATH, "--help"]);

  assert.equal(result.code, 0, result.stderr);
  assert.match(result.stdout, /Checks a Sojourn 9 proof handoff manifest/);
});

test("validates a public manifest without reading or printing the witness path", async () => {
  const tempDir = makeTempManifest({
    utcDay: START_2024_UTC_DAY,
    witness: "/tmp/super-secret-witness.anky",
  });
  const manifestPath = path.join(tempDir, "handoff-manifest.json");

  const result = await runNode([
    SCRIPT_PATH,
    "--manifest",
    manifestPath,
    "--no-chain",
    "--now-ms",
    START_2024_MS,
    "--backend-url",
    "http://127.0.0.1:3000",
  ]);

  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.chain.checked, false);
  assert.equal(report.currentUtcDay, START_2024_UTC_DAY);
  assert.deepEqual(report.utcDayStatus, {
    currentUtcDay: START_2024_UTC_DAY,
    receiptUtcDay: START_2024_UTC_DAY,
    isCurrentDay: true,
    sealWindow: "open",
    secondsUntilRollover: 86400,
    dayRolloverAt: "2024-01-02T00:00:00.000Z",
  });
  assert.equal(report.files.witnessPathPresent, true);
  assert.equal(report.files.witnessRead, false);
  assert.equal(report.nextAction, "run_chain_status_check");
  assert.doesNotMatch(result.stdout, /super-secret-witness/);
});

test("marks stale unsealed handoffs as regenerate-current-day work in no-chain mode", async () => {
  const tempDir = makeTempManifest({
    utcDay: START_2024_UTC_DAY - 1,
  });
  const manifestPath = path.join(tempDir, "handoff-manifest.json");

  const result = await runNode([
    SCRIPT_PATH,
    "--manifest",
    manifestPath,
    "--no-chain",
    "--now-ms",
    START_2024_MS,
  ]);

  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.utcDayStatus.sealWindow, "stale");
  assert.equal(report.utcDayStatus.isCurrentDay, false);
  assert.equal(report.nextAction, "regenerate_current_day_proof");
  assert.ok(report.commands.some((command) => command.id === "prepare-current-day-proof"));
});

test("preserves public manifest inputs when regenerating a stale proof handoff", async () => {
  const tempDir = makeTempManifest({
    backendUrl: "http://127.0.0.1:3000",
    loomAsset: LOOM_ASSET,
    utcDay: START_2024_UTC_DAY - 1,
  });
  const manifestPath = path.join(tempDir, "handoff-manifest.json");

  const result = await runNode([
    SCRIPT_PATH,
    "--manifest",
    manifestPath,
    "--no-chain",
    "--now-ms",
    START_2024_MS,
  ]);

  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  const command = report.commands.find((item) => item.id === "prepare-current-day-proof")?.command;
  assert.match(command, /--loom-asset 4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9/);
  assert.match(command, /--backend-url http:\/\/127\.0\.0\.1:3000/);
  assert.match(command, /--program-id 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX/);
});

test("refuses mainnet, secret-shaped options, and credentialed backend URLs", async () => {
  const tempDir = makeTempManifest({
    utcDay: START_2024_UTC_DAY,
  });
  const manifestPath = path.join(tempDir, "handoff-manifest.json");

  const mainnet = await runNode([
    SCRIPT_PATH,
    "--manifest",
    manifestPath,
    "--cluster",
    "mainnet-beta",
  ]);
  assert.notEqual(mainnet.code, 0);
  assert.match(mainnet.stderr, /devnet-only/);

  const keypair = await runNode([SCRIPT_PATH, "--manifest", manifestPath, "--keypair", "/tmp/id.json"]);
  assert.notEqual(keypair.code, 0);
  assert.match(keypair.stderr, /Unknown option: --keypair/);

  const backend = await runNode([
    SCRIPT_PATH,
    "--manifest",
    manifestPath,
    "--backend-url",
    "https://user:pass@anky.example",
  ]);
  assert.notEqual(backend.code, 0);
  assert.match(backend.stderr, /backend URL must not contain credentials/);
});

test("Helius backfill handoff command includes backfill mode and no unsupported flags", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  const start = source.indexOf('if (nextAction === "backfill_or_post_verified_metadata")');
  const end = source.indexOf('if (nextAction === "regenerate_current_day_proof")');

  assert.ok(start > 0, "missing backfill command branch");
  assert.ok(end > start, "missing next command branch");

  const branch = source.slice(start, end);
  assert.match(branch, /sojourn9:index/);
  assert.match(branch, /--backfill/);
  assert.match(branch, /flag\("--limit", "100"\)/);
  assert.match(branch, /ANKY_CORE_COLLECTION/);
  assert.match(branch, /flag\("--program-id", programId\)/);
  assert.match(branch, /flag\("--proof-verifier", manifest\.verifiedSeal\.verifier\)/);
  assert.doesNotMatch(branch, /flag\("--writer"/);
});

test("post-verified handoffs mark HashSeal ready when the verified-chain check succeeds", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");

  assert.match(source, /function normalizePostVerifiedHashSealStatus/);
  assert.match(source, /VerifiedSeal account already exists/);
  assert.match(source, /VerifiedSeal already landed; HashSeal was confirmed by the verified-chain check/);
  assert.match(source, /chain\?\.verifiedSealLanded\?\.ok === true/);
});

test("VerifiedSeal handoff command pins the manifest program ID", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  const start = source.indexOf('if (nextAction === "send_verifiedseal")');
  const end = source.indexOf('if (nextAction === "backfill_or_post_verified_metadata")');

  assert.ok(start > 0, "missing VerifiedSeal command branch");
  assert.ok(end > start, "missing next command branch");

  const branch = source.slice(start, end);
  assert.match(branch, /sojourn9:prove-record/);
  assert.match(branch, /flag\("--program-id", programId\)/);
});

test("HashSeal handoff command pins the manifest program ID for older manifests", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  const start = source.indexOf('if (nextAction === "send_hashseal")');
  const end = source.indexOf('if (nextAction === "send_verifiedseal")');

  assert.ok(start > 0, "missing HashSeal command branch");
  assert.ok(end > start, "missing next command branch");

  const branch = source.slice(start, end);
  assert.match(branch, /pinProgramId\([\s\S]*programId,\n\s*\)/);
  assert.match(source, /function pinProgramId\([\s\S]*command\.includes\("--program-id"\)[\s\S]*flag\("--program-id", programId\)/);
});

test("HashSeal handoff avoids backend posts when backend status checks fail", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  const start = source.indexOf('if (nextAction === "send_hashseal")');
  const end = source.indexOf('if (nextAction === "send_verifiedseal")');

  assert.ok(start > 0, "missing HashSeal command branch");
  assert.ok(end > start, "missing next command branch");

  const branch = source.slice(start, end);
  assert.match(branch, /const includeBackendUrl = backend == null \|\| backendReadyForPosts\(backend\)/);
  assert.match(branch, /backendUrl: includeBackendUrl \? backendUrl : null/);
  assert.match(branch, /hashseal-send-chain-only/);
  assert.match(branch, /seal-backend-post-after-landing/);
  assert.match(branch, /stripBackendUrl\(manifest\.nextHumanCommand\)/);
  assert.match(source, /function backendReadyForPosts\([\s\S]*sealLookup\?\.ok === true && backend\?\.score\?\.ok === true/);
});

test("VerifiedSeal handoff avoids backend posts when backend status checks fail", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  const start = source.indexOf('if (nextAction === "send_verifiedseal")');
  const end = source.indexOf('if (nextAction === "backfill_or_post_verified_metadata")');

  assert.ok(start > 0, "missing VerifiedSeal command branch");
  assert.ok(end > start, "missing next command branch");

  const branch = source.slice(start, end);
  assert.match(branch, /const includeBackendUrl = backend == null \|\| backendReadyForPosts\(backend\)/);
  assert.match(branch, /verifiedseal-send-chain-only/);
  assert.match(branch, /verifiedseal-backend-post-after-landing/);
  assert.match(branch, /ANKY_INDEXER_WRITE_SECRET/);
  assert.match(branch, /--check-verified-chain/);
  assert.match(branch, /flag\("--backend-signature", "<landed_verified_signature>"\)/);
});

test("Helius handoff avoids backend posts when backend status checks fail", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  const start = source.indexOf('if (nextAction === "backfill_or_post_verified_metadata")');
  const end = source.indexOf('if (nextAction === "regenerate_current_day_proof")');

  assert.ok(start > 0, "missing Helius command branch");
  assert.ok(end > start, "missing next command branch");

  const branch = source.slice(start, end);
  assert.match(branch, /const includeBackendUrl = backend == null \|\| backendReadyForPosts\(backend\)/);
  assert.match(branch, /helius-backfill-snapshot-only/);
  assert.match(branch, /helius-backfill-with-backend-after-reachable/);
  assert.match(branch, /includeBackendUrl && backendUrl != null/);
  assert.match(branch, /ANKY_CORE_COLLECTION/);
  assert.match(branch, /ANKY_INDEXER_WRITE_SECRET/);
});

function makeTempManifest({ backendUrl = null, loomAsset = null, utcDay, witness = "/tmp/demo.anky" }) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-handoff-status-test-"));
  const proof = path.join(tempDir, "proof-with-public-values.bin");
  const receipt = path.join(tempDir, "receipt.json");
  const verifiedReceipt = path.join(tempDir, "verified-receipt.json");
  fs.writeFileSync(proof, "not a real proof\n");
  fs.writeFileSync(receipt, "{}\n");
  fs.writeFileSync(verifiedReceipt, "{}\n");
  fs.writeFileSync(
    path.join(tempDir, "handoff-manifest.json"),
    `${JSON.stringify(
      {
        cluster: "devnet",
        files: {
          proof,
          receipt,
          verifiedReceipt,
          witness,
        },
        nextHumanCommand:
          "cd solana/anky-seal-program && ANKY_SEALER_KEYPAIR_PATH='<writer_keypair_path>' npm run seal -- --send",
        programId: "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX",
        publicInputs: {
          backendUrl,
          loomAsset,
        },
        publicReceipt: {
          acceptedDurationMs: 472000,
          eventCount: 60,
          proofHash: PROOF_HASH,
          riteDurationMs: 480000,
          sessionHash: SESSION_HASH,
          utcDay,
          valid: true,
          writer: WRITER,
        },
      },
      null,
      2,
    )}\n`,
  );
  return tempDir;
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
