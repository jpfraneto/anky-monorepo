#!/usr/bin/env node

import * as anchor from "@coral-xyz/anchor";
import crypto from "node:crypto";
import fs from "node:fs";
import { redactSecretValues } from "../../scripts/sojourn9/redactSecrets.mjs";

const {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} = anchor.web3;

const DEFAULT_PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const DEFAULT_VERIFIER_AUTHORITY = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const DEFAULT_DEVNET_RPC_URL = "https://api.devnet.solana.com";
const DEFAULT_MAINNET_RPC_URL = "https://api.mainnet-beta.solana.com";
const HASH_SEAL_SEED = Buffer.from("hash_seal", "utf8");
const VERIFIED_SEAL_SEED = Buffer.from("verified_seal", "utf8");
const PROTOCOL_VERSION = 1;
const PROOF_PROTOCOL = "ANKY_ZK_PROOF_V0";
const HASH_SEAL_ACCOUNT_SIZE = 120;
const HASH_SEAL_ACCOUNT_DISCRIMINATOR = discriminator("account:HashSeal");
const VERIFIED_SEAL_ACCOUNT_SIZE = 154;
const VERIFIED_SEAL_ACCOUNT_DISCRIMINATOR = discriminator("account:VerifiedSeal");
const TERMINAL_SILENCE_MS = 8_000;
const FULL_ANKY_DURATION_MS = 8 * 60 * 1_000;
const MS_PER_UTC_DAY = 86_400_000;
const SENDER_ENDPOINT = "https://sender.helius-rpc.com/fast";
const SENDER_TIP_LAMPORTS = 200_000;
const SENDER_TIP_ACCOUNTS = [
  "4ACfpUFoaSD9bfPdeu6DBt89gB6ENTeHBXCAi87NhDEE",
  "D2L6yPZ2FmmmTKPgzaMKdhu6EWZcTpLy1Vhx8uvZe7NZ",
  "9bnz4RShgq1hAnLnZbP8kbgBg1kEmcJBYQq3gQbmnSta",
  "5VY91ws6B2hMmBFRsXkoAAdsPHBJwRfBht4DXox3xkwn",
  "2nyhqdwKcJZR2vcqCyrYsaPVdAnFoJjiksCXJ7hfEYgD",
  "2q5pghRs6arqVjRvT5gfgWfWcHWmw1ZuCzphgd5KfWGJ",
  "wyvPkWjVZz1M8fHQnMMCDTQDbkManefNNhweYk5WkcF",
  "3KCKozbAaF75qEU33jtzozcJ29yJuaLJTy2jFdzUY8bT",
  "4vieeGHPYPG2MmyPRcYjdiDmmhN3ww7hsFNap8pVN3Ey",
  "4TQLFNWK8AovT1gFvda5jfw2oJeRMKEmw7aH6MGBJ3or",
];
const BOOLEAN_FLAGS = new Set([
  "--check-chain",
  "--check-hashseal-only",
  "--check-verified-chain",
  "--send",
  "--sp1-proof-verified",
]);
const VALUE_FLAGS = new Set([
  "--backend-signature",
  "--backend-url",
  "--cluster",
  "--keypair",
  "--program-id",
  "--receipt",
  "--session-hash",
  "--status",
  "--utc-day",
  "--writer",
]);

