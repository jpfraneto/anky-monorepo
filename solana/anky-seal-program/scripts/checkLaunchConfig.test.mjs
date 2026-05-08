import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import * as anchor from "@coral-xyz/anchor";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const SCRIPT_PATH = path.join(SCRIPT_DIR, "checkLaunchConfig.mjs");
const PACKAGE_PATH = path.join(SCRIPT_DIR, "..", "package.json");
const PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const CORE_PROGRAM_ID = "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d";
const CORE_COLLECTION = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const LOOM_ASSET = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
const LOOM_OWNER = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
const PROOF_VERIFIER = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";

test("default npm test script does not run live Anchor deployment", () => {
  const packageJson = JSON.parse(fs.readFileSync(PACKAGE_PATH, "utf8"));

  assert.equal(packageJson.scripts.test.includes("anchor test"), false);
  assert.match(packageJson.scripts["test:anchor:live"], /ANKY_ALLOW_LIVE_ANCHOR_TEST/);
  assert.match(packageJson.scripts["test:anchor:live"], /anchor test/);
});

test("passes when configured public devnet accounts match launch expectations", async () => {
  const server = rpcServer(({ params }) => accountFor(params[0]));
  await listen(server);

  try {
    const result = await runNode([
      SCRIPT_PATH,
      "--rpc-url",
      `http://127.0.0.1:${server.address().port}`,
    ]);

    assert.equal(result.code, 0, result.stderr);
    const summary = JSON.parse(result.stdout);
    assert.equal(summary.ok, true);
    assert.equal(summary.programId, PROGRAM_ID);
    assert.equal(summary.coreCollection, CORE_COLLECTION);
    assert.equal(summary.proofVerifier, PROOF_VERIFIER);
    assert.deepEqual(summary.checks, {
      coreCollectionExists: true,
      coreCollectionHasCollectionV1Discriminator: true,
      coreCollectionOwnedByCore: true,
      proofVerifierIsPublicKey: true,
      sealProgramExecutable: true,
      sealProgramExists: true,
    });
  } finally {
    await close(server);
  }
});

test("passes optional real Core Loom asset checks when base fields match launch config", async () => {
  const server = rpcServer(({ params }) => accountFor(params[0]));
  await listen(server);

  try {
    const result = await runNode([
      SCRIPT_PATH,
      "--rpc-url",
      `http://127.0.0.1:${server.address().port}`,
      "--loom-asset",
      LOOM_ASSET,
      "--loom-owner",
      LOOM_OWNER,
    ]);

    assert.equal(result.code, 0, result.stderr);
    const summary = JSON.parse(result.stdout);
    assert.equal(summary.ok, true);
    assert.equal(summary.loomAsset.address, LOOM_ASSET);
    assert.equal(summary.loomAsset.owner, LOOM_OWNER);
    assert.equal(summary.loomAsset.collection, CORE_COLLECTION);
    assert.equal(summary.checks.coreLoomAssetExists, true);
    assert.equal(summary.checks.coreLoomAssetOwnedByCore, true);
    assert.equal(summary.checks.coreLoomAssetHasAssetV1Discriminator, true);
    assert.equal(summary.checks.coreLoomAssetUsesCollectionUpdateAuthority, true);
    assert.equal(summary.checks.coreLoomAssetCollectionMatchesConfig, true);
    assert.equal(summary.checks.coreLoomAssetOwnerMatchesExpected, true);
  } finally {
    await close(server);
  }
});

test("fails optional Core Loom asset checks when collection does not match launch config", async () => {
  const server = rpcServer(({ params }) => {
    if (params[0] === LOOM_ASSET) {
      return account({
        data: buildCoreAssetBase({
          collection: "11111111111111111111111111111111",
          owner: LOOM_OWNER,
        }),
        executable: false,
        owner: CORE_PROGRAM_ID,
      });
    }

    return accountFor(params[0]);
  });
  await listen(server);

  try {
    const result = await runNode([
      SCRIPT_PATH,
      "--rpc-url",
      `http://127.0.0.1:${server.address().port}`,
      "--loom-asset",
      LOOM_ASSET,
      "--loom-owner",
      LOOM_OWNER,
    ]);

    assert.notEqual(result.code, 0);
    const summary = JSON.parse(result.stdout);
    assert.equal(summary.ok, false);
    assert.equal(summary.checks.coreLoomAssetCollectionMatchesConfig, false);
  } finally {
    await close(server);
  }
});

