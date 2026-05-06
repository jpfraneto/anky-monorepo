import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "heliusWebhookManifest.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");
const PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";

test("prints a Helius enhanced webhook manifest without reading an API key", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--webhook-url",
    "https://anky.example/api/helius/anky-seal",
  ]);

  assert.equal(result.code, 0, result.stderr);
  assert.equal(result.stderr, "");
  const manifest = JSON.parse(result.stdout);
  assert.equal(manifest.cluster, "devnet");
  assert.equal(
    manifest.createEndpoint,
    "https://api-devnet.helius-rpc.com/v0/webhooks?api-key=$HELIUS_API_KEY",
  );
  assert.equal(manifest.payload.webhookType, "enhancedDevnet");
  assert.equal(manifest.payload.webhookURL, "https://anky.example/api/helius/anky-seal");
  assert.deepEqual(manifest.payload.accountAddresses, [PROGRAM_ID]);
  assert.deepEqual(manifest.payload.transactionTypes, ["ANY"]);
  assert.equal(manifest.payload.authHeader, "Bearer $ANKY_INDEXER_WRITE_SECRET");
  assert.ok(
    manifest.notes.some((note) =>
      note.includes("retries failed deliveries with exponential backoff for up to 24 hours"),
    ),
  );
  assert.ok(
    manifest.notes.some((note) =>
      note.includes("auto-disable webhooks with very high delivery failure rates"),
    ),
  );
  assert.ok(
    manifest.notes.some((note) => note.includes("cannot deliver to private localhost")),
  );
  assert.doesNotMatch(result.stdout, /secret-api-key/);
});

test("writes a mainnet manifest when explicitly requested", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-helius-manifest-"));
  const outPath = path.join(tempDir, "webhook.json");
  const result = await runNode([
    SCRIPT_PATH,
    "--cluster",
    "mainnet-beta",
    "--webhook-url",
    "https://anky.example/api/helius/anky-seal",
    "--transaction-types",
    "ANY,TRANSFER",
    "--out",
    outPath,
  ]);

  assert.equal(result.code, 0, result.stderr);
  assert.match(result.stdout, /^wrote /);
  const manifest = JSON.parse(fs.readFileSync(outPath, "utf8"));
  assert.equal(manifest.cluster, "mainnet-beta");
  assert.equal(
    manifest.createEndpoint,
    "https://api-mainnet.helius-rpc.com/v0/webhooks?api-key=$HELIUS_API_KEY",
  );
  assert.equal(manifest.payload.webhookType, "enhanced");
  assert.deepEqual(manifest.payload.transactionTypes, ["ANY", "TRANSFER"]);
});

test("rejects webhook URLs that could leak credentials or are not HTTPS", async () => {
  const withCredentials = await runNode([
    SCRIPT_PATH,
    "--webhook-url",
    "https://user:pass@anky.example/api/helius/anky-seal",
  ]);
  assert.notEqual(withCredentials.code, 0);
  assert.match(withCredentials.stderr, /must not contain credentials/);

  const http = await runNode([
    SCRIPT_PATH,
    "--webhook-url",
    "http://anky.example/api/helius/anky-seal",
  ]);
  assert.notEqual(http.code, 0);
  assert.match(http.stderr, /must use https/);
});

test("allows explicit localhost HTTP only for tunnel smoke tests", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--webhook-url",
    "http://127.0.0.1:8787/api/helius/anky-seal",
    "--allow-http-localhost",
  ]);

  assert.equal(result.code, 0, result.stderr);
  assert.equal(
    JSON.parse(result.stdout).payload.webhookURL,
    "http://127.0.0.1:8787/api/helius/anky-seal",
  );
});

test("rejects malformed program IDs, transaction types, and unknown flags", async () => {
  const badProgram = await runNode([
    SCRIPT_PATH,
    "--webhook-url",
    "https://anky.example/api/helius/anky-seal",
    "--program-id",
    "not-a-pubkey",
  ]);
  assert.notEqual(badProgram.code, 0);
  assert.match(badProgram.stderr, /program ID must be a base58 Solana public key/);

  const badType = await runNode([
    SCRIPT_PATH,
    "--webhook-url",
    "https://anky.example/api/helius/anky-seal",
    "--transaction-types",
    "any",
  ]);
  assert.notEqual(badType.code, 0);
  assert.match(badType.stderr, /Invalid Helius transaction type/);

  const unknown = await runNode([
    SCRIPT_PATH,
    "--webhook-url",
    "https://anky.example/api/helius/anky-seal",
    "--api-key",
    "secret-api-key",
  ]);
  assert.notEqual(unknown.code, 0);
  assert.match(unknown.stderr, /Unknown option: --api-key/);
  assert.doesNotMatch(unknown.stderr, /secret-api-key/);
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
          HELIUS_API_KEY: "secret-api-key",
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
