import assert from "node:assert/strict";
import crypto from "node:crypto";
import { execFile } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import * as anchor from "@coral-xyz/anchor";

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "recordVerifiedAnky.mjs");
const WRITER = "4vJ9JU1bJJE96FWS5zNtVM6DfHyWixJjx5KJ4LJh5S7K";
const SESSION_HASH = "28a7b5c28dbed9f0047321860dd6b060fe3fd7fce15480621e1eb65276a659e1";
const PROOF_PROTOCOL = "ANKY_ZK_PROOF_V0";
const PROTOCOL_VERSION = 1;
const PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const VALID_SIGNATURE =
  "2hntvJaJzRkFWt3hTa7Q9oiGyVsTpjMwmzY8WcN52UDMsTyMuzKUtcEhupAe7BcZGeq49dFBhhgoYgeZ79m53sNh";
const HASH_SEAL_SEED = Buffer.from("hash_seal", "utf8");
const VERIFIED_SEAL_SEED = Buffer.from("verified_seal", "utf8");
const HASH_SEAL_ACCOUNT_DISCRIMINATOR = discriminator("account:HashSeal");
const VERIFIED_SEAL_ACCOUNT_DISCRIMINATOR = discriminator("account:VerifiedSeal");
const VERIFIER = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const { PublicKey } = anchor.web3;

test("dry-run validates an SP1 receipt and derives VerifiedSeal accounts without plaintext", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

  const result = await runNode([SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER]);
  assert.equal(result.code, 0, result.stderr);

  const summary = parseFirstJsonObject(result.stdout);
  assert.equal(summary.dryRun, true);
  assert.equal(typeof summary.currentUtcDay, "number");
  assert.equal(summary.receiptUtcDayIsCurrent, summary.utcDay === summary.currentUtcDay);
  assert.equal(summary.writer, WRITER);
  assert.equal(summary.sessionHash, SESSION_HASH);
  assert.equal(summary.proofHash, computeReceiptHash(baseReceipt()));
  assert.equal(summary.protocolVersion, 1);
  assert.match(summary.hashSeal, /^[1-9A-HJ-NP-Za-km-z]+$/);
  assert.match(summary.verifiedSeal, /^[1-9A-HJ-NP-Za-km-z]+$/);
  assert.doesNotMatch(result.stdout, /\.anky plaintext|raw .anky|fixtures\/full\.anky/i);
});

test("dry-run ignores backend URL env when no backend post is requested", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

  const result = await runNode(
    [SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER],
    {
      ANKY_INDEXER_WRITE_SECRET: "",
      ANKY_VERIFIED_SEAL_BACKEND_URL: "http://127.0.0.1:65535",
      ANKY_VERIFIED_SEAL_RECORD_SECRET: "",
    },
  );
  assert.equal(result.code, 0, result.stderr);

  const summary = parseFirstJsonObject(result.stdout);
  assert.equal(summary.backendPost, null);
  assert.equal(summary.dryRun, true);
});

test("dry-run rejects a receipt whose proof hash was not derived from public values", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    proof_hash: "f".repeat(64),
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

  const result = await runNode([SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /proof_hash does not match/i);
  assert.equal(result.stdout, "");
});

test("rejects non-landed backend verified metadata status before chain work", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

  const result = await runNode([
    SCRIPT_PATH,
    "--receipt",
    receiptPath,
    "--writer",
    WRITER,
    "--status",
    "pending",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /backend verified status must be confirmed or finalized/);
  assert.equal(result.stdout, "");
});

test("dry-run rejects a receipt with invalid public timing values", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    event_count: 0,
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

  const result = await runNode([SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /public timing and event count values are invalid/i);
  assert.equal(result.stdout, "");
});

test("dry-run rejects a receipt with inconsistent rite duration", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    accepted_duration_ms: 472000,
    rite_duration_ms: 472000,
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

  const result = await runNode([SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /rite_duration_ms does not match accepted_duration_ms/i);
  assert.equal(result.stdout, "");
});

