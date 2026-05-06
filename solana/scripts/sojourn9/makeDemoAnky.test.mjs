import assert from "node:assert/strict";
import crypto from "node:crypto";
import { execFile } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "makeDemoAnky.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");

test("writes a deterministic demo witness outside the repo and prints public metadata only", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-demo-witness-"));
  const outPath = path.join(tempDir, "demo.anky");

  const result = await runNode([
    SCRIPT_PATH,
    "--out",
    outPath,
    "--started-at-ms",
    "1704067200000",
    "--character",
    "SPACE",
  ]);

  assert.equal(result.code, 0, result.stderr);
  const metadata = JSON.parse(result.stdout);
  const raw = fs.readFileSync(outPath, "utf8");
  assert.equal(metadata.witnessPath, outPath);
  assert.equal(metadata.utcDay, 19723);
  assert.equal(metadata.eventCount, 61);
  assert.equal(metadata.acceptedDurationMs, 479940);
  assert.equal(metadata.riteDurationMs, 487940);
  assert.equal(metadata.sessionHash, sha256Hex(raw));
  assert.equal(raw.split("\n").length, 62);
  assert.match(raw, /^1704067200000 SPACE\n7999 SPACE\n/);
  assert.ok(raw.endsWith("\n8000"));
  assert.doesNotMatch(result.stdout, /7999 SPACE/);
  assert.doesNotMatch(result.stdout, /^1704067200000 SPACE/m);
});

test("refuses to write demo plaintext inside the git worktree", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--out",
    path.join(REPO_ROOT, "sojourn9-demo.anky"),
    "--started-at-ms",
    "1704067200000",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Refusing to write demo \.anky plaintext inside this git worktree/);
});

test("refuses to overwrite an existing witness unless forced", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-demo-witness-"));
  const outPath = path.join(tempDir, "demo.anky");
  fs.writeFileSync(outPath, "existing", "utf8");

  const result = await runNode([
    SCRIPT_PATH,
    "--out",
    outPath,
    "--started-at-ms",
    "1704067200000",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /already exists/);

  const forced = await runNode([
    SCRIPT_PATH,
    "--out",
    outPath,
    "--started-at-ms",
    "1704067200000",
    "--force",
  ]);
  assert.equal(forced.code, 0, forced.stderr);
  assert.notEqual(fs.readFileSync(outPath, "utf8"), "existing");
});

test("rejects unknown demo witness options", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-demo-witness-"));
  const result = await runNode([
    SCRIPT_PATH,
    "--out",
    path.join(tempDir, "demo.anky"),
    "--writer",
    "4vJ9JU1bJJE96FWS5zNtVM6DfHyWixJjx5KJ4LJh5S7K",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Unknown option: --writer/);
});

function runNode(args) {
  return new Promise((resolve) => {
    execFile(process.execPath, args, { cwd: REPO_ROOT }, (error, stdout, stderr) => {
      resolve({
        code: error?.code ?? 0,
        stderr,
        stdout,
      });
    });
  });
}

function sha256Hex(raw) {
  return crypto.createHash("sha256").update(Buffer.from(raw, "utf8")).digest("hex");
}