main().catch((error) => {
  console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
  process.exit(1);
});

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (args.help) {
    printUsage();
    return;
  }

  const cluster = resolveCluster(args.cluster ?? process.env.ANKY_SOLANA_CLUSTER);
  if (cluster === "mainnet-beta" && process.env.ANKY_ALLOW_MAINNET_RECORD_VERIFIED !== "true") {
    throw new Error(
      "Refusing mainnet record_verified_anky. Set ANKY_ALLOW_MAINNET_RECORD_VERIFIED=true only after the launch checklist is complete.",
    );
  }

  const programId = new PublicKey(
    args.programId ?? process.env.ANKY_SEAL_PROGRAM_ID ?? DEFAULT_PROGRAM_ID,
  );
  if (args.checkHashsealOnly === true) {
    await checkHashSealOnly({ args, cluster, programId });
    return;
  }

  const receiptPath = requiredArg(args, "receipt");
  const receipt = readReceipt(receiptPath);
  const send = args.send === true;
  const checkVerifiedChain = args.checkVerifiedChain === true;
  const checkChain = args.checkChain === true || send;
  if (send && checkVerifiedChain) {
    throw new Error("--check-verified-chain is only valid after record_verified_anky has landed.");
  }
  const backendUrl = firstNonempty(args.backendUrl, process.env.ANKY_VERIFIED_SEAL_BACKEND_URL);
  const backendSignature =
    args.backendSignature == null ? null : validateSignature(args.backendSignature);
  const backendStatus = validateBackendVerifiedStatus(args.status ?? "confirmed");
  const willPostBackend = typeof backendUrl === "string" && (send || backendSignature != null);
  const backendSecret = willPostBackend ? resolveBackendWriteSecret() : null;
  if (!send && typeof backendUrl === "string" && backendSignature != null && !checkVerifiedChain) {
    throw new Error(
      "--check-verified-chain is required before posting already-landed VerifiedSeal metadata to the backend.",
    );
  }
  const writer = new PublicKey(args.writer ?? receipt.writer);
  const verifierAuthority = new PublicKey(
    process.env.ANKY_PROOF_VERIFIER_AUTHORITY ?? DEFAULT_VERIFIER_AUTHORITY,
  );
  const sessionHash = normalizeHash(args.sessionHash ?? receipt.session_hash, "session hash");
  const proofHash = normalizeHash(receipt.proof_hash, "proof hash");
  const utcDay = toSafeInteger(args.utcDay ?? receipt.utc_day, "utc day");
  const currentUtcDay = getCurrentUtcDay();

  validateReceipt(receipt, {
    sessionHash,
    utcDay,
    writer: writer.toBase58(),
  });
  if (send && args.sp1ProofVerified !== true) {
    throw new Error(
      "--sp1-proof-verified is required with --send. Run SP1 prove/verify first, or use the proof wrapper which passes this guard only after --sp1-mode prove.",
    );
  }

  const [hashSeal] = PublicKey.findProgramAddressSync(
    [HASH_SEAL_SEED, writer.toBuffer(), Buffer.from(sessionHash, "hex")],
    programId,
  );
  const [verifiedSeal] = PublicKey.findProgramAddressSync(
    [VERIFIED_SEAL_SEED, writer.toBuffer(), Buffer.from(sessionHash, "hex")],
    programId,
  );

  const summary = {
    cluster,
    chainPreflight: null,
    currentUtcDay,
    backendPost: null,
    dryRun: !send,
    hashSeal: hashSeal.toBase58(),
    programId: programId.toBase58(),
    proofHash,
    protocolVersion: PROTOCOL_VERSION,
    receiptUtcDayIsCurrent: utcDay === currentUtcDay,
    sessionHash,
    utcDay,
    verifiedSeal: verifiedSeal.toBase58(),
    verifiedChain: null,
    verifier: verifierAuthority.toBase58(),
    writer: writer.toBase58(),
  };

  const connection =
    checkChain || checkVerifiedChain ? new Connection(resolveRpcUrl(cluster), "confirmed") : null;
  if (connection != null && checkChain) {
    summary.chainPreflight = await readChainPreflight({
      connection,
      hashSeal,
      programId,
      sessionHash,
      utcDay,
      verifiedSeal,
      writer,
    });

    if (!summary.chainPreflight.ok) {
      throw new Error(`Chain preflight failed: ${summary.chainPreflight.reason}`);
    }
  }
  if (connection != null && checkVerifiedChain) {
    summary.verifiedChain = await readVerifiedChain({
      connection,
      hashSeal,
      programId,
      proofHash,
      sessionHash,
      utcDay,
      verifiedSeal,
      verifier: verifierAuthority,
      writer,
    });

    if (!summary.verifiedChain.ok) {
      throw new Error(`VerifiedSeal chain check failed: ${summary.verifiedChain.reason}`);
    }
  }

  if (!send) {
    if (typeof backendUrl === "string" && backendSignature != null) {
      summary.backendPost = await postBackendVerifiedSeal({
        backendUrl,
        proofHash,
        protocolVersion: PROTOCOL_VERSION,
        sessionHash,
        signature: backendSignature,
        status: backendStatus,
        secret: backendSecret,
        utcDay,
        verifier: verifierAuthority.toBase58(),
        wallet: writer.toBase58(),
      });
    }
    console.log(JSON.stringify(summary, null, 2));
    console.log("dry run only; rerun with --send after SP1 proof verification and non-mainnet operator approval.");
    return;
  }

  const verifier = loadVerifierKeypair(args.keypair);
  if (!verifier.publicKey.equals(verifierAuthority)) {
    throw new Error(
      `Verifier keypair public key ${verifier.publicKey.toBase58()} does not match program authority ${verifierAuthority.toBase58()}.`,
    );
  }

  const instruction = buildRecordVerifiedInstruction({
    hashSeal,
    programId,
    proofHash,
    sessionHash,
    utcDay,
    verifiedSeal,
    verifier: verifier.publicKey,
    writer,
  });
  const transaction = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 80_000 }),
    ComputeBudgetProgram.setComputeUnitPrice({
      microLamports: await resolvePriorityFeeMicroLamports({
        accountKeys: [
          programId.toBase58(),
          hashSeal.toBase58(),
          verifiedSeal.toBase58(),
          verifier.publicKey.toBase58(),
          writer.toBase58(),
        ],
        cluster,
      }),
    }),
    instruction,
  );

  const signature =
    cluster === "mainnet-beta"
      ? await sendMainnetViaHeliusSender({
          connection,
          signer: verifier,
          transaction,
        })
      : await sendAndConfirmTransaction(connection, transaction, [verifier], {
          commitment: "confirmed",
          skipPreflight: false,
        });
  if (typeof backendUrl === "string") {
    summary.verifiedChain = await readVerifiedChain({
      connection,
      hashSeal,
      programId,
      proofHash,
      sessionHash,
      utcDay,
      verifiedSeal,
      verifier: verifierAuthority,
      writer,
    });

    if (!summary.verifiedChain.ok) {
      throw new Error(`Post-send VerifiedSeal chain check failed: ${summary.verifiedChain.reason}`);
    }
  }
  const backendPost =
    typeof backendUrl === "string"
      ? await postBackendVerifiedSeal({
          backendUrl,
          proofHash,
          protocolVersion: PROTOCOL_VERSION,
          sessionHash,
          signature,
          status: backendStatus,
          secret: backendSecret,
          utcDay,
          verifier: verifierAuthority.toBase58(),
          wallet: writer.toBase58(),
        })
      : null;

  console.log(
    JSON.stringify(
      {
        ...summary,
        backendPost,
        dryRun: false,
        signature,
      },
      null,
      2,
    ),
  );
}

