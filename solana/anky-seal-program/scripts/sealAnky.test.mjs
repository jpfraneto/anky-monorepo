import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import crypto from "node:crypto";
import http from "node:http";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import * as anchor from "@coral-xyz/anchor";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "sealAnky.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");
const PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const CORE_PROGRAM_ID = "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d";
const CORE_COLLECTION = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const LOOM_ASSET = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
const WRITER = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
const SESSION_HASH = "c4d8d04ee62d4c6080df750ee5a742b71bcf74d8f4e29f84a4966b1eef26d824";
const VALID_SIGNATURE = "1".repeat(64);

test("dry-runs seal_anky without reading keypairs or RPC state", async () => {
  const result = await runNode(
    [
      SCRIPT_PATH,
      "--writer",
      WRITER,
      "--loom-asset",
      LOOM_ASSET,
      "--session-hash",
      SESSION_HASH.toUpperCase(),
      "--utc-day",
      String(currentUtcDay()),
    ],
    {
      ANKY_SOLANA_RPC_URL: "https://devnet.helius-rpc.com/?api-key=secret-api-key",
    },
  );

  assert.equal(result.code, 0, result.stderr);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /dry run only/);
  assert.doesNotMatch(result.stdout, /secret-api-key/);
  const summary = JSON.parse(result.stdout.split("\n").slice(0, -2).join("\n"));
  assert.equal(summary.dryRun, true);
  assert.equal(summary.instruction, "seal_anky");
  assert.equal(summary.programId, PROGRAM_ID);
  assert.equal(summary.writer, WRITER);
  assert.equal(summary.payer, WRITER);
  assert.equal(summary.sponsored, false);
  assert.equal(summary.loomAsset, LOOM_ASSET);
  assert.equal(summary.sessionHash, SESSION_HASH);
  assert.equal(summary.receiptUtcDayIsCurrent, true);
  assert.equal(summary.rpcUrl, "https://devnet.helius-rpc.com/?api-key=<redacted>");
});

test("dry-runs seal_anky with a separate sponsor payer while keeping writer identity", async () => {
  const payer = "11111111111111111111111111111111";
  const result = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--payer",
    payer,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    String(currentUtcDay()),
  ]);

  assert.equal(result.code, 0, result.stderr);
  const summary = JSON.parse(result.stdout.split("\n").slice(0, -2).join("\n"));
  assert.equal(summary.writer, WRITER);
  assert.equal(summary.payer, payer);
  assert.equal(summary.sponsored, true);
});

test("check-chain passes when the Core Loom is owned by the writer and seals are absent", async () => {
  const server = rpcServer(({ params }) => accountFor(params[0]));
  await listen(server);

  try {
    const result = await runNode([
      SCRIPT_PATH,
      "--writer",
      WRITER,
      "--loom-asset",
      LOOM_ASSET,
      "--session-hash",
      SESSION_HASH,
      "--utc-day",
      String(currentUtcDay()),
      "--rpc-url",
      `http://127.0.0.1:${server.address().port}`,
      "--check-chain",
    ]);

    assert.equal(result.code, 0, result.stderr);
    const summary = JSON.parse(result.stdout.split("\n").slice(0, -2).join("\n"));
    assert.equal(summary.chainPreflight.ok, true);
    assert.equal(summary.chainPreflight.loomOwner, WRITER);
  } finally {
    await close(server);
  }
});

test("check-chain rejects a Core Loom owned by another wallet", async () => {
  const otherOwner = "11111111111111111111111111111111";
  const server = rpcServer(({ params }) => {
    if (params[0] === LOOM_ASSET) {
      return account({
        data: buildCoreAssetBase({ collection: CORE_COLLECTION, owner: otherOwner }),
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
      "--writer",
      WRITER,
      "--loom-asset",
      LOOM_ASSET,
      "--session-hash",
      SESSION_HASH,
      "--utc-day",
      String(currentUtcDay()),
      "--rpc-url",
      `http://127.0.0.1:${server.address().port}`,
      "--check-chain",
    ]);

    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /Core Loom asset owner does not match writer/);
  } finally {
    await close(server);
  }
});

test("check-chain rejects non-current UTC days before RPC work", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    "19675",
    "--check-chain",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /preflight requires the current UTC day/);
});