test("dry-run rejects a receipt whose UTC day was not derived from the start time", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19724,
  });

  const result = await runNode([SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /utc_day does not match started_at_ms/i);
  assert.equal(result.stdout, "");
});

test("check-chain confirms the matching HashSeal exists before record_verified_anky", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });
  const programId = new PublicKey(PROGRAM_ID);
  const writer = new PublicKey(WRITER);
  const [hashSeal] = PublicKey.findProgramAddressSync(
    [HASH_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const [verifiedSeal] = PublicKey.findProgramAddressSync(
    [VERIFIED_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const server = rpcServer((method, params) => {
    assert.equal(method, "getMultipleAccounts");
    assert.deepEqual(params[0], [hashSeal.toBase58(), verifiedSeal.toBase58()]);

    return {
      context: { slot: 1 },
      value: [
        {
          data: [buildHashSealAccountData({ writer, sessionHash: SESSION_HASH, utcDay: 19723 }).toString("base64"), "base64"],
          executable: false,
          lamports: 1,
          owner: PROGRAM_ID,
          rentEpoch: 0,
        },
        null,
      ],
    };
  });
  await listen(server);

  try {
    const result = await runNode(
      [SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER, "--check-chain"],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
      },
    );
    assert.equal(result.code, 0, result.stderr);
    const summary = parseFirstJsonObject(result.stdout);
    assert.equal(summary.chainPreflight.ok, true);
    assert.equal(summary.chainPreflight.hashSealAccount.writer, WRITER);
    assert.equal(summary.chainPreflight.hashSealAccount.sessionHash, SESSION_HASH);
    assert.equal(summary.chainPreflight.hashSealAccount.utcDay, 19723);
  } finally {
    await close(server);
  }
});

test("check-hashseal-only confirms the sealed hash before SP1 without a receipt", async () => {
  const programId = new PublicKey(PROGRAM_ID);
  const writer = new PublicKey(WRITER);
  const [hashSeal] = PublicKey.findProgramAddressSync(
    [HASH_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const [verifiedSeal] = PublicKey.findProgramAddressSync(
    [VERIFIED_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const server = rpcServer((method, params) => {
    assert.equal(method, "getMultipleAccounts");
    assert.deepEqual(params[0], [hashSeal.toBase58(), verifiedSeal.toBase58()]);

    return {
      context: { slot: 1 },
      value: [
        {
          data: [buildHashSealAccountData({ writer, sessionHash: SESSION_HASH, utcDay: 19723 }).toString("base64"), "base64"],
          executable: false,
          lamports: 1,
          owner: PROGRAM_ID,
          rentEpoch: 0,
        },
        null,
      ],
    };
  });
  await listen(server);

  try {
    const result = await runNode(
      [
        SCRIPT_PATH,
        "--check-hashseal-only",
        "--writer",
        WRITER,
        "--session-hash",
        SESSION_HASH,
        "--utc-day",
        "19723",
      ],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
      },
    );
    assert.equal(result.code, 0, result.stderr);
    const summary = parseFirstJsonObject(result.stdout);
    assert.equal(summary.mode, "hash_seal_preflight");
    assert.equal(summary.chainPreflight.ok, true);
    assert.equal(summary.writer, WRITER);
    assert.equal(summary.sessionHash, SESSION_HASH);
    assert.doesNotMatch(result.stdout, /\.anky plaintext|raw .anky|fixtures\/full\.anky/i);
  } finally {
    await close(server);
  }
});

test("check-hashseal-only rejects when VerifiedSeal already exists", async () => {
  const programId = new PublicKey(PROGRAM_ID);
  const writer = new PublicKey(WRITER);
  const [hashSeal] = PublicKey.findProgramAddressSync(
    [HASH_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const [verifiedSeal] = PublicKey.findProgramAddressSync(
    [VERIFIED_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const server = rpcServer(() => ({
    context: { slot: 1 },
    value: [
      {
        data: [buildHashSealAccountData({ writer, sessionHash: SESSION_HASH, utcDay: 19723 }).toString("base64"), "base64"],
        executable: false,
        lamports: 1,
        owner: PROGRAM_ID,
        rentEpoch: 0,
      },
      {
        data: [Buffer.alloc(8).toString("base64"), "base64"],
        executable: false,
        lamports: 1,
        owner: PROGRAM_ID,
        rentEpoch: 0,
      },
    ],
  }));
  await listen(server);

  try {
    const result = await runNode(
      [
        SCRIPT_PATH,
        "--check-hashseal-only",
        "--writer",
        WRITER,
        "--session-hash",
        SESSION_HASH,
        "--utc-day",
        "19723",
      ],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
      },
    );

    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /VerifiedSeal account already exists/);
  } finally {
    await close(server);
  }

  assert.match(hashSeal.toBase58(), /^[1-9A-HJ-NP-Za-km-z]+$/);
  assert.match(verifiedSeal.toBase58(), /^[1-9A-HJ-NP-Za-km-z]+$/);
});

test("check-hashseal-only rejects mismatched HashSeal data", async () => {
  const writer = new PublicKey(WRITER);
  const server = rpcServer(() => ({
    context: { slot: 1 },
    value: [
      {
        data: [buildHashSealAccountData({ writer, sessionHash: SESSION_HASH, utcDay: 19724 }).toString("base64"), "base64"],
        executable: false,
        lamports: 1,
        owner: PROGRAM_ID,
        rentEpoch: 0,
      },
      null,
    ],
  }));
  await listen(server);

  try {
    const result = await runNode(
      [
        SCRIPT_PATH,
        "--check-hashseal-only",
        "--writer",
        WRITER,
        "--session-hash",
        SESSION_HASH,
        "--utc-day",
        "19723",
      ],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
      },
    );

    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /HashSeal account does not match receipt writer, session hash, and UTC day/);
  } finally {
    await close(server);
  }
});

test("check-chain rejects when the matching HashSeal is missing", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });
  const server = rpcServer(() => ({
    context: { slot: 1 },
    value: [null, null],
  }));
  await listen(server);

  try {
    const result = await runNode(
      [SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER, "--check-chain"],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
      },
    );
    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /HashSeal account does not exist/);
  } finally {
    await close(server);
  }
});