async function checkHashSealOnly({ args, cluster, programId }) {
  const writer = new PublicKey(requiredArg(args, "writer"));
  const sessionHash = normalizeHash(requiredArg(args, "sessionHash"), "session hash");
  const utcDay = toSafeInteger(requiredArg(args, "utcDay"), "utc day");
  const [hashSeal] = PublicKey.findProgramAddressSync(
    [HASH_SEAL_SEED, writer.toBuffer(), Buffer.from(sessionHash, "hex")],
    programId,
  );
  const [verifiedSeal] = PublicKey.findProgramAddressSync(
    [VERIFIED_SEAL_SEED, writer.toBuffer(), Buffer.from(sessionHash, "hex")],
    programId,
  );
  const connection = new Connection(resolveRpcUrl(cluster), "confirmed");
  const chainPreflight = await readChainPreflight({
    connection,
    hashSeal,
    programId,
    sessionHash,
    utcDay,
    verifiedSeal,
    writer,
  });

  if (!chainPreflight.ok) {
    throw new Error(`HashSeal preflight failed: ${chainPreflight.reason}`);
  }

  console.log(
    JSON.stringify(
      {
        chainPreflight,
        cluster,
        hashSeal: hashSeal.toBase58(),
        mode: "hash_seal_preflight",
        programId: programId.toBase58(),
        sessionHash,
        utcDay,
        verifiedSeal: verifiedSeal.toBase58(),
        writer: writer.toBase58(),
      },
      null,
      2,
    ),
  );
}

