#!/usr/bin/env node

import * as anchor from "@coral-xyz/anchor";
import crypto from "node:crypto";
import fs from "node:fs";
import { redactSecretValues } from "../../scripts/sojourn9/redactSecrets.mjs";

const {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} = anchor.web3;

const DEFAULT_PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const DEFAULT_CORE_PROGRAM_ID = "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d";
const DEFAULT_CORE_COLLECTION = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const DEFAULT_DEVNET_RPC_URL = "https://api.devnet.solana.com";
const LOOM_STATE_SEED = Buffer.from("loom_state", "utf8");
const DAILY_SEAL_SEED = Buffer.from("daily_seal", "utf8");
const HASH_SEAL_SEED = Buffer.from("hash_seal", "utf8");
const HASH_SEAL_ACCOUNT_DISCRIMINATOR = discriminator("account:HashSeal");
const CORE_ASSET_V1_KEY = 1;
const CORE_COLLECTION_V1_KEY = 5;
const CORE_UPDATE_AUTHORITY_COLLECTION = 2;
const MS_PER_UTC_DAY = 86_400_000;
const BOOLEAN_FLAGS = new Set(["--check-chain", "--check-sealed-chain", "--send"]);
const VALUE_FLAGS = new Set([
  "--backend-signature",
  "--backend-url",
  "--cluster",
  "--core-collection",
  "--core-program-id",
  "--keypair",
  "--loom-asset",
  "--program-id",
  "--rpc-url",
  "--session-hash",
  "--utc-day",
  "--writer",
]);

