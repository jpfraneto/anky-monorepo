import assert from "node:assert/strict";
import crypto from "node:crypto";
import { execFile } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "proveAndRecordVerified.mjs");
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");
const requireFromSealProgram = createRequire(
  path.join(REPO_ROOT, "solana", "anky-seal-program", "package.json"),
);
const anchor = requireFromSealProgram("@coral-xyz/anchor");
const WRITER = "4vJ9JU1bJJE96FWS5zNtVM6DfHyWixJjx5KJ4LJh5S7K";
const SESSION_HASH = "28a7b5c28dbed9f0047321860dd6b060fe3fd7fce15480621e1eb65276a659e1";
const VALID_SIGNATURE =
  "2hntvJaJzRkFWt3hTa7Q9oiGyVsTpjMwmzY8WcN52UDMsTyMuzKUtcEhupAe7BcZGeq49dFBhhgoYgeZ79m53sNh";
const PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const VERIFIER = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const HASH_SEAL_SEED = Buffer.from("hash_seal", "utf8");
const VERIFIED_SEAL_SEED = Buffer.from("verified_seal", "utf8");
const HASH_SEAL_ACCOUNT_DISCRIMINATOR = discriminator("account:HashSeal");
const VERIFIED_SEAL_ACCOUNT_DISCRIMINATOR = discriminator("account:VerifiedSeal");
const { PublicKey } = anchor.web3;

test("orchestrates an existing public receipt through the VerifiedSeal dry-run", async () => {
  const receiptPath = writeReceipt();
  const result = await runNode([SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER]);

  assert.equal(result.code, 0, result.stderr);
  const summary = parseFirstJsonObject(result.stdout);
  assert.equal(summary.dryRun, true);
  assert.equal(summary.writer, WRITER);
  assert.equal(summary.sessionHash, SESSION_HASH);
  assert.equal(summary.chainPreflight, null);
});

test("dry-run ignores backend URL env when no backend post is requested", async () => {
  const receiptPath = writeReceipt();
  const result = await runNode(
    [SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER],
    {
      ANKY_VERIFIED_SEAL_BACKEND_URL: "http://127.0.0.1:65535",
    },
  );

  assert.equal(result.code, 0, result.stderr);
  const summary = parseFirstJsonObject(result.stdout);
  assert.equal(summary.backendPost, null);
  assert.equal(summary.dryRun, true);
});

test("refuses already-landed backend metadata posts without a VerifiedSeal chain check", async () => {
  const receiptPath = writeReceipt();
  const result = await runNode(
    [
      SCRIPT_PATH,
      "--receipt",
      receiptPath,
      "--writer",
      WRITER,
      "--backend-url",
      "http://127.0.0.1:65535",
      "--backend-signature",
      VALID_SIGNATURE,
    ],
    {
      ANKY_INDEXER_WRITE_SECRET: "test-secret",
    },
  );

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /check-verified-chain is required/i);
  assert.equal(result.stdout, "");
});

test("refuses env-configured backend metadata posts without a VerifiedSeal chain check", async () => {
  const receiptPath = writeReceipt();
  const result = await runNode(
    [
      SCRIPT_PATH,
      "--receipt",
      receiptPath,
      "--writer",
      WRITER,
      "--backend-signature",
      VALID_SIGNATURE,
    ],
    {
      ANKY_INDEXER_WRITE_SECRET: "test-secret",
      ANKY_VERIFIED_SEAL_BACKEND_URL: "http://127.0.0.1:65535",
    },
  );

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /check-verified-chain is required/i);
  assert.equal(result.stdout, "");
});

