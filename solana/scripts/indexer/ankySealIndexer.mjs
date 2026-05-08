#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { redactSecretValues } from "../sojourn9/redactSecrets.mjs";

const DEFAULT_PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const DEFAULT_PROOF_VERIFIER_AUTHORITY = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const DEFAULT_CLUSTER = "devnet";
const DEFAULT_LIMIT = 100;
const DEFAULT_RPC_RETRIES = 3;
const DEFAULT_MAX_PARTICIPANTS = 3_456;
const DEFAULT_REWARD_BPS = 800;
const SECONDS_PER_DAY = 86_400;
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

const EVENT_DISCRIMINATORS = {
  sealed: discriminator("event:AnkySealed").toString("hex"),
  verified: discriminator("event:AnkyVerified").toString("hex"),
};
const INSTRUCTION_DISCRIMINATORS = {
  sealAnky: discriminator("global:seal_anky").toString("hex"),
  recordVerifiedAnky: discriminator("global:record_verified_anky").toString("hex"),
};
const BOOLEAN_FLAGS = new Set(["--backfill", "--include-non-finalized"]);
const VALUE_FLAGS = new Set([
  "--backend-url",
  "--before",
  "--cluster",
  "--core-collection",
  "--input",
  "--limit",
  "--max-participants",
  "--out",
  "--program-id",
  "--proof-verifier",
  "--reward-bps",
  "--signature",
  "--token-supply",
]);
const SECRET_PATH_RE =
  /(^|[/\\])\.env(?:[./\\]|$)|(^|[/\\])id\.json$|\.anky$|keypair|deployer|wallet|\.pem$/i;

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

  const programIdSource = firstNonempty(args.programId, process.env.ANKY_SEAL_PROGRAM_ID);
  const verifierSource = firstNonempty(
    args.proofVerifier,
    process.env.ANKY_PROOF_VERIFIER_AUTHORITY,
  );
  const cluster = normalizeCluster(
    firstNonempty(args.cluster, process.env.ANKY_SOLANA_CLUSTER, DEFAULT_CLUSTER),
  );
  requireExplicitMainnetConfig({ cluster, programIdSource, verifierSource });
  const programId = resolveBase58PublicKey(programIdSource, DEFAULT_PROGRAM_ID, "program ID");
  const proofVerifierAuthority = resolveBase58PublicKey(
    verifierSource,
    DEFAULT_PROOF_VERIFIER_AUTHORITY,
    "proof verifier authority",
  );
  const inputPath =
    typeof args.input === "string" ? resolvePublicPath(args.input, "--input") : args.input;
  const outputPath =
    typeof args.out === "string" ? resolvePublicPath(args.out, "--out") : args.out;
  const backendUrl = normalizeBackendUrl(firstNonempty(args.backendUrl));
  const coreCollection = firstNonempty(args.coreCollection, process.env.ANKY_CORE_COLLECTION);
  const requireFinalized = args.requireFinalized !== false;
  const rewardBps =
    args.rewardBps == null ? DEFAULT_REWARD_BPS : parseBasisPoints(args.rewardBps, "reward bps");
  const maxParticipants =
    args.maxParticipants == null
      ? DEFAULT_MAX_PARTICIPANTS
      : parsePositiveInteger(args.maxParticipants, "max participants");
  const tokenSupplyRaw =
    args.tokenSupply == null ? null : parseNonNegativeBigInt(args.tokenSupply, "token supply");

  const payloads = [];
  if (typeof inputPath === "string") {
    payloads.push(JSON.parse(fs.readFileSync(inputPath, "utf8")));
  }

  if (args.backfill === true) {
    payloads.push(
      await fetchProgramTransactions({
        before: args.before,
        cluster,
        limit: Number(args.limit ?? DEFAULT_LIMIT),
        programId,
      }),
    );
  }
  const signatures = parseSignatureList(args.signature);
  if (signatures.length > 0) {
    payloads.push(
      await fetchKnownSignatureTransactions({
        cluster,
        signatures,
      }),
    );
  }

  if (payloads.length === 0) {
    throw new Error("Provide --input <json>, --backfill, or --signature <tx_signature>.");
  }

  const events = payloads.flatMap((payload) => extractEvents(payload, programId));
  const snapshot = buildSnapshot(events, {
    maxParticipants,
    proofVerifierAuthority,
    requireFinalized,
    rewardBps,
    tokenSupplyRaw,
  });
  const backendPosts =
    typeof backendUrl === "string"
      ? await postBackendEvents({
          backendUrl,
          coreCollection,
          events: snapshot.events.filter((event) =>
            eventAcceptedForBackend(event, { proofVerifierAuthority, requireFinalized }),
          ),
        })
      : [];
  const failedBackendPost = backendPosts.find((post) => !post.ok);
  if (failedBackendPost != null) {
    throw new Error(`Backend metadata upsert failed: ${JSON.stringify(failedBackendPost)}`);
  }
  const output = {
    generatedAt: new Date().toISOString(),
    programId,
    cluster,
    proofVerifierAuthority,
    requireFinalized,
    backendPosts,
    ...snapshot,
  };

  if (typeof outputPath === "string") {
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    fs.writeFileSync(outputPath, `${JSON.stringify(output, null, 2)}\n`);
    console.log(`wrote ${outputPath}`);
  } else {
    console.log(JSON.stringify(output, null, 2));
  }
}

