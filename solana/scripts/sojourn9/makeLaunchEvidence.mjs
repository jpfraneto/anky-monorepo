#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { redactSecretValues } from "./redactSecrets.mjs";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const AUDIT_SCRIPT = path.join(SCRIPT_DIR, "auditLaunchEvidence.mjs");
const SCORE_AUDIT_SCRIPT = path.resolve(SCRIPT_DIR, "../indexer/auditScoreSnapshot.mjs");
const DEFAULT_MAX_PARTICIPANTS = 3_456;
const DEFAULT_REWARD_BPS = 800;
const SCORE_FORMULA =
  "score = unique_seal_days + verified_days + 2 * floor(each_consecutive_day_run / 7)";
const SECONDS_PER_DAY = 86_400;
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BOOLEAN_FLAGS = new Set([
  "--allow-inferred-finality",
  "--audit",
  "--audit-score-snapshot",
  "--backfill-audited",
  "--score-audited",
]);
const VALUE_FLAGS = new Set([
  "--backend-url",
  "--cluster",
  "--core-collection",
  "--helius-webhook-id",
  "--helius-webhook-type",
  "--loom-asset",
  "--manifest",
  "--out",
  "--program-id",
  "--proof-verifier",
  "--score-snapshot",
  "--seal-signature",
  "--snapshot-time",
  "--sp1-vkey",
  "--verified-signature",
]);
const SECRET_PATH_RE =
  /(^|[/\\])\.env(?:[./\\]|$)|(^|[/\\])id\.json$|\.anky$|keypair|deployer|wallet|\.pem$/i;

main();

function main() {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help === true) {
      printUsage();
      return;
    }

    const evidence = buildEvidence(args);
    const rendered = `${JSON.stringify(evidence, null, 2)}\n`;

    if (typeof args.out === "string") {
      const outPath = resolvePublicPath(args.out, "--out");
      fs.mkdirSync(path.dirname(outPath), { recursive: true });
      fs.writeFileSync(outPath, rendered);
      if (args.audit === true) {
        runAudit(outPath);
      }
      console.log(`wrote ${outPath}`);
      return;
    }

    if (args.audit === true) {
      const tempPath = path.join(
        os.tmpdir(),
        `anky-public-launch-evidence-${process.pid}-${Date.now()}.json`,
      );
      try {
        fs.writeFileSync(tempPath, rendered);
        runAudit(tempPath);
      } finally {
        fs.rmSync(tempPath, { force: true });
      }
    }

    console.log(rendered.trimEnd());
  } catch (error) {
    console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
    process.exit(1);
  }
}

