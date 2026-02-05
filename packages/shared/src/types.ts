// Shared types between API and Web

export interface User {
  id: string;
  walletAddress: string;
  dayBoundaryHour: number;
  timezone: string;
  freeSessionUsed?: boolean;
  polarCustomerId?: string | null;
  subscriptionExpiresAt?: string | null;
  createdAt: string;
}

export interface Agent {
  id: string;
  name: string;
  description?: string | null;
  model?: string | null;
  sessionCount: number;
  freeSessionsRemaining: number;
  totalPaidSessions: number;
  lastActiveAt?: string | null;
  createdAt: string;
}

export interface WritingSessionData {
  id: string;
  shareId: string | null;
  content: string;
  durationSeconds: number;
  wordCount: number;
  wordsPerMinute?: number | null;
  isAnky: boolean;
  isPublic: boolean;
  writerType: string;
  agentId?: string | null;
  createdAt: string;
}

export interface AnkyData {
  id: string;
  writingSessionId: string;
  title?: string | null;
  imageUrl?: string | null;
  imageBase64?: string | null;
  reflection?: string | null;
  imagePrompt?: string | null;
  writingIpfsHash?: string | null;
  imageIpfsHash?: string | null;
  metadataIpfsHash?: string | null;
  isMinted: boolean;
  tokenId?: number | null;
  createdAt: string;
}

export interface GalleryAnky {
  id: string;
  title: string | null;
  imageUrl: string;
  reflection: string | null;
  createdAt: string;
  writerType: string;
  session: {
    shareId: string;
    wordCount: number;
    durationSeconds: number;
  } | null;
}

export interface PaymentProof {
  txHash: string;
  chain: "base";
  method: "usdc" | "anky_token";
}

export type PaymentMethod = "free" | "usdc" | "anky_token";

export interface PaymentInfo {
  type: PaymentMethod;
  freeSessionsRemaining: number;
  txHash?: string;
}

export interface PaymentOption {
  method: string;
  token: string;
  amount: string;
  recipient: string;
  chain: string;
  decimals: number;
}

export interface ApiError {
  error: string;
  details?: Record<string, string[]>;
}

export interface GalleryResponse {
  ankys: GalleryAnky[];
  total: number;
  hasMore: boolean;
}