function extractEvents(payload, programId) {
  if (Array.isArray(payload)) {
    return payload.flatMap((item) => extractEvents(item, programId));
  }

  if (payload == null || typeof payload !== "object") {
    return [];
  }

  const storedWebhookPayload = parseStoredWebhookPayload(payload);
  if (storedWebhookPayload != null) {
    return extractEvents(storedWebhookPayload, programId);
  }

  if (Array.isArray(payload.decodedEvents)) {
    return payload.decodedEvents.map(normalizeDecodedEvent).filter(Boolean);
  }

  const publicMetadataEvent = normalizeDecodedEvent(payload);
  if (publicMetadataEvent != null) {
    return [publicMetadataEvent];
  }

  if (payload.verifiedSeal != null || payload.seal != null) {
    return [payload.seal, payload.verifiedSeal]
      .map((event) =>
        normalizeDecodedEvent({
          ...event,
          utcDay: event?.utcDay ?? payload.utcDay,
        }),
      )
      .filter(Boolean);
  }

  const transactionItems = Array.isArray(payload.transactions)
    ? payload.transactions
    : Array.isArray(payload.result)
      ? payload.result
      : [payload];

  return transactionItems.flatMap((transaction) => {
    const envelope = transactionEnvelope(transaction);
    const logEvents = collectProgramDataLogs(transaction, programId).flatMap((encoded) => {
      const event = decodeAnchorEvent(encoded);
      if (event == null) {
        return [];
      }

      return [
        {
          ...event,
          blockTime: envelope.blockTime,
          commitment: envelope.commitment,
          failed: envelope.failed,
          finalitySource: envelope.finalitySource,
          finalized: envelope.finalized,
          signature: envelope.signature,
          slot: envelope.slot,
        },
      ];
    });
    if (logEvents.length > 0) {
      return logEvents;
    }

    return collectProgramInstructions(transaction, programId).flatMap((instruction) => {
      const event = decodeAnchorInstruction(instruction, programId);
      if (event == null) {
        return [];
      }

      return [
        {
          ...event,
          blockTime: envelope.blockTime,
          commitment: envelope.commitment,
          failed: envelope.failed,
          finalitySource: envelope.finalitySource,
          finalized: envelope.finalized,
          signature: envelope.signature,
          slot: envelope.slot,
        },
      ];
    });
  });
}

function parseStoredWebhookPayload(payload) {
  const value = payload.payloadJson ?? payload.payload_json;
  if (typeof value !== "string" || value.trim().length === 0) {
    return null;
  }

  try {
    return JSON.parse(value);
  } catch (_error) {
    return null;
  }
}

function normalizeDecodedEvent(event) {
  if (event == null || typeof event !== "object") {
    return null;
  }

  const kind = event.kind ?? inferEventKind(event);
  if (kind !== "sealed" && kind !== "verified") {
    return null;
  }
  const writer = event.writer ?? event.wallet;
  const sessionHash = normalizeHashHex(event.sessionHash ?? event.session_hash);
  if (!isBase58PublicKey(writer) || !isHashHex(sessionHash)) {
    return null;
  }

  const utcDay = toOptionalSafeInteger(event.utcDay ?? event.utc_day);
  if (utcDay == null) {
    return null;
  }

  const signature = event.signature ?? event.txSignature ?? event.proofTxSignature;
  const blockTime = toOptionalSafeInteger(event.blockTime ?? event.timestamp);
  const status = event.status ?? event.commitment;

  if (kind === "sealed") {
    const loomAsset = event.loomAsset ?? event.loom_asset;
    if (!isBase58PublicKey(loomAsset)) {
      return null;
    }
    const rollingRoot = normalizeHashHex(event.rollingRoot ?? event.rolling_root);
    if (rollingRoot != null && !isHashHex(rollingRoot)) {
      return null;
    }

    return {
      ...event,
      blockTime,
      commitment: event.commitment,
      finalized: event.finalized === true || status === "finalized",
      kind,
      loomAsset,
      rollingRoot,
      sessionHash,
      signature,
      slot: toOptionalSafeInteger(event.slot),
      status,
      utcDay,
      writer,
    };
  }

  if (kind === "verified") {
    const proofHash = normalizeHashHex(event.proofHash ?? event.proof_hash);
    const protocolVersion = toOptionalSafeInteger(
      event.protocolVersion ?? event.protocol_version,
    );
    if (
      !isHashHex(proofHash) ||
      !isBase58PublicKey(event.verifier) ||
      protocolVersion == null
    ) {
      return null;
    }

    return {
      ...event,
      blockTime,
      commitment: event.commitment,
      finalized: event.finalized === true || status === "finalized",
      kind,
      proofHash,
      protocolVersion,
      sessionHash,
      signature,
      slot: toOptionalSafeInteger(event.slot),
      status,
      utcDay,
      verifier: event.verifier,
      writer,
    };
  }

  return null;
}

function inferEventKind(event) {
  if (
    event.proofHash != null ||
    event.proof_hash != null ||
    event.proofTxSignature != null ||
    event.protocolVersion != null ||
    event.protocol_version != null
  ) {
    return "verified";
  }
  if (event.loomAsset != null || event.loom_asset != null) {
    return "sealed";
  }

  return null;
}

