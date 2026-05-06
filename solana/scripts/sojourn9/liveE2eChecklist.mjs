#!/usr/bin/env node

import { redactSecretValues } from "./redactSecrets.mjs";

const DEFAULT_CLUSTER = "devnet";
const DEFAULT_CORE_COLLECTION = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const DEFAULT_PROOF_VERIFIER_AUTHORITY = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const DEFAULT_WITNESS_PATH = "/tmp/anky-sojourn9-demo.anky";
const SECONDS_PER_DAY = 86_400;
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BOOLEAN_FLAGS = new Set([]);
const VALUE_FLAGS = new Set([
  "--backend-url",
  "--cluster",
  "--core-collection",
  "--loom-asset",
  "--now-ms",
  "--proof-verifier",
  "--session-hash",
  "--utc-day",
  "--webhook-url",
  "--witness-path",
  "--writer",
]);

main();

function main() {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help === true) {
      printUsage();
      return;
    }

    const cluster = normalizeCluster(args.cluster ?? DEFAULT_CLUSTER);
    const writer = requiredPublicKey(args.writer, "writer");
    const loomAsset = requiredPublicKey(args.loomAsset, "loom asset");
    const coreCollection = requiredPublicKey(
      args.coreCollection ?? DEFAULT_CORE_COLLECTION,
      "core collection",
    );
    const proofVerifier = requiredPublicKey(
      args.proofVerifier ?? DEFAULT_PROOF_VERIFIER_AUTHORITY,
      "proof verifier authority",
    );
    const sessionHash = requiredHash(args.sessionHash, "session hash");
    const utcDay = requiredUtcDay(args.utcDay);
    const nowMs = args.nowMs == null ? Date.now() : parseSafeInteger(args.nowMs, "now-ms");
    const utcDayStatus = buildUtcDayStatus(utcDay, nowMs);
    const currentUtcDay = utcDayStatus.currentUtcDay;
    const witnessPath = args.witnessPath ?? DEFAULT_WITNESS_PATH;
    const backendUrl = optionalUrl(args.backendUrl, "backend URL", { allowHttpLocalhost: true });
    const webhookUrl = optionalUrl(args.webhookUrl, "webhook URL", { allowHttpLocalhost: false });

    if (cluster !== "devnet") {
      throw new Error("This live E2E checklist is devnet-only. Stop before mainnet.");
    }

    if (utcDay !== currentUtcDay) {
      throw new Error(
        `utc day ${utcDay} is not current UTC day ${currentUtcDay}; create a same-day witness before seal_anky.`,
      );
    }

    const report = buildChecklist({
      backendUrl,
      cluster,
      coreCollection,
      currentUtcDay,
      loomAsset,
      proofVerifier,
      sessionHash,
      utcDayStatus,
      utcDay,
      webhookUrl,
      witnessPath,
      writer,
    });

    console.log(JSON.stringify(report, null, 2));
  } catch (error) {
    console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
    process.exit(1);
  }
}