main().catch((error) => {
  console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
  process.exit(1);
});

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help === true) {
    printUsage();
    return;
  }

  const cluster = resolveCluster(args.cluster ?? process.env.ANKY_SOLANA_CLUSTER);
  if (cluster === "mainnet-beta") {
    throw new Error(
      "Refusing mainnet seal_anky from this helper. Prove the full devnet loop first, then use the separate mainnet launch checklist.",
    );
  }

  const programId = readPublicKey(
    args.programId ?? process.env.ANKY_SEAL_PROGRAM_ID ?? DEFAULT_PROGRAM_ID,
    "seal program ID",
  );
  const coreProgramId = readPublicKey(
    args.coreProgramId ?? process.env.ANKY_CORE_PROGRAM_ID ?? DEFAULT_CORE_PROGRAM_ID,
    "Core program ID",
  );
  const coreCollection = readPublicKey(
    args.coreCollection ?? process.env.ANKY_CORE_COLLECTION ?? DEFAULT_CORE_COLLECTION,
    "Core collection",
  );
  const loomAsset = readPublicKey(requiredArg(args, "loomAsset"), "Core Loom asset");
  const sessionHash = normalizeHash(requiredArg(args, "sessionHash"), "session hash");
  const utcDay = toSafeInteger(requiredArg(args, "utcDay"), "utc day");
  const currentUtcDay = getCurrentUtcDay();
  const send = args.send === true;
  const checkChain = args.checkChain === true || send;
  const checkSealedChain = args.checkSealedChain === true;
  const backendUrl = firstNonempty(args.backendUrl, process.env.ANKY_SEAL_BACKEND_URL);
  const backendSignature =
    args.backendSignature == null ? null : validateSignature(args.backendSignature);
  const willPostBackend = typeof backendUrl === "string" && (send || backendSignature != null);
  if (typeof backendUrl === "string" && !send && backendSignature == null) {
    throw new Error("--backend-url requires --send or --backend-signature.");
  }
  if (backendSignature != null && !checkSealedChain) {
    throw new Error("--check-sealed-chain is required before posting already-landed seal metadata.");
  }
  if (send && checkSealedChain) {
    throw new Error("--check-sealed-chain is for already-landed seals. Use --check-chain before --send.");
  }

  let signer = null;
  let writer =
    args.writer == null ? null : readPublicKey(args.writer, "writer wallet");
  if (send) {
    signer = loadSealerKeypair(args.keypair);
    if (writer != null && !writer.equals(signer.publicKey)) {
      throw new Error(
        `Writer ${writer.toBase58()} does not match sealer keypair ${signer.publicKey.toBase58()}.`,
      );
    }
    writer = signer.publicKey;
  }
  if (writer == null) {
    throw new Error("--writer is required unless --send loads a sealer keypair.");
  }
  if (send && utcDay !== currentUtcDay) {
    throw new Error(
      `seal_anky can only be sent for the current UTC day. requested=${utcDay} current=${currentUtcDay}`,
    );
  }
  if (checkChain && utcDay !== currentUtcDay) {
    throw new Error(
      `seal_anky preflight requires the current UTC day. requested=${utcDay} current=${currentUtcDay}`,
    );
  }

  const pdas = deriveSealPdas({
    programId,
    sessionHash,
    utcDay,
    writer,
    loomAsset,
  });
  const summary = {
    chainPreflight: null,
    cluster,
    backendPost: null,
    coreCollection: coreCollection.toBase58(),
    coreProgramId: coreProgramId.toBase58(),
    currentUtcDay,
    dailySeal: pdas.dailySeal.toBase58(),
    dryRun: !send,
    hashSeal: pdas.hashSeal.toBase58(),
    instruction: "seal_anky",
    loomAsset: loomAsset.toBase58(),
    loomState: pdas.loomState.toBase58(),
    programId: programId.toBase58(),
    receiptUtcDayIsCurrent: utcDay === currentUtcDay,
    rpcUrl: redactRpcUrl(resolveRpcUrl(cluster, args.rpcUrl)),
    sessionHash,
    signature: null,
    sealedChain: null,
    utcDay,
    writer: writer.toBase58(),
  };

  const connection =
    checkChain || checkSealedChain || send
      ? new Connection(resolveRpcUrl(cluster, args.rpcUrl), "confirmed")
      : null;
  if (connection != null && checkChain) {
    summary.chainPreflight = await readSealPreflight({
      connection,
      coreCollection,
      coreProgramId,
      dailySeal: pdas.dailySeal,
      hashSeal: pdas.hashSeal,
      loomAsset,
      writer,
    });
    if (!summary.chainPreflight.ok) {
      throw new Error(`Seal preflight failed: ${summary.chainPreflight.reason}`);
    }
  }
  if (connection != null && checkSealedChain) {
    summary.sealedChain = await readSealedChain({
      connection,
      hashSeal: pdas.hashSeal,
      loomAsset,
      programId,
      sessionHash,
      utcDay,
      writer,
    });
    if (!summary.sealedChain.ok) {
      throw new Error(`Sealed chain check failed: ${summary.sealedChain.reason}`);
    }
  }

  if (!send) {
    if (willPostBackend) {
      summary.backendPost = await postBackendSeal({
        backendUrl,
        coreCollection: coreCollection.toBase58(),
        loomAsset: loomAsset.toBase58(),
        sessionHash,
        signature: backendSignature,
        status: "confirmed",
        utcDay,
        wallet: writer.toBase58(),
      });
    }
    console.log(JSON.stringify(summary, null, 2));
    console.log("dry run only; rerun with --send from an operator shell after confirming the writer controls the Loom.");
    return;
  }

  const instruction = buildSealAnkyInstruction({
    coreCollection,
    dailySeal: pdas.dailySeal,
    hashSeal: pdas.hashSeal,
    loomAsset,
    loomState: pdas.loomState,
    programId,
    sessionHash,
    utcDay,
    writer,
  });
  const transaction = new Transaction().add(instruction);
  summary.signature = await sendAndConfirmTransaction(connection, transaction, [signer], {
    commitment: "confirmed",
    skipPreflight: false,
  });
  if (willPostBackend) {
    summary.backendPost = await postBackendSeal({
      backendUrl,
      coreCollection: coreCollection.toBase58(),
      loomAsset: loomAsset.toBase58(),
      sessionHash,
      signature: summary.signature,
      status: "confirmed",
      utcDay,
      wallet: writer.toBase58(),
    });
  }

  console.log(JSON.stringify(summary, null, 2));
}

function deriveSealPdas({ programId, sessionHash, utcDay, writer, loomAsset }) {
  const [loomState] = PublicKey.findProgramAddressSync(
    [LOOM_STATE_SEED, loomAsset.toBuffer()],
    programId,
  );
  const [dailySeal] = PublicKey.findProgramAddressSync(
    [DAILY_SEAL_SEED, writer.toBuffer(), encodeI64Le(utcDay)],
    programId,
  );
  const [hashSeal] = PublicKey.findProgramAddressSync(
    [HASH_SEAL_SEED, writer.toBuffer(), Buffer.from(sessionHash, "hex")],
    programId,
  );

  return {
    dailySeal,
    hashSeal,
    loomState,
  };
}

