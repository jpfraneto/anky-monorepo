#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { redactSecretValues } from "./redactSecrets.mjs";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "../../..");
const MAKE_DEMO_SCRIPT = path.join(SCRIPT_DIR, "makeDemoAnky.mjs");
const PROVE_RECORD_SCRIPT = path.join(SCRIPT_DIR, "proveAndRecordVerified.mjs");
const RECORD_VERIFIED_SCRIPT = path.join(
  REPO_ROOT,
  "solana",
  "anky-seal-program",
  "scripts",
  "recordVerifiedAnky.mjs",
);
const DEFAULT_CLUSTER = "devnet";
const DEFAULT_WRITER = "9HuaaPXSfYvf2qK9r7jwtVmsJU97KX3f827sgpxgiiEp";
const DEFAULT_PROTOC = "/home/kithkui/.local/protoc-34.1/bin/protoc";
const SECONDS_PER_DAY = 86_400;
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BOOLEAN_FLAGS = new Set(["--force"]);
const VALUE_FLAGS = new Set([
  "--backend-url",
  "--character",
  "--cluster",
  "--loom-asset",
  "--out-dir",
  "--program-id",
  "--started-at-ms",
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

  const cluster = normalizeCluster(args.cluster ?? DEFAULT_CLUSTER);
  const writer = requiredPublicKey(args.writer ?? DEFAULT_WRITER, "writer");
  const loomAsset =
    typeof args.loomAsset === "string" ? requiredPublicKey(args.loomAsset, "loom asset") : null;
  const backendUrl = optionalUrl(args.backendUrl, "backend URL", { allowHttpLocalhost: true });
  const env = defaultEnv();
  const outputDir =
    typeof args.outDir === "string"
      ? path.resolve(args.outDir)
      : fs.mkdtempSync(path.join(os.tmpdir(), "anky-sojourn9-current-"));

  if (isInsideRepo(outputDir)) {
    throw new Error("Refusing to write demo witness or SP1 artifacts inside this git worktree.");
  }
  fs.mkdirSync(outputDir, { recursive: true });

  const witnessPath = path.join(outputDir, "demo.anky");
  const metadataPath = path.join(outputDir, "metadata.json");
  const sp1Dir = path.join(outputDir, "sp1");
  const verifiedProofDir = path.join(outputDir, "verified-proof");
  const manifestPath = path.join(outputDir, "handoff-manifest.json");

  const demoArgs = [MAKE_DEMO_SCRIPT, "--out", witnessPath];
  pushOptional(demoArgs, "--started-at-ms", args.startedAtMs);
  pushOptional(demoArgs, "--character", args.character);
  if (args.force === true) {
    demoArgs.push("--force");
  }
  const demo = await run(process.execPath, demoArgs, { cwd: REPO_ROOT, env });
  const metadata = parseSingleJson(demo.stdout, "demo witness metadata");
  fs.writeFileSync(metadataPath, `${JSON.stringify(metadata, null, 2)}\n`);

  const utcDayStatus = buildUtcDayStatus(metadata.utcDay, Date.now());
  const currentUtcDay = utcDayStatus.currentUtcDay;
  if (metadata.utcDay !== currentUtcDay) {
    throw new Error(
      `generated witness UTC day ${metadata.utcDay} is not current UTC day ${currentUtcDay}; rerun after clock/date check.`,
    );
  }

  const prove = await run(
    process.execPath,
    [
      PROVE_RECORD_SCRIPT,
      "--file",
      witnessPath,
      "--writer",
      writer,
      "--expected-hash",
      metadata.sessionHash,
      "--utc-day",
      String(metadata.utcDay),
      "--cluster",
      cluster,
      "--out-dir",
      sp1Dir,
    ],
    { cwd: REPO_ROOT, env },
  );
  const proveSummary = findJsonObject(prove.stdout, (value) => value?.hashSeal != null);

  const proofPath = path.join(sp1Dir, "proof-with-public-values.bin");
  const receiptPath = path.join(sp1Dir, "receipt.json");
  const verify = await run(
    process.execPath,
    [
      PROVE_RECORD_SCRIPT,
      "--proof",
      proofPath,
      "--writer",
      writer,
      "--cluster",
      cluster,
      "--out-dir",
      verifiedProofDir,
    ],
    { cwd: REPO_ROOT, env },
  );
  const verifySummary = findJsonObject(verify.stdout, (value) => value?.hashSeal != null);
  const verifiedReceiptPath = path.join(verifiedProofDir, "verified-receipt.json");

  const hashSealCheck = await run(
    process.execPath,
    [
      RECORD_VERIFIED_SCRIPT,
      "--writer",
      writer,
      "--session-hash",
      metadata.sessionHash,
      "--utc-day",
      String(metadata.utcDay),
      "--cluster",
      cluster,
      "--check-hashseal-only",
      ...(typeof args.programId === "string" ? ["--program-id", args.programId] : []),
    ],
    { allowFailure: true, cwd: REPO_ROOT, env },
  );

  const manifest = {
    generatedAt: new Date().toISOString(),
    cluster,
    currentUtcDay,
    utcDayStatus,
    outputDir,
    files: {
      handoffManifest: manifestPath,
      metadata: metadataPath,
      proof: proofPath,
      receipt: receiptPath,
      verifiedReceipt: verifiedReceiptPath,
      witness: witnessPath,
    },
    publicReceipt: {
      acceptedDurationMs: metadata.acceptedDurationMs,
      eventCount: metadata.eventCount,
      proofHash: proveSummary.proofHash,
      riteDurationMs: metadata.riteDurationMs,
      sessionHash: metadata.sessionHash,
      utcDay: metadata.utcDay,
      valid: true,
      writer,
    },
    programId: proveSummary.programId,
    publicInputs: {
      backendUrl,
      loomAsset,
    },
    proofVerified: verifySummary.proofHash === proveSummary.proofHash,
    hashSeal: {
      exists: hashSealCheck.code === 0,
      pda: proveSummary.hashSeal,
      preflightExitCode: hashSealCheck.code,
      preflightMessage: stripAnsi(hashSealCheck.stderr || hashSealCheck.stdout).trim(),
    },
    verifiedSeal: {
      pda: proveSummary.verifiedSeal,
      verifier: proveSummary.verifier,
    },
    nextHumanCommand: hashSealCheck.code === 0
      ? verifiedSealSendCommand({
          cluster,
          programId: proveSummary.programId,
          proofPath,
          writer,
        })
      : sealSendCommand({
          cluster,
          loomAsset,
          programId: proveSummary.programId,
          sessionHash: metadata.sessionHash,
          utcDay: metadata.utcDay,
        }),
    backendFollowupCommands: backendFollowupCommands({
      backendUrl,
      cluster,
      hashSealExists: hashSealCheck.code === 0,
      loomAsset,
      programId: proveSummary.programId,
      receiptPath: verifiedReceiptPath,
      sessionHash: metadata.sessionHash,
      utcDay: metadata.utcDay,
      writer,
    }),
    stopRules: [
      "Do not commit or upload the witness file.",
      "Do not print keypair JSON, private keys, backend secrets, or Helius API keys.",
      "Do not run mainnet commands from this handoff.",
      "Regenerate this handoff after UTC midnight; seal_anky only accepts the current UTC day.",
    ],
  };

  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(JSON.stringify(manifest, null, 2));
}

function buildUtcDayStatus(receiptUtcDay, nowMs) {
  const currentUtcDay = Math.floor(Math.floor(nowMs / 1000) / SECONDS_PER_DAY);
  const nextRolloverMs = (currentUtcDay + 1) * SECONDS_PER_DAY * 1000;
  const secondsUntilRollover = Math.max(0, Math.floor((nextRolloverMs - nowMs) / 1000));
  const isCurrentDay = receiptUtcDay === currentUtcDay;
  const sealWindow = isCurrentDay ? "open" : receiptUtcDay < currentUtcDay ? "stale" : "future";
  return {
    currentUtcDay,
    receiptUtcDay,
    isCurrentDay,
    sealWindow,
    secondsUntilRollover,
    dayRolloverAt: new Date(nextRolloverMs).toISOString(),
  };
}

function sealSendCommand({ cluster, loomAsset, programId, sessionHash, utcDay }) {
  return [
    "cd solana/anky-seal-program &&",
    envAssign("ANKY_SEALER_KEYPAIR_PATH", "<writer_keypair_path>"),
    "npm run seal --",
    flag("--loom-asset", loomAsset ?? "<core_asset_v1_loom>"),
    flag("--session-hash", sessionHash),
    flag("--utc-day", utcDay),
    flag("--cluster", cluster),
    ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
    "--check-chain",
    "--send",
  ].join(" ");
}

function verifiedSealSendCommand({ cluster, programId, proofPath, writer }) {
  return [
    "cd solana/anky-seal-program &&",
    envAssign("ANKY_VERIFIER_KEYPAIR_PATH", "<verifier_authority_keypair_path>"),
    "npm run sojourn9:prove-record --",
    flag("--proof", proofPath),
    flag("--writer", writer),
    flag("--cluster", cluster),
    ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
    "--check-chain",
    "--send",
  ].join(" ");
}

function backendFollowupCommands({
  backendUrl,
  cluster,
  hashSealExists,
  loomAsset,
  programId,
  receiptPath,
  sessionHash,
  utcDay,
  writer,
}) {
  if (backendUrl == null) {
    return [];
  }
  if (hashSealExists) {
    return [
      {
        id: "verifiedseal-backend-post-after-landing",
        command: [
          "cd solana/anky-seal-program &&",
          envAssign("ANKY_INDEXER_WRITE_SECRET", "<backend_write_secret>"),
          "npm run record-verified --",
          flag("--receipt", receiptPath),
          flag("--writer", writer),
          flag("--cluster", cluster),
          ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
          "--check-verified-chain",
          flag("--backend-signature", "<landed_verified_signature>"),
          flag("--backend-url", backendUrl),
        ].join(" "),
        note: "Run only after record_verified_anky lands, the VerifiedSeal PDA is readable, and the backend is reachable.",
      },
    ];
  }
  return [
    {
      id: "seal-backend-post-after-landing",
      command: [
        "cd solana/anky-seal-program && npm run seal --",
        flag("--writer", writer),
        flag("--loom-asset", loomAsset ?? "<core_asset_v1_loom>"),
        flag("--session-hash", sessionHash),
        flag("--utc-day", utcDay),
        flag("--cluster", cluster),
        ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
        "--check-sealed-chain",
        flag("--backend-signature", "<landed_seal_signature>"),
        flag("--backend-url", backendUrl),
      ].join(" "),
      note: "Run only after seal_anky lands, the HashSeal PDA is readable, and the backend is reachable.",
    },
  ];
}

function envAssign(name, value) {
  return `${name}=${shQuote(String(value))}`;
}

function flag(name, value) {
  return `${name} ${shQuote(String(value))}`;
}

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (BOOLEAN_FLAGS.has(arg)) {
      args[toCamel(arg)] = true;
      continue;
    }
    if (VALUE_FLAGS.has(arg)) {
      const value = argv[i + 1];
      if (value == null || value.startsWith("--")) {
        throw new Error(`${arg} requires a value.`);
      }
      args[toCamel(arg)] = value;
      i += 1;
      continue;
    }
    throw new Error(`Unknown option: ${arg}`);
  }
  return args;
}