function buildChecklist({
  backendUrl,
  cluster,
  coreCollection,
  currentUtcDay,
  loomAsset,
  proofVerifier,
  sessionHash,
  utcDayStatus,
  utcDay,
  webhookUrl,
  witnessPath,
  writer,
}) {
  const backendTarget = backendUrl ?? "<backend_url>";
  const commands = [
    {
      id: "readiness",
      description: "Run local no-secret launch checks.",
      command: "cd solana/anky-seal-program && npm run sojourn9:readiness && npm run sojourn9:privacy",
      secrets: [],
    },
    {
      id: "public-devnet-config",
      description: "Verify public devnet program, Core collection, and owned Core Loom base fields.",
      command: joinCommand("cd solana/anky-seal-program && npm run check-config --", [
        "--cluster",
        cluster,
        "--loom-asset",
        loomAsset,
        "--loom-owner",
        writer,
      ]),
      secrets: [],
    },
    {
      id: "hashseal-preflight",
      description: "Check that the same-day HashSeal can be created and does not already exist.",
      command: sealCommand({
        checkChain: true,
        cluster,
        loomAsset,
        sessionHash,
        utcDay,
        writer,
      }),
      secrets: [],
    },
    {
      id: "hashseal-send",
      description: "Send seal_anky on devnet with the writer-owned Loom keypair.",
      command: joinCommand(
        "cd solana/anky-seal-program && ANKY_SEALER_KEYPAIR_PATH=<writer_keypair_path> npm run seal --",
        [
          "--loom-asset",
          loomAsset,
          "--session-hash",
          sessionHash,
          "--utc-day",
          String(utcDay),
          "--cluster",
          cluster,
          "--check-chain",
          "--send",
          ...(backendUrl == null ? [] : ["--backend-url", backendUrl]),
        ],
      ),
      secrets: ["writer keypair path", ...(backendUrl == null ? [] : ["backend URL only, no secret"])],
    },
    {
      id: "sp1-verifiedseal-preflight",
      description: "Run SP1 prove locally and verify matching public HashSeal before any VerifiedSeal send.",
      command: proveRecordCommand({
        backendUrl: null,
        checkChain: true,
        checkChainFirst: true,
        cluster,
        expectedHash: sessionHash,
        send: false,
        utcDay,
        witnessPath,
        writer,
      }),
      secrets: [],
    },
    {
      id: "sp1-verifiedseal-send",
      description: "Run SP1 prove and submit record_verified_anky on devnet through the verifier authority.",
      command: `${
        backendUrl == null
          ? "ANKY_VERIFIER_KEYPAIR_PATH=<verifier_authority_keypair_path>"
          : "ANKY_VERIFIER_KEYPAIR_PATH=<verifier_authority_keypair_path> ANKY_INDEXER_WRITE_SECRET=<backend_write_secret>"
      } ${proveRecordCommand({
          backendUrl,
          checkChain: true,
          checkChainFirst: true,
          cluster,
          expectedHash: sessionHash,
          send: true,
          utcDay,
          witnessPath,
          writer,
        })}`,
      secrets: [
        "verifier authority keypair path",
        ...(backendUrl == null ? [] : ["backend indexer write secret"]),
      ],
    },
    {
      id: "helius-backfill",
      description: "Backfill finalized program transactions through Helius and upsert public metadata.",
      command: `${
        backendUrl == null
          ? "HELIUS_API_KEY=<configured_in_shell> ANKY_SOLANA_CLUSTER=devnet"
          : "HELIUS_API_KEY=<configured_in_shell> ANKY_SOLANA_CLUSTER=devnet ANKY_INDEXER_WRITE_SECRET=<backend_write_secret>"
      } ${joinCommand(
        `ANKY_CORE_COLLECTION=${shQuote(coreCollection)} ANKY_PROOF_VERIFIER_AUTHORITY=${shQuote(proofVerifier)} node solana/scripts/indexer/ankySealIndexer.mjs`,
        [
          "--backfill",
          "--limit",
          "100",
          ...(backendUrl == null ? [] : ["--backend-url", backendUrl]),
          "--out",
          "sojourn9/devnet-score-snapshot.json",
        ],
      )}`,
      secrets: [
        "Helius API key",
        ...(backendUrl == null ? [] : ["backend indexer write secret"]),
      ],
    },
    {
      id: "backend-score-check",
      description: "Confirm backend Score V1 view after finalized metadata is indexed.",
      command: `curl ${shQuote(`${backendTarget}/api/mobile/seals/score?wallet=${encodeURIComponent(writer)}`)}`,
      secrets: backendUrl == null ? ["backend URL"] : [],
    },
  ];

  if (webhookUrl != null) {
    commands.splice(6, 0, {
      id: "helius-webhook-manifest",
      description: "Generate a Helius enhancedDevnet webhook creation manifest without reading API keys.",
      command: joinCommand("node solana/scripts/indexer/heliusWebhookManifest.mjs", [
        "--cluster",
        cluster,
        "--webhook-url",
        webhookUrl,
        "--out",
        "/tmp/anky-helius-webhook.json",
      ]),
      secrets: [],
    });
  }

  return {
    generatedAt: new Date().toISOString(),
    cluster,
    currentUtcDay,
    utcDayStatus,
    launchReadyAfterChecklist: false,
    publicInputs: {
      backendUrl: backendUrl ?? null,
      coreCollection,
      loomAsset,
      proofVerifier,
      sessionHash,
      utcDay,
      webhookUrl: webhookUrl ?? null,
      witnessPath,
      writer,
    },
    commands,
    stopRules: [
      "Do not paste or print keypair JSON, private keys, API keys, or backend write secrets.",
      "Do not run any mainnet command from this checklist.",
      "Do not upload or commit the private .anky witness.",
      "Do not mark launch ready until the live phone flow and finalized Helius score snapshot are verified.",
    ],
    remainingManualGates: [
      "writer keypair must own the supplied Core Loom and have devnet SOL",
      "verifier authority keypair/custody must be approved by the human",
      "backend migrations 019_mobile_verified_seal_receipts, 020_mobile_helius_webhook_events, and 021_mobile_helius_webhook_signature_dedupe must be applied before backend metadata posts; apply 022_credit_ledger_entries as part of the full backend migration chain",
      "launch backend must set ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF=true",
      "Helius API key or Helius RPC URL must be configured outside Codex",
      "mobile app must be run against the resulting devnet backend/indexed state",
    ],
  };
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

function sealCommand({ checkChain, cluster, loomAsset, sessionHash, utcDay, writer }) {
  return [
    "cd solana/anky-seal-program && npm run seal --",
    "--writer",
    writer,
    "--loom-asset",
    loomAsset,
    "--session-hash",
    sessionHash,
    "--utc-day",
    String(utcDay),
    "--cluster",
    cluster,
    ...(checkChain ? ["--check-chain"] : []),
  ].reduce((command, part, index) => {
    if (index === 0) {
      return part;
    }
    return `${command} ${shQuote(part)}`;
  }, "");
}

function proveRecordCommand({
  backendUrl,
  checkChain,
  checkChainFirst,
  cluster,
  expectedHash,
  send,
  utcDay,
  witnessPath,
  writer,
}) {
  return joinCommand("node solana/scripts/sojourn9/proveAndRecordVerified.mjs", [
    "--file",
    witnessPath,
    "--writer",
    writer,
    "--expected-hash",
    expectedHash,
    "--utc-day",
    String(utcDay),
    "--cluster",
    cluster,
    ...(checkChainFirst ? ["--check-chain-first"] : []),
    ...(checkChain ? ["--check-chain"] : []),
    ...(backendUrl == null ? [] : ["--backend-url", backendUrl]),
    ...(send ? ["--send"] : []),
  ]);
}

function joinCommand(prefix, parts) {
  return [prefix, ...parts.map(shQuote)].join(" ");
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

function requiredPublicKey(value, label) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`--${label.replaceAll(" ", "-")} is required.`);
  }
  const decoded = decodeBase58(value.trim());
  if (decoded.length !== 32) {
    throw new Error(`${label} must be a 32-byte Solana public key.`);
  }
  return value.trim();
}

