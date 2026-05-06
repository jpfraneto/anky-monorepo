import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const AUDIT_PATH = path.join(SCRIPT_DIR, "auditScoreSnapshot.mjs");
const INDEXER_PATH = path.join(SCRIPT_DIR, "ankySealIndexer.mjs");
const FIXTURE_PATH = path.join(SCRIPT_DIR, "fixtures", "anky-seal-events.json");
const REPO_ROOT = path.resolve(SCRIPT_DIR, "../../..");

test("audits a finalized score snapshot with reward allocation", async () => {
  const snapshotPath = await writeFixtureSnapshot(["--token-supply", "1000"]);
  const result = await runNode([
    AUDIT_PATH,
    "--snapshot",
    snapshotPath,
    "--require-allocation",
  ]);

  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.deepEqual(report.issues, []);
});

test("rejects snapshots whose scores do not recompute from public events", async () => {
  const snapshotPath = await writeFixtureSnapshot([]);
  const snapshot = JSON.parse(fs.readFileSync(snapshotPath, "utf8"));
  snapshot.scores[0].score += 1;
  fs.writeFileSync(snapshotPath, `${JSON.stringify(snapshot, null, 2)}\n`);

  const result = await runNode([AUDIT_PATH, "--snapshot", snapshotPath]);

  assert.notEqual(result.code, 0);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, false);
  assert.ok(report.issues.some((issue) => /scores do not recompute/.test(issue)));
});

test("rejects non-finalized reward snapshots unless explicitly allowed", async () => {
  const snapshotPath = await writeFixtureSnapshot([]);
  const snapshot = JSON.parse(fs.readFileSync(snapshotPath, "utf8"));
  snapshot.requireFinalized = false;
  snapshot.events[0].commitment = "confirmed";
  snapshot.events[0].finalized = false;
  fs.writeFileSync(snapshotPath, `${JSON.stringify(snapshot, null, 2)}\n`);

  const result = await runNode([AUDIT_PATH, "--snapshot", snapshotPath]);

  assert.notEqual(result.code, 0);
  const report = JSON.parse(result.stdout);
  assert.ok(report.issues.some((issue) => /requireFinalized/.test(issue)));
  assert.ok(report.issues.some((issue) => /not finalized/.test(issue)));
});

test("rejects private-looking fields and unknown options", async () => {
  const snapshotPath = await writeFixtureSnapshot([]);
  const snapshot = JSON.parse(fs.readFileSync(snapshotPath, "utf8"));
  snapshot.rawAnky = "private text";
  fs.writeFileSync(snapshotPath, `${JSON.stringify(snapshot, null, 2)}\n`);

  const privateField = await runNode([AUDIT_PATH, "--snapshot", snapshotPath]);
  assert.notEqual(privateField.code, 0);
  assert.match(privateField.stdout, /private\/plaintext-like field/);

  const unknown = await runNode([AUDIT_PATH, "--snapshot", snapshotPath, "--keypair", "/tmp/id.json"]);
  assert.notEqual(unknown.code, 0);
  assert.match(unknown.stderr, /Unknown option: --keypair/);

  const ankyPath = await runNode([AUDIT_PATH, "--snapshot", "/tmp/private.anky"]);
  assert.notEqual(ankyPath.code, 0);
  assert.match(ankyPath.stderr, /--snapshot must point to a public JSON file/);
});

test("rejects complete .anky plaintext values under generic fields", async () => {
  const snapshotPath = await writeFixtureSnapshot([]);
  const snapshot = JSON.parse(fs.readFileSync(snapshotPath, "utf8"));
  snapshot.operatorNote = "1710000000000 a\n0001 SPACE\n8000";
  fs.writeFileSync(snapshotPath, `${JSON.stringify(snapshot, null, 2)}\n`);

  const result = await runNode([AUDIT_PATH, "--snapshot", snapshotPath]);

  assert.notEqual(result.code, 0);
  assert.match(result.stdout, /complete \.anky plaintext-like value/);
});

async function writeFixtureSnapshot(extraArgs) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-snapshot-audit-"));
  const snapshotPath = path.join(tempDir, "snapshot.json");
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
