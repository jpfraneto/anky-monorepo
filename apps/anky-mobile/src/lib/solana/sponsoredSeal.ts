import { Connection, PublicKey } from "@solana/web3.js";

import type { AnkyApiClient } from "../api/ankyApi";
import { AnkyApiError } from "../api/ankyApi";
import type { AnkySolanaCluster } from "./ankySolanaConfig";
import { sealAnky, type SealAnkyResult } from "./sealAnky";
import type { AnkySolanaWallet } from "./walletTypes";

const USER_SEAL_MIN_LAMPORTS = 6_000_000;

export type SealAnkyWithPayerPolicyInput = {
  api: AnkyApiClient | null;
  canonical: boolean;
  connection: Connection;
  coreCollection: string;
  loomAsset: string;
  network: AnkySolanaCluster;
  programId: string;
  sessionHashHex: string;
  sessionUtcDay: number;
  wallet: AnkySolanaWallet;
};

export async function sealAnkyWithPayerPolicy({
  api,
  canonical,
  connection,
  coreCollection,
  loomAsset,
  network,
  programId,
  sessionHashHex,
  sessionUtcDay,
  wallet,
}: SealAnkyWithPayerPolicyInput): Promise<SealAnkyResult> {
  const userCanPay = await walletHasEnoughSolForSeal(connection, wallet.publicKey);

  if (userCanPay || api == null) {
    try {
      return await sealAnky({
        connection,
        coreCollection,
        loomAsset,
        network,
        programId,
        sessionHashHex,
        sessionUtcDay,
        wallet,
      });
    } catch (error) {
      if (api == null || !needsSolanaFunding(error)) {
        throw error;
      }
    }
  }

  const prepared = await api.prepareMobileSeal({
    canonical,
    coreCollection,
    loomAsset,
    sessionHash: sessionHashHex,
    utcDay: sessionUtcDay,
    wallet: wallet.publicKey,
  }).catch((error: unknown) => {
    throw friendlySponsorshipError(error);
  });

  if (!prepared.sponsor || prepared.transactionBase64 == null) {
    throw new Error(prepared.message ?? "this wallet has enough SOL, so it should pay for this seal.");
  }

  return sealAnky({
    connection,
    coreCollection,
    loomAsset,
    network,
    payer: prepared.payer,
    preparedBlockhash: {
      blockhash: prepared.blockhash,
      lastValidBlockHeight: prepared.lastValidBlockHeight,
    },
    preparedTransactionBase64: prepared.transactionBase64,
    programId,
    sessionHashHex,
    sessionUtcDay,
    wallet,
  });
}

export function needsSolanaFunding(error: unknown): boolean {
  if (!(error instanceof Error)) {
    return false;
  }

  return /no record of a prior credit|attempted to debit|insufficient|lamports|0x1|fund/i.test(
    error.message,
  );
}

async function walletHasEnoughSolForSeal(
  connection: Connection,
  wallet: string,
): Promise<boolean> {
  try {
    const balance = await connection.getBalance(new PublicKey(wallet), "confirmed");

    return balance >= USER_SEAL_MIN_LAMPORTS;
  } catch {
    return true;
  }
}

function friendlySponsorshipError(error: unknown): Error {
  if (error instanceof AnkyApiError) {
    if (error.status === 402 || error.status === 403 || error.status === 429) {
      return new Error(
        "this wallet needs SOL for gas and Anky cannot sponsor this seal right now.",
      );
    }

    if (error.status === 503) {
      return new Error(
        "this wallet needs SOL for gas and seal sponsorship is not available right now.",
      );
    }
  }

  if (error instanceof Error) {
    return error;
  }

  return new Error("seal sponsorship is unavailable right now.");
}
