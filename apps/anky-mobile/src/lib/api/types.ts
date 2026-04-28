import type { LoomSeal } from "../solana/types";

export type ProcessingType =
  | "reflection"
  | "image"
  | "full_anky"
  | "deep_mirror"
  | "full_sojourn_archive";

export const PROCESSING_TYPES: ProcessingType[] = [
  "reflection",
  "image",
  "full_anky",
  "deep_mirror",
  "full_sojourn_archive",
];

export const CREDIT_COSTS: Record<ProcessingType, number> = {
  deep_mirror: 8,
  full_anky: 5,
  full_sojourn_archive: 88,
  image: 3,
  reflection: 1,
};

export type CreditReceipt = {
  creditsRemaining: number;
  creditsSpent: number;
  expiresAt: number;
  issuedAt: number;
  nonce: string;
  processingType: ProcessingType;
  receiptVersion: 1;
  signature: string;
  ticketId: string;
};

export type CarpetEntry = {
  anky: string;
  sessionHash: string;
};

export type AnkyCarpet = {
  carpetVersion: 1;
  createdAt: number;
  entries: CarpetEntry[];
  purpose: ProcessingType;
};

export type AppConfigResponse = {
  processing: {
    devPlaintextProcessingAllowed: boolean;
    publicKey?: string;
  };
  sojourn: {
    dayLengthSeconds: 86400;
    number: 9;
    startsAtUtc: string;
  };
  solana: {
    ankyProgramId?: string;
    cluster: "devnet" | "mainnet-beta";
  };
};

export type CreditBalanceResponse = {
  creditsRemaining: number;
};

export type CreateCheckoutRequest = {
  packageId: string;
};

export type CreateCheckoutResponse = {
  checkoutUrl: string;
};

export type CreateProcessingTicketRequest = {
  estimatedEntryCount: number;
  processingType: ProcessingType;
  sessionHashes: string[];
};

export type CreateProcessingTicketResponse = {
  receipt: CreditReceipt;
};

export type RunProcessingRequest = {
  encryptedCarpet: string;
  encryptionScheme?: "dev_plaintext" | "x25519_v1";
  receipt: CreditReceipt;
};

export type ProcessingArtifact =
  | {
      kind: "reflection";
      markdown: string;
      sessionHash: string;
    }
  | {
      kind: "title";
      sessionHash: string;
      title: string;
    }
  | {
      imageBase64?: string;
      imageUrl?: string;
      kind: "image";
      mimeType: "image/png" | "image/jpeg" | "image/webp";
      sessionHash: string;
    }
  | {
      carpetHash: string;
      kind: "deep_mirror";
      markdown: string;
    }
  | {
      carpetHash: string;
      kind: "full_sojourn_archive";
      markdown: string;
      summaryJson?: unknown;
    };

export type RunProcessingResponse = {
  artifacts: ProcessingArtifact[];
  processingType: ProcessingType;
};

export type SealLookupResponse = {
  seals: LoomSeal[];
};

export type SealLookupQuery =
  | {
      loomId: string;
      sessionHash?: never;
      wallet?: never;
    }
  | {
      loomId?: never;
      sessionHash: string;
      wallet?: never;
    }
  | {
      loomId?: never;
      sessionHash?: never;
      wallet: string;
    };

export function isProcessingType(value: unknown): value is ProcessingType {
  return typeof value === "string" && PROCESSING_TYPES.includes(value as ProcessingType);
}

export function assertProcessingType(value: unknown): asserts value is ProcessingType {
  if (!isProcessingType(value)) {
    throw new Error("Unknown processing type.");
  }
}

export function isCreditReceipt(value: unknown): value is CreditReceipt {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const receipt = value as Partial<CreditReceipt>;

  return (
    receipt.receiptVersion === 1 &&
    typeof receipt.ticketId === "string" &&
    isProcessingType(receipt.processingType) &&
    typeof receipt.creditsSpent === "number" &&
    Number.isSafeInteger(receipt.creditsSpent) &&
    receipt.creditsSpent >= 0 &&
    typeof receipt.creditsRemaining === "number" &&
    Number.isSafeInteger(receipt.creditsRemaining) &&
    receipt.creditsRemaining >= 0 &&
    typeof receipt.issuedAt === "number" &&
    Number.isSafeInteger(receipt.issuedAt) &&
    typeof receipt.expiresAt === "number" &&
    Number.isSafeInteger(receipt.expiresAt) &&
    receipt.expiresAt > receipt.issuedAt &&
    typeof receipt.nonce === "string" &&
    receipt.nonce.length > 0 &&
    typeof receipt.signature === "string" &&
    receipt.signature.length > 0
  );
}

export function assertCreditReceipt(value: unknown): asserts value is CreditReceipt {
  if (!isCreditReceipt(value)) {
    throw new Error("Invalid credit receipt.");
  }
}
