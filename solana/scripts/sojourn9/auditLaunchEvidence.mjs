#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { redactSecretValues } from "./redactSecrets.mjs";

const DEFAULT_MAX_PARTICIPANTS = 3_456;
const DEFAULT_REWARD_BPS = 800;
const SCORE_FORMULA =
  "score = unique_seal_days + verified_days + 2 * floor(each_consecutive_day_run / 7)";
const CREDIT_LEDGER_MIGRATION = "022_credit_ledger_entries";
const VERIFIED_SEAL_BACKEND_MIGRATIONS = [
  "019_mobile_verified_seal_receipts",
  "020_mobile_helius_webhook_events",
  "021_mobile_helius_webhook_signature_dedupe",
];
const SECONDS_PER_DAY = 86_400;
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BOOLEAN_FLAGS = new Set(["--print-template"]);
const VALUE_FLAGS = new Set(["--evidence"]);
const SECRET_PATH_RE =
  /(^|[/\\])\.env(?:[./\\]|$)|(^|[/\\])id\.json$|\.anky$|keypair|deployer|wallet|\.pem$/i;
const PRIVATE_KEY_RE =
  /(?:rawAnky|raw_anky|ankyPlaintext|anky_plaintext|plaintext|writingText|writing_text|reconstructedText|reconstructed_text|sp1Witness|sp1_witness|proofWitness|proof_witness|privateInput|private_input|privateInputs|private_inputs|witnessBytes|witness_bytes|fileBytes|file_bytes|fileContents|file_contents|keypair|privateKey|private_key|secret|apiKey|api_key|accessToken|access_token|bearer|authorization|envFile|env_file|mnemonic|seedPhrase|seed_phrase|deployer)/i;

main();

function main() {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help === true) {
      printUsage();
      return;
    }
    if (args.printTemplate === true) {
      if (args.evidence != null) {
        throw new Error("--print-template cannot be combined with --evidence.");
      }
      console.log(JSON.stringify(buildLaunchEvidenceTemplate(), null, 2));
      return;
    }

    const evidencePath = resolvePublicEvidencePath(requiredArg(args, "evidence"));
    const evidence = JSON.parse(fs.readFileSync(evidencePath, "utf8"));
    const issues = auditLaunchEvidence(evidence);
    const report = {
      auditedAt: new Date().toISOString(),
      ok: issues.length === 0,
      issues,
      summary: {
        evidence: evidencePath,
        cluster: typeof evidence?.cluster === "string" ? evidence.cluster : null,
        programId: typeof evidence?.programId === "string" ? evidence.programId : null,
        coreCollection:
          typeof evidence?.coreCollection === "string" ? evidence.coreCollection : null,
        proofVerifierAuthority:
          typeof evidence?.proofVerifierAuthority === "string"
            ? evidence.proofVerifierAuthority
            : null,
        protocolVersion:
          Number.isSafeInteger(evidence?.protocolVersion) ? evidence.protocolVersion : null,
        devnetUtcDay:
          Number.isSafeInteger(evidence?.devnetE2E?.utcDay) ? evidence.devnetE2E.utcDay : null,
        devnetSealWindow:
          typeof evidence?.devnetE2E?.utcDayStatus?.sealWindow === "string"
            ? evidence.devnetE2E.utcDayStatus.sealWindow
            : null,
        devnetDayRolloverAt:
          typeof evidence?.devnetE2E?.utcDayStatus?.dayRolloverAt === "string"
            ? evidence.devnetE2E.utcDayStatus.dayRolloverAt
            : null,
      },
    };

    console.log(JSON.stringify(report, null, 2));
    if (issues.length > 0) {
      process.exit(1);
    }
  } catch (error) {
    console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
    process.exit(1);
  }
}