async function readSealPreflight({
  connection,
  coreCollection,
  coreProgramId,
  dailySeal,
  hashSeal,
  loomAsset,
  writer,
}) {
  const [collectionAccount, loomAccount, dailySealAccount, hashSealAccount] =
    await Promise.all([
      connection.getAccountInfo(coreCollection, "confirmed"),
      connection.getAccountInfo(loomAsset, "confirmed"),
      connection.getAccountInfo(dailySeal, "confirmed"),
      connection.getAccountInfo(hashSeal, "confirmed"),
    ]);

  if (dailySealAccount != null) {
    return {
      ok: false,
      reason: "DailySeal already exists for this writer and UTC day",
    };
  }
  if (hashSealAccount != null) {
    return {
      ok: false,
      reason: "HashSeal already exists for this writer and session hash",
    };
  }
  if (collectionAccount == null) {
    return {
      ok: false,
      reason: "Core collection account does not exist",
    };
  }
  if (!collectionAccount.owner.equals(coreProgramId)) {
    return {
      ok: false,
      reason: "Core collection account is not owned by the Metaplex Core program",
    };
  }
  if (collectionAccount.data?.[0] !== CORE_COLLECTION_V1_KEY) {
    return {
      ok: false,
      reason: "Core collection account is not a CollectionV1 base account",
    };
  }
  if (loomAccount == null) {
    return {
      ok: false,
      reason: "Core Loom asset account does not exist",
    };
  }
  if (!loomAccount.owner.equals(coreProgramId)) {
    return {
      ok: false,
      reason: "Core Loom asset account is not owned by the Metaplex Core program",
    };
  }
  const parsedLoom = parseCoreAssetBase(loomAccount.data);
  if (parsedLoom == null) {
    return {
      ok: false,
      reason: "Core Loom asset account is not an AssetV1 base account",
    };
  }
  if (parsedLoom.owner !== writer.toBase58()) {
    return {
      ok: false,
      reason: "Core Loom asset owner does not match writer",
      loomOwner: parsedLoom.owner,
    };
  }
  if (parsedLoom.collection !== coreCollection.toBase58()) {
    return {
      ok: false,
      reason: "Core Loom asset collection does not match configured collection",
      loomCollection: parsedLoom.collection,
    };
  }

  return {
    collection: coreCollection.toBase58(),
    loomOwner: parsedLoom.owner,
    ok: true,
  };
}

async function readSealedChain({
  connection,
  hashSeal,
  loomAsset,
  programId,
  sessionHash,
  utcDay,
  writer,
}) {
  const hashSealAccount = await connection.getAccountInfo(hashSeal, "confirmed");
  if (hashSealAccount == null) {
    return {
      ok: false,
      reason: "HashSeal account does not exist",
    };
  }
  if (!hashSealAccount.owner.equals(programId)) {
    return {
      ok: false,
      reason: "HashSeal account is not owned by the Anky Seal Program",
    };
  }

  const decoded = decodeHashSeal(hashSealAccount.data);
  if (decoded == null) {
    return {
      ok: false,
      reason: "HashSeal account data is not a valid HashSeal",
    };
  }
  if (
    decoded.writer !== writer.toBase58() ||
    decoded.loomAsset !== loomAsset.toBase58() ||
    decoded.sessionHash !== sessionHash ||
    decoded.utcDay !== utcDay
  ) {
    return {
      ok: false,
      reason: "HashSeal account does not match writer, Loom asset, session hash, and UTC day",
      hashSealAccount: decoded,
    };
  }

  return {
    hashSealAccount: decoded,
    ok: true,
  };
}

function decodeHashSeal(data) {
  if (!Buffer.isBuffer(data) || data.length < 120) {
    return null;
  }
  if (!data.subarray(0, 8).equals(HASH_SEAL_ACCOUNT_DISCRIMINATOR)) {
    return null;
  }

  return {
    loomAsset: new PublicKey(data.subarray(40, 72)).toBase58(),
    sessionHash: data.subarray(72, 104).toString("hex"),
    timestamp: Number(data.readBigInt64LE(112)),
    utcDay: Number(data.readBigInt64LE(104)),
    writer: new PublicKey(data.subarray(8, 40)).toBase58(),
  };
}

async function postBackendSeal({
  backendUrl,
  coreCollection,
  loomAsset,
  sessionHash,
  signature,
  status,
  utcDay,
  wallet,
}) {
  const response = await fetch(`${normalizeBackendUrl(backendUrl)}/api/mobile/seals/record`, {
    body: JSON.stringify({
      coreCollection,
      loomAsset,
      sessionHash,
      signature,
      status,
      utcDay,
      wallet,
    }),
    headers: {
      "content-type": "application/json",
    },
    method: "POST",
  });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`Backend seal metadata post failed with HTTP ${response.status}: ${body}`);
  }

  return {
    body: parseJsonBody(body),
    ok: true,
    status: response.status,
  };
}

