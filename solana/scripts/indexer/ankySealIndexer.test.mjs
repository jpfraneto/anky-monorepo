import assert from "node:assert/strict";
import crypto from "node:crypto";
import { execFile } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const INDEXER_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), "ankySealIndexer.mjs");
const FIXTURE_PATH = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "fixtures",
  "anky-seal-events.json",
);
const PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const PROOF_VERIFIER = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const VALID_SIGNATURE =
  "2hntvJaJzRkFWt3hTa7Q9oiGyVsTpjMwmzY8WcN52UDMsTyMuzKUtcEhupAe7BcZGeq49dFBhhgoYgeZ79m53sNh";
const SECOND_VALID_SIGNATURE = "5".repeat(88);

test("scores unique finalized sealed and verified days from Anchor event logs", async () => {
  const result = await runNode([INDEXER_PATH, "--input", FIXTURE_PATH]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.indexedEvents, 2);
  assert.equal(snapshot.summary.sealedEvents, 1);
  assert.equal(snapshot.summary.verifiedEvents, 1);
  assert.equal(snapshot.summary.scoreRows, 1);
  assert.equal(snapshot.summary.totalScore, 3);
  assert.deepEqual(snapshot.scores[0].sealedDays, [19999]);
  assert.equal(snapshot.scores[0].uniqueSealDays, 1);
  assert.equal(snapshot.scores[0].verifiedSealDays, 1);
  assert.equal(snapshot.scores[0].score, 3);
});

test("excludes non-finalized events unless explicitly requested", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixture = JSON.parse(fs.readFileSync(FIXTURE_PATH, "utf8"));
  fixture[0].commitment = "confirmed";
  fixture[1].commitment = "confirmed";
  const fixturePath = path.join(tempDir, "confirmed-events.json");
  fs.writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);

  const finalizedOnly = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(finalizedOnly.code, 0, finalizedOnly.stderr);
  assert.equal(JSON.parse(finalizedOnly.stdout).summary.totalScore, 0);

  const includeNonFinalized = await runNode([
    INDEXER_PATH,
    "--input",
    fixturePath,
    "--include-non-finalized",
  ]);
  assert.equal(includeNonFinalized.code, 0, includeNonFinalized.stderr);
  assert.equal(JSON.parse(includeNonFinalized.stdout).summary.totalScore, 3);
});

test("ignores matching Anchor event data emitted outside the Anky Seal Program invocation", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixture = JSON.parse(fs.readFileSync(FIXTURE_PATH, "utf8"));
  const otherProgram = "11111111111111111111111111111111";
  for (const transaction of fixture) {
    transaction.logMessages = transaction.logMessages.map((line) =>
      line.replaceAll(PROGRAM_ID, otherProgram),
    );
  }
  const fixturePath = path.join(tempDir, "other-program-events.json");
  fs.writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.indexedEvents, 0);
  assert.equal(snapshot.summary.totalScore, 0);
});

test("exports deterministic 8 percent raw reward allocations when token supply is provided", async () => {
  const result = await runNode([INDEXER_PATH, "--input", FIXTURE_PATH, "--token-supply", "1000"]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.allocationRule.rewardBps, 800);
  assert.equal(snapshot.allocationRule.tokenSupplyRaw, "1000");
  assert.equal(snapshot.summary.rewardPoolRaw, "80");
  assert.equal(snapshot.scores[0].rewardAllocationRaw, "80");
});