async function postBackendVerifiedSeal({
  backendUrl,
  proofHash,
  protocolVersion,
  secret,
  sessionHash,
  signature,
  status,
  utcDay,
  verifier,
  wallet,
}) {
  const response = await fetch(`${backendUrl.replace(/\/+$/, "")}/api/mobile/seals/verified/record`, {
    body: JSON.stringify({
      proofHash,
      protocolVersion,
      sessionHash,
      signature,
      status,
      utcDay,
      verifier,
      wallet,
    }),
    headers: {
      "content-type": "application/json",
      "x-anky-indexer-secret": secret,
    },
    method: "POST",
  });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`Backend verified metadata post failed with HTTP ${response.status}: ${body}`);
  }

  return {
    ok: true,
    status: response.status,
  };
}

function resolveBackendWriteSecret() {
  const indexerSecret = process.env.ANKY_INDEXER_WRITE_SECRET?.trim() ?? "";
  if (indexerSecret.length > 0) {
    return indexerSecret;
  }

  const verifiedSealRecordSecret = process.env.ANKY_VERIFIED_SEAL_RECORD_SECRET?.trim() ?? "";
  if (verifiedSealRecordSecret.length > 0) {
    return verifiedSealRecordSecret;
  }

  throw new Error(
    "ANKY_INDEXER_WRITE_SECRET or ANKY_VERIFIED_SEAL_RECORD_SECRET is required for backend verified metadata posts.",
  );
}

async function sendMainnetViaHeliusSender({ connection, signer, transaction }) {
  transaction.add(
    SystemProgram.transfer({
      fromPubkey: signer.publicKey,
      lamports: SENDER_TIP_LAMPORTS,
      toPubkey: randomSenderTipAccount(),
    }),
  );

  const latestBlockhash = await connection.getLatestBlockhash("confirmed");
  transaction.feePayer = signer.publicKey;
  transaction.recentBlockhash = latestBlockhash.blockhash;
  transaction.sign(signer);

  const response = await fetch(process.env.ANKY_SENDER_ENDPOINT ?? SENDER_ENDPOINT, {
    body: JSON.stringify({
      id: crypto.randomUUID(),
      jsonrpc: "2.0",
      method: "sendTransaction",
      params: [
        transaction.serialize().toString("base64"),
        {
          encoding: "base64",
          maxRetries: 0,
          skipPreflight: true,
        },
      ],
    }),
    headers: { "content-type": "application/json" },
    method: "POST",
  });
  const json = await response.json();
  if (json.error != null) {
    throw new Error(`Helius Sender failed: ${json.error.message ?? JSON.stringify(json.error)}`);
  }

  await connection.confirmTransaction(
    {
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      signature: json.result,
    },
    "confirmed",
  );

  return json.result;
}

async function resolvePriorityFeeMicroLamports({ accountKeys, cluster }) {
  if (cluster !== "mainnet-beta") {
    return Number(process.env.ANKY_RECORD_VERIFIED_MICROLAMPORTS ?? "0");
  }

  const heliusKey = process.env.HELIUS_API_KEY?.trim();
  if (heliusKey == null || heliusKey.length === 0) {
    throw new Error("HELIUS_API_KEY is required for mainnet priority fee estimation.");
  }

  const response = await fetch(`https://mainnet.helius-rpc.com/?api-key=${heliusKey}`, {
    body: JSON.stringify({
      id: crypto.randomUUID(),
      jsonrpc: "2.0",
      method: "getPriorityFeeEstimate",
      params: [
        {
          accountKeys,
          options: {
            priorityLevel: "High",
          },
        },
      ],
    }),
    headers: { "content-type": "application/json" },
    method: "POST",
  });
  const json = await response.json();
  if (json.error != null) {
    throw new Error(
      `getPriorityFeeEstimate failed: ${json.error.message ?? JSON.stringify(json.error)}`,
    );
  }

  const estimate = Number(json.result?.priorityFeeEstimate);
  if (!Number.isFinite(estimate) || estimate < 0) {
    throw new Error("getPriorityFeeEstimate returned an invalid priority fee.");
  }

  return Math.ceil(estimate);
}