function parseJsonBody(body) {
  try {
    return JSON.parse(body);
  } catch (_error) {
    return body;
  }
}

function normalizeBackendUrl(value) {
  let url;
  try {
    url = new URL(value);
  } catch (_error) {
    throw new Error("--backend-url must be a valid absolute URL.");
  }
  if (url.username !== "" || url.password !== "") {
    throw new Error("--backend-url must not contain credentials.");
  }
  if (url.search !== "" || url.hash !== "") {
    throw new Error("--backend-url must not contain query strings or fragments.");
  }

  return url.toString().replace(/\/+$/, "");
}

function parseCoreAssetBase(data) {
  if (!Buffer.isBuffer(data) || data.length < 1 + 32 + 1) {
    return null;
  }
  if (data[0] !== CORE_ASSET_V1_KEY) {
    return null;
  }

  const owner = new PublicKey(data.subarray(1, 33)).toBase58();
  const updateAuthorityKind = data[33];
  const collection =
    updateAuthorityKind === CORE_UPDATE_AUTHORITY_COLLECTION && data.length >= 66
      ? new PublicKey(data.subarray(34, 66)).toBase58()
      : null;

  return {
    collection,
    owner,
    updateAuthorityKind,
  };
}

function buildSealAnkyInstruction({
  coreCollection,
  dailySeal,
  hashSeal,
  loomAsset,
  loomState,
  programId,
  sessionHash,
  utcDay,
  writer,
}) {
  return new TransactionInstruction({
    data: buildSealAnkyInstructionData(sessionHash, utcDay),
    keys: [
      { pubkey: writer, isSigner: true, isWritable: true },
      { pubkey: loomAsset, isSigner: false, isWritable: false },
      { pubkey: coreCollection, isSigner: false, isWritable: false },
      { pubkey: loomState, isSigner: false, isWritable: true },
      { pubkey: dailySeal, isSigner: false, isWritable: true },
      { pubkey: hashSeal, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId,
  });
}

function buildSealAnkyInstructionData(sessionHash, utcDay) {
  return Buffer.concat([
    discriminator("global:seal_anky"),
    Buffer.from(sessionHash, "hex"),
    encodeI64Le(utcDay),
  ]);
}

function discriminator(value) {
  return crypto.createHash("sha256").update(value).digest().subarray(0, 8);
}

function encodeI64Le(value) {
  const buffer = Buffer.alloc(8);
  buffer.writeBigInt64LE(BigInt(value));
  return buffer;
}

function normalizeHash(value, label) {
  if (typeof value !== "string" || !/^[0-9a-fA-F]{64}$/.test(value.trim())) {
    throw new Error(`${label} must be exactly 64 hex characters.`);
  }

  return value.trim().toLowerCase();
}

function validateSignature(value) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error("signature is required.");
  }
  const signature = value.trim();
  try {
    if (decodeBase58(signature).length !== 64) {
      throw new Error("wrong length");
    }
  } catch (_error) {
    throw new Error("signature must be a base58 Solana transaction signature.");
  }

  return signature;
}

function decodeBase58(value) {
  const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
  let decoded = 0n;
  for (const character of value) {
    const digit = alphabet.indexOf(character);
    if (digit < 0) {
      throw new Error("invalid base58");
    }
    decoded = decoded * 58n + BigInt(digit);
  }

  let hex = decoded.toString(16);
  if (hex.length % 2 === 1) {
    hex = `0${hex}`;
  }
  const bytes = decoded === 0n ? [] : [...Buffer.from(hex, "hex")];
  for (const character of value) {
    if (character === "1") {
      bytes.unshift(0);
    } else {
      break;
    }
  }

  return Buffer.from(bytes);
}

function toSafeInteger(value, label) {
  const number = Number(value);
  if (!Number.isSafeInteger(number)) {
    throw new Error(`${label} must be a safe integer.`);
  }

  return number;
}

function getCurrentUtcDay(nowMs = Date.now()) {
  return Math.floor(nowMs / MS_PER_UTC_DAY);
}