function transactionEnvelope(transaction) {
  const signature =
    transaction.signature ??
    transaction.transaction?.signatures?.[0] ??
    transaction.transaction?.transaction?.signatures?.[0] ??
    transaction.meta?.signature ??
    null;
  const slot = toOptionalSafeInteger(transaction.slot ?? transaction.context?.slot);
  const blockTime = toOptionalSafeInteger(
    transaction.blockTime ?? transaction.timestamp ?? transaction.meta?.blockTime,
  );
  const commitment =
    transaction.commitment ??
    transaction.confirmationStatus ??
    transaction.status ??
    transaction.meta?.confirmationStatus ??
    null;

  return {
    blockTime,
    commitment,
    failed: transactionFailed(transaction),
    finalitySource: transaction.finalitySource ?? null,
    finalized: transaction.finalized === true || commitment === "finalized",
    signature,
    slot,
  };
}

function transactionFailed(transaction) {
  if (transaction == null || typeof transaction !== "object") {
    return false;
  }

  return (
    transaction.failed === true ||
    transaction.status === "failed" ||
    transaction.confirmationStatus === "failed" ||
    transaction.transactionError != null ||
    transaction.error != null ||
    transaction.err != null ||
    transaction.meta?.err != null ||
    transaction.transaction?.meta?.err != null ||
    transaction.raw?.meta?.err != null ||
    transaction.rawTransaction?.meta?.err != null
  );
}

function collectLogMessages(transaction) {
  const candidates = [
    transaction.logMessages,
    transaction.logs,
    transaction.meta?.logMessages,
    transaction.transaction?.meta?.logMessages,
    transaction.raw?.meta?.logMessages,
    transaction.rawTransaction?.meta?.logMessages,
  ];

  return candidates.flatMap((value) => (Array.isArray(value) ? value : []));
}

function collectProgramDataLogs(transaction, programId) {
  const stack = [];
  const dataLogs = [];

  for (const line of collectLogMessages(transaction)) {
    if (typeof line !== "string") {
      continue;
    }

    const invoke = line.match(/^Program ([1-9A-HJ-NP-Za-km-z]+) invoke \[\d+\]$/);
    if (invoke != null) {
      stack.push(invoke[1]);
      continue;
    }

    const completed = line.match(/^Program ([1-9A-HJ-NP-Za-km-z]+) (success|failed: .*)$/);
    if (completed != null) {
      const index = stack.lastIndexOf(completed[1]);
      if (index >= 0) {
        stack.splice(index, 1);
      }
      continue;
    }

    const prefix = "Program data: ";
    if (line.startsWith(prefix) && stack[stack.length - 1] === programId) {
      dataLogs.push(line.slice(prefix.length).trim());
    }
  }

  return dataLogs;
}

function collectProgramInstructions(transaction, programId) {
  const instructions = [];
  const candidates = [
    transaction.instructions,
    transaction.transaction?.message?.instructions,
    transaction.transaction?.transaction?.message?.instructions,
    transaction.raw?.instructions,
    transaction.raw?.transaction?.message?.instructions,
    transaction.rawTransaction?.instructions,
    transaction.rawTransaction?.transaction?.message?.instructions,
  ];

  for (const candidate of candidates) {
    collectProgramInstructionsInto(candidate, programId, instructions);
  }

  return instructions;
}

function collectProgramInstructionsInto(value, programId, instructions) {
  if (!Array.isArray(value)) {
    return;
  }

  for (const instruction of value) {
    if (instruction == null || typeof instruction !== "object") {
      continue;
    }
    if (instruction.programId === programId) {
      instructions.push(instruction);
    }
    collectProgramInstructionsInto(instruction.innerInstructions, programId, instructions);
    collectProgramInstructionsInto(instruction.instructions, programId, instructions);
  }
}

function decodeAnchorEvent(encoded) {
  const buffer = Buffer.from(encoded, "base64");
  const eventDiscriminator = buffer.subarray(0, 8).toString("hex");

  if (eventDiscriminator === EVENT_DISCRIMINATORS.sealed) {
    return decodeAnkySealed(buffer);
  }

  if (eventDiscriminator === EVENT_DISCRIMINATORS.verified) {
    return decodeAnkyVerified(buffer);
  }

  return null;
}

function decodeAnchorInstruction(instruction, programId) {
  if (instruction?.programId !== programId) {
    return null;
  }
  const buffer = decodeInstructionData(instruction.data);
  if (buffer == null || buffer.length < 8) {
    return null;
  }

  const instructionDiscriminator = buffer.subarray(0, 8).toString("hex");
  if (instructionDiscriminator === INSTRUCTION_DISCRIMINATORS.sealAnky) {
    return decodeSealAnkyInstruction(buffer, instruction.accounts);
  }
  if (instructionDiscriminator === INSTRUCTION_DISCRIMINATORS.recordVerifiedAnky) {
    return decodeRecordVerifiedAnkyInstruction(buffer, instruction.accounts);
  }

  return null;
}