function randomSenderTipAccount() {
  const index = crypto.randomInt(0, SENDER_TIP_ACCOUNTS.length);
  return new PublicKey(SENDER_TIP_ACCOUNTS[index]);
}

function buildRecordVerifiedInstruction({
  hashSeal,
  programId,
  proofHash,
  sessionHash,
  utcDay,
  verifiedSeal,
  verifier,
  writer,
}) {
  const data = Buffer.concat([
    discriminator("global:record_verified_anky"),
    Buffer.from(sessionHash, "hex"),
    i64Le(utcDay),
    Buffer.from(proofHash, "hex"),
    u16Le(PROTOCOL_VERSION),
  ]);

  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: verifier, isSigner: true, isWritable: true },
      { pubkey: writer, isSigner: false, isWritable: false },
      { pubkey: hashSeal, isSigner: false, isWritable: false },
      { pubkey: verifiedSeal, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

function validateReceipt(receipt, expected) {
  const startedAtMs = toSafeInteger(receipt.started_at_ms, "receipt started_at_ms");
  const acceptedDurationMs = toSafeInteger(
    receipt.accepted_duration_ms,
    "receipt accepted_duration_ms",
  );
  const riteDurationMs = toSafeInteger(receipt.rite_duration_ms, "receipt rite_duration_ms");
  const eventCount = toSafeInteger(receipt.event_count, "receipt event_count");

  if (receipt.version !== PROTOCOL_VERSION) {
    throw new Error(`Receipt version ${receipt.version} is not supported.`);
  }
  if (receipt.protocol !== PROOF_PROTOCOL) {
    throw new Error(`Receipt protocol ${receipt.protocol} is not supported.`);
  }
  if (receipt.valid !== true || receipt.duration_ok !== true) {
    throw new Error("Receipt is not valid and duration_ok.");
  }
  if (startedAtMs < 0 || acceptedDurationMs < 0 || riteDurationMs < 0 || eventCount <= 0) {
    throw new Error("Receipt public timing and event count values are invalid.");
  }
  if (riteDurationMs !== acceptedDurationMs + TERMINAL_SILENCE_MS) {
    throw new Error("Receipt rite_duration_ms does not match accepted_duration_ms plus terminal silence.");
  }
  if (riteDurationMs < FULL_ANKY_DURATION_MS) {
    throw new Error("Receipt rite_duration_ms is shorter than a full 8-minute Anky.");
  }
  if (receipt.utc_day !== Math.floor(startedAtMs / MS_PER_UTC_DAY)) {
    throw new Error("Receipt utc_day does not match started_at_ms.");
  }
  if (receipt.writer !== expected.writer) {
    throw new Error("Receipt writer does not match the requested writer.");
  }
  if (receipt.session_hash !== expected.sessionHash) {
    throw new Error("Receipt session_hash does not match the requested session hash.");
  }
  if (receipt.utc_day !== expected.utcDay) {
    throw new Error("Receipt utc_day does not match the requested UTC day.");
  }

  const expectedProofHash = computeReceiptHash(receipt);
  if (receipt.proof_hash !== expectedProofHash) {
    throw new Error("Receipt proof_hash does not match its public values.");
  }
}

async function readChainPreflight({
  connection,
  hashSeal,
  programId,
  sessionHash,
  utcDay,
  verifiedSeal,
  writer,
}) {
  const [hashSealAccount, verifiedSealAccount] = await connection.getMultipleAccountsInfo(
    [hashSeal, verifiedSeal],
    "confirmed",
  );

  if (hashSealAccount == null) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: missingHashSealReason(utcDay),
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }

  if (!hashSealAccount.owner.equals(programId)) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "HashSeal account is not owned by the Anky Seal Program",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }

  const decodedHashSeal = decodeHashSeal(hashSealAccount.data);
  if (decodedHashSeal == null) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "HashSeal account data is not a valid HashSeal",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }

  if (
    decodedHashSeal.writer !== writer.toBase58() ||
    decodedHashSeal.sessionHash !== sessionHash ||
    decodedHashSeal.utcDay !== utcDay
  ) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "HashSeal account does not match receipt writer, session hash, and UTC day",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }

  if (verifiedSealAccount != null) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "VerifiedSeal account already exists",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }

  return {
    hashSeal: hashSeal.toBase58(),
    hashSealAccount: decodedHashSeal,
    ok: true,
    verifiedSeal: verifiedSeal.toBase58(),
  };
}