function auditLaunchEvidence(evidence) {
  const issues = [];
  if (evidence == null || typeof evidence !== "object" || Array.isArray(evidence)) {
    return ["launch evidence must be a JSON object"];
  }

  scanForPrivateFields(evidence, [], issues);

  if (evidence.templateOnly === true) {
    issues.push("templateOnly must not be true in final launch evidence");
  }

  const cluster = evidence.cluster;
  if (cluster !== "devnet" && cluster !== "mainnet-beta") {
    issues.push("cluster must be devnet or mainnet-beta");
  }
  if (!isBase58PublicKey(evidence.programId)) {
    issues.push("programId must be a 32-byte Solana public key");
  }
  if (!isBase58PublicKey(evidence.coreCollection)) {
    issues.push("coreCollection must be a 32-byte Metaplex Core collection public key");
  }
  if (!isBase58PublicKey(evidence.proofVerifierAuthority)) {
    issues.push("proofVerifierAuthority must be a 32-byte Solana public key");
  }
  if (evidence.protocolVersion !== 1) {
    issues.push("protocolVersion must be 1");
  }
  if (evidence.sp1Vkey != null && !/^0x[0-9a-f]{64}$/.test(evidence.sp1Vkey)) {
    issues.push("sp1Vkey must be 0x plus 32 lowercase hex bytes");
  }
  if (!isIsoTimestamp(evidence.snapshotTime)) {
    issues.push("snapshotTime must be an ISO timestamp");
  }

  auditDevnetE2E(evidence.devnetE2E, issues);
  auditBackendEvidence(evidence.backend, issues);
  auditHeliusEvidence(evidence.helius, cluster, evidence.programId, issues);
  auditScoreSnapshotEvidence(evidence.scoreSnapshot, issues);
  auditClaims(evidence.claims, cluster, issues);
  if (cluster === "mainnet-beta") {
    auditMainnetEvidence(evidence.mainnet, issues);
  }

  return issues;
}

function auditDevnetE2E(devnetE2E, issues) {
  if (devnetE2E == null || typeof devnetE2E !== "object" || Array.isArray(devnetE2E)) {
    issues.push("devnetE2E public transaction evidence is required");
    return;
  }

  if (!isBase58PublicKey(devnetE2E.writer)) {
    issues.push("devnetE2E.writer must be a 32-byte Solana public key");
  }
  if (!isBase58PublicKey(devnetE2E.loomAsset)) {
    issues.push("devnetE2E.loomAsset must be a 32-byte Core Loom asset public key");
  }
  if (!isHashHex(devnetE2E.sessionHash)) {
    issues.push("devnetE2E.sessionHash must be a 32-byte lowercase hex string");
  }
  if (!isHashHex(devnetE2E.proofHash)) {
    issues.push("devnetE2E.proofHash must be a 32-byte lowercase hex string");
  }
  if (!Number.isSafeInteger(devnetE2E.utcDay) || devnetE2E.utcDay < 0) {
    issues.push("devnetE2E.utcDay must be a non-negative safe integer");
  }
  auditUtcDayStatus(devnetE2E.utcDayStatus, devnetE2E.utcDay, "devnetE2E.utcDayStatus", issues);
  if (!isSolanaSignature(devnetE2E.sealSignature)) {
    issues.push("devnetE2E.sealSignature must be a real 64-byte Solana signature");
  }
  if (!isSolanaSignature(devnetE2E.verifiedSignature)) {
    issues.push("devnetE2E.verifiedSignature must be a real 64-byte Solana signature");
  }
  if (
    typeof devnetE2E.sealSignature === "string" &&
    devnetE2E.sealSignature === devnetE2E.verifiedSignature
  ) {
    issues.push("devnetE2E sealSignature and verifiedSignature must be distinct landed transactions");
  }
  if (!isOrbTxLink(devnetE2E.sealOrbUrl, devnetE2E.sealSignature)) {
    issues.push("devnetE2E.sealOrbUrl must be an Orb transaction link for sealSignature");
  }
  if (!isOrbTxLink(devnetE2E.verifiedOrbUrl, devnetE2E.verifiedSignature)) {
    issues.push("devnetE2E.verifiedOrbUrl must be an Orb transaction link for verifiedSignature");
  }
  if (devnetE2E.sp1ProofVerified !== true) {
    issues.push("devnetE2E.sp1ProofVerified must be true");
  }
  if (devnetE2E.hashSealLanded !== true) {
    issues.push("devnetE2E.hashSealLanded must be true");
  }
  if (devnetE2E.verifiedSealLanded !== true) {
    issues.push("devnetE2E.verifiedSealLanded must be true");
  }
}