function decodeInstructionData(value) {
  if (typeof value !== "string" || value.trim().length === 0) {
    return null;
  }
  const trimmed = value.trim();

  try {
    return base58Decode(trimmed);
  } catch (_error) {
    try {
      return Buffer.from(trimmed, "base64");
    } catch (_nestedError) {
      return null;
    }
  }
}

function decodeSealAnkyInstruction(buffer, accounts) {
  if (buffer.length < 48) {
    return null;
  }
  const accountKeys = normalizeInstructionAccounts(accounts);
  const hasSeparatePayer = accountKeys.length === 3 || accountKeys.length >= 8;
  const writer = accountKeys[0];
  const payer = hasSeparatePayer ? accountKeys[1] : writer;
  const loomAsset = hasSeparatePayer ? accountKeys[2] : accountKeys[1];
  if (!isBase58PublicKey(writer) || !isBase58PublicKey(payer) || !isBase58PublicKey(loomAsset)) {
    return null;
  }

  return {
    kind: "sealed",
    loomAsset,
    payer,
    sessionHash: readHash(buffer, 8),
    utcDay: readI64(buffer, 40),
    writer,
  };
}

function decodeRecordVerifiedAnkyInstruction(buffer, accounts) {
  if (buffer.length < 82) {
    return null;
  }
  const accountKeys = normalizeInstructionAccounts(accounts);
  const verifier = accountKeys[0];
  const writer = accountKeys[1];
  if (!isBase58PublicKey(verifier) || !isBase58PublicKey(writer)) {
    return null;
  }

  return {
    kind: "verified",
    proofHash: readHash(buffer, 48),
    protocolVersion: buffer.readUInt16LE(80),
    sessionHash: readHash(buffer, 8),
    utcDay: readI64(buffer, 40),
    verifier,
    writer,
  };
}

function normalizeInstructionAccounts(accounts) {
  if (!Array.isArray(accounts)) {
    return [];
  }

  return accounts.map((account) => {
    if (typeof account === "string") {
      return account;
    }
    if (account != null && typeof account === "object") {
      return account.pubkey ?? account.account ?? account.address ?? null;
    }

    return null;
  });
}

function decodeAnkySealed(buffer) {
  if (buffer.length < 152) {
    return null;
  }

  let offset = 8;
  const writer = readPubkey(buffer, offset);
  offset += 32;
  const loomAsset = readPubkey(buffer, offset);
  offset += 32;
  const sessionHash = readHash(buffer, offset);
  offset += 32;
  const utcDay = readI64(buffer, offset);
  offset += 8;
  const totalSeals = readU64(buffer, offset);
  offset += 8;
  const rollingRoot = readHash(buffer, offset);
  offset += 32;
  const timestamp = buffer.length >= offset + 8 ? readI64(buffer, offset) : null;

  return {
    kind: "sealed",
    loomAsset,
    rollingRoot,
    sessionHash,
    timestamp,
    totalSeals,
    utcDay,
    writer,
  };
}

function decodeAnkyVerified(buffer) {
  if (buffer.length < 146) {
    return null;
  }

  let offset = 8;
  const writer = readPubkey(buffer, offset);
  offset += 32;
  const sessionHash = readHash(buffer, offset);
  offset += 32;
  const utcDay = readI64(buffer, offset);
  offset += 8;
  const proofHash = readHash(buffer, offset);
  offset += 32;
  const verifier = readPubkey(buffer, offset);
  offset += 32;
  const protocolVersion = buffer.readUInt16LE(offset);
  offset += 2;
  const timestamp = buffer.length >= offset + 8 ? readI64(buffer, offset) : null;

  return {
    kind: "verified",
    proofHash,
    protocolVersion,
    sessionHash,
    timestamp,
    utcDay,
    verifier,
    writer,
  };
}

function buildSnapshot(
  events,
  { maxParticipants, proofVerifierAuthority, requireFinalized, rewardBps, tokenSupplyRaw },
) {
  const sealedEvents = dedupeEvents(
    events.filter((event) => event.kind === "sealed" && eventIsUsable(event, requireFinalized)),
  );
  const verifiedEvents = dedupeEvents(
    events.filter(
      (event) =>
        event.kind === "verified" &&
        eventIsUsable(event, requireFinalized) &&
        eventMatchesVerifiedPolicy(event, proofVerifierAuthority),
    ),
  );
  const rejectedVerifiedEvents = events.filter(
    (event) =>
      event.kind === "verified" &&
      eventIsUsable(event, requireFinalized) &&
      !eventMatchesVerifiedPolicy(event, proofVerifierAuthority),
  );
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

  const scores = [...wallets.entries()]
    .map(([wallet, state]) => {
      const sealedDays = [...state.sealedDays.keys()].map(Number).sort((a, b) => a - b);
      const verifiedDays = new Set(
        [...state.verifiedSeals.values()]
          .filter((event) => {
            const sealed = state.sealedDays.get(`${event.utcDay}`);

            return sealed != null && sealed.sessionHash === event.sessionHash;
          })
          .map((event) => event.utcDay),
      );
      const streakBonus = computeStreakBonus(sealedDays);
      const uniqueSealDays = sealedDays.length;
      const verifiedSealDays = verifiedDays.size;

      return {
        wallet,
        uniqueSealDays,
        verifiedSealDays,
        streakBonus,
        score: uniqueSealDays + (2 * verifiedSealDays) + streakBonus,
        sealedDays,
      };
    })
    .filter((row) => row.score > 0)
    .sort((a, b) => b.score - a.score || a.wallet.localeCompare(b.wallet));

  const participantScores = scores.slice(0, maxParticipants);
  const totalScore = participantScores.reduce((sum, row) => sum + row.score, 0);
  const allocation =
    tokenSupplyRaw == null
      ? null
      : computeRewardAllocations({
          rewardBps,
          scores: participantScores,
          tokenSupplyRaw,
          totalScore,
        });

  return {
    allocationRule: allocation?.rule,
    events,
    scoringRule: {
      version: 1,
      uniqueFinalizedDailySeal: 1,
      finalizedVerifiedSealBonus: 2,
      completedSevenDayStreakBonus: 2,
      maxParticipants,
      formula: "score = unique_seal_days + (2 * verified_seal_days) + streak_bonus",
    },
    scores: participantScores,
    summary: {
      excludedByParticipantCap: scores.length - participantScores.length,
      indexedEvents: events.length,
      participantCap: maxParticipants,
      scoreRows: participantScores.length,
      sealedEvents: sealedEvents.length,
      finalizedEventsInferredFromBackfillRequest: events.filter(
        (event) => event.finalitySource === "requested_finalized_commitment",
      ).length,
      totalScore,
      uncappedScoreRows: scores.length,
      verifiedEvents: verifiedEvents.length,
      rejectedVerifiedEvents: rejectedVerifiedEvents.length,
      rewardPoolRaw: allocation?.rewardPoolRaw,
    },
  };
}

