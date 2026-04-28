import { computeSessionHashSync, parseAnky } from "../ankyProtocol";
import {
  AnkyCarpet,
  AppConfigResponse,
  CarpetEntry,
  CreateProcessingTicketRequest,
  ProcessingType,
  assertProcessingType,
} from "../api/types";

export type ProcessingCarpetPayload = {
  encryptedCarpet: string;
  encryptionScheme: "dev_plaintext" | "x25519_v1";
};

export function buildAnkyCarpet(
  purpose: ProcessingType,
  entries: CarpetEntry[],
  createdAt = Date.now(),
): AnkyCarpet {
  assertProcessingType(purpose);

  if (!Number.isSafeInteger(createdAt) || createdAt < 0) {
    throw new Error("Carpet createdAt must be a non-negative safe integer.");
  }

  if (entries.length === 0) {
    throw new Error("A carpet needs at least one .anky entry.");
  }

  const normalizedEntries = entries.map((entry, index) => {
    const sessionHash = entry.sessionHash.toLowerCase();

    if (!/^[a-f0-9]{64}$/.test(sessionHash)) {
      throw new Error(`Carpet entry ${index + 1} has an invalid session hash.`);
    }

    const parsed = parseAnky(entry.anky);

    if (!parsed.valid) {
      throw new Error(`Carpet entry ${index + 1} is not a valid closed .anky file.`);
    }

    if (computeSessionHashSync(entry.anky) !== sessionHash) {
      throw new Error(`Carpet entry ${index + 1} does not match its session hash.`);
    }

    return {
      anky: entry.anky,
      sessionHash,
    };
  });

  return {
    carpetVersion: 1,
    createdAt,
    entries: normalizedEntries,
    purpose,
  };
}

export function buildCarpetFromAnkyStrings(
  purpose: ProcessingType,
  ankys: string[],
  createdAt = Date.now(),
): AnkyCarpet {
  const entries = ankys.map((anky) => ({
    anky,
    sessionHash: computeSessionHashSync(anky),
  }));

  return buildAnkyCarpet(purpose, entries, createdAt);
}

export function createProcessingTicketRequest(
  carpet: AnkyCarpet,
): CreateProcessingTicketRequest {
  return {
    estimatedEntryCount: carpet.entries.length,
    processingType: carpet.purpose,
    sessionHashes: carpet.entries.map((entry) => entry.sessionHash),
  };
}

export function createProcessingCarpetPayload(
  carpet: AnkyCarpet,
  processingConfig: AppConfigResponse["processing"],
): ProcessingCarpetPayload {
  if (processingConfig.devPlaintextProcessingAllowed) {
    return {
      encryptedCarpet: JSON.stringify(carpet),
      encryptionScheme: "dev_plaintext",
    };
  }

  if (processingConfig.publicKey != null) {
    throw new Error("x25519_v1 carpet encryption is not implemented in this mobile client yet.");
  }

  throw new Error("Processing is unavailable because encrypted carpet upload is not configured.");
}
