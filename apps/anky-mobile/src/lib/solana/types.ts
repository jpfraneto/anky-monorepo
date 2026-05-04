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
};

export type SealAnkyResult = {
  blockTime?: number;
  loomId: string;
  network?: "devnet" | "mainnet-beta";
  sessionHash: string;
  slot?: number;
  txSignature: string;
  writer: string;
};

export type LoomSeal = SealAnkyResult & {
  createdAt?: string;
};

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