test("applies a deterministic reward participant cap before allocation", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "participant-cap.json");
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      {
        decodedEvents: [
          {
            kind: "sealed",
            writer: "11111111111111111111111111111111",
            loomAsset: "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9",
            sessionHash: "a".repeat(64),
            utcDay: 20000,
            finalized: true,
            signature: VALID_SIGNATURE,
            slot: 1,
          },
          {
            kind: "sealed",
            writer: "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp",
            loomAsset: "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9",
            sessionHash: "b".repeat(64),
            utcDay: 20000,
            finalized: true,
            signature: SECOND_VALID_SIGNATURE,
            slot: 2,
          },
        ],
      },
      null,
      2,
    )}\n`,
  );

  const result = await runNode([
    INDEXER_PATH,
    "--input",
    fixturePath,
    "--max-participants",
    "1",
    "--token-supply",
    "1000",
  ]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.scoringRule.maxParticipants, 1);
  assert.equal(snapshot.summary.participantCap, 1);
  assert.equal(snapshot.summary.uncappedScoreRows, 2);
  assert.equal(snapshot.summary.excludedByParticipantCap, 1);
  assert.equal(snapshot.summary.scoreRows, 1);
  assert.equal(snapshot.summary.totalScore, 1);
  assert.equal(snapshot.scores[0].wallet, "11111111111111111111111111111111");
  assert.equal(snapshot.scores[0].rewardAllocationRaw, "80");
});

test("counts VerifiedSeal bonus only when the sealed day and session hash match", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "mismatched-verified.json");
  const wallet = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      {
        decodedEvents: [
          {
            kind: "sealed",
            writer: wallet,
            loomAsset: "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9",
            sessionHash: "a".repeat(64),
            utcDay: 20000,
            finalized: true,
            signature: VALID_SIGNATURE,
            slot: 1,
          },
          {
            kind: "verified",
            writer: wallet,
            sessionHash: "b".repeat(64),
            proofHash: "c".repeat(64),
            verifier: PROOF_VERIFIER,
            protocolVersion: 1,
            utcDay: 20000,
            finalized: true,
            signature: SECOND_VALID_SIGNATURE,
            slot: 2,
          },
        ],
      },
      null,
      2,
    )}\n`,
  );

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.sealedEvents, 1);
  assert.equal(snapshot.summary.verifiedEvents, 1);
  assert.equal(snapshot.scores[0].uniqueSealDays, 1);
  assert.equal(snapshot.scores[0].verifiedSealDays, 0);
  assert.equal(snapshot.scores[0].score, 1);
});

test("does not emit participant score rows for verified-only events", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "verified-only.json");
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      {
        decodedEvents: [
          {
            kind: "verified",
            writer: "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp",
            sessionHash: "b".repeat(64),
            proofHash: "c".repeat(64),
            verifier: PROOF_VERIFIER,
            protocolVersion: 1,
            utcDay: 20000,
            finalized: true,
            signature: SECOND_VALID_SIGNATURE,
            slot: 2,
          },
        ],
      },
      null,
      2,
    )}\n`,
  );

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.verifiedEvents, 1);
  assert.equal(snapshot.summary.scoreRows, 0);
  assert.deepEqual(snapshot.scores, []);
});

test("decodes Helius enhanced instruction payloads when Anchor logs are absent", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "enhanced-instructions.json");
  const wallet = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
  const loomAsset = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
  const sessionHash = "a".repeat(64);
  const proofHash = "b".repeat(64);
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      [
        enhancedInstructionTransaction({
          accounts: [wallet, loomAsset],
          data: sealInstructionData({ sessionHash, utcDay: 20000 }),
          signature: VALID_SIGNATURE,
        }),
        enhancedInstructionTransaction({
          accounts: [PROOF_VERIFIER, wallet],
          data: recordVerifiedInstructionData({
            proofHash,
            protocolVersion: 1,
            sessionHash,
            utcDay: 20000,
          }),
          signature: SECOND_VALID_SIGNATURE,
          slot: 2,
        }),
      ],
      null,
      2,
    )}\n`,
  );

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.indexedEvents, 2);
  assert.equal(snapshot.summary.sealedEvents, 1);
  assert.equal(snapshot.summary.verifiedEvents, 1);
  assert.equal(snapshot.scores[0].score, 3);
  assert.equal(snapshot.events[0].kind, "sealed");
  assert.equal(snapshot.events[0].sessionHash, sessionHash);
  assert.equal(snapshot.events[1].kind, "verified");
  assert.equal(snapshot.events[1].proofHash, proofHash);
});