test("requires an actual send or landed signature before backend metadata posts", async () => {
  const noSignature = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    String(currentUtcDay()),
    "--backend-url",
    "http://127.0.0.1:1",
  ]);
  assert.notEqual(noSignature.code, 0);
  assert.match(noSignature.stderr, /--backend-url requires --send or --backend-signature/);

  const noChainCheck = await runNode([
    SCRIPT_PATH,
    "--writer",
    WRITER,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    String(currentUtcDay()),
    "--backend-url",
    "http://127.0.0.1:1",
    "--backend-signature",
    VALID_SIGNATURE,
  ]);
  assert.notEqual(noChainCheck.code, 0);
  assert.match(noChainCheck.stderr, /--check-sealed-chain is required/);
});

test("posts already-landed public seal metadata after a HashSeal chain check", async () => {
  const utcDay = currentUtcDay();
  const hashSeal = deriveHashSeal({ sessionHash: SESSION_HASH, utcDay, writer: WRITER });
  let posted = null;
  const rpc = rpcServer(({ params }) => {
    if (params[0] === hashSeal) {
      return account({
        data: buildHashSealAccountData({
          loomAsset: LOOM_ASSET,
          sessionHash: SESSION_HASH,
          utcDay,
          writer: WRITER,
        }),
        executable: false,
        owner: PROGRAM_ID,
      });
    }

    return accountFor(params[0]);
  });
  const backend = http.createServer((req, res) => {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      posted = {
        body: JSON.parse(body),
        method: req.method,
        path: req.url,
      };
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ recorded: true }));
    });
  });
  await Promise.all([listen(rpc), listen(backend)]);

  try {
    const result = await runNode([
      SCRIPT_PATH,
      "--writer",
      WRITER,
      "--loom-asset",
      LOOM_ASSET,
      "--session-hash",
      SESSION_HASH,
      "--utc-day",
      String(utcDay),
      "--rpc-url",
      `http://127.0.0.1:${rpc.address().port}`,
      "--backend-url",
      `http://127.0.0.1:${backend.address().port}`,
      "--backend-signature",
      VALID_SIGNATURE,
      "--check-sealed-chain",
    ]);

    assert.equal(result.code, 0, result.stderr);
    const summary = JSON.parse(result.stdout.split("\n").slice(0, -2).join("\n"));
    assert.equal(summary.sealedChain.ok, true);
    assert.equal(summary.backendPost.ok, true);
    assert.deepEqual(posted, {
      body: {
        coreCollection: CORE_COLLECTION,
        loomAsset: LOOM_ASSET,
        sessionHash: SESSION_HASH,
        signature: VALID_SIGNATURE,
        status: "confirmed",
        utcDay,
        wallet: WRITER,
      },
      method: "POST",
      path: "/api/mobile/seals/record",
    });
  } finally {
    await Promise.all([close(rpc), close(backend)]);
  }
});

test("sealed-chain backend post rejects a HashSeal recorded for a different Loom", async () => {
  const utcDay = currentUtcDay();
  const hashSeal = deriveHashSeal({ sessionHash: SESSION_HASH, utcDay, writer: WRITER });
  const differentLoom = "11111111111111111111111111111111";
  const rpc = rpcServer(({ params }) => {
    if (params[0] === hashSeal) {
      return account({
        data: buildHashSealAccountData({
          loomAsset: differentLoom,
          sessionHash: SESSION_HASH,
          utcDay,
          writer: WRITER,
        }),
        executable: false,
        owner: PROGRAM_ID,
      });
    }

    return accountFor(params[0]);
  });
  const backend = http.createServer((_req, res) => {
    res.writeHead(500);
    res.end("should not be called");
  });
  await Promise.all([listen(rpc), listen(backend)]);

  try {
    const result = await runNode([
      SCRIPT_PATH,
      "--writer",
      WRITER,
      "--loom-asset",
      LOOM_ASSET,
      "--session-hash",
      SESSION_HASH,
      "--utc-day",
      String(utcDay),
      "--rpc-url",
      `http://127.0.0.1:${rpc.address().port}`,
      "--backend-url",
      `http://127.0.0.1:${backend.address().port}`,
      "--backend-signature",
      VALID_SIGNATURE,
      "--check-sealed-chain",
    ]);

    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /HashSeal account does not match writer, Loom asset/);
  } finally {
    await Promise.all([close(rpc), close(backend)]);
  }
});

