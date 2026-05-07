#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { redactSecretValues } from "../sojourn9/redactSecrets.mjs";

const DEFAULT_PROOF_VERIFIER_AUTHORITY = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const DEFAULT_MAX_PARTICIPANTS = 3_456;
const DEFAULT_REWARD_BPS = 800;
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const SCORE_FORMULA =
  "score = unique_seal_days + (2 * verified_seal_days) + streak_bonus";
const BOOLEAN_FLAGS = new Set([
  "--allow-inferred-finality",
  "--allow-non-finalized-events",
  "--require-allocation",
]);
const VALUE_FLAGS = new Set([
  "--max-participants",
  "--proof-verifier",
  "--reward-bps",
  "--snapshot",
]);
const SECRET_PATH_RE =
  /(^|[/\\])\.env(?:[./\\]|$)|(^|[/\\])id\.json$|\.anky$|keypair|deployer|wallet|\.pem$/i;
const PRIVATE_KEY_RE =
  /(?:rawAnky|raw_anky|ankyPlaintext|anky_plaintext|plaintext|writingText|writing_text|reconstructedText|reconstructed_text|sp1Witness|sp1_witness|proofWitness|proof_witness|privateInput|private_input|privateInputs|private_inputs|witnessBytes|witness_bytes|fileBytes|file_bytes|fileContents|file_contents)/;

main();

