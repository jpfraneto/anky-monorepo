import type { LoomSeal } from "../solana/types";
import type { ThreadMode, ThreadRole } from "../thread/types";

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
    collectionUri?: string;
    coreCollection?: string;
    coreProgramId?: string;
    loomMetadataBaseUrl?: string;
    proofVerifierAuthority?: string;
    rpcUrl?: string;
    sealProgramId?: string;
    sealVerification?: string;
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

export type MobileSealScoreResponse = {
  finalizedOnly: boolean;
  formula: string;
  network: string;
  proofVerifierAuthority: string;
  score: number;
  sealedDays: number[];
  streakBonus: number;
  uniqueSealDays: number;
  verifiedDays: number[];
  verifiedSealDays: number;
  wallet: string;
};

export type MobileSolanaConfigResponse = {
  cluster: "devnet" | "mainnet-beta";
  collectionUri: string;
  coreCollection: string;
  coreProgramId: string;
  loomMetadataBaseUrl: string;
  network: "devnet" | "mainnet-beta";
  rpcUrl: string;
  proofVerifierAuthority: string;
  sealProgramId: string;
  sealVerification: string;
};

export type MobileCreditAccount = {
  createdAt: string;
  creditsRemaining: number;
  identityId: string;
  updatedAt: string;
};

export type MobileCreditResponse = {
  account: MobileCreditAccount;
  initialCredits: number;
};

export type MobileSpendCreditsRequest = {
  amount: number;
  identityId: string;
  metadata?: unknown;
  reason: string;
  relatedId?: string;
};

export type MobileSpendCreditsResponse = {
  account: MobileCreditAccount;
  creditsSpent: number;
};

export type CreditLedgerEntry = {
  amount: number;
  createdAt: string;
  id: string;
  kind: "adjustment" | "gift" | "purchase" | "spend";
  label: string;
  metadata?: unknown;
  referenceId?: string;
  source: string;
  userId: string;
};

export type CreditLedgerResponse = {
  entries: CreditLedgerEntry[];
};

export type ClaimWelcomeCreditGiftResponse = {
  balanceSource: "revenuecat";
  entries: CreditLedgerEntry[];
  granted: boolean;
  ok: boolean;
};

export type SyncCreditPurchaseHistoryRequest = {
  identityId?: string;
  packageId: string;
  productId: string;
  purchaseToken?: string;
  purchasedAt?: string;
  transactionId: string;
};

export type SyncCreditPurchaseHistoryResponse = {
  entries: CreditLedgerEntry[];
  inserted: boolean;
  ok: boolean;
};

export type MobileMintAuthorizationRequest = {
  collection?: string;
  inviteCode?: string;
  loomIndex: number;
  payer?: string;
  wallet: string;
};

export type MobileMintAuthorizationResponse = {
  allowed: boolean;
  authorizationId: string;
  collection: string;
  expiresAt: string;
  loomIndex: number;
  mode: "self_funded" | "invite_code";
  owner: string;
  payer: string;
  reason?: string;
  signature: string;
  sponsor: boolean;
  sponsorPayer?: string;
};

export type PrepareMobileLoomMintRequest = {
  authorizationId: string;
  collection?: string;
  loomIndex: number;
  metadataUri?: string;
  payer?: string;
  wallet: string;
};

export type PrepareMobileLoomMintResponse = {
  asset: string;
  authorizationId: string;
  blockhash: string;
  collection: string;
  collectionAuthority: string;
  lastValidBlockHeight: number;
  loomIndex: number;
  mode: "self_funded" | "invite_code";
  name: string;
  owner: string;
  payer: string;
  transactionBase64: string;
  uri: string;
};

export type RecordMobileLoomMintRequest = {
  coreCollection: string;
  loomAsset: string;
  loomIndex?: number;
  metadataUri?: string;
  mintMode?: string;
  signature: string;
  status?: "confirmed" | "finalized" | "processed" | "pending" | "failed";
  wallet: string;
};

export type MobileLoomMint = {
  coreCollection: string;
  createdAt: string;
  id: string;
  loomAsset: string;
  loomIndex?: number;
  metadataUri?: string;
  mintMode?: string;
  network: "devnet" | "mainnet-beta";
  signature: string;
  status: string;
  wallet: string;
};

export type RecordMobileLoomMintResponse = {
  loom: MobileLoomMint;
  recorded: boolean;
};

export type MobileLoomLookupResponse = {
  looms: MobileLoomMint[];
};

export type MobileReflectionRequest = {
  anky: string;
  identityId: string;
  processingType?: "full_anky" | "reflection";
  sessionHash: string;
};

export type MobileReflectionJob = {
  createdAt: string;
  creditsSpent: number;
  error?: string;
  id: string;
  identityId: string;
  processingType: string;
  request?: unknown;
  result?: unknown;
  sessionHash: string;
  status: string;
  updatedAt: string;
};

export type MobileReflectionResponse = {
  artifacts: ProcessingArtifact[];
  creditsRemaining: number;
  creditsSpent: number;
  job: MobileReflectionJob;
};

export type MobileReflectionJobResponse = {
  job: MobileReflectionJob;
};

export type RecordMobileSealRequest = {
  blockTime?: number;
  coreCollection: string;
  loomAsset: string;
  sessionHash: string;
  signature: string;
  slot?: number;
  status?: "confirmed" | "finalized" | "processed" | "pending" | "failed";
  utcDay?: number;
  wallet: string;
};

export type RecordMobileSealResponse = {
  recorded: boolean;
  seal: LoomSeal;
};

export type ThreadApiMessage = {
  role: ThreadRole;
  content: string;
  createdAt: string;
  id?: string;
};

export type SendThreadMessageRequest = {
  existingReflection?: string;
  messages: ThreadApiMessage[];
  mode: ThreadMode;
  rawAnky: string;
  reconstructedText: string;
  reflectionKind?: "full" | "quick";
  sessionHash: string;
  userMessage: string;
};

export type SendThreadMessageResponse = {
  message: ThreadApiMessage & {
    role: "anky";
  };
};

export type PrivyAuthRequest = {
  auth_token: string;
  siws_message?: string;
  siws_signature?: string;
  wallet_address?: string;
};

export type BackendAuthResponse = {
  email?: string;
  ok: boolean;
  session_token: string;
  user_id: string;
  username?: string;
  wallet_address?: string;
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