test("does not score failed Helius enhanced transactions even when instruction data is decodable", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "failed-enhanced-instructions.json");
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      [
        {
          ...enhancedInstructionTransaction({
            accounts: [
              "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp",
              "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9",
            ],
            data: sealInstructionData({ sessionHash: "a".repeat(64), utcDay: 20000 }),
            signature: VALID_SIGNATURE,
          }),
          transactionError: { InstructionError: [0, "InvalidAccountData"] },
        },
      ],
      null,
      2,
    )}\n`,
  );

  const result = await runNode([INDEXER_PATH, "--input", fixturePath, "--include-non-finalized"]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.indexedEvents, 1);
  assert.equal(snapshot.summary.sealedEvents, 0);
  assert.equal(snapshot.summary.totalScore, 0);
});

test("does not treat missing input commitment as finalized", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "unknown-finality-enhanced-instructions.json");
  const transaction = enhancedInstructionTransaction({
    accounts: [
      "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp",
      "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9",
    ],
    data: sealInstructionData({ sessionHash: "a".repeat(64), utcDay: 20000 }),
    signature: VALID_SIGNATURE,
  });
  delete transaction.commitment;
  delete transaction.finalized;
  fs.writeFileSync(fixturePath, `${JSON.stringify([transaction], null, 2)}\n`);

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.indexedEvents, 1);
  assert.equal(snapshot.summary.sealedEvents, 0);
  assert.equal(snapshot.summary.totalScore, 0);
});

test("indexes backend-exported Helius webhook receipt rows without ad hoc JSON transforms", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "webhook-receipt-rows.json");
  const wallet = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
  const loomAsset = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
  const sessionHash = "c".repeat(64);
  const webhookPayload = [
    enhancedInstructionTransaction({
      accounts: [wallet, loomAsset],
      data: sealInstructionData({ sessionHash, utcDay: 20001 }),
      signature: VALID_SIGNATURE,
    }),
  ];
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      [
        {
          id: "receipt-row",
          payload_hash: "d".repeat(64),
          payload_json: JSON.stringify(webhookPayload),
        },
      ],
      null,
      2,
    )}\n`,
  );

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.indexedEvents, 1);
  assert.equal(snapshot.summary.sealedEvents, 1);
  assert.equal(snapshot.scores[0].wallet, wallet);
  assert.deepEqual(snapshot.scores[0].sealedDays, [20001]);
});

test("scores public operator VerifiedSeal metadata when it includes utcDay", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "operator-metadata.json");
  const wallet = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      [
        {
          decodedEvents: [
            {
              kind: "sealed",
              writer: wallet,
              loomAsset: "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9",
              sessionHash: "a".repeat(64),
              utcDay: 20000,
              finalized: true,
              signature: VALID_SIGNATURE,
              slot: 1,
            },
          ],
        },
        {
          proofHash: "C".repeat(64),
          proofTxSignature: SECOND_VALID_SIGNATURE,
          protocolVersion: 1,
          sessionHash: "A".repeat(64),
          status: "finalized",
          utcDay: 20000,
          verifier: PROOF_VERIFIER,
          wallet,
        },
      ],
      null,
      2,
    )}\n`,
  );

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.indexedEvents, 2);
  assert.equal(snapshot.summary.sealedEvents, 1);
  assert.equal(snapshot.summary.verifiedEvents, 1);
  assert.equal(snapshot.scores[0].verifiedSealDays, 1);
  assert.equal(snapshot.scores[0].score, 3);
  assert.equal(snapshot.events[1].kind, "verified");
  assert.equal(snapshot.events[1].signature, SECOND_VALID_SIGNATURE);
  assert.equal(snapshot.events[1].proofHash, "c".repeat(64));
  assert.equal(snapshot.events[1].sessionHash, "a".repeat(64));
});

test("drops public VerifiedSeal metadata without utcDay before scoring", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "operator-metadata-missing-day.json");
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      {
        proofHash: "c".repeat(64),
        protocolVersion: 1,
        sessionHash: "a".repeat(64),
        status: "finalized",
        txSignature: "verified",
        verifier: PROOF_VERIFIER,
        wallet: "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp",
      },
      null,
      2,
    )}\n`,
  );

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.indexedEvents, 0);
  assert.equal(snapshot.summary.totalScore, 0);
});

test("drops malformed decoded event fixtures before scoring", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "malformed-decoded-events.json");
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      {
        decodedEvents: [
          {
            kind: "sealed",
            writer: "not-a-wallet",
            loomAsset: "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9",
            sessionHash: "a".repeat(64),
            utcDay: 20000,
            finalized: true,
            signature: VALID_SIGNATURE,
          },
          {
            kind: "verified",
            writer: "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp",
            sessionHash: "not-a-hash",
            proofHash: "c".repeat(64),
            verifier: PROOF_VERIFIER,
            protocolVersion: 1,
            utcDay: 20000,
            finalized: true,
            signature: SECOND_VALID_SIGNATURE,
          },
        ],
      },
      null,
      2,
    )}\n`,
  );

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.indexedEvents, 0);
  assert.equal(snapshot.summary.totalScore, 0);
});

test("filters verified events whose verifier does not match the configured authority", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "wrong-verifier.json");
  const wallet = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      {
        decodedEvents: [
          {
            kind: "sealed",
            writer: wallet,
            loomAsset: "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9",
            sessionHash: "a".repeat(64),
            utcDay: 20000,
            finalized: true,
            signature: VALID_SIGNATURE,
            slot: 1,
          },
          {
            kind: "verified",
            writer: wallet,
            sessionHash: "a".repeat(64),
            proofHash: "c".repeat(64),
            verifier: "11111111111111111111111111111111",
            protocolVersion: 1,
            utcDay: 20000,
            finalized: true,
            signature: SECOND_VALID_SIGNATURE,
            slot: 2,
          },
        ],
      },
      null,
      2,
    )}\n`,
  );

  const result = await runNode([INDEXER_PATH, "--input", fixturePath]);
  assert.equal(result.code, 0, result.stderr);

  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.summary.sealedEvents, 1);
  assert.equal(snapshot.summary.verifiedEvents, 0);
  assert.equal(snapshot.summary.rejectedVerifiedEvents, 1);
  assert.equal(snapshot.scores[0].verifiedSealDays, 0);
  assert.equal(snapshot.scores[0].score, 1);
});