async function readVerifiedChain({
  connection,
  hashSeal,
  programId,
  proofHash,
  sessionHash,
  utcDay,
  verifiedSeal,
  verifier,
  writer,
}) {
  const [hashSealAccount, verifiedSealAccount] = await connection.getMultipleAccountsInfo(
    [hashSeal, verifiedSeal],
    "confirmed",
  );

  if (hashSealAccount == null) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: missingHashSealReason(utcDay),
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }
  if (!hashSealAccount.owner.equals(programId)) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "HashSeal account is not owned by the Anky Seal Program",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }

  const decodedHashSeal = decodeHashSeal(hashSealAccount.data);
  if (decodedHashSeal == null) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "HashSeal account data is not a valid HashSeal",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }
  if (
    decodedHashSeal.writer !== writer.toBase58() ||
    decodedHashSeal.sessionHash !== sessionHash ||
    decodedHashSeal.utcDay !== utcDay
  ) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "HashSeal account does not match receipt writer, session hash, and UTC day",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }

  if (verifiedSealAccount == null) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "VerifiedSeal account does not exist",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }
  if (!verifiedSealAccount.owner.equals(programId)) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "VerifiedSeal account is not owned by the Anky Seal Program",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }

  const decodedVerifiedSeal = decodeVerifiedSeal(verifiedSealAccount.data);
  if (decodedVerifiedSeal == null) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "VerifiedSeal account data is not valid",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }
  if (
    decodedVerifiedSeal.writer !== writer.toBase58() ||
    decodedVerifiedSeal.sessionHash !== sessionHash ||
    decodedVerifiedSeal.utcDay !== utcDay ||
    decodedVerifiedSeal.proofHash !== proofHash ||
    decodedVerifiedSeal.verifier !== verifier.toBase58() ||
    decodedVerifiedSeal.protocolVersion !== PROTOCOL_VERSION
  ) {
    return {
      hashSeal: hashSeal.toBase58(),
      ok: false,
      reason: "VerifiedSeal account does not match the public receipt",
      verifiedSeal: verifiedSeal.toBase58(),
    };
  }

  return {
    hashSeal: hashSeal.toBase58(),
    hashSealAccount: decodedHashSeal,
    ok: true,
    verifiedSeal: verifiedSeal.toBase58(),
    verifiedSealAccount: decodedVerifiedSeal,
  };
}

function decodeHashSeal(data) {
  if (!Buffer.isBuffer(data) || data.length < HASH_SEAL_ACCOUNT_SIZE) {
    return null;
  }
  if (!data.subarray(0, 8).equals(HASH_SEAL_ACCOUNT_DISCRIMINATOR)) {
    return null;
  }

  let offset = 8;
  const writer = new PublicKey(data.subarray(offset, offset + 32)).toBase58();
  offset += 32;
  const loomAsset = new PublicKey(data.subarray(offset, offset + 32)).toBase58();
  offset += 32;
  const sessionHash = data.subarray(offset, offset + 32).toString("hex");
  offset += 32;
  const utcDay = Number(data.readBigInt64LE(offset));
  offset += 8;
  const timestamp = Number(data.readBigInt64LE(offset));

  return {
    loomAsset,
    sessionHash,
    timestamp,
    utcDay,
    writer,
  };
}

