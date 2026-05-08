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
const DAILY_SEAL_SEED = Buffer.from("daily_seal", "utf8");
const HASH_SEAL_SEED = Buffer.from("hash_seal", "utf8");
const MS_PER_UTC_DAY = 86_400_000;

export type SealAnkyInput = {
  wallet: AnkySolanaWallet;
  connection: Connection;
  programId: string;
  network: AnkySolanaCluster;
  sessionHashHex: string;
  sessionUtcDay: number;
  loomAsset: string;
  coreCollection: string;
  payer?: string;
  preparedTransactionBase64?: string;
  preparedBlockhash?: {
    blockhash: string;
    lastValidBlockHeight: number;
  };
};

export type SealAnkyResult = {
  version: 1;
  network: AnkySolanaCluster;
  session_hash: string;
  utc_day: number;
  loom_asset: string;
  writer: string;
  payer: string;
  sponsored: boolean;
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
  sessionUtcDay,
  loomAsset,
  coreCollection,
  payer,
  preparedTransactionBase64,
  preparedBlockhash,
}: SealAnkyInput): Promise<SealAnkyResult> {
  const normalizedSessionHash = normalizeSessionHash(sessionHashHex);
  const sessionHashBytes = hexToBytes(normalizedSessionHash);
  const normalizedUtcDay = normalizeSessionUtcDay(sessionUtcDay);

  assertCurrentUtcDay(normalizedUtcDay);

  const writer = new PublicKey(wallet.publicKey);
  const payerPublicKey = new PublicKey(payer ?? wallet.publicKey);
  const programPublicKey = new PublicKey(programId);
  const loomAssetPublicKey = new PublicKey(loomAsset);
  const coreCollectionPublicKey = new PublicKey(coreCollection);
  const utcDayBytes = encodeI64Le(normalizedUtcDay);
  const [loomState] = PublicKey.findProgramAddressSync(
    [LOOM_STATE_SEED, loomAssetPublicKey.toBuffer()],
    programPublicKey,
  );
  const [dailySeal] = PublicKey.findProgramAddressSync(
    [DAILY_SEAL_SEED, writer.toBuffer(), utcDayBytes],
    programPublicKey,
  );
  const [hashSeal] = PublicKey.findProgramAddressSync(
    [HASH_SEAL_SEED, writer.toBuffer(), Buffer.from(sessionHashBytes)],
    programPublicKey,
  );

  const transaction =
    preparedTransactionBase64 == null
      ? new Transaction().add(
          buildSealAnkyInstruction({
            coreCollection: coreCollectionPublicKey,
            dailySeal,
            hashSeal,
            loomAsset: loomAssetPublicKey,
            loomState,
            payer: payerPublicKey,
            programId: programPublicKey,
            sessionHashBytes,
            utcDay: normalizedUtcDay,
            writer,
          }),
        )
      : Transaction.from(Buffer.from(preparedTransactionBase64, "base64"));

  assertSealTransactionPayer(transaction, payerPublicKey);

  const signature = await signAndSendWalletTransaction({
    wallet,
    connection,
    latestBlockhash: preparedBlockhash,
    transaction,
  });

  return {
    version: 1,
    network,
    session_hash: normalizedSessionHash,
    utc_day: normalizedUtcDay,
    loom_asset: loomAssetPublicKey.toBase58(),
    writer: writer.toBase58(),
    payer: payerPublicKey.toBase58(),
    sponsored: !payerPublicKey.equals(writer),
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

export function getUtcDayFromUnixMs(unixMs: number): number {
  if (!Number.isFinite(unixMs)) {
    throw new Error("session timestamp must be finite.");
  }

  return Math.floor(unixMs / MS_PER_UTC_DAY);
}

export function getCurrentUtcDay(nowMs = Date.now()): number {
  return getUtcDayFromUnixMs(nowMs);
}

export function isCurrentUtcDay(utcDay: number, nowMs = Date.now()): boolean {
  return normalizeSessionUtcDay(utcDay) === getCurrentUtcDay(nowMs);
}

function normalizeSessionUtcDay(utcDay: number): number {
  if (!Number.isSafeInteger(utcDay)) {
    throw new Error("sessionUtcDay must be a safe integer UTC day.");
  }

  return utcDay;
}

function assertCurrentUtcDay(utcDay: number): void {
  if (!isCurrentUtcDay(utcDay)) {
    throw new Error("Only an Anky from the current UTC day can be sealed.");
  }
}

export function buildSealAnkyInstructionData(sessionHashBytes: Uint8Array, utcDay: number): Buffer {
  if (sessionHashBytes.length !== 32) {
    throw new Error("sessionHashHex must decode to exactly 32 bytes.");
  }

  const discriminator = sha256(utf8ToBytes("global:seal_anky")).slice(0, 8);
  const data = Buffer.alloc(48);
  data.set(discriminator, 0);
  data.set(sessionHashBytes, 8);
  data.set(encodeI64Le(utcDay), 40);
  return data;
}

export function buildSealAnkyInstruction({
  coreCollection,
  dailySeal,
  hashSeal,
  loomAsset,
  loomState,
  payer,
  programId,
  sessionHashBytes,
  utcDay,
  writer,
}: {
  coreCollection: PublicKey;
  dailySeal: PublicKey;
  hashSeal: PublicKey;
  loomAsset: PublicKey;
  loomState: PublicKey;
  payer: PublicKey;
  programId: PublicKey;
  sessionHashBytes: Uint8Array;
  utcDay: number;
  writer: PublicKey;
}): TransactionInstruction {
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: writer, isSigner: true, isWritable: false },
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: loomAsset, isSigner: false, isWritable: false },
      { pubkey: coreCollection, isSigner: false, isWritable: false },
      { pubkey: loomState, isSigner: false, isWritable: true },
      { pubkey: dailySeal, isSigner: false, isWritable: true },
      { pubkey: hashSeal, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: buildSealAnkyInstructionData(sessionHashBytes, utcDay),
  });
}