test("sends the indexer secret header when posting backend verified metadata", async () => {
  const fixturePath = writeFixtureWithValidSignatures();
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
      res.end(JSON.stringify({ ok: true }));
    });
  });
  await listen(server);

  try {
    const address = server.address();
    const result = await runNode(
      [
        INDEXER_PATH,
        "--input",
        fixturePath,
        "--backend-url",
        `http://127.0.0.1:${address.port}`,
        "--core-collection",
        "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
      ],
      { ANKY_INDEXER_WRITE_SECRET: "test-secret" },
    );
    assert.equal(result.code, 0, result.stderr);
  } finally {
    await close(server);
  }

  const verifiedRequest = requests.find((request) => request.path === "/api/mobile/seals/verified/record");
  assert.ok(verifiedRequest, "expected a verified backend post");
  assert.equal(verifiedRequest.headers["x-anky-indexer-secret"], "test-secret");
  const verifiedBody = JSON.parse(verifiedRequest.body);
  assert.equal(verifiedBody.protocolVersion, 1);
  assert.equal(verifiedBody.utcDay, 19999);
  const sealRequest = requests.find((request) => request.path === "/api/mobile/seals/record");
  assert.ok(sealRequest, "expected a seal backend post");
  assert.equal(JSON.parse(sealRequest.body).utcDay, 19999);
});

test("requires indexer write secret before backend metadata posts", async () => {
  const requests = [];
  const server = http.createServer((req, res) => {
    requests.push(req.url);
    req.resume();
    req.on("end", () => {
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ ok: true }));
    });
  });
  await listen(server);

  try {
    const result = await runNode(
      [
        INDEXER_PATH,
        "--input",
        FIXTURE_PATH,
        "--backend-url",
        `http://127.0.0.1:${server.address().port}`,
        "--core-collection",
        "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
      ],
      { ANKY_INDEXER_WRITE_SECRET: "" },
    );
    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /ANKY_INDEXER_WRITE_SECRET is required with --backend-url/);
  } finally {
    await close(server);
  }

  assert.deepEqual(requests, []);
});

test("rejects unknown indexer options instead of silently ignoring them", async () => {
  const result = await runNode([INDEXER_PATH, "--input", FIXTURE_PATH, "--proof-verifer", PROOF_VERIFIER]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Unknown option: --proof-verifer/);
  assert.equal(result.stdout, "");
});

test("rejects secret-shaped indexer input and output paths", async () => {
  const inputResult = await runNode([INDEXER_PATH, "--input", ".env"]);
  assert.notEqual(inputResult.code, 0);
  assert.match(inputResult.stderr, /--input must not point at \.env, \.anky, keypair, wallet, deployer, pem, or id\.json files/);
  assert.equal(inputResult.stdout, "");

  const ankyInputResult = await runNode([INDEXER_PATH, "--input", "/tmp/private.anky"]);
  assert.notEqual(ankyInputResult.code, 0);
  assert.match(ankyInputResult.stderr, /--input must not point at \.env, \.anky, keypair, wallet, deployer, pem, or id\.json files/);
  assert.equal(ankyInputResult.stdout, "");

  const outputResult = await runNode([
    INDEXER_PATH,
    "--input",
    FIXTURE_PATH,
    "--out",
    "/tmp/wallet-score-snapshot.json",
  ]);
  assert.notEqual(outputResult.code, 0);
  assert.match(outputResult.stderr, /--out must not point at \.env, \.anky, keypair, wallet, deployer, pem, or id\.json files/);
  assert.equal(outputResult.stdout, "");
});