test("fails when the configured Core collection is not owned by Metaplex Core", async () => {
  const server = rpcServer(({ params }) => {
    if (params[0] === CORE_COLLECTION) {
      return account({
        data: Buffer.from([5]),
        executable: false,
        owner: "11111111111111111111111111111111",
      });
    }

    return accountFor(params[0]);
  });
  await listen(server);

  try {
    const result = await runNode([
      SCRIPT_PATH,
      "--rpc-url",
      `http://127.0.0.1:${server.address().port}`,
    ]);

    assert.notEqual(result.code, 0);
    const summary = JSON.parse(result.stdout);
    assert.equal(summary.ok, false);
    assert.equal(summary.checks.coreCollectionOwnedByCore, false);
  } finally {
    await close(server);
  }
});

test("refuses read-only mainnet checks without explicit approval", async () => {
  const result = await runNode([SCRIPT_PATH, "--cluster", "mainnet-beta"]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Refusing mainnet readiness check/);
  assert.equal(result.stdout, "");
});

test("rejects malformed public key configuration before RPC reads", async () => {
  const result = await runNode([SCRIPT_PATH, "--program-id", "not-a-pubkey"]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /seal program ID must be a base58 Solana public key/);
  assert.equal(result.stdout, "");
});

test("rejects unknown launch config options instead of silently ignoring them", async () => {
  const result = await runNode([SCRIPT_PATH, "--proof-verifer", PROOF_VERIFIER]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Unknown option: --proof-verifer/);
  assert.equal(result.stdout, "");
});

function accountFor(address) {
  if (address === PROGRAM_ID) {
    return account({
      data: Buffer.alloc(0),
      executable: true,
      owner: "BPFLoaderUpgradeab1e11111111111111111111111",
    });
  }
  if (address === CORE_COLLECTION) {
    return account({
      data: Buffer.from([5, 1, 2, 3]),
      executable: false,
      owner: CORE_PROGRAM_ID,
    });
  }
  if (address === LOOM_ASSET) {
    return account({
      data: buildCoreAssetBase({ collection: CORE_COLLECTION, owner: LOOM_OWNER }),
      executable: false,
      owner: CORE_PROGRAM_ID,
    });
  }

  return null;
}

function buildCoreAssetBase({ collection, owner }) {
  return Buffer.concat([
    Buffer.from([1]),
    new anchor.web3.PublicKey(owner).toBuffer(),
    Buffer.from([2]),
    new anchor.web3.PublicKey(collection).toBuffer(),
  ]);
}

function account({ data, executable, owner }) {
  return {
    context: { slot: 1 },
    value: {
      data: [data.toString("base64"), "base64"],
      executable,
      lamports: 1,
      owner,
      rentEpoch: 0,
    },
  };
}

function rpcServer(handler) {
  return http.createServer((req, res) => {
    let body = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      const payload = JSON.parse(body);
      assert.equal(payload.method, "getAccountInfo");
      const result = handler(payload);
      res.writeHead(200, { "content-type": "application/json" });
      res.end(
        JSON.stringify({
          id: payload.id,
          jsonrpc: "2.0",
          result,
        }),
      );
    });
  });
}

function runNode(args, env = {}) {
  return new Promise((resolve) => {
    execFile(
      process.execPath,
      args,
      {
        cwd: path.dirname(SCRIPT_PATH),
        env: {
          ...process.env,
          ...env,
          ANKY_ALLOW_MAINNET_READINESS_CHECK: "",
          ANKY_SOLANA_RPC_URL: "",
          HELIUS_API_KEY: "",
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

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
}

function close(server) {
  return new Promise((resolve, reject) => {
    server.close((error) => {
      if (error) {
        reject(error);
      } else {
        resolve();
      }
    });
  });
}