function auditUtcDayStatus(status, receiptUtcDay, label, issues) {
  if (status == null || typeof status !== "object" || Array.isArray(status)) {
    issues.push(`${label} is required`);
    return;
  }
  const currentUtcDay = status.currentUtcDay;
  const statusReceiptUtcDay = status.receiptUtcDay;
  if (!Number.isSafeInteger(currentUtcDay) || currentUtcDay < 0) {
    issues.push(`${label}.currentUtcDay must be a non-negative safe integer`);
  }
  if (!Number.isSafeInteger(statusReceiptUtcDay) || statusReceiptUtcDay < 0) {
    issues.push(`${label}.receiptUtcDay must be a non-negative safe integer`);
  }
  if (Number.isSafeInteger(receiptUtcDay) && statusReceiptUtcDay !== receiptUtcDay) {
    issues.push(`${label}.receiptUtcDay must match devnetE2E.utcDay`);
  }
  if (!Number.isSafeInteger(currentUtcDay) || !Number.isSafeInteger(statusReceiptUtcDay)) {
    return;
  }
  const expectedIsCurrentDay = statusReceiptUtcDay === currentUtcDay;
  const expectedSealWindow = expectedIsCurrentDay
    ? "open"
    : statusReceiptUtcDay < currentUtcDay
      ? "stale"
      : "future";
  if (status.isCurrentDay !== expectedIsCurrentDay) {
    issues.push(`${label}.isCurrentDay is inconsistent with UTC day values`);
  }
  if (status.sealWindow !== expectedSealWindow) {
    issues.push(`${label}.sealWindow must be ${expectedSealWindow}`);
  }
  if (!Number.isSafeInteger(status.secondsUntilRollover) || status.secondsUntilRollover < 0) {
    issues.push(`${label}.secondsUntilRollover must be a non-negative safe integer`);
  }
  const expectedRolloverAt = new Date((currentUtcDay + 1) * SECONDS_PER_DAY * 1000).toISOString();
  if (status.dayRolloverAt !== expectedRolloverAt) {
    issues.push(`${label}.dayRolloverAt must be ${expectedRolloverAt}`);
  }
}

function auditBackendEvidence(backend, issues) {
  if (backend == null || typeof backend !== "object" || Array.isArray(backend)) {
    issues.push("backend launch evidence is required");
    return;
  }
  if (!isHttpsUrlWithoutCredentials(backend.url)) {
    issues.push("backend.url must be an HTTPS URL without credentials");
  }
  if (backend.requireVerifiedSealChainProof !== true) {
    issues.push("backend.requireVerifiedSealChainProof must be true for launch evidence");
  }
  const migrations = normalizeMigrationSet(backend.migrationsApplied);
  for (const migration of VERIFIED_SEAL_BACKEND_MIGRATIONS) {
    if (!migrations.has(migration)) {
      issues.push(`backend.migrationsApplied must include ${migration}`);
    }
  }
  if (
    (backend.fullMigrationChainApplied === true ||
      backend.fullMigrationChainChecked === true ||
      backend.fullBackendMigrationChainApplied === true) &&
    !migrations.has(CREDIT_LEDGER_MIGRATION)
  ) {
    issues.push(`backend.migrationsApplied must include ${CREDIT_LEDGER_MIGRATION}`);
  }
}

function auditHeliusEvidence(helius, cluster, programId, issues) {
  if (helius == null || typeof helius !== "object" || Array.isArray(helius)) {
    issues.push("Helius webhook/backfill evidence is required");
    return;
  }
  if (typeof helius.webhookId !== "string" || !/^[A-Za-z0-9_-]{8,}$/.test(helius.webhookId)) {
    issues.push("helius.webhookId must be a non-secret public webhook identifier");
  }
  const expectedWebhookType = cluster === "mainnet-beta" ? "enhanced" : "enhancedDevnet";
  if (helius.webhookType !== expectedWebhookType) {
    issues.push(`helius.webhookType must be ${expectedWebhookType}`);
  }
  if (
    !Array.isArray(helius.webhookAccountAddresses) ||
    helius.webhookAccountAddresses.length !== 1 ||
    helius.webhookAccountAddresses[0] !== programId
  ) {
    issues.push("helius.webhookAccountAddresses must contain only the Anky Seal Program ID");
  }
  if (helius.receiverPath !== "/api/helius/anky-seal") {
    issues.push("helius.receiverPath must be /api/helius/anky-seal");
  }
  if (helius.backfillMethod !== "getTransactionsForAddress") {
    issues.push("helius.backfillMethod must be getTransactionsForAddress");
  }
  if (helius.backfillCommitment !== "finalized") {
    issues.push("helius.backfillCommitment must be finalized");
  }
  if (helius.backfillAudited !== true) {
    issues.push("helius.backfillAudited must be true");
  }
  if (helius.requireFinalized !== true) {
    issues.push("helius.requireFinalized must be true");
  }
  if (helius.dedupeBySignature !== true) {
    issues.push("helius.dedupeBySignature must be true");
  }
}

