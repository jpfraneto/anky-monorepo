import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "prepareCurrentDayProof.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");
const WRITER = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";

test("prints current-day proof preparation usage", async () => {
  const result = await runNode([SCRIPT_PATH, "--help"]);

  assert.equal(result.code, 0, result.stderr);
  assert.match(result.stdout, /Generates a same-day demo \.anky witness outside the repo/);
});

test("refuses to write proof handoff artifacts inside the repo", async () => {
  const result = await runNode([SCRIPT_PATH, "--writer", WRITER, "--out-dir", "sojourn9/proof"]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Refusing to write demo witness or SP1 artifacts inside this git worktree/);
});

test("refuses mainnet proof preparation and secret-shaped options", async () => {
  const mainnet = await runNode([SCRIPT_PATH, "--writer", WRITER, "--cluster", "mainnet-beta"]);
  assert.notEqual(mainnet.code, 0);
  assert.match(mainnet.stderr, /devnet-only/);

  const keypair = await runNode([SCRIPT_PATH, "--writer", WRITER, "--keypair", "/tmp/id.json"]);
  assert.notEqual(keypair.code, 0);
  assert.match(keypair.stderr, /Unknown option: --keypair/);
});

test("rejects backend URLs that could leak credentials", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--backend-url",
    "https://user:pass@anky.example",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /backend URL must not contain credentials/);
});

test("pins the proved program ID in generated HashSeal and VerifiedSeal handoff commands", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  assert.match(source, /utcDayStatus,/);
  assert.match(source, /function buildUtcDayStatus/);
  assert.match(source, /sealWindow/);
  assert.match(source, /secondsUntilRollover/);
  assert.match(source, /dayRolloverAt/);
  assert.match(source, /verifiedSealSendCommand\(\{[\s\S]*programId: proveSummary\.programId/);
  assert.match(source, /sealSendCommand\(\{[\s\S]*programId: proveSummary\.programId/);
  assert.match(source, /function sealSendCommand\(\{[\s\S]*programId[\s\S]*flag\("--program-id", programId\)/);
  assert.match(source, /function verifiedSealSendCommand\(\{[\s\S]*programId[\s\S]*flag\("--program-id", programId\)/);
});

test("keeps proof-prep chain commands separate from backend metadata posts", () => {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  const sealCommand = functionSource(source, "sealSendCommand", "verifiedSealSendCommand");
  const verifiedCommand = functionSource(source, "verifiedSealSendCommand", "backendFollowupCommands");

  assert.doesNotMatch(sealCommand, /backendUrl|--backend-url|ANKY_INDEXER_WRITE_SECRET/);
  assert.doesNotMatch(verifiedCommand, /backendUrl|--backend-url|ANKY_INDEXER_WRITE_SECRET/);
  assert.match(verifiedCommand, /cd solana\/anky-seal-program &&/);
  assert.match(verifiedCommand, /npm run sojourn9:prove-record --/);
  assert.match(source, /backendFollowupCommands:\s*backendFollowupCommands\(\{/);
  assert.match(source, /id: "seal-backend-post-after-landing"/);
  assert.match(source, /id: "verifiedseal-backend-post-after-landing"/);
  assert.match(source, /flag\("--backend-signature", "<landed_seal_signature>"\)/);
  assert.match(source, /flag\("--backend-signature", "<landed_verified_signature>"\)/);
});

function functionSource(source, startName, endName) {
  const start = source.indexOf(`function ${startName}`);
  const end = source.indexOf(`function ${endName}`, start + 1);
  assert.notEqual(start, -1, `${startName} source not found`);
  assert.notEqual(end, -1, `${endName} source not found`);
  return source.slice(start, end);
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