function eventAcceptedForBackend(event, { proofVerifierAuthority, requireFinalized }) {
  if (!eventIsUsable(event, requireFinalized)) {
    return false;
  }
  if (!isSolanaSignature(event.signature)) {
    return false;
  }
  if (event.kind !== "verified") {
    return true;
  }

  return eventMatchesVerifiedPolicy(event, proofVerifierAuthority);
}

function eventMatchesVerifiedPolicy(event, proofVerifierAuthority) {
  return event.protocolVersion === 1 && event.verifier === proofVerifierAuthority;
}

function computeRewardAllocations({ rewardBps, scores, tokenSupplyRaw, totalScore }) {
  const rewardPoolRaw = (tokenSupplyRaw * BigInt(rewardBps)) / 10_000n;
  if (totalScore <= 0 || rewardPoolRaw === 0n) {
    for (const row of scores) {
      row.rewardAllocationRaw = "0";
    }

    return {
      rewardPoolRaw: rewardPoolRaw.toString(),
      rule: allocationRule(rewardBps, tokenSupplyRaw, rewardPoolRaw),
    };
  }

  const totalScoreRaw = BigInt(totalScore);
  const rowsWithRemainders = scores.map((row, index) => {
    const weighted = rewardPoolRaw * BigInt(row.score);
    const base = weighted / totalScoreRaw;
    const remainder = weighted % totalScoreRaw;
    row.rewardAllocationRaw = base.toString();

    return {
      index,
      remainder,
      wallet: row.wallet,
    };
  });

  let distributed = scores.reduce((sum, row) => sum + BigInt(row.rewardAllocationRaw), 0n);
  let leftover = rewardPoolRaw - distributed;
  rowsWithRemainders.sort(
    (left, right) =>
      compareBigIntDesc(left.remainder, right.remainder) || left.wallet.localeCompare(right.wallet),
  );

  for (const row of rowsWithRemainders) {
    if (leftover === 0n) {
      break;
    }
    scores[row.index].rewardAllocationRaw = (
      BigInt(scores[row.index].rewardAllocationRaw) + 1n
    ).toString();
    leftover -= 1n;
  }

  return {
    rewardPoolRaw: rewardPoolRaw.toString(),
    rule: allocationRule(rewardBps, tokenSupplyRaw, rewardPoolRaw),
  };
}

function allocationRule(rewardBps, tokenSupplyRaw, rewardPoolRaw) {
  return {
    version: 1,
    tokenSupplyRaw: tokenSupplyRaw.toString(),
    rewardBps,
    rewardPoolRaw: rewardPoolRaw.toString(),
    formula:
      "reward_pool_raw = token_supply_raw * reward_bps / 10000; wallet_allocation_raw = deterministic floor-plus-remainder share by score",
  };
}

function compareBigIntDesc(left, right) {
  if (left > right) {
    return -1;
  }
  if (left < right) {
    return 1;
  }

  return 0;
}