test("passes backend metadata options through after a landed VerifiedSeal chain check", async () => {
  const receiptPath = writeReceipt();
  const writer = new PublicKey(WRITER);
  const programId = new PublicKey(PROGRAM_ID);
  const proofHash = computeReceiptHash(baseReceipt());
  const [hashSeal] = PublicKey.findProgramAddressSync(
    [HASH_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const [verifiedSeal] = PublicKey.findProgramAddressSync(
    [VERIFIED_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const rpc = rpcServer((payload) => {
    assert.equal(payload.method, "getMultipleAccounts");
    assert.deepEqual(payload.params[0], [hashSeal.toBase58(), verifiedSeal.toBase58()]);

    return {
      context: { slot: 1 },
      value: [
        {
          data: [
            buildHashSealAccountData({ writer, sessionHash: SESSION_HASH, utcDay: 19723 }).toString("base64"),
            "base64",
          ],
          executable: false,
          lamports: 1,
          owner: PROGRAM_ID,
          rentEpoch: 0,
        },
        {
          data: [
            buildVerifiedSealAccountData({ proofHash, sessionHash: SESSION_HASH, utcDay: 19723, writer }).toString("base64"),
            "base64",
          ],
          executable: false,
          lamports: 1,
          owner: PROGRAM_ID,
          rentEpoch: 0,
        },
      ],
    };
  });
  const requests = [];
  const backend = http.createServer((req, res) => {
    let body = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      requests.push({ body, headers: req.headers, path: req.url });
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ recorded: true }));
    });
  });
  await listen(rpc);
  await listen(backend);

  try {
    const result = await runNode(
      [
        SCRIPT_PATH,
        "--receipt",
        receiptPath,
        "--writer",
        WRITER,
        "--check-verified-chain",
        "--backend-url",
        `http://127.0.0.1:${backend.address().port}`,
        "--backend-signature",
        VALID_SIGNATURE,
      ],
      {
        ANKY_INDEXER_WRITE_SECRET: "test-secret",
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${rpc.address().port}`,
      },
    );

    assert.equal(result.code, 0, result.stderr);
    assert.equal(parseFirstJsonObject(result.stdout).backendPost.ok, true);
  } finally {
    await close(rpc);
    await close(backend);
  }

  assert.equal(requests.length, 1);
  assert.equal(requests[0].path, "/api/mobile/seals/verified/record");
  assert.equal(requests[0].headers["x-anky-indexer-secret"], "test-secret");
  const body = JSON.parse(requests[0].body);
  assert.equal(body.signature, VALID_SIGNATURE);
  assert.equal(body.utcDay, 19723);
});

test("requires backend write secret before SP1 or operator work", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--file",
    "solana/anky-zk-proof/fixtures/full.anky",
    "--writer",
    WRITER,
    "--expected-hash",
    SESSION_HASH,
    "--backend-url",
    "http://127.0.0.1:65535",
    "--send",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(
    result.stderr,
    /ANKY_INDEXER_WRITE_SECRET or ANKY_VERIFIED_SEAL_RECORD_SECRET is required/,
  );
  assert.doesNotMatch(result.stderr, /cargo|ANKY_VERIFIER_KEYPAIR_PATH|--keypair/);
});

test("requires expected hash when a private .anky file path is supplied", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--file",
    "solana/anky-zk-proof/fixtures/full.anky",
    "--writer",
    WRITER,
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--expected-hash is required/);
});

test("requires exactly one receipt, proof, or private file input", async () => {
  const receiptPath = writeReceipt();
  const result = await runNode([
    SCRIPT_PATH,
    "--receipt",
    receiptPath,
    "--file",
    "solana/anky-zk-proof/fixtures/full.anky",
    "--writer",
    WRITER,
    "--expected-hash",
    SESSION_HASH,
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Provide exactly one of --file, --proof, or --receipt/);
  assert.equal(result.stdout, "");
});

test("refuses send mode with SP1 execute before cargo or operator work", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--file",
    "solana/anky-zk-proof/fixtures/full.anky",
    "--writer",
    WRITER,
    "--expected-hash",
    SESSION_HASH,
    "--sp1-mode",
    "execute",
    "--send",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--send requires --sp1-mode prove/);
  assert.doesNotMatch(result.stderr, /cargo|record_verified_anky|ANKY_VERIFIER_KEYPAIR_PATH|--keypair/);
  assert.equal(result.stdout, "");
});

test("refuses explicit proof verification attestation with SP1 execute", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--file",
    "solana/anky-zk-proof/fixtures/full.anky",
    "--writer",
    WRITER,
    "--expected-hash",
    SESSION_HASH,
    "--sp1-mode",
    "execute",
    "--sp1-proof-verified",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /cannot be combined with --sp1-mode execute/);
  assert.doesNotMatch(result.stderr, /cargo|record_verified_anky|ANKY_VERIFIER_KEYPAIR_PATH|--keypair/);
  assert.equal(result.stdout, "");
});

test("refuses SP1 mode override for saved-proof verification", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--proof",
    "/tmp/fake-proof-with-public-values.bin",
    "--writer",
    WRITER,
    "--sp1-mode",
    "prove",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--sp1-mode is only valid with --file/);
  assert.doesNotMatch(result.stderr, /cargo|ANKY_VERIFIER_KEYPAIR_PATH|--keypair/);
  assert.equal(result.stdout, "");
});

test("check-chain-first is only valid before private-file SP1 proving", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--proof",
    "/tmp/fake-proof-with-public-values.bin",
    "--writer",
    WRITER,
    "--check-chain-first",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--check-chain-first is only valid with --file/);
  assert.doesNotMatch(result.stderr, /cargo|ANKY_VERIFIER_KEYPAIR_PATH|--keypair/);
  assert.equal(result.stdout, "");
});

test("verifies an existing proof artifact before invoking the VerifiedSeal operator", async () => {
  const fakeCargo = writeFakeCargo();
  const proofPath = path.join(fakeCargo.tempDir, "proof-with-public-values.bin");
  fs.writeFileSync(proofPath, "fake proof bytes");

  const result = await runNode(
    [
      SCRIPT_PATH,
      "--proof",
      proofPath,
      "--writer",
      WRITER,
    ],
    {
      PATH: `${fakeCargo.binDir}${path.delimiter}${process.env.PATH}`,
    },
  );

  assert.equal(result.code, 0, result.stderr);
  const cargoArgs = JSON.parse(fs.readFileSync(fakeCargo.argsPath, "utf8"));
  assert.deepEqual(cargoArgs.slice(0, 4), ["run", "--release", "--", "--verify"]);
  assert.equal(cargoArgs[cargoArgs.indexOf("--proof") + 1], proofPath);
  assert.match(cargoArgs[cargoArgs.indexOf("--receipt-out") + 1], /verified-receipt\.json$/);

  const summary = parseFirstJsonObject(result.stdout);
  assert.equal(summary.dryRun, true);
  assert.equal(summary.writer, WRITER);
  assert.equal(summary.sessionHash, SESSION_HASH);
});

test("refuses raw receipt sends because the wrapper cannot verify the SP1 proof", async () => {
  const receiptPath = writeReceipt();
  const result = await runNode([
    SCRIPT_PATH,
    "--receipt",
    receiptPath,
    "--writer",
    WRITER,
    "--send",
    "--sp1-proof-verified",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--send is not allowed with raw --receipt/);
  assert.equal(result.stdout, "");
});

test("refuses to write SP1 receipt and proof artifacts inside the repo", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--file",
    "solana/anky-zk-proof/fixtures/full.anky",
    "--writer",
    WRITER,
    "--expected-hash",
    SESSION_HASH,
    "--out-dir",
    path.join(REPO_ROOT, "sojourn9", "sp1-out"),
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Refusing to write SP1 receipt\/proof artifacts inside this git worktree/);
  assert.doesNotMatch(result.stderr, /cargo/);
  assert.equal(result.stdout, "");
});

test("requires UTC day for pre-SP1 chain checks without reading the private file", async () => {
  const result = await runNode([
    SCRIPT_PATH,
    "--file",
    "solana/anky-zk-proof/fixtures/full.anky",
    "--writer",
    WRITER,
    "--expected-hash",
    SESSION_HASH,
    "--check-chain-first",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--utc-day is required/);
});

test("rejects pre-SP1 chain checks when an existing receipt is supplied", async () => {
  const receiptPath = writeReceipt();
  const result = await runNode([
    SCRIPT_PATH,
    "--receipt",
    receiptPath,
    "--writer",
    WRITER,
    "--check-chain-first",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--check-chain-first is only valid with --file/);
});

test("passes verified-chain check mode through to the VerifiedSeal operator", async () => {
  const receiptPath = writeReceipt();
  const result = await runNode([
    SCRIPT_PATH,
    "--receipt",
    receiptPath,
    "--writer",
    WRITER,
    "--send",
    "--check-verified-chain",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--send is not allowed with raw --receipt/);
});

test("rejects unknown wrapper options instead of silently ignoring them", async () => {
  const receiptPath = writeReceipt();
  const result = await runNode([
    SCRIPT_PATH,
    "--receipt",
    receiptPath,
    "--writer",
    WRITER,
    "--check-chain-fist",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Unknown option: --check-chain-fist/);
  assert.equal(result.stdout, "");
});

function writeReceipt() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-prove-record-"));
  const receipt = baseReceipt();
  receipt.proof_hash = computeReceiptHash(receipt);
  const receiptPath = path.join(tempDir, "receipt.json");
  fs.writeFileSync(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`);

  return receiptPath;
}

function writeFakeCargo() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-fake-cargo-"));
  const binDir = path.join(tempDir, "bin");
  const argsPath = path.join(tempDir, "cargo-args.json");
  fs.mkdirSync(binDir);
  const receipt = baseReceipt();
  receipt.proof_hash = computeReceiptHash(receipt);
  const script = `#!/usr/bin/env node
const fs = require("node:fs");
const args = process.argv.slice(2);
fs.writeFileSync(${JSON.stringify(argsPath)}, JSON.stringify(args));
const receiptOut = args[args.indexOf("--receipt-out") + 1];
fs.writeFileSync(receiptOut, ${JSON.stringify(`${JSON.stringify(receipt, null, 2)}\n`)});
`;
  const cargoPath = path.join(binDir, "cargo");
  fs.writeFileSync(cargoPath, script, { mode: 0o755 });

  return { argsPath, binDir, tempDir };
}

function baseReceipt() {
  return {
    version: 1,
    protocol: "ANKY_ZK_PROOF_V0",
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
    started_at_ms: 1704067200000,
    accepted_duration_ms: 472000,
    rite_duration_ms: 480000,
    event_count: 42,
    valid: true,
    duration_ok: true,
    proof_hash: "",
  };
}

function computeReceiptHash(receipt) {
  const payload = [
    receipt.protocol,
    receipt.version,
    receipt.writer,
    receipt.session_hash,
    receipt.utc_day,
    receipt.started_at_ms,
    receipt.accepted_duration_ms,
    receipt.rite_duration_ms,
    receipt.event_count,
    receipt.duration_ok,
  ].join("|");

  return crypto.createHash("sha256").update(payload).digest("hex");
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
          ANKY_INDEXER_WRITE_SECRET: "",
          ANKY_VERIFIED_SEAL_BACKEND_URL: "",
          ANKY_VERIFIED_SEAL_RECORD_SECRET: "",
          ...env,
          ANKY_ALLOW_MAINNET_RECORD_VERIFIED: "",
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

function parseFirstJsonObject(stdout) {
  const match = stdout.match(/^\{[\s\S]*?\n\}/);
  assert.ok(match, `expected JSON object in stdout:\n${stdout}`);

  return JSON.parse(match[0]);
}

function buildHashSealAccountData({ writer, sessionHash, utcDay }) {
  const buffer = Buffer.alloc(120);
  let offset = 0;
  HASH_SEAL_ACCOUNT_DISCRIMINATOR.copy(buffer, offset);
  offset += 8;
  writer.toBuffer().copy(buffer, offset);
  offset += 32;
  PublicKey.default.toBuffer().copy(buffer, offset);
  offset += 32;
  Buffer.from(sessionHash, "hex").copy(buffer, offset);
  offset += 32;
  buffer.writeBigInt64LE(BigInt(utcDay), offset);
  offset += 8;
  buffer.writeBigInt64LE(1n, offset);

  return buffer;
}

function buildVerifiedSealAccountData({ proofHash, sessionHash, utcDay, writer }) {
  const buffer = Buffer.alloc(154);
  let offset = 0;
  VERIFIED_SEAL_ACCOUNT_DISCRIMINATOR.copy(buffer, offset);
  offset += 8;
  writer.toBuffer().copy(buffer, offset);
  offset += 32;
  Buffer.from(sessionHash, "hex").copy(buffer, offset);
  offset += 32;
  buffer.writeBigInt64LE(BigInt(utcDay), offset);
  offset += 8;
  Buffer.from(proofHash, "hex").copy(buffer, offset);
  offset += 32;
  new PublicKey(VERIFIER).toBuffer().copy(buffer, offset);
  offset += 32;
  buffer.writeUInt16LE(1, offset);
  offset += 2;
  buffer.writeBigInt64LE(1n, offset);

  return buffer;
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

function discriminator(preimage) {
  return crypto.createHash("sha256").update(preimage).digest().subarray(0, 8);
}