function main() {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help === true) {
      printUsage();
      return;
    }

    const snapshotPath = resolvePublicSnapshotPath(requiredArg(args, "snapshot"));
    const snapshot = JSON.parse(fs.readFileSync(snapshotPath, "utf8"));
    const proofVerifierAuthority = resolvePublicKey(
      args.proofVerifier,
      snapshot.proofVerifierAuthority ?? DEFAULT_PROOF_VERIFIER_AUTHORITY,
      "proof verifier authority",
    );
    const maxParticipants =
      args.maxParticipants == null
        ? DEFAULT_MAX_PARTICIPANTS
        : parsePositiveInteger(args.maxParticipants, "max participants");
    const rewardBps =
      args.rewardBps == null ? DEFAULT_REWARD_BPS : parseBasisPoints(args.rewardBps, "reward bps");
    const issues = [];

    auditSnapshot(snapshot, {
      allowInferredFinality: args.allowInferredFinality === true,
      allowNonFinalizedEvents: args.allowNonFinalizedEvents === true,
      maxParticipants,
      proofVerifierAuthority,
      requireAllocation: args.requireAllocation === true,
      rewardBps,
    }, issues);

    const report = {
      auditedAt: new Date().toISOString(),
      checked: {
        maxParticipants,
        proofVerifierAuthority,
        requireAllocation: args.requireAllocation === true,
        rewardBps,
        snapshot: snapshotPath,
      },
      ok: issues.length === 0,
      issues,
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

function auditSnapshot(snapshot, options, issues) {
  if (snapshot == null || typeof snapshot !== "object" || Array.isArray(snapshot)) {
    issues.push("snapshot must be a JSON object");
    return;
  }
  scanForPrivateFields(snapshot, [], issues);

  const events = Array.isArray(snapshot.events) ? snapshot.events : [];
  const scores = Array.isArray(snapshot.scores) ? snapshot.scores : [];
  const scoringRule = snapshot.scoringRule ?? {};
  const summary = snapshot.summary ?? {};

  if (snapshot.requireFinalized !== true && !options.allowNonFinalizedEvents) {
    issues.push("snapshot.requireFinalized must be true for launch scoring");
  }
  if (snapshot.proofVerifierAuthority !== options.proofVerifierAuthority) {
    issues.push("snapshot proofVerifierAuthority does not match the expected verifier");
  }
  if (scoringRule.version !== 1) {
    issues.push("scoringRule.version must be 1");
  }
  if (scoringRule.formula !== SCORE_FORMULA) {
    issues.push("scoringRule.formula does not match Score V1");
  }
  if (!Number.isSafeInteger(scoringRule.maxParticipants) || scoringRule.maxParticipants > options.maxParticipants) {
    issues.push(`scoringRule.maxParticipants must be <= ${options.maxParticipants}`);
  }
  if (!Number.isSafeInteger(summary.participantCap) || summary.participantCap > options.maxParticipants) {
    issues.push(`summary.participantCap must be <= ${options.maxParticipants}`);
  }
  if (summary.indexedEvents !== events.length) {
    issues.push("summary.indexedEvents does not match events.length");
  }
  if (summary.scoreRows !== scores.length) {
    issues.push("summary.scoreRows does not match scores.length");
  }
  if (!options.allowInferredFinality && Number(summary.finalizedEventsInferredFromBackfillRequest ?? 0) > 0) {
    issues.push("snapshot has inferred finalized events; rerun audit with --allow-inferred-finality only if this is documented");
  }
  if (options.requireAllocation) {
    if (snapshot.allocationRule == null || summary.rewardPoolRaw == null) {
      issues.push("reward allocation is required but allocationRule or summary.rewardPoolRaw is missing");
    } else if (snapshot.allocationRule.rewardBps !== options.rewardBps) {
      issues.push(`allocationRule.rewardBps must be ${options.rewardBps}`);
    }
  }

  for (const event of events) {
    auditEvent(event, options, issues);
  }

  const recomputed = recomputeScores(events, {
    maxParticipants: Number(summary.participantCap ?? scoringRule.maxParticipants ?? options.maxParticipants),
    proofVerifierAuthority: options.proofVerifierAuthority,
    requireFinalized: snapshot.requireFinalized !== false,
  });
  compareScores(scores, recomputed.scores, issues);
  compareSummary(summary, recomputed, issues);
  auditAllocation(scores, snapshot, options, issues);
}

function auditEvent(event, options, issues) {
  if (event == null || typeof event !== "object") {
    issues.push("snapshot events must be objects");
    return;
  }
  if (event.kind !== "sealed" && event.kind !== "verified") {
    issues.push(`unknown event kind: ${event.kind}`);
    return;
  }
  if (!isBase58PublicKey(event.writer)) {
    issues.push(`${event.kind} event has invalid writer public key`);
  }
  if (!isHashHex(event.sessionHash)) {
    issues.push(`${event.kind} event has invalid sessionHash`);
  }
  if (!Number.isSafeInteger(event.utcDay) || event.utcDay < 0) {
    issues.push(`${event.kind} event has invalid utcDay`);
  }
  if (!isSolanaSignature(event.signature)) {
    issues.push(`${event.kind} event does not include a real 64-byte Solana signature`);
  }
  if (event.failed === true || event.status === "failed" || event.commitment === "failed") {
    issues.push(`${event.kind} event is failed and must not be present in a launch snapshot`);
  }
  if (!options.allowNonFinalizedEvents && !eventIsFinalized(event)) {
    issues.push(`${event.kind} event is not finalized`);
  }
  if (event.kind === "sealed" && !isBase58PublicKey(event.loomAsset)) {
    issues.push("sealed event has invalid loomAsset");
  }
  if (event.kind === "verified") {
    if (!isHashHex(event.proofHash)) {
      issues.push("verified event has invalid proofHash");
    }
    if (!isBase58PublicKey(event.verifier)) {
      issues.push("verified event has invalid verifier");
    }
    if (!Number.isSafeInteger(event.protocolVersion)) {
      issues.push("verified event has invalid protocolVersion");
    }
  }
}

function recomputeScores(events, { maxParticipants, proofVerifierAuthority, requireFinalized }) {
  const usable = events.filter((event) => eventIsUsable(event, requireFinalized));
  const sealedEvents = dedupeEvents(usable.filter((event) => event.kind === "sealed"));
  const verifiedUsable = usable.filter((event) => event.kind === "verified");
  const verifiedEvents = dedupeEvents(
    verifiedUsable.filter(
      (event) => event.verifier === proofVerifierAuthority && event.protocolVersion === 1,
    ),
  );
  const rejectedVerifiedEvents = verifiedUsable.length - verifiedEvents.length;
  const wallets = new Map();

  for (const event of sealedEvents) {
    const wallet = ensureWallet(wallets, event.writer);
    const key = `${event.utcDay}`;
    const existing = wallet.sealedDays.get(key);
    if (existing == null || compareEventFreshness(event, existing) > 0) {
      wallet.sealedDays.set(key, event);
    }
  }

  for (const event of verifiedEvents) {
    const wallet = ensureWallet(wallets, event.writer);
    const key = `${event.utcDay}:${event.sessionHash}`;
    const existing = wallet.verifiedSeals.get(key);
    if (existing == null || compareEventFreshness(event, existing) > 0) {
      wallet.verifiedSeals.set(key, event);
    }
  }

  const uncappedScores = [...wallets.entries()]
    .map(([wallet, state]) => {
      const sealedDays = [...state.sealedDays.keys()].map(Number).sort((a, b) => a - b);
      const verifiedDays = new Set(
        [...state.verifiedSeals.values()]
          .filter((event) => state.sealedDays.get(`${event.utcDay}`)?.sessionHash === event.sessionHash)
          .map((event) => event.utcDay),
      );
      const uniqueSealDays = sealedDays.length;
      const verifiedSealDays = verifiedDays.size;
      const streakBonus = computeStreakBonus(sealedDays);
      return {
        sealedDays,
        score: uniqueSealDays + (2 * verifiedSealDays) + streakBonus,
        streakBonus,
        uniqueSealDays,
        verifiedSealDays,
        wallet,
      };
    })
    .filter((row) => row.score > 0)
    .sort((a, b) => b.score - a.score || a.wallet.localeCompare(b.wallet));
  const scores = uncappedScores.slice(0, maxParticipants);

  return {
    scores,
    summary: {
      excludedByParticipantCap: uncappedScores.length - scores.length,
      scoreRows: scores.length,
      sealedEvents: sealedEvents.length,
      totalScore: scores.reduce((sum, row) => sum + row.score, 0),
      uncappedScoreRows: uncappedScores.length,
      verifiedEvents: verifiedEvents.length,
      rejectedVerifiedEvents,
    },
  };
}

function compareScores(actual, expected, issues) {
  const actualComparable = actual.map((row) => ({
    sealedDays: row.sealedDays,
    score: row.score,
    streakBonus: row.streakBonus,
    uniqueSealDays: row.uniqueSealDays,
    verifiedSealDays: row.verifiedSealDays,
    wallet: row.wallet,
  }));
  if (JSON.stringify(actualComparable) !== JSON.stringify(expected)) {
    issues.push("scores do not recompute from finalized public events under Score V1");
  }
  for (let index = 1; index < actual.length; index += 1) {
    const previous = actual[index - 1];
    const current = actual[index];
    if (previous.score < current.score || (previous.score === current.score && previous.wallet > current.wallet)) {
      issues.push("scores are not sorted by score desc, then wallet asc");
      break;
    }
  }
}

function compareSummary(summary, recomputed, issues) {
  for (const key of [
    "excludedByParticipantCap",
    "scoreRows",
    "sealedEvents",
    "totalScore",
    "uncappedScoreRows",
    "verifiedEvents",
    "rejectedVerifiedEvents",
  ]) {
    if (summary[key] !== recomputed.summary[key]) {
      issues.push(`summary.${key} does not match recomputed Score V1 value`);
    }
  }
}

function auditAllocation(scores, snapshot, options, issues) {
  if (snapshot.allocationRule == null) {
    return;
  }
  const rewardPoolRaw = parseBigIntMaybe(snapshot.summary?.rewardPoolRaw);
  if (rewardPoolRaw == null) {
    issues.push("allocationRule exists but summary.rewardPoolRaw is invalid");
    return;
  }
  let totalAllocated = 0n;
  for (const score of scores) {
    const allocation = parseBigIntMaybe(score.rewardAllocationRaw);
    if (allocation == null) {
      issues.push(`score row for ${score.wallet} is missing rewardAllocationRaw`);
      continue;
    }
    totalAllocated += allocation;
  }
  if (totalAllocated !== rewardPoolRaw) {
    issues.push("sum of score rewardAllocationRaw values does not equal summary.rewardPoolRaw");
  }
  if (options.requireAllocation && scores.length > 0 && rewardPoolRaw <= 0n) {
    issues.push("required reward allocation has a zero reward pool");
  }
}

function scanForPrivateFields(value, path, issues) {
  if (Array.isArray(value)) {
    value.forEach((item, index) => scanForPrivateFields(item, [...path, String(index)], issues));
    return;
  }
  if (value == null || typeof value !== "object") {
    if (typeof value === "string" && looksLikeCompleteAnkyPlaintext(value)) {
      issues.push(`complete .anky plaintext-like value is present at ${path.join(".") || "<root>"}`);
    }
    return;
  }
  for (const [key, nested] of Object.entries(value)) {
    const nestedPath = [...path, key];
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

function eventIsUsable(event, requireFinalized) {
  if (!isSolanaSignature(event.signature)) {
    return false;
  }
  if (event.failed === true || event.status === "failed" || event.commitment === "failed") {
    return false;
  }
  return !requireFinalized || eventIsFinalized(event);
}

function eventIsFinalized(event) {
  return event.finalized === true || event.commitment === "finalized" || event.status === "finalized";
}

function dedupeEvents(events) {
  const byKey = new Map();
  for (const event of events) {
    const key = `${event.kind}:${event.writer}:${event.sessionHash}:${event.utcDay}:${event.signature ?? ""}`;
    const existing = byKey.get(key);
    if (existing == null || compareEventFreshness(event, existing) > 0) {
      byKey.set(key, event);
    }
  }
  return [...byKey.values()];
}

function compareEventFreshness(left, right) {
  return (
    Number(left.slot ?? 0) - Number(right.slot ?? 0) ||
    Number(left.blockTime ?? left.timestamp ?? 0) - Number(right.blockTime ?? right.timestamp ?? 0)
  );
}

function ensureWallet(wallets, wallet) {
  if (!wallets.has(wallet)) {
    wallets.set(wallet, {
      sealedDays: new Map(),
      verifiedSeals: new Map(),
    });
  }
  return wallets.get(wallet);
}

function computeStreakBonus(sortedDays) {
  if (sortedDays.length === 0) {
    return 0;
  }
  let bonus = 0;
  let run = 1;
  for (let index = 1; index < sortedDays.length; index += 1) {
    if (sortedDays[index] === sortedDays[index - 1] + 1) {
      run += 1;
      continue;
    }
    bonus += Math.floor(run / 7) * 2;
    run = 1;
  }
  return bonus + Math.floor(run / 7) * 2;
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
  let decoded = 0n;
  for (const char of value) {
    const digit = BASE58_ALPHABET.indexOf(char);
    if (digit < 0) {
      throw new Error("invalid base58 character");
    }
    decoded = decoded * 58n + BigInt(digit);
  }
  const bytes = [];
  while (decoded > 0n) {
    bytes.push(Number(decoded & 0xffn));
    decoded >>= 8n;
  }
  for (const char of value) {
    if (char !== "1") {
      break;
    }
    bytes.push(0);
  }
  return Uint8Array.from(bytes.reverse());
}

function resolvePublicKey(value, fallback, label) {
  const resolved = typeof value === "string" && value.trim().length > 0 ? value.trim() : fallback;
  if (!isBase58PublicKey(resolved)) {
    throw new Error(`${label} must be a base58 Solana public key.`);
  }
  return resolved;
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

function printUsage() {
  console.log(`Audits a Sojourn 9 score snapshot produced by ankySealIndexer.mjs.

Usage:
  node solana/scripts/indexer/auditScoreSnapshot.mjs --snapshot sojourn9/devnet-score-snapshot.json [options]

Options:
  --allow-inferred-finality      Permit events marked finalized by a finalized Helius backfill request.
  --allow-non-finalized-events   Permit non-finalized events for live UI snapshots. Do not use for rewards.
  --max-participants <n>         Maximum published participant cap. Defaults to ${DEFAULT_MAX_PARTICIPANTS}.
  --proof-verifier <pubkey>      Expected Sojourn 9 proof verifier authority.
  --require-allocation           Require token allocation fields and nonzero reward pool.
  --reward-bps <bps>             Expected reward basis points. Defaults to ${DEFAULT_REWARD_BPS}.
  --snapshot <path>              Required indexer snapshot JSON.

The audit reads only public snapshot JSON and rejects private/plaintext-like fields.`);
}

function resolvePublicSnapshotPath(value) {
  if (SECRET_PATH_RE.test(value)) {
    throw new Error("--snapshot must point to a public JSON file, not an env/.anky/keypair/wallet/deployer file.");
  }
  const snapshotPath = path.resolve(value);
  if (SECRET_PATH_RE.test(snapshotPath)) {
    throw new Error("--snapshot must point to a public JSON file, not an env/.anky/keypair/wallet/deployer file.");
  }

  return snapshotPath;
}

function requiredArg(args, name) {
  const value = args[name];
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`--${name} is required.`);
  }
  return value.trim();
}

function parsePositiveInteger(value, label) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new Error(`${label} must be a positive integer.`);
  }
  return parsed;
}

function parseBasisPoints(value, label) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 0 || parsed > 10_000) {
    throw new Error(`${label} must be an integer from 0 to 10000.`);
  }
  return parsed;
}

function parseBigIntMaybe(value) {
  if (typeof value !== "string" || !/^\d+$/.test(value)) {
    return null;
  }
  return BigInt(value);
}

function toCamel(flag) {
  return flag
    .replace(/^--/, "")
    .replace(/-([a-z])/g, (_match, char) => char.toUpperCase());
}