function eventIsUsable(event, requireFinalized) {
  if (!isSolanaSignature(event.signature)) {
    return false;
  }
  if (event.failed === true || event.status === "failed" || event.commitment === "failed") {
    return false;
  }
  if (!requireFinalized) {
    return true;
  }

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

async function fetchProgramTransactions({ before, cluster, limit, programId }) {
  if (!hasConfiguredHeliusBackfill()) {
    throw new Error(
      "--backfill requires HELIUS_API_KEY or ANKY_SOLANA_RPC_URL pointing at a Helius RPC endpoint. Use --input fixtures when no Helius key is available.",
    );
  }

  return fetchHeliusProgramTransactions({ before, cluster, limit, programId });
}

async function fetchKnownSignatureTransactions({ cluster, signatures }) {
  if (!hasConfiguredHeliusBackfill()) {
    throw new Error(
      "--signature requires HELIUS_API_KEY or ANKY_SOLANA_RPC_URL pointing at a Helius RPC endpoint. Use --input fixtures when no Helius key is available.",
    );
  }

  const requestedCommitment = "finalized";
  const transactions = [];
  for (const signature of signatures) {
    const transaction = await rpc(resolveRpcUrl(cluster), "getTransaction", [
      signature,
      {
        commitment: requestedCommitment,
        encoding: "json",
        maxSupportedTransactionVersion: 0,
      },
    ]);
    if (transaction == null) {
      throw new Error(`getTransaction returned no result for ${signature}`);
    }

    transactions.push(normalizeKnownSignatureTransaction(transaction, {
      requestedCommitment,
      signature,
    }));
  }

  return { transactions };
}

async function fetchHeliusProgramTransactions({ before, cluster, limit, programId }) {
  const requestedCommitment = "finalized";
  const result = await rpc(resolveRpcUrl(cluster), "getTransactionsForAddress", [
    programId,
    {
      commitment: requestedCommitment,
      limit,
      paginationToken: before,
      transactionDetails: "full",
    },
  ]);
  const transactions = Array.isArray(result)
    ? result
    : Array.isArray(result?.transactions)
      ? result.transactions
      : [];

  return {
    paginationToken: typeof result?.paginationToken === "string" ? result.paginationToken : null,
    transactions: transactions.map((transaction) =>
      normalizeBackfillTransaction(transaction, requestedCommitment),
    ),
  };
}

function normalizeKnownSignatureTransaction(transaction, { requestedCommitment, signature }) {
  return {
    ...transaction,
    commitment: requestedCommitment,
    finalized: requestedCommitment === "finalized",
    finalitySource: "known_signature_finalized_getTransaction",
    signature:
      transaction.signature ??
      transaction.transaction?.signatures?.[0] ??
      transaction.transaction?.transaction?.signatures?.[0] ??
      signature,
  };
}

function normalizeBackfillTransaction(transaction, requestedCommitment) {
  const responseCommitment = transaction.commitment ?? transaction.confirmationStatus ?? null;
  const inferredFinalized = responseCommitment == null && requestedCommitment === "finalized";
  const commitment = responseCommitment ?? (inferredFinalized ? requestedCommitment : null);

  return {
    ...transaction,
    commitment,
    finalitySource: inferredFinalized ? "requested_finalized_commitment" : "response_commitment",
    finalized:
      transaction.finalized === true ||
      responseCommitment === "finalized" ||
      inferredFinalized,
    signature:
      transaction.signature ??
      transaction.transaction?.signatures?.[0] ??
      transaction.transaction?.transaction?.signatures?.[0],
  };
}

async function postBackendEvents({ backendUrl, coreCollection, events }) {
  if (typeof coreCollection !== "string" || coreCollection.trim().length === 0) {
    throw new Error("--core-collection or ANKY_CORE_COLLECTION is required with --backend-url.");
  }
  if (!hasIndexerWriteSecret()) {
    throw new Error("ANKY_INDEXER_WRITE_SECRET is required with --backend-url.");
  }

  const results = [];

  for (const event of events.filter((candidate) => candidate.kind === "sealed")) {
    const response = await postJson(backendUrl, "/api/mobile/seals/record", {
      blockTime: event.blockTime ?? event.timestamp,
      coreCollection,
      loomAsset: event.loomAsset,
      sessionHash: event.sessionHash,
      signature: event.signature,
      slot: event.slot,
      status: event.finalized ? "finalized" : "confirmed",
      utcDay: event.utcDay,
      wallet: event.writer,
    });

    results.push({
      body: response.body,
      kind: "sealed",
      ok: response.ok,
      signature: event.signature,
      status: response.status,
    });
  }

  for (const event of events.filter((candidate) => candidate.kind === "verified")) {
    const response = await postJson(backendUrl, "/api/mobile/seals/verified/record", {
      blockTime: event.blockTime ?? event.timestamp,
      proofHash: event.proofHash,
      protocolVersion: event.protocolVersion,
      sessionHash: event.sessionHash,
      signature: event.signature,
      slot: event.slot,
      status: event.finalized ? "finalized" : "confirmed",
      utcDay: event.utcDay,
      verifier: event.verifier,
      wallet: event.writer,
    });

    results.push({
      body: response.body,
      kind: "verified",
      ok: response.ok,
      signature: event.signature,
      status: response.status,
    });
  }

  return results;
}

async function postJson(backendUrl, endpointPath, body) {
  const headers = { "content-type": "application/json" };
  headers["x-anky-indexer-secret"] = process.env.ANKY_INDEXER_WRITE_SECRET.trim();

  const response = await fetch(`${backendUrl.replace(/\/+$/, "")}${endpointPath}`, {
    body: JSON.stringify(body),
    headers,
    method: "POST",
  });

  return {
    body: await response.text(),
    ok: response.ok,
    status: response.status,
  };
}

async function rpc(rpcUrl, method, params) {
  const maxAttempts = Number(process.env.ANKY_INDEXER_RPC_RETRIES ?? DEFAULT_RPC_RETRIES);
  let lastError = null;

  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      const response = await fetch(rpcUrl, {
        body: JSON.stringify({
          id: crypto.randomUUID(),
          jsonrpc: "2.0",
          method,
          params,
        }),
        headers: { "content-type": "application/json" },
        method: "POST",
      });

      if (!response.ok) {
        const body = await response.text();
        if (shouldRetryHttpStatus(response.status) && attempt < maxAttempts) {
          await waitForRetry(attempt);
          continue;
        }

        throw new Error(`${method} failed with HTTP ${response.status}: ${body}`);
      }

      const json = await response.json();
      if (json.error != null) {
        const code = Number(json.error.code);
        if (shouldRetryRpcCode(code) && attempt < maxAttempts) {
          await waitForRetry(attempt);
          continue;
        }

        throw new Error(`${method} failed: ${json.error.message ?? JSON.stringify(json.error)}`);
      }

      return json.result;
    } catch (error) {
      lastError = error;
      if (attempt >= maxAttempts) {
        break;
      }

      await waitForRetry(attempt);
    }
  }

  throw lastError ?? new Error(`${method} failed.`);
}