test("rejects invalid clusters and mainnet defaults", async () => {
  const invalidCluster = await runNode([INDEXER_PATH, "--input", FIXTURE_PATH, "--cluster", "localnet"]);
  assert.notEqual(invalidCluster.code, 0);
  assert.match(invalidCluster.stderr, /--cluster must be devnet or mainnet-beta/);

  const mainnetDefault = await runNode(
    [INDEXER_PATH, "--input", FIXTURE_PATH, "--cluster", "mainnet-beta"],
    {
      ANKY_PROOF_VERIFIER_AUTHORITY: "",
      ANKY_SEAL_PROGRAM_ID: "",
    },
  );
  assert.notEqual(mainnetDefault.code, 0);
  assert.match(mainnetDefault.stderr, /mainnet-beta indexing requires an explicit --program-id/);
});

test("rejects credentialed backend URLs before posting metadata", async () => {
  const result = await runNode(
    [
      INDEXER_PATH,
      "--input",
      FIXTURE_PATH,
      "--backend-url",
      "https://operator:secret@example.com",
      "--core-collection",
      "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
    ],
    { ANKY_INDEXER_WRITE_SECRET: "test-secret" },
  );

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--backend-url must be an HTTPS URL without credentials unless it is localhost HTTP/);
  assert.equal(result.stdout, "");
});

test("rejects non-local plaintext backend URLs before posting metadata", async () => {
  const result = await runNode(
    [
      INDEXER_PATH,
      "--input",
      FIXTURE_PATH,
      "--backend-url",
      "http://example.com",
      "--core-collection",
      "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
    ],
    { ANKY_INDEXER_WRITE_SECRET: "test-secret" },
  );

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--backend-url must be an HTTPS URL without credentials unless it is localhost HTTP/);
  assert.equal(result.stdout, "");
});

test("ignores empty backend URL values instead of enabling backend post mode", async () => {
  const result = await runNode(
    [INDEXER_PATH, "--input", FIXTURE_PATH, "--backend-url", ""],
    { ANKY_INDEXER_WRITE_SECRET: "" },
  );

  assert.equal(result.code, 0, result.stderr);
  const snapshot = JSON.parse(result.stdout);
  assert.deepEqual(snapshot.backendPosts, []);
  assert.equal(snapshot.summary.indexedEvents, 2);
});

test("falls back to env core collection when CLI core collection is empty", async () => {
  const fixturePath = writeFixtureWithValidSignatures();
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
        path: req.url,
      });
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ ok: true }));
    });
  });
  await listen(server);

  try {
    const result = await runNode(
      [
        INDEXER_PATH,
        "--input",
        fixturePath,
        "--backend-url",
        `http://127.0.0.1:${server.address().port}`,
        "--core-collection",
        "",
      ],
      {
        ANKY_CORE_COLLECTION: "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
        ANKY_INDEXER_WRITE_SECRET: "test-secret",
      },
    );
    assert.equal(result.code, 0, result.stderr);
  } finally {
    await close(server);
  }

  const sealRequest = requests.find((request) => request.path === "/api/mobile/seals/record");
  assert.ok(sealRequest, "expected a seal backend post");
  assert.equal(
    JSON.parse(sealRequest.body).coreCollection,
    "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
  );
});

test("does not post backend verified metadata for an unexpected verifier", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixturePath = path.join(tempDir, "wrong-verifier-backend.json");
  const wallet = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
  fs.writeFileSync(
    fixturePath,
    `${JSON.stringify(
      {
        decodedEvents: [
          {
            kind: "sealed",
            writer: wallet,
            loomAsset: "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9",
            sessionHash: "a".repeat(64),
            utcDay: 20000,
            finalized: true,
            signature: VALID_SIGNATURE,
            slot: 1,
          },
          {
            kind: "verified",
            writer: wallet,
            sessionHash: "a".repeat(64),
            proofHash: "c".repeat(64),
            verifier: "11111111111111111111111111111111",
            protocolVersion: 1,
            utcDay: 20000,
            finalized: true,
            signature: VALID_SIGNATURE,
            slot: 2,
          },
        ],
      },
      null,
      2,
    )}\n`,
  );
  const requests = [];
  const server = http.createServer((req, res) => {
    req.resume();
    req.on("end", () => {
      requests.push(req.url);
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ ok: true }));
    });
  });
  await listen(server);

  try {
    const result = await runNode(
      [
        INDEXER_PATH,
        "--input",
        fixturePath,
        "--backend-url",
        `http://127.0.0.1:${server.address().port}`,
        "--core-collection",
        "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
      ],
      { ANKY_INDEXER_WRITE_SECRET: "test-secret" },
    );
    assert.equal(result.code, 0, result.stderr);
  } finally {
    await close(server);
  }

  assert.deepEqual(requests, ["/api/mobile/seals/record"]);
});