function requiredHash(value, label) {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) {
    throw new Error(`${label} must be a 32-byte lowercase hex string.`);
  }
  return value;
}

function requiredUtcDay(value) {
  if (value == null) {
    throw new Error("--utc-day is required.");
  }
  const parsed = parseSafeInteger(value, "utc-day");
  if (parsed < 0) {
    throw new Error("utc-day must be non-negative.");
  }
  return parsed;
}

function normalizeCluster(value) {
  if (value === "devnet") {
    return value;
  }
  if (value === "mainnet-beta") {
    throw new Error("This live E2E checklist is devnet-only. Stop before mainnet.");
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

function parseSafeInteger(value, label) {
  if (!/^\d+$/.test(String(value))) {
    throw new Error(`${label} must be a non-negative integer.`);
  }
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed)) {
    throw new Error(`${label} is outside JavaScript's safe integer range.`);
  }
  return parsed;
}

function decodeBase58(value) {
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
  node solana/scripts/sojourn9/liveE2eChecklist.mjs \\
    --writer <wallet_pubkey> \\
    --loom-asset <core_asset_v1_loom> \\
    --session-hash <64_hex_hash> \\
    --utc-day <current_utc_day> \\
    --backend-url https://<backend>

This prints a no-secret devnet checklist for:
  check config -> seal_anky -> SP1 prove -> record_verified_anky -> Helius backfill -> score check

It validates public inputs only and prints placeholders for keypair paths, backend secrets, and Helius API keys.`);
}