function shouldRetryHttpStatus(status) {
  return status === 429 || status >= 500;
}

function shouldRetryRpcCode(code) {
  return code === 429 || code === -32005 || code === -32004;
}

function waitForRetry(attempt) {
  const baseMs = Number(process.env.ANKY_INDEXER_RETRY_BASE_MS ?? "250");
  const delayMs = Math.max(0, baseMs) * 2 ** Math.max(0, attempt - 1);

  return new Promise((resolve) => {
    setTimeout(resolve, delayMs);
  });
}

function resolveRpcUrl(cluster) {
  if (process.env.ANKY_SOLANA_RPC_URL != null && process.env.ANKY_SOLANA_RPC_URL.trim() !== "") {
    return process.env.ANKY_SOLANA_RPC_URL.trim();
  }

  if (process.env.HELIUS_API_KEY != null && process.env.HELIUS_API_KEY.trim() !== "") {
    const host = cluster === "mainnet-beta" ? "mainnet" : "devnet";
    return `https://${host}.helius-rpc.com/?api-key=${process.env.HELIUS_API_KEY.trim()}`;
  }

  throw new Error(
    "HELIUS_API_KEY or ANKY_SOLANA_RPC_URL is required for Helius backfill.",
  );
}

function normalizeCluster(value) {
  const cluster = firstNonempty(value) ?? DEFAULT_CLUSTER;
  if (cluster !== "devnet" && cluster !== "mainnet-beta") {
    throw new Error("--cluster must be devnet or mainnet-beta.");
  }

  return cluster;
}

function requireExplicitMainnetConfig({ cluster, programIdSource, verifierSource }) {
  if (cluster !== "mainnet-beta") {
    return;
  }
  if (firstNonempty(programIdSource) == null) {
    throw new Error(
      "mainnet-beta indexing requires an explicit --program-id or ANKY_SEAL_PROGRAM_ID; the devnet default must not be used for mainnet.",
    );
  }
  if (firstNonempty(verifierSource) == null) {
    throw new Error(
      "mainnet-beta indexing requires an explicit --proof-verifier or ANKY_PROOF_VERIFIER_AUTHORITY; the devnet default must not be used for mainnet.",
    );
  }
}

function resolvePublicPath(value, flagName) {
  const trimmed = firstNonempty(value);
  if (trimmed == null) {
    return trimmed;
  }
  if (SECRET_PATH_RE.test(trimmed)) {
    throw new Error(`${flagName} must not point at .env, .anky, keypair, wallet, deployer, pem, or id.json files.`);
  }

  return trimmed;
}

function normalizeBackendUrl(value) {
  const trimmed = firstNonempty(value);
  if (trimmed == null) {
    return trimmed;
  }

  let parsed;
  try {
    parsed = new URL(trimmed);
  } catch (_error) {
    throw new Error("--backend-url must be an HTTP or HTTPS URL without credentials.");
  }

  const isLocalhost = parsed.hostname === "localhost" || parsed.hostname === "127.0.0.1";
  if (
    (parsed.protocol !== "https:" && !(parsed.protocol === "http:" && isLocalhost)) ||
    parsed.username !== "" ||
    parsed.password !== ""
  ) {
    throw new Error("--backend-url must be an HTTPS URL without credentials unless it is localhost HTTP.");
  }

  return trimmed;
}

function hasConfiguredHeliusBackfill() {
  return (
    (process.env.HELIUS_API_KEY != null && process.env.HELIUS_API_KEY.trim() !== "") ||
    (process.env.ANKY_SOLANA_RPC_URL != null && process.env.ANKY_SOLANA_RPC_URL.trim() !== "")
  );
}

function hasIndexerWriteSecret() {
  return (
    process.env.ANKY_INDEXER_WRITE_SECRET != null &&
    process.env.ANKY_INDEXER_WRITE_SECRET.trim() !== ""
  );
}

function readPubkey(buffer, offset) {
  return base58Encode(buffer.subarray(offset, offset + 32));
}

function readHash(buffer, offset) {
  return buffer.subarray(offset, offset + 32).toString("hex");
}

function readI64(buffer, offset) {
  return Number(buffer.readBigInt64LE(offset));
}