function buildEvidence(args) {
  const manifestPath = resolvePublicPath(requiredArg(args, "manifest"), "--manifest");
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  if (manifest == null || typeof manifest !== "object" || Array.isArray(manifest)) {
    throw new Error("handoff manifest must be a JSON object.");
  }

  const publicReceipt = normalizePublicReceipt(manifest.publicReceipt);
  const utcDayStatus = normalizeUtcDayStatus(manifest.utcDayStatus, publicReceipt.utcDay, manifest);
  const cluster = normalizeCluster(args.cluster ?? manifest.cluster ?? "devnet");
  const programId = requiredPublicKey(args.programId ?? manifest.programId, "program ID");
  const loomAsset = requiredPublicKey(
    args.loomAsset ?? manifest.publicInputs?.loomAsset,
    "Core Loom asset",
  );
  const coreCollection = requiredPublicKey(args.coreCollection, "Core collection");
  const proofVerifier = requiredPublicKey(
    args.proofVerifier ?? manifest.verifiedSeal?.verifier,
    "proof verifier authority",
  );
  const sp1Vkey = requiredSp1Vkey(args.sp1Vkey);
  const sealSignature = requiredSignature(args.sealSignature, "seal signature");
  const verifiedSignature = requiredSignature(args.verifiedSignature, "verified signature");
  if (sealSignature === verifiedSignature) {
    throw new Error("--seal-signature and --verified-signature must be distinct transactions.");
  }
  const backendUrl = requiredHttpsUrl(args.backendUrl, "backend URL");
  const snapshotTime = requiredIsoTimestamp(args.snapshotTime, "snapshot time");
  const scoreSnapshot = requiredPublicArtifactPath(args.scoreSnapshot, "score snapshot");
  const scoreSnapshotAuditPath = path.resolve(scoreSnapshot);
  const webhookId = requiredWebhookId(args.heliusWebhookId);
  const webhookType = args.heliusWebhookType ?? (cluster === "devnet" ? "enhancedDevnet" : "enhanced");

  if (manifest.proofVerified !== true) {
    throw new Error("manifest.proofVerified must be true before building launch evidence.");
  }
  if (webhookType !== "enhancedDevnet") {
    throw new Error("--helius-webhook-type must be enhancedDevnet for devnet evidence.");
  }
  if (args.auditScoreSnapshot === true) {
    runScoreSnapshotAudit({
      allowInferredFinality: args.allowInferredFinality === true,
      proofVerifier,
      snapshotPath: scoreSnapshotAuditPath,
    });
  }
  if (args.scoreAudited !== true && args.auditScoreSnapshot !== true) {
    throw new Error("--score-audited is required after running sojourn9:audit-snapshot.");
  }
  if (args.backfillAudited !== true) {
    throw new Error("--backfill-audited is required after finalized Helius backfill has been checked.");
  }

  return {
    cluster,
    programId,
    coreCollection,
    proofVerifierAuthority: proofVerifier,
    protocolVersion: 1,
    sp1Vkey,
    snapshotTime,
    devnetE2E: {
      writer: publicReceipt.writer,
      loomAsset,
      sessionHash: publicReceipt.sessionHash,
      proofHash: publicReceipt.proofHash,
      utcDay: publicReceipt.utcDay,
      utcDayStatus,
      sp1ProofVerified: manifest.proofVerified === true,
      hashSealLanded: true,
      verifiedSealLanded: true,
      sealSignature,
      sealOrbUrl: `https://orbmarkets.io/tx/${sealSignature}`,
      verifiedSignature,
      verifiedOrbUrl: `https://orbmarkets.io/tx/${verifiedSignature}`,
    },
    backend: {
      url: backendUrl,
      requireVerifiedSealChainProof: true,
      migrationsApplied: [
        "019_mobile_verified_seal_receipts",
        "020_mobile_helius_webhook_events",
        "021_mobile_helius_webhook_signature_dedupe",
      ],
    },
    helius: {
      webhookId,
      webhookType,
      webhookAccountAddresses: [programId],
      receiverPath: "/api/helius/anky-seal",
      backfillMethod: "getTransactionsForAddress",
      backfillCommitment: "finalized",
      requireFinalized: true,
      dedupeBySignature: true,
      backfillAudited: true,
    },
    scoreSnapshot: {
      path: scoreSnapshot,
      audited: true,
      requireFinalized: true,
      formula: SCORE_FORMULA,
      rewardBps: DEFAULT_REWARD_BPS,
      participantCap: DEFAULT_MAX_PARTICIPANTS,
    },
    claims: {
      directOnchainSp1: false,
      mainnetDeployment: false,
      hashEncryptsWriting: false,
      anonymousWriting: false,
    },
  };
}

function normalizeUtcDayStatus(status, receiptUtcDay, manifest) {
  if (status == null || typeof status !== "object" || Array.isArray(status)) {
    return deriveLegacyUtcDayStatus(manifest, receiptUtcDay);
  }
  const currentUtcDay = requiredSafeInteger(status.currentUtcDay, "manifest.utcDayStatus.currentUtcDay");
  const normalizedReceiptUtcDay = requiredSafeInteger(
    status.receiptUtcDay,
    "manifest.utcDayStatus.receiptUtcDay",
  );
  const secondsUntilRollover = requiredSafeInteger(
    status.secondsUntilRollover,
    "manifest.utcDayStatus.secondsUntilRollover",
  );
  if (normalizedReceiptUtcDay !== receiptUtcDay) {
    throw new Error("manifest.utcDayStatus.receiptUtcDay must match manifest.publicReceipt.utcDay.");
  }
  const expectedIsCurrentDay = normalizedReceiptUtcDay === currentUtcDay;
  const expectedSealWindow = expectedIsCurrentDay
    ? "open"
    : normalizedReceiptUtcDay < currentUtcDay
      ? "stale"
      : "future";
  if (status.isCurrentDay !== expectedIsCurrentDay) {
    throw new Error("manifest.utcDayStatus.isCurrentDay is inconsistent with the UTC day values.");
  }
  if (status.sealWindow !== expectedSealWindow) {
    throw new Error("manifest.utcDayStatus.sealWindow is inconsistent with the UTC day values.");
  }
  const expectedRolloverAt = new Date((currentUtcDay + 1) * SECONDS_PER_DAY * 1000).toISOString();
  if (status.dayRolloverAt !== expectedRolloverAt) {
    throw new Error("manifest.utcDayStatus.dayRolloverAt is inconsistent with currentUtcDay.");
  }
  return {
    currentUtcDay,
    receiptUtcDay: normalizedReceiptUtcDay,
    isCurrentDay: status.isCurrentDay,
    sealWindow: status.sealWindow,
    secondsUntilRollover,
    dayRolloverAt: status.dayRolloverAt,
  };
}