test("fails the run when backend metadata upsert fails", async () => {
  const fixturePath = writeFixtureWithValidSignatures();
  const server = http.createServer((req, res) => {
    if (req.url === "/api/mobile/seals/verified/record") {
      res.writeHead(500, { "content-type": "application/json" });
      res.end(JSON.stringify({ error: "verified rejected" }));
      return;
    }

    res.writeHead(200, { "content-type": "application/json" });
    res.end(JSON.stringify({ ok: true }));
  });
  await listen(server);

  try {
    const result = await runNode(
      [
        INDEXER_PATH,
        "--input",
        fixturePath,
        "--backend-url",
        `http://127.0.0.1:${server.address().port}`,
        "--core-collection",
        "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
      ],
      { ANKY_INDEXER_WRITE_SECRET: "test-secret" },
    );

    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /Backend metadata upsert failed/);
    assert.match(result.stderr, /verified rejected/);
  } finally {
    await close(server);
  }
});

test("does not post backend metadata for events without real Solana signatures", async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixture = JSON.parse(fs.readFileSync(FIXTURE_PATH, "utf8"));
  for (const transaction of fixture) {
    transaction.signature = `fixture_${transaction.slot}`;
  }
  const fixturePath = path.join(tempDir, "invalid-signatures.json");
  fs.writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
  const requests = [];
  const server = http.createServer((req, res) => {
    requests.push(req.url);
    req.resume();
    req.on("end", () => {
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ ok: true }));
    });
  });
  await listen(server);

  try {
    const result = await runNode(
      [
        INDEXER_PATH,
        "--input",
        fixturePath,
        "--backend-url",
        `http://127.0.0.1:${server.address().port}`,
        "--core-collection",
        "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
      ],
      { ANKY_INDEXER_WRITE_SECRET: "test-secret" },
    );
    assert.equal(result.code, 0, result.stderr);
    const snapshot = JSON.parse(result.stdout);
    assert.equal(snapshot.summary.indexedEvents, 2);
    assert.equal(snapshot.summary.sealedEvents, 0);
    assert.equal(snapshot.summary.verifiedEvents, 0);
    assert.equal(snapshot.summary.totalScore, 0);
  } finally {
    await close(server);
  }

  assert.deepEqual(requests, []);
});