function readU64(buffer, offset) {
  const value = buffer.readBigUInt64LE(offset);

  return value <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(value) : value.toString();
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

function discriminator(preimage) {
  return crypto.createHash("sha256").update(preimage).digest().subarray(0, 8);
}

function toSafeInteger(value, label) {
  const parsed = typeof value === "number" ? value : Number(value);
  if (!Number.isSafeInteger(parsed)) {
    throw new Error(`${label} must be a safe integer.`);
  }

  return parsed;
}

function toOptionalSafeInteger(value) {
  if (value == null) {
    return null;
  }

  const parsed = Number(value);
  return Number.isSafeInteger(parsed) ? parsed : null;
}

function normalizeHashHex(value) {
  return typeof value === "string" ? value.trim().toLowerCase() : value;
}

function parseNonNegativeBigInt(value, label) {
  if (typeof value !== "string" || !/^\d+$/.test(value.trim())) {
    throw new Error(`${label} must be a non-negative integer string.`);
  }

  return BigInt(value.trim());
}

function parseBasisPoints(value, label) {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0 || parsed > 10_000) {
    throw new Error(`${label} must be an integer between 0 and 10000.`);
  }

  return parsed;
}

function parsePositiveInteger(value, label) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new Error(`${label} must be a positive safe integer.`);
  }

  return parsed;
}

function resolveBase58PublicKey(value, fallback, label) {
  const resolved =
    typeof value === "string" && value.trim().length > 0
      ? value.trim()
      : fallback;
  if (!isBase58PublicKey(resolved)) {
    throw new Error(`${label} must be a base58 Solana public key.`);
  }

  return resolved;
}

function firstNonempty(...values) {
  for (const value of values) {
    if (typeof value === "string" && value.trim().length > 0) {
      return value.trim();
    }
  }

  return null;
}

function isBase58PublicKey(value) {
  if (typeof value !== "string") {
    return false;
  }

  try {
    return base58Decode(value).length === 32;
  } catch (_error) {
    return false;
  }
}

function isSolanaSignature(value) {
  if (typeof value !== "string") {
    return false;
  }

  try {
    return base58Decode(value).length === 64;
  } catch (_error) {
    return false;
  }
}

function isHashHex(value) {
  return typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
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
    if (arg === "--backfill") {
      args.backfill = true;
      continue;
    }
    if (arg === "--include-non-finalized") {
      args.requireFinalized = false;
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

function parseSignatureList(value) {
  if (value == null) {
    return [];
  }
  const signatures = String(value)
    .split(",")
    .map((signature) => signature.trim())
    .filter(Boolean);
  if (signatures.length === 0) {
    throw new Error("--signature must include at least one Solana transaction signature.");
  }
  for (const signature of signatures) {
    if (!isSolanaSignature(signature)) {
      throw new Error(`--signature contains an invalid Solana transaction signature: ${signature}`);
    }
  }

  return signatures;
}

function printUsage() {
  console.log(`Usage:
  node solana/scripts/indexer/ankySealIndexer.mjs --input solana/scripts/indexer/fixtures/anky-seal-events.json
  HELIUS_API_KEY=... node solana/scripts/indexer/ankySealIndexer.mjs --backfill --limit 100 --out sojourn9/score-snapshot.json
  HELIUS_API_KEY=... node solana/scripts/indexer/ankySealIndexer.mjs --signature <seal_tx>,<verified_tx> --out sojourn9/score-snapshot.json

Options:
  --input <path>              Helius webhook payload, getTransaction JSON, or decodedEvents fixture.
  --backfill                  Fetch finalized program transactions with Helius getTransactionsForAddress.
  --signature <sig[,sig...]>  Fetch known finalized transaction signatures with Helius getTransaction.
  --program-id <pubkey>       Defaults to ANKY_SEAL_PROGRAM_ID or the devnet Anky Seal Program.
  --proof-verifier <pubkey>   Defaults to ANKY_PROOF_VERIFIER_AUTHORITY or Sojourn 9 verifier.
  --cluster <cluster>         devnet or mainnet-beta. Defaults to ANKY_SOLANA_CLUSTER or devnet.
  --limit <n>                 Backfill signature page size. Defaults to ${DEFAULT_LIMIT}.
  --before <signature>        Backfill pagination cursor.
  --include-non-finalized     Include non-finalized events in scoring.
  --max-participants <n>      Reward participant cap. Defaults to ${DEFAULT_MAX_PARTICIPANTS}.
  --backend-url <url>         POST public seal/verified metadata into the mobile backend.
  --core-collection <pubkey>  Required with --backend-url unless ANKY_CORE_COLLECTION is set.
  --token-supply <raw_units>  Optional total token supply in raw units for reward allocation.
  --reward-bps <bps>          Reward pool basis points. Defaults to ${DEFAULT_REWARD_BPS} (8%).
  --out <path>                Write snapshot JSON instead of printing.

Backfill requires HELIUS_API_KEY or ANKY_SOLANA_RPC_URL pointing at a Helius RPC endpoint.
Backend posts require ANKY_INDEXER_WRITE_SECRET; the secret value is sent only as x-anky-indexer-secret.
The script indexes only public Anchor event logs and never accepts .anky plaintext.`);
}
