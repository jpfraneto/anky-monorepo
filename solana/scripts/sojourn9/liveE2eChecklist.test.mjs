import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "liveE2eChecklist.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");
const WRITER = "4vJ9JU1bJJE96FWS5zNtVM6DfHyWixJjx5KJ4LJh5S7K";
const LOOM_ASSET = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
const SESSION_HASH = "28a7b5c28dbed9f0047321860dd6b060fe3fd7fce15480621e1eb65276a659e1";
const START_2024_MS = "1704067200000";
const START_2024_UTC_DAY = "19723";

test("prints a no-secret devnet E2E checklist from public inputs", async () => {
  const result = await runNode(
    [
      SCRIPT_PATH,
      "--writer",
      WRITER,
      "--loom-asset",
      LOOM_ASSET,
      "--session-hash",
      SESSION_HASH,
      "--utc-day",
      START_2024_UTC_DAY,
      "--now-ms",
      START_2024_MS,
      "--backend-url",
      "http://127.0.0.1:3000",
      "--webhook-url",
      "https://anky.example/api/helius/anky-seal",
    ],
    {
      ANKY_INDEXER_WRITE_SECRET: "actual-secret",
      ANKY_VERIFIER_KEYPAIR_PATH: "/do/not/print.json",
      HELIUS_API_KEY: "actual-helius-key",
    },
  );

  assert.equal(result.code, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.cluster, "devnet");
  assert.equal(report.currentUtcDay, 19723);
  assert.deepEqual(report.utcDayStatus, {
    currentUtcDay: 19723,
    receiptUtcDay: 19723,
    isCurrentDay: true,
    sealWindow: "open",
    secondsUntilRollover: 86400,
    dayRolloverAt: "2024-01-02T00:00:00.000Z",
  });
  assert.equal(report.launchReadyAfterChecklist, false);
  assert.equal(report.publicInputs.writer, WRITER);
  assert.equal(report.publicInputs.sessionHash, SESSION_HASH);
  assert.ok(report.commands.some((command) => command.id === "hashseal-send"));
  assert.ok(report.commands.some((command) => command.id === "sp1-verifiedseal-send"));
  assert.ok(report.commands.some((command) => command.id === "helius-webhook-manifest"));
  assert.match(
    report.commands.find((command) => command.id === "public-devnet-config").command,
    /^cd solana\/anky-seal-program && npm run check-config -- --cluster devnet /,
  );
  assert.match(
    report.commands.find((command) => command.id === "sp1-verifiedseal-send").command,
    /ANKY_VERIFIER_KEYPAIR_PATH=<verifier_authority_keypair_path>/,
  );
  assert.match(
    report.commands.find((command) => command.id === "helius-backfill").command,
    /HELIUS_API_KEY=<configured_in_shell>/,
  );
  assert.doesNotMatch(result.stdout, /actual-secret|actual-helius-key|do\/not\/print/);
});

test("refuses stale UTC days before printing operator commands", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    "19722",
    "--now-ms",
    START_2024_MS,
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /is not current UTC day 19723/);
  assert.equal(result.stdout, "");
});

test("refuses mainnet and secret-shaped options", async () => {
  const mainnet = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    START_2024_UTC_DAY,
    "--now-ms",
    START_2024_MS,
    "--cluster",
    "mainnet-beta",
  ]);
  assert.notEqual(mainnet.code, 0);
  assert.match(mainnet.stderr, /devnet-only/);

  const keypair = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    START_2024_UTC_DAY,
    "--now-ms",
    START_2024_MS,
    "--keypair",
    "/tmp/id.json",
  ]);
  assert.notEqual(keypair.code, 0);
  assert.match(keypair.stderr, /Unknown option: --keypair/);
});

test("rejects backend and webhook URLs that could leak credentials", async () => {
  const backend = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    START_2024_UTC_DAY,
    "--now-ms",
    START_2024_MS,
    "--backend-url",
    "https://user:pass@anky.example",
  ]);
  assert.notEqual(backend.code, 0);
  assert.match(backend.stderr, /must not contain credentials/);

  const webhook = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    START_2024_UTC_DAY,
    "--now-ms",
    START_2024_MS,
    "--webhook-url",
    "http://127.0.0.1:3000/api/helius/anky-seal",
  ]);
  assert.notEqual(webhook.code, 0);
  assert.match(webhook.stderr, /webhook URL must use HTTPS/);
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