test("refuses already-landed backend metadata posts without a VerifiedSeal chain check", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });
  const requests = [];
  const server = http.createServer((req, res) => {
    let body = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      requests.push({
        body,
        headers: req.headers,
        path: req.url,
      });
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ recorded: true }));
    });
  });
  await listen(server);

  try {
    const result = await runNode(
      [
        SCRIPT_PATH,
        "--receipt",
        receiptPath,
        "--writer",
        WRITER,
        "--backend-url",
        `http://127.0.0.1:${server.address().port}`,
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

  } finally {
    await close(server);
  }

  assert.equal(requests.length, 0);
});

test("requires backend write secret before VerifiedSeal chain check or backend post", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });
  const rpcRequests = [];
  const rpc = rpcServer((method, params) => {
    rpcRequests.push({ method, params });

    return { context: { slot: 1 }, value: [null, null] };
  });
  const backendRequests = [];
  const backend = http.createServer((req, res) => {
    backendRequests.push(req.url);
    req.resume();
    req.on("end", () => {
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
        ANKY_INDEXER_WRITE_SECRET: "",
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${rpc.address().port}`,
        ANKY_VERIFIED_SEAL_RECORD_SECRET: "",
      },
    );
    assert.notEqual(result.code, 0);
    assert.match(
      result.stderr,
      /ANKY_INDEXER_WRITE_SECRET or ANKY_VERIFIED_SEAL_RECORD_SECRET is required/,
    );
    assert.equal(result.stdout, "");
  } finally {
    await close(rpc);
    await close(backend);
  }

  assert.deepEqual(rpcRequests, []);
  assert.deepEqual(backendRequests, []);
});

test("requires backend write secret before send mode loads verifier keypair", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

  const result = await runNode(
    [
      SCRIPT_PATH,
      "--receipt",
      receiptPath,
      "--writer",
      WRITER,
      "--backend-url",
      "http://127.0.0.1:65535",
      "--send",
    ],
    {
      ANKY_INDEXER_WRITE_SECRET: "",
      ANKY_VERIFIED_SEAL_RECORD_SECRET: "",
    },
  );

  assert.notEqual(result.code, 0);
  assert.match(
    result.stderr,
    /ANKY_INDEXER_WRITE_SECRET or ANKY_VERIFIED_SEAL_RECORD_SECRET is required/,
  );
  assert.doesNotMatch(result.stderr, /ANKY_VERIFIER_KEYPAIR_PATH|--keypair/);
});

test("send mode requires explicit local SP1 proof verification before chain or keypair work", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });
  const rpcRequests = [];
  const rpc = rpcServer((method, params) => {
    rpcRequests.push({ method, params });

    return { context: { slot: 1 }, value: [null, null] };
  });
  await listen(rpc);

  try {
    const result = await runNode(
      [SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER, "--send"],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${rpc.address().port}`,
      },
    );

    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /--sp1-proof-verified is required with --send/);
    assert.doesNotMatch(result.stderr, /ANKY_VERIFIER_KEYPAIR_PATH|--keypair/);
    assert.equal(result.stdout, "");
  } finally {
    await close(rpc);
  }

  assert.deepEqual(rpcRequests, []);
});