function deriveLegacyUtcDayStatus(manifest, receiptUtcDay) {
  const currentUtcDay = requiredSafeInteger(
    manifest?.currentUtcDay,
    "manifest.currentUtcDay",
  );
  const generatedAt = requiredIsoTimestamp(manifest?.generatedAt, "manifest.generatedAt");
  const generatedAtMs = Date.parse(generatedAt);
  const generatedUtcDay = Math.floor(Math.floor(generatedAtMs / 1000) / SECONDS_PER_DAY);
  if (generatedUtcDay !== currentUtcDay) {
    throw new Error("manifest.generatedAt must agree with manifest.currentUtcDay.");
  }
  const nextRolloverMs = (currentUtcDay + 1) * SECONDS_PER_DAY * 1000;
  const secondsUntilRollover = Math.max(0, Math.floor((nextRolloverMs - generatedAtMs) / 1000));
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

function normalizePublicReceipt(receipt) {
  if (receipt == null || typeof receipt !== "object" || Array.isArray(receipt)) {
    throw new Error("manifest.publicReceipt is required.");
  }
  if (receipt.valid !== true) {
    throw new Error("manifest public receipt must be valid.");
  }
  return {
    writer: requiredPublicKey(receipt.writer, "receipt writer"),
    sessionHash: requiredHash(receipt.sessionHash, "receipt session hash"),
    proofHash: requiredHash(receipt.proofHash, "receipt proof hash"),
    utcDay: requiredSafeInteger(receipt.utcDay, "receipt UTC day"),
  };
}

function runAudit(evidencePath) {
  const result = spawnSync(process.execPath, [AUDIT_SCRIPT, "--evidence", evidencePath], {
    encoding: "utf8",
    env: {
      PATH: process.env.PATH ?? "",
    },
    maxBuffer: 1024 * 1024,
  });
  if (result.status !== 0) {
    throw new Error(
      `public launch evidence audit failed:\n${redactSecretValues(result.stderr || result.stdout)}`,
    );
  }
}

function runScoreSnapshotAudit({ allowInferredFinality, proofVerifier, snapshotPath }) {
  if (!fs.existsSync(snapshotPath)) {
    throw new Error("--audit-score-snapshot requires --score-snapshot to point to an existing public JSON file.");
  }
  const args = [
    SCORE_AUDIT_SCRIPT,
    "--snapshot",
    snapshotPath,
    "--proof-verifier",
    proofVerifier,
    "--reward-bps",
    String(DEFAULT_REWARD_BPS),
    "--require-allocation",
    ...(allowInferredFinality ? ["--allow-inferred-finality"] : []),
  ];
  const result = spawnSync(process.execPath, args, {
    encoding: "utf8",
    env: {
      PATH: process.env.PATH ?? "",
    },
    maxBuffer: 1024 * 1024,
  });
  if (result.status !== 0) {
    throw new Error(
      `score snapshot audit failed:\n${redactSecretValues(result.stderr || result.stdout)}`,
    );
  }
}

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (BOOLEAN_FLAGS.has(arg)) {
      args[toCamel(arg)] = true;
      continue;
    }
    if (VALUE_FLAGS.has(arg)) {
      const value = argv[index + 1];
      if (value == null || value.startsWith("--")) {
        throw new Error(`${arg} requires a value.`);
      }
      args[toCamel(arg)] = value;
      index += 1;
      continue;
    }
    throw new Error(`Unknown option: ${arg}`);
  }
  return args;
}

function requiredArg(args, name) {
  const value = args[name];
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`--${name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} is required.`);
  }
  return value.trim();
}

function resolvePublicPath(value, label) {
  if (typeof value !== "string" || value.trim().length === 0 || SECRET_PATH_RE.test(value)) {
    throw new Error(`${label} must be a public non-secret path.`);
  }
  const resolved = path.resolve(value);
  if (SECRET_PATH_RE.test(resolved)) {
    throw new Error(`${label} must be a public non-secret path.`);
  }
  return resolved;
}