function decodeVerifiedSeal(data) {
  if (!Buffer.isBuffer(data) || data.length < VERIFIED_SEAL_ACCOUNT_SIZE) {
    return null;
  }
  if (!data.subarray(0, 8).equals(VERIFIED_SEAL_ACCOUNT_DISCRIMINATOR)) {
    return null;
  }

  let offset = 8;
  const writer = new PublicKey(data.subarray(offset, offset + 32)).toBase58();
  offset += 32;
  const sessionHash = data.subarray(offset, offset + 32).toString("hex");
  offset += 32;
  const utcDay = Number(data.readBigInt64LE(offset));
  offset += 8;
  const proofHash = data.subarray(offset, offset + 32).toString("hex");
  offset += 32;
  const verifier = new PublicKey(data.subarray(offset, offset + 32)).toBase58();
  offset += 32;
  const protocolVersion = data.readUInt16LE(offset);
  offset += 2;
  const timestamp = Number(data.readBigInt64LE(offset));

  return {
    proofHash,
    protocolVersion,
    sessionHash,
    timestamp,
    utcDay,
    verifier,
    writer,
  };
}

function readReceipt(path) {
  const parsed = JSON.parse(fs.readFileSync(path, "utf8"));
  if (typeof parsed !== "object" || parsed == null) {
    throw new Error("Receipt must be a JSON object.");
  }

  return parsed;
}

function loadVerifierKeypair(pathArg) {
  const path = pathArg ?? process.env.ANKY_VERIFIER_KEYPAIR_PATH ?? process.env.ANCHOR_WALLET;
  if (path == null || path.trim().length === 0) {
    throw new Error("ANKY_VERIFIER_KEYPAIR_PATH or --keypair is required when --send is used.");
  }

  const secret = JSON.parse(fs.readFileSync(path, "utf8"));
  if (!Array.isArray(secret)) {
    throw new Error("Verifier keypair file must contain a Solana keypair byte array.");
  }

  return Keypair.fromSecretKey(Uint8Array.from(secret));
}

function resolveRpcUrl(cluster) {
  if (process.env.ANKY_SOLANA_RPC_URL != null && process.env.ANKY_SOLANA_RPC_URL.trim() !== "") {
    return process.env.ANKY_SOLANA_RPC_URL.trim();
  }

  if (process.env.HELIUS_API_KEY != null && process.env.HELIUS_API_KEY.trim() !== "") {
    const host = cluster === "mainnet-beta" ? "mainnet" : "devnet";
    return `https://${host}.helius-rpc.com/?api-key=${process.env.HELIUS_API_KEY.trim()}`;
  }

  return cluster === "mainnet-beta" ? DEFAULT_MAINNET_RPC_URL : DEFAULT_DEVNET_RPC_URL;
}

function resolveCluster(value) {
  if (value == null || value === "" || value === "devnet") {
    return "devnet";
  }
  if (value === "mainnet-beta") {
    return "mainnet-beta";
  }

  throw new Error("--cluster must be devnet or mainnet-beta.");
}

function discriminator(preimage) {
  return crypto.createHash("sha256").update(preimage).digest().subarray(0, 8);
}

function i64Le(value) {
  const buffer = Buffer.alloc(8);
  buffer.writeBigInt64LE(BigInt(value));
  return buffer;
}

function u16Le(value) {
  const buffer = Buffer.alloc(2);
  buffer.writeUInt16LE(value);
  return buffer;
}

function normalizeHash(value, label) {
  if (typeof value !== "string" || !/^[0-9a-fA-F]{64}$/.test(value.trim())) {
    throw new Error(`${label} must be 64 hex characters.`);
  }

  return value.trim().toLowerCase();
}

function validateSignature(value) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error("signature is required.");
  }

  const bytes = anchor.utils.bytes.bs58.decode(value.trim());
  if (bytes.length !== 64) {
    throw new Error("signature must decode to a 64-byte Solana signature.");
  }

  return value.trim();
}