function assertSealTransactionPayer(transaction: Transaction, payer: PublicKey): void {
  if (transaction.feePayer == null) {
    transaction.feePayer = payer;
    return;
  }

  if (!transaction.feePayer.equals(payer)) {
    throw new Error("Prepared seal transaction payer does not match the selected payer.");
  }
}

function encodeI64Le(value: number): Buffer {
  const buffer = Buffer.alloc(8);

  buffer.writeBigInt64LE(BigInt(value));

  return buffer;
}

async function signAndSendWalletTransaction({
  wallet,
  connection,
  latestBlockhash,
  transaction,
}: {
  wallet: AnkySolanaWallet;
  connection: Connection;
  latestBlockhash?: {
    blockhash: string;
    lastValidBlockHeight: number;
  };
  transaction: Transaction;
}): Promise<string> {
  const confirmationBlockhash =
    latestBlockhash ??
    (transaction.recentBlockhash == null
      ? await connection.getLatestBlockhash("confirmed")
      : undefined);

  if (transaction.feePayer == null) {
    transaction.feePayer = new PublicKey(wallet.publicKey);
  }
  if (transaction.recentBlockhash == null && confirmationBlockhash != null) {
    transaction.recentBlockhash = confirmationBlockhash.blockhash;
  }

  if (wallet.signAndSendTransaction) {
    const signature = await wallet.signAndSendTransaction(transaction);
    await confirmSealTransaction(connection, signature, confirmationBlockhash);
    return signature;
  }

  const signedTransaction = await wallet.signTransaction(transaction);
  const signature = await connection.sendRawTransaction(signedTransaction.serialize());
  await confirmSealTransaction(connection, signature, confirmationBlockhash);

  return signature;
}

async function confirmSealTransaction(
  connection: Connection,
  signature: string,
  latestBlockhash?: {
    blockhash: string;
    lastValidBlockHeight: number;
  },
): Promise<void> {
  if (latestBlockhash == null) {
    await connection.confirmTransaction(signature, "confirmed");
    return;
  }

  await connection.confirmTransaction(
    {
      signature,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    },
    "confirmed",
  );
}