test("proof-verified send mode reaches public HashSeal preflight", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });
  const rpcRequests = [];
  const rpc = rpcServer((method, params) => {
    rpcRequests.push({ method, params });

    return { context: { slot: 1 }, value: [null, null] };
  });
  await listen(rpc);

  try {
    const result = await runNode(
      [
        SCRIPT_PATH,
        "--receipt",
        receiptPath,
        "--writer",
        WRITER,
        "--send",
        "--sp1-proof-verified",
      ],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${rpc.address().port}`,
      },
    );

    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /Chain preflight failed/i);
    assert.doesNotMatch(result.stderr, /--sp1-proof-verified is required/);
  } finally {
    await close(rpc);
  }

  assert.equal(rpcRequests.length, 1);
  assert.equal(rpcRequests[0].method, "getMultipleAccounts");
});

test("checks landed VerifiedSeal account before posting already-landed backend metadata with secret fallback", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });
  const programId = new PublicKey(PROGRAM_ID);
  const writer = new PublicKey(WRITER);
  const proofHash = computeReceiptHash(baseReceipt());
  const [hashSeal] = PublicKey.findProgramAddressSync(
    [HASH_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const [verifiedSeal] = PublicKey.findProgramAddressSync(
    [VERIFIED_SEAL_SEED, writer.toBuffer(), Buffer.from(SESSION_HASH, "hex")],
    programId,
  );
  const rpc = rpcServer((method, params) => {
    assert.equal(method, "getMultipleAccounts");
    assert.deepEqual(params[0], [hashSeal.toBase58(), verifiedSeal.toBase58()]);

    return {
      context: { slot: 1 },
      value: [
        {
          data: [buildHashSealAccountData({ writer, sessionHash: SESSION_HASH, utcDay: 19723 }).toString("base64"), "base64"],
          executable: false,
          lamports: 1,
          owner: PROGRAM_ID,
          rentEpoch: 0,
        },
        {
          data: [buildVerifiedSealAccountData({ proofHash, sessionHash: SESSION_HASH, utcDay: 19723, writer }).toString("base64"), "base64"],
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
      requests.push({
        body,
        headers: req.headers,
        path: req.url,
      });
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
        ANKY_INDEXER_WRITE_SECRET: "",
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${rpc.address().port}`,
        ANKY_VERIFIED_SEAL_RECORD_SECRET: "test-secret",
      },
    );
    assert.equal(result.code, 0, result.stderr);
    const summary = parseFirstJsonObject(result.stdout);
    assert.equal(summary.verifiedChain.ok, true);
    assert.equal(summary.verifiedChain.verifiedSealAccount.proofHash, proofHash);
    assert.deepEqual(summary.backendPost, { ok: true, status: 200 });
  } finally {
    await close(rpc);
    await close(backend);
  }

  assert.equal(requests.length, 1);
  assert.equal(requests[0].path, "/api/mobile/seals/verified/record");
  assert.equal(requests[0].headers["x-anky-indexer-secret"], "test-secret");
  const body = JSON.parse(requests[0].body);
  assert.equal(body.wallet, WRITER);
  assert.equal(body.sessionHash, SESSION_HASH);
  assert.equal(body.proofHash, proofHash);
  assert.equal(body.verifier, VERIFIER);
  assert.equal(body.protocolVersion, 1);
  assert.equal(body.signature, VALID_SIGNATURE);
  assert.equal(body.status, "confirmed");
  assert.equal(body.utcDay, 19723);
});