function validateBackendVerifiedStatus(value) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error("status is required.");
  }
  const status = value.trim();
  if (status !== "confirmed" && status !== "finalized") {
    throw new Error("backend verified status must be confirmed or finalized.");
  }

  return status;
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

function toSafeInteger(value, label) {
  const parsed = typeof value === "number" ? value : Number(value);
  if (!Number.isSafeInteger(parsed)) {
    throw new Error(`${label} must be a safe integer.`);
  }

  return parsed;
}

function getCurrentUtcDay() {
  return Math.floor(Date.now() / MS_PER_UTC_DAY);
}

function missingHashSealReason(utcDay) {
  const currentUtcDay = getCurrentUtcDay();
  if (utcDay !== currentUtcDay) {
    return `matching HashSeal account does not exist; receipt UTC day ${utcDay} is not current UTC day ${currentUtcDay}, so a new devnet seal for this historical receipt cannot be created today`;
  }

  return "matching HashSeal account does not exist";
}

function requiredArg(args, name) {
  const value = args[name];
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`--${name} is required.`);
  }

  return value;
}

function firstNonempty(...values) {
  for (const value of values) {
    if (typeof value === "string" && value.trim().length > 0) {
      return value.trim();
    }
  }

  return null;
}

function parseArgs(argv) {
  const args = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (!BOOLEAN_FLAGS.has(arg) && !VALUE_FLAGS.has(arg)) {
      if (!arg.startsWith("--")) {
        throw new Error(`Unexpected argument: ${arg}`);
      }
      throw new Error(`Unknown option: ${arg}`);
    }
    if (arg === "--send") {
      args.send = true;
      continue;
    }
    if (arg === "--check-chain") {
      args.checkChain = true;
      continue;
    }
    if (arg === "--check-hashseal-only") {
      args.checkHashsealOnly = true;
      continue;
    }
    if (arg === "--check-verified-chain") {
      args.checkVerifiedChain = true;
      continue;
    }
    if (arg === "--sp1-proof-verified") {
      args.sp1ProofVerified = true;
      continue;
    }
    if (!arg.startsWith("--")) {
      throw new Error(`Unexpected argument: ${arg}`);
    }

    const key = arg.slice(2).replace(/-([a-z])/g, (_match, letter) => letter.toUpperCase());
    const value = argv[index + 1];
    if (value == null || value.startsWith("--")) {
      throw new Error(`${arg} requires a value.`);
    }
    args[key] = value;
    index += 1;
  }

  return args;
}

function printUsage() {
  console.log(`Usage:
  npm run record-verified -- --receipt ../anky-zk-proof/sp1/script/receipt.json --writer <wallet>

Options:
  --receipt <path>        SP1 receipt JSON emitted after local proof verification.
  --writer <pubkey>       Writer wallet; defaults to receipt.writer.
  --session-hash <hex>    Expected session hash; defaults to receipt.session_hash.
  --utc-day <day>         Expected UTC day; defaults to receipt.utc_day.
  --program-id <pubkey>   Seal program; defaults to ANKY_SEAL_PROGRAM_ID or devnet constant.
  --cluster <cluster>     devnet or mainnet-beta. Defaults to ANKY_SOLANA_CLUSTER or devnet.
  --check-chain           Confirm matching HashSeal exists and VerifiedSeal is absent.
  --check-hashseal-only   Check writer/session-hash/utc-day HashSeal before running SP1.
  --check-verified-chain  Confirm matching HashSeal and VerifiedSeal exist before backend post.
  --backend-url <url>     POST verified metadata after --send or with --backend-signature.
  --backend-signature <s> Record an already-landed record_verified_anky signature in backend.
  --status <status>       Backend verified status for metadata posts: confirmed or finalized.
                          Defaults to confirmed.
  --keypair <path>        Verifier authority keypair path for --send.
  --sp1-proof-verified    Required with --send after local SP1 proof verification.
  --send                  Submit record_verified_anky. Omitted means dry-run only.

The script never reads .anky plaintext. Execute-only receipts are valid for dry-runs only; run SP1 prove/verify and pass --sp1-proof-verified before --send.`);
}