function auditScoreSnapshotEvidence(scoreSnapshot, issues) {
  if (scoreSnapshot == null || typeof scoreSnapshot !== "object" || Array.isArray(scoreSnapshot)) {
    issues.push("scoreSnapshot public audit evidence is required");
    return;
  }
  if (!isPublicPath(scoreSnapshot.path)) {
    issues.push("scoreSnapshot.path must be a public non-secret JSON artifact path");
  }
  if (scoreSnapshot.audited !== true) {
    issues.push("scoreSnapshot.audited must be true");
  }
  if (scoreSnapshot.requireFinalized !== true) {
    issues.push("scoreSnapshot.requireFinalized must be true");
  }
  if (scoreSnapshot.formula !== SCORE_FORMULA) {
    issues.push("scoreSnapshot.formula must match Score V1");
  }
  if (scoreSnapshot.rewardBps !== DEFAULT_REWARD_BPS) {
    issues.push(`scoreSnapshot.rewardBps must be ${DEFAULT_REWARD_BPS}`);
  }
  if (scoreSnapshot.participantCap !== DEFAULT_MAX_PARTICIPANTS) {
    issues.push(`scoreSnapshot.participantCap must be ${DEFAULT_MAX_PARTICIPANTS}`);
  }
}

function auditClaims(claims, cluster, issues) {
  if (claims == null) {
    return;
  }
  if (claims.directOnchainSp1 === true) {
    issues.push("claims.directOnchainSp1 must not be true for Sojourn 9");
  }
  if (claims.mainnetDeployment === true && cluster !== "mainnet-beta") {
    issues.push("claims.mainnetDeployment cannot be true for devnet evidence");
  }
  if (claims.hashEncryptsWriting === true || claims.anonymousWriting === true) {
    issues.push("claims must not say the hash encrypts writing or that writing is anonymous");
  }
}

function auditMainnetEvidence(mainnet, issues) {
  if (mainnet == null || typeof mainnet !== "object" || Array.isArray(mainnet)) {
    issues.push("mainnet public launch evidence is required for mainnet-beta");
    return;
  }
  if (!isSolanaSignature(mainnet.deploymentSignature)) {
    issues.push("mainnet.deploymentSignature must be a real 64-byte Solana signature");
  }
  if (!isOrbTxLink(mainnet.deploymentOrbUrl, mainnet.deploymentSignature)) {
    issues.push("mainnet.deploymentOrbUrl must be an Orb transaction link for deploymentSignature");
  }
  if (!isOrbAddressLink(mainnet.programOrbUrl)) {
    issues.push("mainnet.programOrbUrl must be an Orb account link");
  }
  if (!isOrbAddressLink(mainnet.collectionOrbUrl)) {
    issues.push("mainnet.collectionOrbUrl must be an Orb account link");
  }
}