function requiredPublicArtifactPath(value, label) {
  if (typeof value !== "string" || value.trim().length === 0 || SECRET_PATH_RE.test(value)) {
    throw new Error(`${label} must be a public non-secret artifact path.`);
  }
  return value.trim();
}

function normalizeCluster(value) {
  if (value === "devnet") {
    return value;
  }
  if (value === "mainnet-beta") {
    throw new Error("makeLaunchEvidence is devnet-handoff only. Stop before mainnet evidence.");
  }
  throw new Error("--cluster must be devnet.");
}

function requiredPublicKey(value, label) {
  if (!isBase58PublicKey(value)) {
    throw new Error(`${label} must be a 32-byte Solana public key.`);
  }
  return value.trim();
}

function requiredSignature(value, label) {
  if (!isSolanaSignature(value)) {
    throw new Error(`${label} must be a real 64-byte Solana signature.`);
  }
  return value.trim();
}

function requiredHash(value, label) {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value.trim())) {
    throw new Error(`${label} must be a 32-byte lowercase hex string.`);
  }
  return value.trim();
}

function requiredSp1Vkey(value) {
  if (typeof value !== "string" || !/^0x[0-9a-f]{64}$/.test(value.trim())) {
    throw new Error("--sp1-vkey must be 0x plus 32 lowercase hex bytes.");
  }
  return value.trim();
}

function requiredSafeInteger(value, label) {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${label} must be a non-negative safe integer.`);
  }
  return value;
}

function requiredIsoTimestamp(value, label) {
  if (typeof value !== "string") {
    throw new Error(`${label} must be an ISO timestamp.`);
  }
  const parsedMs = Date.parse(value);
  if (!Number.isFinite(parsedMs) || new Date(parsedMs).toISOString() !== value) {
    throw new Error(`${label} must be an ISO timestamp.`);
  }
  return value;
}

function requiredHttpsUrl(value, label) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${label} is required.`);
  }
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    throw new Error(`${label} must be a valid URL.`);
  }
  if (parsed.protocol !== "https:" || parsed.username !== "" || parsed.password !== "" || parsed.search !== "" || parsed.hash !== "") {
    throw new Error(`${label} must be an HTTPS URL without credentials, query strings, or fragments.`);
  }
  return parsed.toString().replace(/\/$/, "");
}

function requiredWebhookId(value) {
  if (typeof value !== "string" || !/^[A-Za-z0-9_-]{8,}$/.test(value.trim())) {
    throw new Error("--helius-webhook-id must be a non-secret public webhook identifier.");
  }
  return value.trim();
}

function isBase58PublicKey(value) {
  if (typeof value !== "string") {
    return false;
  }
  try {
    return base58Decode(value).length === 32;
  } catch {
    return false;
  }
}

function isSolanaSignature(value) {
  if (typeof value !== "string") {
    return false;
  }
  try {
    return base58Decode(value).length === 64;
  } catch {
    return false;
  }
}

function base58Decode(value) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error("invalid base58 string");
  }

  let decoded = 0n;
  for (const character of value) {
    const digit = BASE58_ALPHABET.indexOf(character);
    if (digit < 0) {
      throw new Error("invalid base58 character");
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

function toCamel(flag) {
  return flag
    .replace(/^--/, "")
    .replace(/-([a-z])/g, (_match, char) => char.toUpperCase());
}

function printUsage() {
  console.log(`Builds a public Sojourn 9 launch evidence JSON file from a proof handoff manifest.

Usage:
  node solana/scripts/sojourn9/makeLaunchEvidence.mjs \\
    --manifest /tmp/anky-sojourn9-current-.../handoff-manifest.json \\
    --core-collection <collection_pubkey> \\
    --sp1-vkey 0x... \\
    --seal-signature <landed_seal_signature> \\
    --verified-signature <landed_verified_signature> \\
    --backend-url https://<backend> \\
    --helius-webhook-id <webhook_id> \\
    --score-snapshot sojourn9/devnet-score-snapshot.json \\
    --snapshot-time <utc_iso_timestamp> \\
    --score-audited \\
    --audit-score-snapshot \\
    --backfill-audited \\
    --audit

This reads only public handoff metadata. It never reads the private .anky witness,
accepts no keypair paths or API keys, derives Orb links from real signatures, and
is devnet-only; use the mainnet checklist for later public mainnet evidence.
Use --audit-score-snapshot to run the public Score V1 snapshot auditor directly.
Use --allow-inferred-finality only when a finalized Helius backfill omitted per-transaction commitment and that condition is documented.`);
}