function normalizeCluster(value) {
  if (value === "devnet") {
    return value;
  }
  if (value === "mainnet-beta") {
    throw new Error("Current-day proof preparation is devnet-only. Stop before mainnet.");
  }
  throw new Error("--cluster must be devnet.");
}

function optionalUrl(value, label, { allowHttpLocalhost }) {
  if (value == null || value.trim().length === 0) {
    return null;
  }
  let parsed;
  try {
    parsed = new URL(value);
  } catch (_error) {
    throw new Error(`${label} must be a valid URL.`);
  }
  const localhost = parsed.hostname === "localhost" || parsed.hostname === "127.0.0.1";
  if (parsed.username || parsed.password) {
    throw new Error(`${label} must not contain credentials.`);
  }
  if (parsed.protocol !== "https:" && !(allowHttpLocalhost && parsed.protocol === "http:" && localhost)) {
    throw new Error(`${label} must use HTTPS${allowHttpLocalhost ? " unless it is localhost" : ""}.`);
  }
  return parsed.toString().replace(/\/$/, "");
}

function requiredPublicKey(value, label) {
  const decoded = decodeBase58(value);
  if (decoded.length !== 32) {
    throw new Error(`${label} must be a 32-byte Solana public key.`);
  }
  return value;
}

function decodeBase58(value) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error("public keys must be base58 encoded.");
  }
  let bytes = [0];
  for (const char of value) {
    const carryStart = BASE58_ALPHABET.indexOf(char);
    if (carryStart < 0) {
      throw new Error("public keys must be base58 encoded.");
    }
    let carry = carryStart;
    for (let i = 0; i < bytes.length; i += 1) {
      const next = bytes[i] * 58 + carry;
      bytes[i] = next & 0xff;
      carry = next >> 8;
    }
    while (carry > 0) {
      bytes.push(carry & 0xff);
      carry >>= 8;
    }
  }
  for (const char of value) {
    if (char !== "1") {
      break;
    }
    bytes.push(0);
  }
  return Uint8Array.from(bytes.reverse());
}