test("rejects invalid reward basis points", async () => {
  const result = await runNode([
    INDEXER_PATH,
    "--input",
    FIXTURE_PATH,
    "--token-supply",
    "1000",
    "--reward-bps",
    "10001",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /reward bps must be an integer between 0 and 10000/);
});

test("rejects invalid participant caps", async () => {
  const result = await runNode([
    INDEXER_PATH,
    "--input",
    FIXTURE_PATH,
    "--max-participants",
    "0",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /max participants must be a positive safe integer/);
});

test("rejects a malformed proof verifier authority", async () => {
  const result = await runNode([
    INDEXER_PATH,
    "--input",
    FIXTURE_PATH,
    "--proof-verifier",
    "not-a-pubkey",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /proof verifier authority must be a base58 Solana public key/);
});

test("rejects base58 verifier strings that are not 32-byte public keys", async () => {
  const result = await runNode([
    INDEXER_PATH,
    "--input",
    FIXTURE_PATH,
    "--proof-verifier",
    "1".repeat(44),
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /proof verifier authority must be a base58 Solana public key/);
});

test("rejects malformed program IDs before RPC or log parsing", async () => {
  const result = await runNode([
    INDEXER_PATH,
    "--input",
    FIXTURE_PATH,
    "--program-id",
    "not-a-pubkey",
  ]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /program ID must be a base58 Solana public key/);
});

test("rejects backfill without Helius configuration", async () => {
  const result = await runNode([INDEXER_PATH, "--backfill", "--limit", "1"], {
    ANKY_SOLANA_RPC_URL: "",
    HELIUS_API_KEY: "",
  });

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /--backfill requires HELIUS_API_KEY or ANKY_SOLANA_RPC_URL/);
});

test("retries transient Helius RPC failures during backfill", async () => {
  const requests = [];
  const server = http.createServer((req, res) => {
    let body = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      const payload = JSON.parse(body);
      requests.push(payload.method);

      if (requests.length === 1) {
        res.writeHead(429, { "content-type": "text/plain" });
        res.end("rate limited");
        return;
      }

      if (payload.method === "getTransactionsForAddress") {
        res.writeHead(200, { "content-type": "application/json" });
        res.end(
          JSON.stringify({
            id: payload.id,
            jsonrpc: "2.0",
            result: {
              transactions: [
                {
                  blockTime: 1727913600,
                  commitment: "finalized",
                  finalized: true,
                  meta: { logMessages: [] },
                  signature: "fixture_signature",
                  slot: 123,
                },
              ],
            },
          }),
        );
        return;
      }

      res.writeHead(500, { "content-type": "text/plain" });
      res.end("unexpected method");
    });
  });
  await listen(server);

  try {
    const result = await runNode(
      [INDEXER_PATH, "--backfill", "--limit", "1"],
      {
        ANKY_INDEXER_RETRY_BASE_MS: "0",
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
        HELIUS_API_KEY: "",
      },
    );
    assert.equal(result.code, 0, result.stderr);
    assert.deepEqual(requests, ["getTransactionsForAddress", "getTransactionsForAddress"]);
    assert.equal(JSON.parse(result.stdout).summary.indexedEvents, 0);
  } finally {
    await close(server);
  }
});

test("uses Helius getTransactionsForAddress backfill when an API key is configured", async () => {
  const requests = [];
  const server = http.createServer((req, res) => {
    let body = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      const payload = JSON.parse(body);
      requests.push(payload);
      res.writeHead(200, { "content-type": "application/json" });
      res.end(
        JSON.stringify({
          id: payload.id,
          jsonrpc: "2.0",
          result: {
            paginationToken: "next-page",
            transactions: [
              {
                commitment: "finalized",
                finalized: true,
                meta: { logMessages: [] },
                signature: "fixture_signature",
                slot: 123,
                timestamp: 1727913600,
              },
            ],
          },
        }),
      );
    });
  });
  await listen(server);

  try {
    const result = await runNode(
      [INDEXER_PATH, "--backfill", "--limit", "1", "--before", "page-token"],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
        HELIUS_API_KEY: "test-key",
      },
    );
    assert.equal(result.code, 0, result.stderr);
    assert.equal(requests.length, 1);
    assert.equal(requests[0].method, "getTransactionsForAddress");
    assert.deepEqual(requests[0].params, [
      PROGRAM_ID,
      {
        commitment: "finalized",
        limit: 1,
        paginationToken: "page-token",
        transactionDetails: "full",
      },
    ]);
    assert.equal(JSON.parse(result.stdout).summary.indexedEvents, 0);
  } finally {
    await close(server);
  }
});

test("fetches known finalized signatures with Helius getTransaction", async () => {
  const requests = [];
  const wallet = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
  const loomAsset = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
  const sessionHash = "a".repeat(64);
  const proofHash = "b".repeat(64);
  const sealTransaction = enhancedInstructionTransaction({
    accounts: [wallet, loomAsset],
    data: sealInstructionData({ sessionHash, utcDay: 20000 }),
    signature: VALID_SIGNATURE,
  });
  const verifiedTransaction = enhancedInstructionTransaction({
    accounts: [PROOF_VERIFIER, wallet],
    data: recordVerifiedInstructionData({
      proofHash,
      protocolVersion: 1,
      sessionHash,
      utcDay: 20000,
    }),
    signature: SECOND_VALID_SIGNATURE,
    slot: 2,
  });
  delete sealTransaction.commitment;
  delete sealTransaction.finalized;
  delete verifiedTransaction.commitment;
  delete verifiedTransaction.finalized;

  const bySignature = new Map([
    [VALID_SIGNATURE, sealTransaction],
    [SECOND_VALID_SIGNATURE, verifiedTransaction],
  ]);
  const server = http.createServer((req, res) => {
    let body = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      const payload = JSON.parse(body);
      requests.push(payload);
      const signature = payload.params?.[0];
      res.writeHead(200, { "content-type": "application/json" });
      res.end(
        JSON.stringify({
          id: payload.id,
          jsonrpc: "2.0",
          result: bySignature.get(signature) ?? null,
        }),
      );
    });
  });
  await listen(server);

  try {
    const result = await runNode(
      [
        INDEXER_PATH,
        "--signature",
        `${VALID_SIGNATURE},${SECOND_VALID_SIGNATURE}`,
      ],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
        HELIUS_API_KEY: "",
      },
    );
    assert.equal(result.code, 0, result.stderr);
    assert.equal(requests.length, 2);
    assert.deepEqual(requests.map((request) => request.method), ["getTransaction", "getTransaction"]);
    assert.deepEqual(requests[0].params, [
      VALID_SIGNATURE,
      {
        commitment: "finalized",
        encoding: "json",
        maxSupportedTransactionVersion: 0,
      },
    ]);
    const snapshot = JSON.parse(result.stdout);
    assert.equal(snapshot.summary.indexedEvents, 2);
    assert.equal(snapshot.summary.totalScore, 3);
    assert.equal(snapshot.events[0].finalitySource, "known_signature_finalized_getTransaction");
    assert.equal(snapshot.events[0].finalized, true);
    assert.equal(snapshot.events[1].finalized, true);
  } finally {
    await close(server);
  }
});