function readPublicKey(value, label) {
  try {
    return new PublicKey(value);
  } catch (_error) {
    throw new Error(`${label} must be a base58 Solana public key.`);
  }
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

function resolveRpcUrl(cluster, cliRpcUrl) {
  if (typeof cliRpcUrl === "string" && cliRpcUrl.trim() !== "") {
    return cliRpcUrl.trim();
  }
  if (process.env.ANKY_SOLANA_RPC_URL != null && process.env.ANKY_SOLANA_RPC_URL.trim() !== "") {
    return process.env.ANKY_SOLANA_RPC_URL.trim();
  }

  return cluster === "devnet" ? DEFAULT_DEVNET_RPC_URL : null;
}

function redactRpcUrl(value) {
  return value.replace(/([?&]api-key=)[^&]+/i, "$1<redacted>");
}

function loadSealerKeypair(pathArg) {
  const keypairPath = pathArg ?? process.env.ANKY_SEALER_KEYPAIR_PATH ?? process.env.ANCHOR_WALLET;
  if (keypairPath == null || keypairPath.trim() === "") {
    throw new Error("ANKY_SEALER_KEYPAIR_PATH, ANCHOR_WALLET, or --keypair is required with --send.");
  }

  const parsed = JSON.parse(fs.readFileSync(keypairPath, "utf8"));
  if (!Array.isArray(parsed) || parsed.length !== 64) {
    throw new Error("Sealer keypair file must contain a Solana keypair byte array.");
  }

  return Keypair.fromSecretKey(Uint8Array.from(parsed));
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
    if (arg === "--check-chain") {
      args.checkChain = true;
      continue;
    }
    if (arg === "--check-sealed-chain") {
      args.checkSealedChain = true;
      continue;
    }
    if (arg === "--send") {
      args.send = true;
      continue;
    }

    const value = argv[index + 1];
    if (value == null || value.startsWith("--")) {
      throw new Error(`${arg} requires a value.`);
    }
    const key = arg.slice(2).replace(/-([a-z])/g, (_match, letter) => letter.toUpperCase());
    args[key] = value;
    index += 1;
  }

  return args;
}

function requiredArg(args, name) {
  const value = args[name];
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`--${name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} is required.`);
  }

  return value.trim();
}

function firstNonempty(...values) {
  for (const value of values) {
    if (typeof value === "string" && value.trim() !== "") {
      return value.trim();
    }
  }

  return null;
}

function printUsage() {
  console.log(`Usage:
  npm run seal -- \\
    --writer <writer_wallet> \\
    --loom-asset <core_asset_v1_loom> \\
    --session-hash <sha256_hex> \\
    --utc-day <current_utc_day> \\
    --cluster devnet \\
    --check-chain

  ANKY_SEALER_KEYPAIR_PATH=<writer_keypair_path> npm run seal -- \\
    --loom-asset <core_asset_v1_loom> \\
    --session-hash <sha256_hex> \\
    --utc-day <current_utc_day> \\
  --cluster devnet \\
  --check-chain \\
  --send

  npm run seal -- \\
    --writer <writer_wallet> \\
    --loom-asset <core_asset_v1_loom> \\
    --session-hash <sha256_hex> \\
    --utc-day <current_utc_day> \\
    --backend-url <backend_url> \\
    --backend-signature <landed_seal_signature> \\
    --check-sealed-chain

Options:
  --writer <pubkey>           Writer wallet; required unless --send loads a sealer keypair.
  --loom-asset <pubkey>       Metaplex Core Loom asset owned by writer.
  --session-hash <hex>        SHA-256 over exact .anky UTF-8 bytes.
  --utc-day <number>          UTC day derived from .anky started_at_ms.
  --cluster <cluster>         devnet only for this helper. Defaults to devnet.
  --check-chain               Check public Core/DailySeal/HashSeal state before send.
  --check-sealed-chain        Check an already-landed HashSeal before backend metadata post.
  --send                      Send seal_anky on devnet. Requires sealer keypair.
  --keypair <path>            Writer/sealer keypair path for --send.
  --backend-url <url>         Post public seal metadata after --send or --backend-signature.
  --backend-signature <sig>   Landed seal tx signature for chain-checked backend metadata post.
  --rpc-url <url>             Override ANKY_SOLANA_RPC_URL.
  --program-id <pubkey>       Defaults to ANKY_SEAL_PROGRAM_ID or Sojourn 9 devnet program.
  --core-collection <pubkey>  Defaults to ANKY_CORE_COLLECTION or Sojourn 9 devnet collection.
  --core-program-id <pubkey>  Defaults to ANKY_CORE_PROGRAM_ID or Metaplex Core.

This helper never reads .anky plaintext. It needs only public hash/day/Loom values.
It refuses mainnet; do the full devnet E2E before any separate mainnet checklist.`);
}
