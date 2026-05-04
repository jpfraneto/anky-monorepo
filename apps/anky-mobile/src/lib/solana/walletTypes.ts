import type { Transaction } from "@solana/web3.js";

export type AnkySolanaWallet = {
  publicKey: string;
  signTransaction: (transaction: Transaction) => Promise<Transaction>;
  signAndSendTransaction?: (transaction: Transaction) => Promise<string>;
};
