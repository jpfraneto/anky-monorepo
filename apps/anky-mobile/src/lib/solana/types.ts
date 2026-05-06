export type Loom = {
  id: string;
  imageUrl?: string;
  latestSessionHash?: string;
  name: string;
  ownerWallet: string;
  rollingRoot?: string;
  totalSeals?: number;
};

export type LoomOwnership = {
  loom: Loom;
  owns: boolean;
};

export type SealAnkyInput = {
  loomId: string;
  sessionHash: string;
  sessionUtcDay: number;
};

export type SealAnkyResult = {
  blockTime?: number;
  loomId: string;
  network?: "devnet" | "mainnet-beta";
  sessionHash: string;
  slot?: number;
  txSignature: string;
  utcDay?: number;
  writer: string;
};

export type LoomSeal = SealAnkyResult & {
  createdAt?: string;
  proofBlockTime?: number;
  proofCreatedAt?: string;
  proofHash?: string;
  proofProtocolVersion?: number;
  proofUtcDay?: number;
  proofSlot?: number;
  proofStatus?: "confirmed" | "failed" | "finalized" | "pending" | "processed";
  proofTxSignature?: string;
  proofVerifier?: string;
};

export function getLoomSealProofState(
  seal: LoomSeal | null | undefined,
  expectedProofVerifier?: string,
): "failed" | "none" | "proving" | "verified" {
  if (seal?.proofStatus === "confirmed" || seal?.proofStatus === "finalized") {
    if (
      typeof expectedProofVerifier !== "string" ||
      expectedProofVerifier.length === 0 ||
      seal.proofVerifier !== expectedProofVerifier ||
      seal.proofProtocolVersion !== 1 ||
      !Number.isSafeInteger(seal.proofUtcDay) ||
      (Number.isSafeInteger(seal.utcDay) && seal.proofUtcDay !== seal.utcDay) ||
      !isProofHash(seal.proofHash) ||
      !isNonemptyString(seal.proofTxSignature)
    ) {
      return "failed";
    }

    return "verified";
  }

  if (seal?.proofStatus === "pending" || seal?.proofStatus === "processed") {
    return "proving";
  }

  if (seal?.proofStatus === "failed") {
    return "failed";
  }

  return "none";
}

function isProofHash(value: string | undefined): boolean {
  return typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
}

function isNonemptyString(value: string | undefined): boolean {
  return typeof value === "string" && value.trim().length > 0;
}

export type LoomClient = {
  getOwnedLooms(): Promise<Loom[]>;
  getSelectedLoom(): Promise<Loom | null>;
  sealAnky(input: SealAnkyInput): Promise<SealAnkyResult>;
};

export function assertSessionHash(value: string): void {
  if (!/^[a-f0-9]{64}$/.test(value)) {
    throw new Error("sessionHash must be a 32-byte lowercase hex string.");
  }
}