test("send mode refuses mainnet and requires a keypair on devnet", async () => {
  const mainnet = await runNode([
    SCRIPT_PATH,
    "--cluster",
    "mainnet-beta",
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    String(currentUtcDay()),
    "--send",
  ]);
  assert.notEqual(mainnet.code, 0);
  assert.match(mainnet.stderr, /Refusing mainnet seal_anky/);

  const devnet = await runNode([
    SCRIPT_PATH,
    "--loom-asset",
    LOOM_ASSET,
    "--session-hash",
    SESSION_HASH,
    "--utc-day",
    String(currentUtcDay()),
    "--send",
  ]);
  assert.notEqual(devnet.code, 0);
  assert.match(devnet.stderr, /ANKY_SEALER_KEYPAIR_PATH/);
});

test("rejects unknown seal helper options", async () => {
  const result = await runNode([SCRIPT_PATH, "--read-env-file", ".env"]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Unknown option: --read-env-file/);
  assert.equal(result.stdout, "");
});

function accountFor(address) {
  if (address === CORE_COLLECTION) {
    return account({
      data: Buffer.from([5, 1, 2, 3]),
      executable: false,
      owner: CORE_PROGRAM_ID,
    });
  }
  if (address === LOOM_ASSET) {
    return account({
      data: buildCoreAssetBase({ collection: CORE_COLLECTION, owner: WRITER }),
      executable: false,
      owner: CORE_PROGRAM_ID,
    });
  }

  return noAccount();
}

function buildCoreAssetBase({ collection, owner }) {
  return Buffer.concat([
    Buffer.from([1]),
    new anchor.web3.PublicKey(owner).toBuffer(),
    Buffer.from([2]),
    new anchor.web3.PublicKey(collection).toBuffer(),
  ]);
}

function buildHashSealAccountData({ loomAsset, sessionHash, utcDay, writer }) {
  const data = Buffer.alloc(120);
  discriminator("account:HashSeal").copy(data, 0);
  new anchor.web3.PublicKey(writer).toBuffer().copy(data, 8);
  new anchor.web3.PublicKey(loomAsset).toBuffer().copy(data, 40);
  Buffer.from(sessionHash, "hex").copy(data, 72);
  data.writeBigInt64LE(BigInt(utcDay), 104);
  data.writeBigInt64LE(1n, 112);
  return data;
}

function deriveHashSeal({ sessionHash, utcDay: _utcDay, writer }) {
  const [hashSeal] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("hash_seal", "utf8"),
      new anchor.web3.PublicKey(writer).toBuffer(),
      Buffer.from(sessionHash, "hex"),
    ],
    new anchor.web3.PublicKey(PROGRAM_ID),
  );

  return hashSeal.toBase58();
}

function discriminator(value) {
  return crypto.createHash("sha256").update(value).digest().subarray(0, 8);
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

function noAccount() {
  return {
    context: { slot: 1 },
    value: null,
  };
}

function rpcServer(handler) {
  return http.createServer((req, res) => {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      const request = JSON.parse(body);
      const result = handler(request);
      res.writeHead(200, { "content-type": "application/json" });
      res.end(
        JSON.stringify({
          id: request.id,
          jsonrpc: "2.0",
          result,
        }),
      );
    });
  });
}

function listen(server) {
  return new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
}

function close(server) {
  return new Promise((resolve, reject) => {
    server.close((error) => {
      if (error) {
        reject(error);
        return;
      }
      resolve();
    });
  });
}

function currentUtcDay() {
  return Math.floor(Date.now() / 86_400_000);
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
