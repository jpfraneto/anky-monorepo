import { sha256 } from "@noble/hashes/sha2.js";
import { hexToBytes, utf8ToBytes } from "@noble/hashes/utils.js";
import {
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { Buffer } from "buffer";

import type { AnkySolanaWallet } from "./walletTypes";
import type { AnkySolanaCluster } from "./ankySolanaConfig";

const LOOM_STATE_SEED = new Uint8Array([
  108, 111, 111, 109, 95, 115, 116, 97, 116, 101,
]);

export type SealAnkyInput = {
  wallet: AnkySolanaWallet;
  connection: Connection;
  programId: string;
  network: AnkySolanaCluster;
  sessionHashHex: string;
  loomAsset: string;
  coreCollection: string;
};

export type SealAnkyResult = {
  version: 1;
  network: AnkySolanaCluster;
  session_hash: string;
  loom_asset: string;
  writer: string;
  signature: string;
  status: "confirmed";
  created_at: string;
};

export async function sealAnky({
  wallet,
  connection,
  programId,
  network,
  sessionHashHex,
  loomAsset,
  coreCollection,
}: SealAnkyInput): Promise<SealAnkyResult> {
  const normalizedSessionHash = normalizeSessionHash(sessionHashHex);
  const sessionHashBytes = hexToBytes(normalizedSessionHash);
  const writer = new PublicKey(wallet.publicKey);
  const programPublicKey = new PublicKey(programId);
  const loomAssetPublicKey = new PublicKey(loomAsset);
  const coreCollectionPublicKey = new PublicKey(coreCollection);
  const [loomState] = PublicKey.findProgramAddressSync(
    [LOOM_STATE_SEED, loomAssetPublicKey.toBuffer()],
    programPublicKey,
  );

  const transaction = new Transaction().add(
    new TransactionInstruction({
      programId: programPublicKey,
      keys: [
        { pubkey: writer, isSigner: true, isWritable: true },
        { pubkey: loomAssetPublicKey, isSigner: false, isWritable: false },
        { pubkey: coreCollectionPublicKey, isSigner: false, isWritable: false },
        { pubkey: loomState, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: buildSealAnkyInstructionData(sessionHashBytes),
    }),
  );

  const signature = await signAndSendWalletTransaction({
    wallet,
    connection,
    transaction,
  });

  return {
    version: 1,
    network,
    session_hash: normalizedSessionHash,
    loom_asset: loomAssetPublicKey.toBase58(),
    writer: writer.toBase58(),
    signature,
    status: "confirmed",
    created_at: new Date().toISOString(),
  };
}

export function normalizeSessionHash(sessionHashHex: string): string {
  const value = sessionHashHex.trim();

  if (!/^[0-9a-fA-F]{64}$/.test(value)) {
    throw new Error("sessionHashHex must be exactly 64 hex characters.");
  }

  return value.toLowerCase();
}

function buildSealAnkyInstructionData(sessionHashBytes: Uint8Array): Buffer {
  if (sessionHashBytes.length !== 32) {
    throw new Error("sessionHashHex must decode to exactly 32 bytes.");
  }

  const discriminator = sha256(utf8ToBytes("global:seal_anky")).slice(0, 8);
  const data = Buffer.alloc(40);
  data.set(discriminator, 0);
  data.set(sessionHashBytes, 8);
  return data;
}

async function signAndSendWalletTransaction({
  wallet,
  connection,
  transaction,
}: {
  wallet: AnkySolanaWallet;
  connection: Connection;
  transaction: Transaction;
}): Promise<string> {
  const latestBlockhash = await connection.getLatestBlockhash("confirmed");
  transaction.feePayer = new PublicKey(wallet.publicKey);
  transaction.recentBlockhash = latestBlockhash.blockhash;

  if (wallet.signAndSendTransaction) {
    const signature = await wallet.signAndSendTransaction(transaction);
    await connection.confirmTransaction(
      {
        signature,
        blockhash: latestBlockhash.blockhash,
        lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      },
      "confirmed",
    );
    return signature;
  }

  const signedTransaction = await wallet.signTransaction(transaction);
  const signature = await connection.sendRawTransaction(signedTransaction.serialize());
  await connection.confirmTransaction(
    {
      signature,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    },
    "confirmed",
  );

  return signature;
}