function run(command, args, options) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      env: options.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0 || options.allowFailure === true) {
        resolve({ code: code ?? 1, signal, stdout, stderr });
      } else {
        reject(new Error(`${command} ${args.join(" ")} failed with ${code ?? signal}\n${stderr}`));
      }
    });
  });
}

function defaultEnv() {
  const env = { ...process.env };
  if (typeof env.PROTOC !== "string" && fs.existsSync(DEFAULT_PROTOC)) {
    env.PROTOC = DEFAULT_PROTOC;
  }
  return env;
}

function parseSingleJson(source, label) {
  try {
    return JSON.parse(source);
  } catch (error) {
    throw new Error(`Could not parse ${label}: ${error.message}`);
  }
}

function findJsonObject(source, predicate) {
  const objects = extractJsonObjects(source);
  const found = objects.find(predicate);
  if (found == null) {
    throw new Error("Could not find expected public summary JSON in command output.");
  }
  return found;
}

function extractJsonObjects(source) {
  const objects = [];
  let depth = 0;
  let start = -1;
  let inString = false;
  let escaping = false;

  for (let i = 0; i < source.length; i += 1) {
    const char = source[i];
    if (inString) {
      if (escaping) {
        escaping = false;
      } else if (char === "\\") {
        escaping = true;
      } else if (char === '"') {
        inString = false;
      }
      continue;
    }
    if (char === '"') {
      inString = true;
      continue;
    }
    if (char === "{") {
      if (depth === 0) {
        start = i;
      }
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0 && start >= 0) {
        try {
          objects.push(JSON.parse(source.slice(start, i + 1)));
        } catch (_error) {
          // Ignore non-JSON brace blocks in tool output.
        }
      }
    }
  }

  return objects;
}

function pushOptional(args, flag, value) {
  if (typeof value === "string" && value.trim().length > 0) {
    args.push(flag, value);
  }
}

function isInsideRepo(candidate) {
  const relative = path.relative(REPO_ROOT, candidate);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function stripAnsi(value) {
  return value.replace(/\u001b\[[0-9;]*m/g, "");
}

function shQuote(value) {
  if (/^[A-Za-z0-9_./:=@%+,-]+$/.test(value)) {
    return value;
  }
  return `'${value.replaceAll("'", "'\\''")}'`;
}

function toCamel(flag) {
  return flag
    .replace(/^--/, "")
    .replace(/-([a-z])/g, (_match, char) => char.toUpperCase());
}

function printUsage() {
  console.log(`Usage:
  node solana/scripts/sojourn9/prepareCurrentDayProof.mjs --writer <wallet> --loom-asset <core_asset>

Generates a same-day demo .anky witness outside the repo, runs SP1 prove,
verifies the saved proof artifact, checks whether the public HashSeal exists,
and writes a no-secret handoff manifest next to the temp artifacts.`);
}