test("marks backfill transactions without response commitment as inferred from finalized request", async () => {
  const requests = [];
  const wallet = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
  const loomAsset = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
  const server = http.createServer((req, res) => {
    let body = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      body += chunk;
    });
    req.on("end", () => {
      const payload = JSON.parse(body);
      requests.push(payload);
      const transaction = enhancedInstructionTransaction({
        accounts: [wallet, loomAsset],
        data: sealInstructionData({ sessionHash: "a".repeat(64), utcDay: 20000 }),
        signature: VALID_SIGNATURE,
      });
      delete transaction.commitment;
      delete transaction.finalized;
      res.writeHead(200, { "content-type": "application/json" });
      res.end(
        JSON.stringify({
          id: payload.id,
          jsonrpc: "2.0",
          result: {
            transactions: [transaction],
          },
        }),
      );
    });
  });
  await listen(server);

  try {
    const result = await runNode(
      [INDEXER_PATH, "--backfill", "--limit", "1"],
      {
        ANKY_SOLANA_RPC_URL: `http://127.0.0.1:${server.address().port}`,
        HELIUS_API_KEY: "",
      },
    );
    assert.equal(result.code, 0, result.stderr);
    const snapshot = JSON.parse(result.stdout);
    assert.equal(snapshot.summary.sealedEvents, 1);
    assert.equal(snapshot.summary.finalizedEventsInferredFromBackfillRequest, 1);
    assert.equal(snapshot.events[0].finalitySource, "requested_finalized_commitment");
  } finally {
    await close(server);
  }

  assert.equal(requests[0].params[1].commitment, "finalized");
});

function runNode(args, env = {}) {
  return new Promise((resolve) => {
    execFile(
      process.execPath,
      args,
      {
        cwd: path.dirname(INDEXER_PATH),
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

function enhancedInstructionTransaction({ accounts, data, signature, slot = 1 }) {
  return {
    commitment: "finalized",
    finalized: true,
    instructions: [
      {
        accounts,
        data,
        programId: PROGRAM_ID,
      },
    ],
    signature,
    slot,
    timestamp: 1728000000 + slot,
  };
}

function sealInstructionData({ sessionHash, utcDay }) {
  const buffer = Buffer.alloc(48);
  discriminator("global:seal_anky").copy(buffer, 0);
  Buffer.from(sessionHash, "hex").copy(buffer, 8);
  buffer.writeBigInt64LE(BigInt(utcDay), 40);

  return base58Encode(buffer);
}

function recordVerifiedInstructionData({ proofHash, protocolVersion, sessionHash, utcDay }) {
  const buffer = Buffer.alloc(82);
  discriminator("global:record_verified_anky").copy(buffer, 0);
  Buffer.from(sessionHash, "hex").copy(buffer, 8);
  buffer.writeBigInt64LE(BigInt(utcDay), 40);
  Buffer.from(proofHash, "hex").copy(buffer, 48);
  buffer.writeUInt16LE(protocolVersion, 80);

  return base58Encode(buffer);
}

function discriminator(preimage) {
  return crypto.createHash("sha256").update(preimage).digest().subarray(0, 8);
}

function base58Encode(bytes) {
  let value = BigInt(`0x${Buffer.from(bytes).toString("hex") || "0"}`);
  let encoded = "";

  while (value > 0n) {
    const remainder = Number(value % 58n);
    value /= 58n;
    encoded = BASE58_ALPHABET[remainder] + encoded;
  }

  for (const byte of bytes) {
    if (byte === 0) {
      encoded = `1${encoded}`;
    } else {
      break;
    }
  }

  return encoded || "1";
}

function writeFixtureWithValidSignatures() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-indexer-"));
  const fixture = JSON.parse(fs.readFileSync(FIXTURE_PATH, "utf8"));
  for (const transaction of fixture) {
    transaction.signature = VALID_SIGNATURE;
  }
  const fixturePath = path.join(tempDir, "valid-signatures.json");
  fs.writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);

  return fixturePath;
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
