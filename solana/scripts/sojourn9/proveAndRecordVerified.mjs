#!/usr/bin/env node

import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { redactSecretValues } from "./redactSecrets.mjs";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "../../..");
const SP1_SCRIPT_DIR = path.join(REPO_ROOT, "solana", "anky-zk-proof", "sp1", "script");
const RECORD_VERIFIED_SCRIPT = path.join(
  REPO_ROOT,
  "solana",
  "anky-seal-program",
  "scripts",
  "recordVerifiedAnky.mjs",
);
const BOOLEAN_FLAGS = new Set([
  "--check-chain",
  "--check-chain-first",
  "--check-verified-chain",
  "--send",
  "--sp1-proof-verified",
]);
const VALUE_FLAGS = new Set([
  "--backend-signature",
  "--backend-url",
  "--cluster",
  "--expected-hash",
  "--file",
  "--keypair",
  "--out-dir",
  "--program-id",
  "--proof",
  "--receipt",
  "--session-hash",
  "--sp1-mode",
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

  const writer = requiredArg(args, "writer");
  const cluster = args.cluster ?? process.env.ANKY_SOLANA_CLUSTER ?? "devnet";
  const send = args.send === true;
  const checkChain = args.checkChain === true || send;
  const checkChainFirst = args.checkChainFirst === true;
  const backendUrl = firstNonempty(args.backendUrl, process.env.ANKY_VERIFIED_SEAL_BACKEND_URL);
  const backendSignature = firstNonempty(args.backendSignature);
  const willPostBackend = typeof backendUrl === "string" && (send || typeof backendSignature === "string");
  const requestedSp1Mode = args.sp1Mode ?? "prove";
  const hasFile = typeof args.file === "string";
  const hasProof = typeof args.proof === "string";
  const hasReceipt = typeof args.receipt === "string";
  if ([hasFile, hasProof, hasReceipt].filter(Boolean).length !== 1) {
    throw new Error("Provide exactly one of --file, --proof, or --receipt.");
  }
  if (hasProof && typeof args.sp1Mode === "string") {
    throw new Error("--sp1-mode is only valid with --file.");
  }
  if (send && hasReceipt) {
    throw new Error(
      "--send is not allowed with raw --receipt in this wrapper. Use --file so SP1 prove runs now, or --proof so the saved SP1 proof is verified locally before the chain write.",
    );
  }
  if (hasFile && requestedSp1Mode !== "prove" && send) {
    throw new Error("--send requires --sp1-mode prove so the chain write follows local SP1 proof verification.");
  }
  if (hasFile && requestedSp1Mode !== "prove" && args.sp1ProofVerified === true) {
    throw new Error("--sp1-proof-verified cannot be combined with --sp1-mode execute.");
  }
  if (willPostBackend) {
    requireBackendWriteSecret();
  }
  if (
    !send &&
    typeof backendUrl === "string" &&
    typeof backendSignature === "string" &&
    args.checkVerifiedChain !== true
  ) {
    throw new Error(
      "--check-verified-chain is required before posting already-landed VerifiedSeal metadata to the backend.",
    );
  }
  if (checkChainFirst && !hasFile) {
    throw new Error("--check-chain-first is only valid with --file. Use --check-chain with --proof or --receipt.");
  }
  if (checkChainFirst && typeof args.receipt !== "string") {
    await runHashSealPreflight(args, writer, cluster);
  }
  const receiptPath = await resolveReceiptPath(args, writer);

  const recordArgs = [
    RECORD_VERIFIED_SCRIPT,
    "--receipt",
    receiptPath,
    "--writer",
    writer,
    "--cluster",
    cluster,
  ];

  pushOptional(recordArgs, "--session-hash", args.sessionHash);
  pushOptional(recordArgs, "--utc-day", args.utcDay);
  pushOptional(recordArgs, "--program-id", args.programId);
  if (willPostBackend) {
    pushOptional(recordArgs, "--backend-url", backendUrl);
  }
  pushOptional(recordArgs, "--backend-signature", backendSignature);
  pushOptional(recordArgs, "--status", args.status);
  pushOptional(recordArgs, "--keypair", args.keypair);

  if (checkChain) {
    recordArgs.push("--check-chain");
  }
  if (args.checkVerifiedChain === true) {
    recordArgs.push("--check-verified-chain");
  }
  if (send) {
    recordArgs.push("--send");
  }
  if ((hasFile && requestedSp1Mode === "prove") || hasProof || args.sp1ProofVerified === true) {
    recordArgs.push("--sp1-proof-verified");
  }

  await run(process.execPath, recordArgs, {
    cwd: REPO_ROOT,
    env: process.env,
  });
}

async function runHashSealPreflight(args, writer, cluster) {
  const expectedHash = requiredArg(args, "expectedHash");
  const utcDay = requiredArg(args, "utcDay");
  const preflightArgs = [
    RECORD_VERIFIED_SCRIPT,
    "--check-hashseal-only",
    "--writer",
    writer,
    "--session-hash",
    expectedHash,
    "--utc-day",
    utcDay,
    "--cluster",
    cluster,
  ];

  pushOptional(preflightArgs, "--program-id", args.programId);

  await run(process.execPath, preflightArgs, {
    cwd: REPO_ROOT,
    env: process.env,
  });
}

async function resolveReceiptPath(args, writer) {
  if (typeof args.receipt === "string") {
    return path.resolve(args.receipt);
  }

  if (typeof args.proof === "string") {
    const outputDir = resolveOutputDir(args);
    const receiptOut = path.join(outputDir, "verified-receipt.json");
    await run(
      "cargo",
      [
        "run",
        "--release",
        "--",
        "--verify",
        "--proof",
        path.resolve(args.proof),
        "--receipt-out",
        receiptOut,
      ],
      {
        cwd: SP1_SCRIPT_DIR,
        env: process.env,
      },
    );

    return receiptOut;
  }

  const file = requiredArg(args, "file");
  const expectedHash = requiredArg(args, "expectedHash");
  const sp1Mode = args.sp1Mode ?? "prove";
  if (sp1Mode !== "prove" && sp1Mode !== "execute") {
    throw new Error("--sp1-mode must be prove or execute.");
  }

  const outputDir = resolveOutputDir(args);
  const receiptOut = path.join(outputDir, "receipt.json");
  const proofOut = path.join(outputDir, "proof-with-public-values.bin");
  const sp1Args = [
    "run",
    "--release",
    "--",
    `--${sp1Mode}`,
    "--file",
    path.resolve(file),
    "--writer",
    writer,
    "--expected-hash",
    expectedHash,
    "--receipt-out",
    receiptOut,
  ];

  if (sp1Mode === "prove") {
    sp1Args.push("--proof-out", proofOut);
  }

  await run("cargo", sp1Args, {
    cwd: SP1_SCRIPT_DIR,
    env: process.env,
  });

  return receiptOut;
}

function resolveOutputDir(args) {
  const outputDir =
    typeof args.outDir === "string"
      ? path.resolve(args.outDir)
      : fs.mkdtempSync(path.join(os.tmpdir(), "anky-sp1-verified-"));
  if (isInsideRepo(outputDir)) {
    throw new Error(
      "Refusing to write SP1 receipt/proof artifacts inside this git worktree. Use a temp path such as /tmp/anky-sp1-verified.",
    );
  }
  fs.mkdirSync(outputDir, { recursive: true });

  return outputDir;
}

function run(command, args, options) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      ...options,
      stdio: "inherit",
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} exited with ${signal ?? code}`));
      }
    });
  });
}

function pushOptional(argv, flag, value) {
  if (typeof value === "string" && value.trim().length > 0) {
    argv.push(flag, value);
  }
}

function requiredArg(args, name) {
  const value = args[name];
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`--${name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} is required.`);
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

function requireBackendWriteSecret() {
  const indexerSecret = process.env.ANKY_INDEXER_WRITE_SECRET?.trim() ?? "";
  const verifiedSealRecordSecret = process.env.ANKY_VERIFIED_SEAL_RECORD_SECRET?.trim() ?? "";
  if (indexerSecret.length === 0 && verifiedSealRecordSecret.length === 0) {
    throw new Error(
      "ANKY_INDEXER_WRITE_SECRET or ANKY_VERIFIED_SEAL_RECORD_SECRET is required for backend verified metadata posts.",
    );
  }
}

function isInsideRepo(candidatePath) {
  const relative = path.relative(REPO_ROOT, candidatePath);

  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
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
    if (arg === "--check-verified-chain") {
      args.checkVerifiedChain = true;
      continue;
    }
    if (arg === "--check-chain-first") {
      args.checkChainFirst = true;
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
  node solana/scripts/sojourn9/proveAndRecordVerified.mjs \\
    --file solana/anky-zk-proof/fixtures/full.anky \\
    --writer <writer_wallet> \\
    --expected-hash <sealed_session_hash> \\
    --check-chain

  node solana/scripts/sojourn9/proveAndRecordVerified.mjs \\
    --receipt /tmp/receipt.json \\
    --writer <writer_wallet> \\
    --check-verified-chain \\
    --backend-url <backend_url> \\
    --backend-signature <record_verified_anky_signature>

Options:
  --file <path>             Private .anky witness path for SP1. Not read by this wrapper.
  --expected-hash <hex>     Required with --file; passed into SP1 witness validation.
  --proof <path>            Existing SP1 proof-with-public-values file to verify locally.
  --receipt <path>          Existing public receipt JSON; skips SP1.
  --writer <pubkey>         Writer wallet/public identity.
  --sp1-mode <mode>         prove or execute. Defaults to prove.
  --out-dir <path>          Directory for public receipt/proof outputs. Defaults to /tmp.
  --cluster <cluster>       devnet or mainnet-beta. Defaults to ANKY_SOLANA_CLUSTER or devnet.
  --check-chain-first       Check public HashSeal before running SP1. Requires --utc-day.
  --check-chain             Confirm matching HashSeal exists before record_verified_anky.
  --check-verified-chain    Confirm matching VerifiedSeal exists before backend metadata post.
  --sp1-proof-verified      Existing-receipt operator attestation; --file mode passes this only after prove.
  --send                    Submit record_verified_anky through the lower-level operator.
  --backend-url <url>       POST verified metadata after send or with --backend-signature.
  --backend-signature <s>   Record an already-landed record_verified_anky signature in backend.
  --status <status>         Backend proof status. Defaults to confirmed.

This wrapper never reads or logs .anky plaintext. It passes the file path to SP1 and only hands public receipt metadata to the VerifiedSeal operator.`);
}
