#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { redactSecretValues } from "./redactSecrets.mjs";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "../../..");
const RECORD_VERIFIED_SCRIPT = path.join(
  REPO_ROOT,
  "solana",
  "anky-seal-program",
  "scripts",
  "recordVerifiedAnky.mjs",
);
const DEFAULT_CLUSTER = "devnet";
const SECONDS_PER_DAY = 86_400;
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BOOLEAN_FLAGS = new Set(["--no-chain"]);
const VALUE_FLAGS = new Set([
  "--backend-url",
  "--cluster",
  "--manifest",
  "--now-ms",
  "--program-id",
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

  const manifestPath = path.resolve(requiredArg(args, "manifest"));
  const manifest = readManifest(manifestPath);
  const publicReceipt = readPublicReceipt(manifest);
  const cluster = normalizeCluster(args.cluster ?? manifest.cluster ?? DEFAULT_CLUSTER);
  const programId = args.programId ?? manifest.programId;
  const backendUrl = optionalUrl(
    args.backendUrl ?? manifest.publicInputs?.backendUrl,
    "backend URL",
    { allowHttpLocalhost: true },
  );
  const currentNowMs = nowMs(args);
  const utcDayStatus = buildUtcDayStatus(publicReceipt.utcDay, currentNowMs);
  const currentUtcDay = utcDayStatus.currentUtcDay;
  const files = summarizeFiles(manifest.files ?? {});

  const chain =
    args.noChain === true
      ? {
          checked: false,
          hashSealReady: null,
          verifiedSealLanded: null,
        }
      : {
          checked: true,
          hashSealReady: await checkHashSeal({
            cluster,
            programId,
            publicReceipt,
          }),
          verifiedSealLanded: await checkVerifiedSeal({
            cluster,
            programId,
            publicReceipt,
            receiptPath: manifest.files?.verifiedReceipt ?? manifest.files?.receipt,
          }),
        };
  normalizePostVerifiedHashSealStatus(chain);

  const backend =
    backendUrl == null
      ? null
      : await queryBackend({
          backendUrl,
          publicReceipt,
        });
  const nextAction = chooseNextAction({
    backend,
    chain,
    currentUtcDay,
    publicReceipt,
  });

  const report = {
    checkedAt: new Date().toISOString(),
    cluster,
    currentUtcDay,
    utcDayStatus,
    manifestPath,
    publicReceipt,
    files,
    chain,
    backend,
    nextAction,
    commands: buildCommands({
      backend,
      backendUrl,
      cluster,
      manifest,
      nextAction,
      programId,
      publicReceipt,
    }),
    stopRules: [
      "This status checker reads only public receipt/manifest metadata and never reads the witness file.",
      "Do not paste keypair JSON, private keys, backend secrets, or Helius API keys into CLI arguments.",
      "Do not run mainnet commands from this handoff.",
      "Regenerate the proof handoff after UTC midnight if the matching HashSeal does not exist.",
    ],
  };

  console.log(JSON.stringify(report, null, 2));
}

function readManifest(manifestPath) {
  if (!fs.existsSync(manifestPath)) {
    throw new Error(`manifest does not exist: ${manifestPath}`);
  }
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  if (typeof manifest !== "object" || manifest == null) {
    throw new Error("manifest must be a JSON object.");
  }
  return manifest;
}

function readPublicReceipt(manifest) {
  const receipt = manifest.publicReceipt;
  if (typeof receipt !== "object" || receipt == null) {
    throw new Error("manifest.publicReceipt is required.");
  }

  const writer = requiredPublicKey(receipt.writer, "publicReceipt.writer");
  const sessionHash = normalizeHash(receipt.sessionHash, "publicReceipt.sessionHash");
  const proofHash = normalizeHash(receipt.proofHash, "publicReceipt.proofHash");
  const utcDay = toSafeInteger(receipt.utcDay, "publicReceipt.utcDay");
  const eventCount = toSafeInteger(receipt.eventCount, "publicReceipt.eventCount");
  const acceptedDurationMs = toSafeInteger(
    receipt.acceptedDurationMs,
    "publicReceipt.acceptedDurationMs",
  );
  const riteDurationMs = toSafeInteger(receipt.riteDurationMs, "publicReceipt.riteDurationMs");

  return {
    acceptedDurationMs,
    eventCount,
    proofHash,
    riteDurationMs,
    sessionHash,
    utcDay,
    valid: receipt.valid === true,
    writer,
  };
}

function summarizeFiles(files) {
  return {
    proofExists: pathExists(files.proof),
    receiptExists: pathExists(files.receipt),
    verifiedReceiptExists: pathExists(files.verifiedReceipt),
    witnessPathPresent: typeof files.witness === "string" && files.witness.length > 0,
    witnessRead: false,
  };
}

async function checkHashSeal({ cluster, programId, publicReceipt }) {
  const args = [
    RECORD_VERIFIED_SCRIPT,
    "--writer",
    publicReceipt.writer,
    "--session-hash",
    publicReceipt.sessionHash,
    "--utc-day",
    String(publicReceipt.utcDay),
    "--cluster",
    cluster,
    "--check-hashseal-only",
  ];
  pushOptional(args, "--program-id", programId);
  const result = await run(process.execPath, args, { cwd: REPO_ROOT, env: process.env });

  return chainResult(result);
}

async function checkVerifiedSeal({ cluster, programId, publicReceipt, receiptPath }) {
  if (typeof receiptPath !== "string" || !fs.existsSync(receiptPath)) {
    return {
      checked: false,
      exitCode: null,
      ok: false,
      reason: "verified receipt file is missing",
    };
  }

  const args = [
    RECORD_VERIFIED_SCRIPT,
    "--receipt",
    receiptPath,
    "--writer",
    publicReceipt.writer,
    "--cluster",
    cluster,
    "--check-verified-chain",
  ];
  pushOptional(args, "--program-id", programId);
  const result = await run(process.execPath, args, { cwd: REPO_ROOT, env: process.env });

  return chainResult(result);
}

function chainResult(result) {
  const output = stripAnsi(`${result.stderr}\n${result.stdout}`).trim();
  const parsed = findJsonObject(output);
  return {
    checked: true,
    exitCode: result.code,
    ok: result.code === 0,
    reason: result.code === 0 ? null : summarizeFailure(output),
    summary: result.code === 0 && parsed != null ? parsed : null,
  };
}

function normalizePostVerifiedHashSealStatus(chain) {
  if (
    chain?.hashSealReady?.ok === false &&
    chain?.verifiedSealLanded?.ok === true &&
    chain.hashSealReady.reason === "HashSeal preflight failed: VerifiedSeal account already exists"
  ) {
    chain.hashSealReady = {
      ...chain.hashSealReady,
      ok: true,
      postVerified: true,
      reason: "VerifiedSeal already landed; HashSeal was confirmed by the verified-chain check.",
      summary: chain.verifiedSealLanded.summary?.verifiedChain ?? chain.hashSealReady.summary,
    };
  }
}

async function queryBackend({ backendUrl, publicReceipt }) {
  const base = backendUrl.replace(/\/+$/, "");
  const sealParams = new URLSearchParams({
    sessionHash: publicReceipt.sessionHash,
    wallet: publicReceipt.writer,
  });
  const scoreParams = new URLSearchParams({ wallet: publicReceipt.writer });
  const sealLookup = await fetchJson(`${base}/api/mobile/seals?${sealParams.toString()}`);
  const score = await fetchJson(`${base}/api/mobile/seals/score?${scoreParams.toString()}`);

  return {
    url: backendUrl,
    sealLookup: summarizeSealLookup(sealLookup, publicReceipt),
    score: summarizeScore(score),
  };
}

async function fetchJson(url) {
  try {
    const response = await fetch(url);
    const text = await response.text();
    let body = null;
    try {
      body = text.length > 0 ? JSON.parse(text) : null;
    } catch {
      body = null;
    }
    return {
      body,
      ok: response.ok,
      status: response.status,
    };
  } catch (error) {
    return {
      body: null,
      ok: false,
      status: null,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

function summarizeSealLookup(result, publicReceipt) {
  const seals = Array.isArray(result.body?.seals) ? result.body.seals : [];
  const matchingSeals = seals.filter((seal) => seal.sessionHash === publicReceipt.sessionHash);
  return {
    error: result.error ?? null,
    matchingCount: matchingSeals.length,
    ok: result.ok,
    status: result.status,
    statuses: matchingSeals.map((seal) => ({
      proofHash: seal.verifiedSeal?.proofHash ?? null,
      proofStatus: seal.proofStatus ?? null,
      sealStatus: seal.status ?? null,
      verifiedStatus: seal.verifiedSeal?.status ?? null,
    })),
  };
}

function summarizeScore(result) {
  return {
    error: result.error ?? null,
    ok: result.ok,
    score: Number.isFinite(result.body?.score) ? result.body.score : null,
    status: result.status,
    totalDays: Number.isFinite(result.body?.uniqueSealDays)
      ? result.body.uniqueSealDays
      : null,
    verifiedDays: Number.isFinite(result.body?.verifiedDays) ? result.body.verifiedDays : null,
  };
}

function buildUtcDayStatus(receiptUtcDay, currentNowMs) {
  const currentUtcDay = Math.floor(Math.floor(currentNowMs / 1000) / SECONDS_PER_DAY);
  const nextRolloverMs = (currentUtcDay + 1) * SECONDS_PER_DAY * 1000;
  const secondsUntilRollover = Math.max(0, Math.floor((nextRolloverMs - currentNowMs) / 1000));
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

function chooseNextAction({ backend, chain, currentUtcDay, publicReceipt }) {
  const isCurrentDay = publicReceipt.utcDay === currentUtcDay;
  if (chain.checked !== true) {
    return isCurrentDay ? "run_chain_status_check" : "regenerate_current_day_proof";
  }
  if (chain.verifiedSealLanded?.ok === true) {
    if (backend == null) {
      return "backfill_or_post_verified_metadata";
    }
    const hasBackendVerified = backend.sealLookup.statuses.some(
      (status) => status.verifiedStatus === "confirmed" || status.verifiedStatus === "finalized",
    );
    return hasBackendVerified ? "complete_devnet_handoff" : "backfill_or_post_verified_metadata";
  }
  if (chain.hashSealReady?.ok === true) {
    return "send_verifiedseal";
  }
  if (isCurrentDay) {
    return "send_hashseal";
  }
  return "blocked_historical_hashseal_missing";
}

function buildCommands({ backend, backendUrl, cluster, manifest, nextAction, programId, publicReceipt }) {
  if (nextAction === "send_hashseal") {
    const includeBackendUrl = backend == null || backendReadyForPosts(backend);
    const command =
      buildHashSealSendCommand({
        backendUrl: includeBackendUrl ? backendUrl : null,
        cluster,
        loomAsset: manifest.publicInputs?.loomAsset,
        programId,
        publicReceipt,
      }) ??
      pinProgramId(
        includeBackendUrl
          ? manifest.nextHumanCommand
          : stripBackendUrl(manifest.nextHumanCommand),
        programId,
      ) ??
      "rerun npm run sojourn9:prepare-proof with --loom-asset";
    const commands = [
      {
        id: includeBackendUrl ? "hashseal-send" : "hashseal-send-chain-only",
        command,
        ...(includeBackendUrl
          ? {}
          : {
              note: "Backend status check failed, so this command only sends the on-chain HashSeal. Post public seal metadata after the backend is reachable.",
            }),
      },
    ];
    if (!includeBackendUrl && backendUrl != null && typeof manifest.publicInputs?.loomAsset === "string") {
      commands.push({
        id: "seal-backend-post-after-landing",
        command: [
          "cd solana/anky-seal-program && npm run seal --",
          flag("--writer", publicReceipt.writer),
          flag("--loom-asset", manifest.publicInputs.loomAsset),
          flag("--session-hash", publicReceipt.sessionHash),
          flag("--utc-day", publicReceipt.utcDay),
          flag("--cluster", cluster),
          ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
          "--check-sealed-chain",
          flag("--backend-signature", "<landed_seal_signature>"),
          flag("--backend-url", backendUrl),
        ].join(" "),
        note: "Run this only after the HashSeal transaction lands and the backend is reachable.",
      });
    }
    return commands;
  }
  if (nextAction === "send_verifiedseal") {
    const proof = manifest.files?.proof ?? "<proof-with-public-values.bin>";
    const receipt = manifest.files?.verifiedReceipt ?? manifest.files?.receipt ?? "<verified-receipt.json>";
    const includeBackendUrl = backend == null || backendReadyForPosts(backend);
    const commands = [
      {
        id: includeBackendUrl ? "verifiedseal-send" : "verifiedseal-send-chain-only",
        command: [
          "cd solana/anky-seal-program &&",
          envAssign("ANKY_VERIFIER_KEYPAIR_PATH", "<verifier_authority_keypair_path>"),
          ...(includeBackendUrl && backendUrl != null
            ? [envAssign("ANKY_INDEXER_WRITE_SECRET", "<backend_write_secret>")]
            : []),
          "npm run sojourn9:prove-record --",
          flag("--proof", proof),
          flag("--writer", publicReceipt.writer),
          flag("--cluster", cluster),
          ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
          "--check-chain",
          ...(includeBackendUrl && backendUrl != null ? [flag("--backend-url", backendUrl)] : []),
          "--send",
        ].join(" "),
        ...(includeBackendUrl
          ? {}
          : {
              note: "Backend status check failed, so this command only sends the on-chain VerifiedSeal. Post public verified metadata after the backend is reachable.",
            }),
      },
    ];
    if (!includeBackendUrl && backendUrl != null) {
      commands.push({
        id: "verifiedseal-backend-post-after-landing",
        command: [
          "cd solana/anky-seal-program &&",
          envAssign("ANKY_INDEXER_WRITE_SECRET", "<backend_write_secret>"),
          "npm run record-verified --",
          flag("--receipt", receipt),
          flag("--writer", publicReceipt.writer),
          flag("--cluster", cluster),
          ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
          "--check-verified-chain",
          flag("--backend-signature", "<landed_verified_signature>"),
          flag("--backend-url", backendUrl),
        ].join(" "),
        note: "Run this only after record_verified_anky lands and the backend is reachable.",
      });
    }
    return commands;
  }
  if (nextAction === "backfill_or_post_verified_metadata") {
    const includeBackendUrl = backend == null || backendReadyForPosts(backend);
    const baseBackfillCommand = [
      "cd solana/anky-seal-program &&",
      envAssign("HELIUS_API_KEY", "<configured_in_shell>"),
      ...(includeBackendUrl && backendUrl != null
        ? [
            envAssign("ANKY_INDEXER_WRITE_SECRET", "<backend_write_secret>"),
            envAssign("ANKY_CORE_COLLECTION", "<core_collection_pubkey>"),
          ]
        : []),
      "npm run sojourn9:index --",
      "--backfill",
      flag("--limit", "100"),
      flag("--cluster", cluster),
      ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
      ...(typeof manifest.verifiedSeal?.verifier === "string"
        ? [flag("--proof-verifier", manifest.verifiedSeal.verifier)]
        : []),
      ...(includeBackendUrl && backendUrl != null ? [flag("--backend-url", backendUrl)] : []),
    ].join(" ");
    const commands = [
      {
        id: includeBackendUrl ? "helius-backfill" : "helius-backfill-snapshot-only",
        command: baseBackfillCommand,
        ...(includeBackendUrl
          ? {}
          : {
              note: "Backend status check failed, so this command only backfills/audits public chain data. Rerun backend posting after the backend is reachable.",
            }),
      },
    ];
    if (!includeBackendUrl && backendUrl != null) {
      commands.push({
        id: "helius-backfill-with-backend-after-reachable",
        command: [
          "cd solana/anky-seal-program &&",
          envAssign("HELIUS_API_KEY", "<configured_in_shell>"),
          envAssign("ANKY_INDEXER_WRITE_SECRET", "<backend_write_secret>"),
          envAssign("ANKY_CORE_COLLECTION", "<core_collection_pubkey>"),
          "npm run sojourn9:index --",
          "--backfill",
          flag("--limit", "100"),
          flag("--cluster", cluster),
          ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
          ...(typeof manifest.verifiedSeal?.verifier === "string"
            ? [flag("--proof-verifier", manifest.verifiedSeal.verifier)]
            : []),
          flag("--backend-url", backendUrl),
        ].join(" "),
        note: "Run this only after the backend is reachable and ANKY_INDEXER_WRITE_SECRET is configured.",
      });
    }
    return commands;
  }
  if (nextAction === "regenerate_current_day_proof") {
    return [
      {
        id: "prepare-current-day-proof",
        command: [
          "cd solana/anky-seal-program && npm run sojourn9:prepare-proof --",
          flag("--writer", publicReceipt.writer),
          ...(typeof manifest.publicInputs?.loomAsset === "string"
            ? [flag("--loom-asset", manifest.publicInputs.loomAsset)]
            : []),
          flag("--cluster", cluster),
          ...(backendUrl == null ? [] : [flag("--backend-url", backendUrl)]),
          ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
        ].join(" "),
      },
    ];
  }
  if (nextAction === "run_chain_status_check") {
    return [
      {
        id: "handoff-status",
        command: "cd solana/anky-seal-program && npm run sojourn9:handoff-status -- --manifest <handoff-manifest.json>",
      },
    ];
  }
  return [];
}

function backendReadyForPosts(backend) {
  return backend?.sealLookup?.ok === true && backend?.score?.ok === true;
}

function buildHashSealSendCommand({ backendUrl, cluster, loomAsset, programId, publicReceipt }) {
  if (typeof loomAsset !== "string" || loomAsset.trim().length === 0) {
    return null;
  }
  return [
    "cd solana/anky-seal-program &&",
    envAssign("ANKY_SEALER_KEYPAIR_PATH", "<writer_keypair_path>"),
    "npm run seal --",
    flag("--loom-asset", loomAsset),
    flag("--session-hash", publicReceipt.sessionHash),
    flag("--utc-day", publicReceipt.utcDay),
    flag("--cluster", cluster),
    ...(typeof programId === "string" ? [flag("--program-id", programId)] : []),
    "--check-chain",
    "--send",
    ...(backendUrl == null ? [] : [flag("--backend-url", backendUrl)]),
  ].join(" ");
}

function pinProgramId(command, programId) {
  if (
    typeof command !== "string" ||
    typeof programId !== "string" ||
    command.includes("--program-id") ||
    !command.includes("npm run seal")
  ) {
    return command;
  }
  return `${command} ${flag("--program-id", programId)}`;
}

function stripBackendUrl(command) {
  if (typeof command !== "string") {
    return command;
  }
  return command.replace(/\s+--backend-url\s+(?:"[^"]*"|'[^']*'|\S+)/, "").trim();
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

function printUsage() {
  console.log(`Checks a Sojourn 9 proof handoff manifest without reading the private witness.

Usage:
  node solana/scripts/sojourn9/checkProofHandoff.mjs --manifest /tmp/anky-sojourn9-current-.../handoff-manifest.json [options]

Options:
  --backend-url <url>   Optional backend URL override for public status reads.
  --cluster <cluster>   devnet only. Defaults to manifest.cluster or devnet.
  --manifest <path>     Required handoff manifest from sojourn9:prepare-proof.
  --no-chain            Validate only local public manifest metadata; skip chain/backend checks.
  --now-ms <ms>         Override current time for tests.
  --program-id <key>    Optional Anky Seal Program override.

This command accepts no keypair paths, backend secrets, Helius API keys, or .anky plaintext.`);
}

function requiredArg(args, name) {
  const value = args[name];
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`--${name} is required.`);
  }
  return value.trim();
}

function requiredPublicKey(value, label) {
  if (typeof value !== "string" || value.trim().length < 32 || value.trim().length > 44) {
    throw new Error(`${label} must be a base58 public key.`);
  }
  for (const char of value.trim()) {
    if (!BASE58_ALPHABET.includes(char)) {
      throw new Error(`${label} must be a base58 public key.`);
    }
  }
  return value.trim();
}

function normalizeHash(value, label) {
  if (typeof value !== "string" || !/^[0-9a-fA-F]{64}$/.test(value.trim())) {
    throw new Error(`${label} must be 64 hex characters.`);
  }
  return value.trim().toLowerCase();
}

function normalizeCluster(value) {
  if (value == null || value === "" || value === "devnet") {
    return "devnet";
  }
  if (value === "mainnet-beta") {
    throw new Error("Proof handoff status is devnet-only until the mainnet launch checklist is complete.");
  }
  throw new Error("--cluster must be devnet.");
}

function optionalUrl(value, label, { allowHttpLocalhost = false } = {}) {
  if (value == null || value === "") {
    return null;
  }
  let parsed;
  try {
    parsed = new URL(String(value));
  } catch {
    throw new Error(`${label} must be a valid URL.`);
  }
  if (parsed.username !== "" || parsed.password !== "") {
    throw new Error(`${label} must not contain credentials.`);
  }
  const isLocalhost = parsed.hostname === "localhost" || parsed.hostname === "127.0.0.1";
  if (parsed.protocol !== "https:" && !(allowHttpLocalhost && isLocalhost && parsed.protocol === "http:")) {
    throw new Error(`${label} must use HTTPS unless it is localhost HTTP.`);
  }
  return parsed.toString().replace(/\/$/, "");
}

function nowMs(args) {
  if (args.nowMs == null) {
    return Date.now();
  }
  const value = Number(args.nowMs);
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error("--now-ms must be a non-negative safe integer.");
  }
  return value;
}

function toSafeInteger(value, label) {
  const parsed = typeof value === "number" ? value : Number(value);
  if (!Number.isSafeInteger(parsed)) {
    throw new Error(`${label} must be a safe integer.`);
  }
  return parsed;
}

function pathExists(value) {
  return typeof value === "string" && fs.existsSync(value);
}

function pushOptional(args, flagName, value) {
  if (typeof value === "string" && value.trim().length > 0) {
    args.push(flagName, value.trim());
  }
}

function run(command, args, options) {
  return new Promise((resolve) => {
    const child = spawn(command, args, {
      ...options,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString("utf8");
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });
    child.on("error", (error) => {
      resolve({
        code: 1,
        stderr: error.message,
        stdout,
      });
    });
    child.on("exit", (code) => {
      resolve({
        code: code ?? 1,
        stderr,
        stdout,
      });
    });
  });
}

function summarizeFailure(output) {
  const lines = output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const last = lines.at(-1);
  return last ?? "chain check failed";
}

function findJsonObject(output) {
  const trimmed = output.trim();
  const first = trimmed.indexOf("{");
  const last = trimmed.lastIndexOf("}");
  if (first < 0 || last <= first) {
    return null;
  }
  try {
    return JSON.parse(trimmed.slice(first, last + 1));
  } catch {
    return null;
  }
}

function stripAnsi(value) {
  return value.replace(/\u001b\[[0-9;]*m/g, "");
}

function envAssign(name, value) {
  return `${name}=${shQuote(String(value))}`;
}

function flag(name, value) {
  return `${name} ${shQuote(String(value))}`;
}

function shQuote(value) {
  if (/^[A-Za-z0-9_./:=@+-]+$/.test(value)) {
    return value;
  }
  return `'${value.replaceAll("'", "'\\''")}'`;
}

function toCamel(flag) {
  return flag
    .replace(/^--/, "")
    .replace(/-([a-z])/g, (_match, char) => char.toUpperCase());
}