test("check-verified-chain rejects when the VerifiedSeal account is missing", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });
  const writer = new PublicKey(WRITER);
  const server = rpcServer(() => ({
    context: { slot: 1 },
    value: [
      {
        data: [buildHashSealAccountData({ writer, sessionHash: SESSION_HASH, utcDay: 19723 }).toString("base64"), "base64"],
        executable: false,
        lamports: 1,
        owner: PROGRAM_ID,
        rentEpoch: 0,
      },
      null,
    ],
  }));
  await listen(server);

  try {
    const result = await runNode(
      [SCRIPT_PATH, "--receipt", receiptPath, "--writer", WRITER, "--check-verified-chain"],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
      },
    );

    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /VerifiedSeal account does not exist/);
  } finally {
    await close(server);
  }
});

test("dry-run rejects a receipt whose session hash does not match the requested hash", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

  const result = await runNode([
    SCRIPT_PATH,
    "--receipt",
    receiptPath,
    "--writer",
    WRITER,
    "--session-hash",
    "0".repeat(64),
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /session_hash does not match/i);
  assert.equal(result.stdout, "");
});

test("rejects unknown operator options instead of silently ignoring them", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

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

test("mainnet dry-run is gated unless launch approval is explicit", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-record-verified-"));
  const receiptPath = writeReceipt(tempDir, {
    writer: WRITER,
    session_hash: SESSION_HASH,
    utc_day: 19723,
  });

  const result = await runNode([
    SCRIPT_PATH,
    "--receipt",
    receiptPath,
    "--writer",
    WRITER,
    "--cluster",
    "mainnet-beta",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Refusing mainnet record_verified_anky/);
  assert.equal(result.stdout, "");
});

function writeReceipt(tempDir, overrides) {
  const receiptPath = path.join(tempDir, "receipt.json");
  const receipt = {
    ...baseReceipt(),
    ...overrides,
  };
  if (overrides.proof_hash == null) {
    receipt.proof_hash = computeReceiptHash(receipt);
  }

  fs.writeFileSync(
    receiptPath,
    `${JSON.stringify(
      receipt,
      null,
      2,
    )}\n`,
  );

  return receiptPath;
}

function baseReceipt() {
  return {
    version: PROTOCOL_VERSION,
    protocol: PROOF_PROTOCOL,
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
  buffer.writeUInt16LE(PROTOCOL_VERSION, offset);
  offset += 2;
  buffer.writeBigInt64LE(1n, offset);

  return buffer;
}

function computeReceiptHash(receipt) {
  const payload = [
    PROOF_PROTOCOL,
    PROTOCOL_VERSION,
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

function discriminator(preimage) {
  return crypto.createHash("sha256").update(preimage).digest().subarray(0, 8);
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
      const result = handler(payload.method, payload.params);
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