function buildLaunchEvidenceTemplate() {
  const signature = "<real_64_byte_solana_signature>";
  const secondSignature = "<different_real_64_byte_solana_signature>";

  return {
    templateOnly: true,
    cluster: "devnet",
    programId: "<anky_seal_program_public_key>",
    coreCollection: "<metaplex_core_collection_public_key>",
    proofVerifierAuthority: "<proof_verifier_authority_public_key>",
    protocolVersion: 1,
    sp1Vkey: "0x<64_lowercase_hex_chars>",
    snapshotTime: "<utc_snapshot_timestamp_iso>",
    devnetE2E: {
      writer: "<writer_public_key>",
      loomAsset: "<core_loom_asset_public_key>",
      sessionHash: "<64_lowercase_hex_session_hash>",
      proofHash: "<64_lowercase_hex_proof_hash>",
      utcDay: 0,
      utcDayStatus: {
        currentUtcDay: 0,
        receiptUtcDay: 0,
        isCurrentDay: true,
        sealWindow: "open",
        secondsUntilRollover: 0,
        dayRolloverAt: "1970-01-02T00:00:00.000Z",
      },
      sp1ProofVerified: true,
      hashSealLanded: true,
      verifiedSealLanded: true,
      sealSignature: signature,
      sealOrbUrl: `https://orbmarkets.io/tx/${signature}`,
      verifiedSignature: secondSignature,
      verifiedOrbUrl: `https://orbmarkets.io/tx/${secondSignature}`,
    },
    backend: {
      url: "https://<public_backend_host>",
      requireVerifiedSealChainProof: true,
      migrationsApplied: [
        "019_mobile_verified_seal_receipts",
        "020_mobile_helius_webhook_events",
        "021_mobile_helius_webhook_signature_dedupe",
        "022_credit_ledger_entries",
      ],
    },
    helius: {
      webhookId: "<public_helius_webhook_id>",
      webhookType: "enhancedDevnet",
      webhookAccountAddresses: ["<anky_seal_program_public_key>"],
      receiverPath: "/api/helius/anky-seal",
      backfillMethod: "getTransactionsForAddress",
      backfillCommitment: "finalized",
      requireFinalized: true,
      dedupeBySignature: true,
      backfillAudited: true,
    },
    scoreSnapshot: {
      path: "sojourn9/devnet-score-snapshot.json",
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
    mainnet: {
      deploymentSignature: "<mainnet_deployment_signature_after_it_exists>",
      deploymentOrbUrl:
        "https://orbmarkets.io/tx/<mainnet_deployment_signature_after_it_exists>",
      programOrbUrl: "https://orbmarkets.io/address/<mainnet_program_public_key>",
      collectionOrbUrl: "https://orbmarkets.io/address/<mainnet_collection_public_key>",
    },
  };
}

function scanForPrivateFields(value, fieldPath, issues) {
  if (Array.isArray(value)) {
    value.forEach((item, index) => scanForPrivateFields(item, [...fieldPath, String(index)], issues));
    return;
  }
  if (value == null || typeof value !== "object") {
    if (typeof value === "string") {
      const joinedPath = fieldPath.join(".") || "<root>";
      if (looksLikeCompleteAnkyPlaintext(value)) {
        issues.push(`complete .anky plaintext-like value is present at ${joinedPath}`);
      }
      if (redactSecretValues(value) !== value) {
        issues.push(`secret-looking value is present at ${joinedPath}`);
      }
    }
    return;
  }
  for (const [key, nested] of Object.entries(value)) {
    const nestedPath = [...fieldPath, key];
    if (PRIVATE_KEY_RE.test(key)) {
      issues.push(`private/plaintext-like field is present at ${nestedPath.join(".")}`);
    }
    scanForPrivateFields(nested, nestedPath, issues);
  }
}

function looksLikeCompleteAnkyPlaintext(value) {
  return (
    typeof value === "string" &&
    value.includes("\n") &&
    value.includes("8000") &&
    (isClosedAnky(value, { allowLiteralSpace: false }) ||
      isClosedAnky(value, { allowLiteralSpace: true }))
  );
}

function isClosedAnky(value, { allowLiteralSpace }) {
  if (
    value.length === 0 ||
    value.charCodeAt(0) === 0xfeff ||
    value.includes("\r") ||
    !value.endsWith("\n8000") ||
    countOccurrences(value, "\n8000") !== 1
  ) {
    return false;
  }

  const lines = value.split("\n");
  const first = lines.shift();
  if (!captureLineHasValidTimeAndCharacter(first, { allowLiteralSpace, firstLine: true })) {
    return false;
  }
  for (const line of lines) {
    if (line === "8000") {
      return true;
    }
    if (!captureLineHasValidTimeAndCharacter(line, { allowLiteralSpace, firstLine: false })) {
      return false;
    }
  }
  return false;
}

function captureLineHasValidTimeAndCharacter(line, { allowLiteralSpace, firstLine }) {
  if (typeof line !== "string") {
    return false;
  }
  const separator = line.indexOf(" ");
  if (separator < 0) {
    return false;
  }
  const time = line.slice(0, separator);
  const token = line.slice(separator + 1);
  if (firstLine) {
    if (!/^\d+$/.test(time)) {
      return false;
    }
  } else if (!/^\d{4}$/.test(time) || Number(time) > 7_999) {
    return false;
  }
  return isAcceptedAnkyToken(token, { allowLiteralSpace });
}

function isAcceptedAnkyToken(token, { allowLiteralSpace }) {
  if (token === "SPACE") {
    return true;
  }
  if (token === " ") {
    return allowLiteralSpace;
  }
  if ([...token].length !== 1) {
    return false;
  }
  const codepoint = token.codePointAt(0);
  return codepoint > 31 && codepoint !== 127;
}

function countOccurrences(value, pattern) {
  let count = 0;
  let index = value.indexOf(pattern);
  while (index >= 0) {
    count += 1;
    index = value.indexOf(pattern, index + pattern.length);
  }
  return count;
}

function isOrbTxLink(value, expectedSignature) {
  if (!isSolanaSignature(expectedSignature) || typeof value !== "string") {
    return false;
  }
  try {
    const parsed = new URL(value);
    return (
      parsed.protocol === "https:" &&
      parsed.hostname === "orbmarkets.io" &&
      parsed.pathname === `/tx/${expectedSignature}` &&
      parsed.search === "" &&
      parsed.hash === "" &&
      parsed.username === "" &&
      parsed.password === ""
    );
  } catch {
    return false;
  }
}

function isOrbAddressLink(value) {
  if (typeof value !== "string") {
    return false;
  }
  try {
    const parsed = new URL(value);
    return (
      parsed.protocol === "https:" &&
      parsed.hostname === "orbmarkets.io" &&
      parsed.pathname.startsWith("/address/") &&
      isBase58PublicKey(parsed.pathname.slice("/address/".length)) &&
      parsed.search === "" &&
      parsed.hash === "" &&
      parsed.username === "" &&
      parsed.password === ""
    );
  } catch {
    return false;
  }
}

function isHttpsUrlWithoutCredentials(value) {
  if (typeof value !== "string") {
    return false;
  }
  try {
    const parsed = new URL(value);
    return (
      parsed.protocol === "https:" &&
      parsed.username === "" &&
      parsed.password === "" &&
      parsed.search === "" &&
      parsed.hash === ""
    );
  } catch {
    return false;
  }
}

function isPublicPath(value) {
  return (
    typeof value === "string" &&
    value.trim().length > 0 &&
    !SECRET_PATH_RE.test(value) &&
    !value.includes("\0")
  );
}

function isIsoTimestamp(value) {
  if (typeof value !== "string" || value.trim().length === 0) {
    return false;
  }
  const parsedMs = Date.parse(value);
  return Number.isFinite(parsedMs) && new Date(parsedMs).toISOString() === value;
}

function normalizeMigrationSet(value) {
  const migrations = new Set();
  if (Array.isArray(value)) {
    for (const item of value) {
      const migration = normalizeMigrationName(item);
      if (migration != null) {
        migrations.add(migration);
      }
    }
    return migrations;
  }
  if (value != null && typeof value === "object") {
    for (const [key, applied] of Object.entries(value)) {
      if (applied !== true) {
        continue;
      }
      const migration = normalizeMigrationName(key);
      if (migration != null) {
        migrations.add(migration);
      }
    }
  }
  return migrations;
}

function normalizeMigrationName(value) {
  const text = String(value ?? "").trim().replace(/\\/g, "/").split("/").pop() ?? "";
  const migration = text.replace(/\.sql$/i, "").toLowerCase();
  if (/^0?19(?:_mobile_verified_seal_receipts)?$/.test(migration)) {
    return "019_mobile_verified_seal_receipts";
  }
  if (/^0?20(?:_mobile_helius_webhook_events)?$/.test(migration)) {
    return "020_mobile_helius_webhook_events";
  }
  if (/^0?21(?:_mobile_helius_webhook_signature_dedupe)?$/.test(migration)) {
    return "021_mobile_helius_webhook_signature_dedupe";
  }
  if (/^0?22(?:_credit_ledger_entries)?$/.test(migration)) {
    return CREDIT_LEDGER_MIGRATION;
  }
  return null;
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

function isHashHex(value) {
  return typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
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

function resolvePublicEvidencePath(value) {
  if (SECRET_PATH_RE.test(value)) {
    throw new Error("--evidence must point to a public JSON file, not an env/.anky/keypair/wallet/deployer file.");
  }
  const evidencePath = path.resolve(value);
  if (SECRET_PATH_RE.test(evidencePath)) {
    throw new Error("--evidence must point to a public JSON file, not an env/.anky/keypair/wallet/deployer file.");
  }
  return evidencePath;
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
    throw new Error(`--${name} is required.`);
  }
  return value.trim();
}

function toCamel(flag) {
  return flag
    .replace(/^--/, "")
    .replace(/-([a-z])/g, (_match, char) => char.toUpperCase());
}

function printUsage() {
  console.log(`Audits a public Sojourn 9 launch evidence JSON file.

Usage:
  node solana/scripts/sojourn9/auditLaunchEvidence.mjs --evidence sojourn9/public-launch-evidence.json
  node solana/scripts/sojourn9/auditLaunchEvidence.mjs --print-template

The audit reads only public launch evidence JSON. It rejects secret-looking fields,
env/.anky/keypair paths, complete .anky plaintext-like values, missing finalized Score V1
audit markers, missing Helius webhook/backfill evidence, and non-Orb transaction links.`);
}
