import { create, fetchCollection, mplCore } from "@metaplex-foundation/mpl-core";
import {
  createNoopSigner,
  createSignerFromKeypair,
  publicKey,
} from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  fromWeb3JsKeypair,
  toWeb3JsLegacyTransaction,
} from "@metaplex-foundation/umi-web3js-adapters";
import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { Buffer } from "buffer";

import type { AnkySolanaWallet } from "./walletTypes";

const DEFAULT_LOOM_METADATA_BASE_URL = "https://anky.app/devnet/metadata/looms";

export type InviteMintAuthorization = {
  allowed: boolean;
  authorizationId?: string;
  mode?: "self_funded" | "invite_code";
  payer?: string;
  sponsor?: boolean;
  sponsorPayer?: string;
  expiresAt?: string;
  reason?: string;
  signature?: string;
};

export type ValidateInviteCodeInput = {
  inviteCode: string;
  owner: string;
  payer: string;
  collection: string;
  loomIndex: number;
};

export type CreateMintAuthorizationInput = {
  inviteCode?: string;
  owner: string;
  payer: string;
  collection: string;
  loomIndex: number;
};

export type BuildCoreLoomMintTransactionInput = {
  authorization?: InviteMintAuthorization;
  collection: PublicKey;
  connection: Connection;
  loomIndex: number;
  name: string;
  owner: PublicKey;
  payer: PublicKey;
  uri: string;
};

export type BuildCoreLoomMintTransactionResult = {
  asset: PublicKey;
  latestBlockhash?: LatestBlockhash;
  transaction: Transaction;
};

export type LatestBlockhash = {
  blockhash: string;
  lastValidBlockHeight: number;
};

export type MintAnkyLoomInput = {
  wallet: AnkySolanaWallet;
  payer?: string;
  connection: Connection;
  collection: string;
  loomIndex: number;
  inviteCode?: string;
  metadataUri?: string;
  onStatus?: (status: MintAnkyLoomStatus) => void;
  createMintAuthorization?: (
    input: CreateMintAuthorizationInput,
  ) => Promise<InviteMintAuthorization>;
  validateInviteCode?: (
    input: ValidateInviteCodeInput,
  ) => Promise<InviteMintAuthorization>;
  buildCoreLoomMintTransaction?: (
    input: BuildCoreLoomMintTransactionInput,
  ) => Promise<BuildCoreLoomMintTransactionResult>;
};

export type MintAnkyLoomStatus = "authorizing" | "confirming" | "preparing" | "signing";

export type MintAnkyLoomResult = {
  asset: string;
  owner: string;
  collection: string;
  mintMode: "self_funded" | "invite_code";
  signature: string;
  name: string;
  uri: string;
};

export type PrepareCoreLoomMintTransactionInput = {
  authorizationId: string;
  collection: string;
  loomIndex: number;
  metadataUri?: string;
  payer: string;
  wallet: string;
};

export type PreparedCoreLoomMintTransactionResponse = {
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

export type PrepareCoreLoomMintTransaction = (
  input: PrepareCoreLoomMintTransactionInput,
) => Promise<PreparedCoreLoomMintTransactionResponse>;

export async function mintAnkyLoom({
  wallet,
  payer,
  connection,
  collection,
  loomIndex,
  inviteCode,
  metadataUri,
  onStatus,
  createMintAuthorization,
  validateInviteCode,
  buildCoreLoomMintTransaction,
}: MintAnkyLoomInput): Promise<MintAnkyLoomResult> {
  const owner = new PublicKey(wallet.publicKey);
  const collectionPublicKey = new PublicKey(collection);
  const loomNumber = formatLoomNumber(loomIndex);
  const name = `Anky Sojourn 9 Loom #${loomNumber}`;
  const uri = metadataUri ?? `${DEFAULT_LOOM_METADATA_BASE_URL}/${loomNumber}.json`;

  let authorization: InviteMintAuthorization | undefined;
  let payerPublicKey = new PublicKey(payer ?? wallet.publicKey);
  if (createMintAuthorization) {
    onStatus?.("authorizing");
    authorization = await createMintAuthorization({
      inviteCode,
      owner: owner.toBase58(),
      payer: payerPublicKey.toBase58(),
      collection: collectionPublicKey.toBase58(),
      loomIndex,
    });

    if (!authorization.allowed) {
      throw new Error(authorization.reason ?? "Loom minting is not authorized.");
    }
    payerPublicKey = new PublicKey(
      authorization.payer ?? authorization.sponsorPayer ?? payerPublicKey.toBase58(),
    );
  } else if (inviteCode) {
    if (!validateInviteCode) {
      throw new Error(
        "mintAnkyLoom invite-code minting requires a validateInviteCode backend hook before a Core mint transaction can be built.",
      );
    }

    onStatus?.("authorizing");
    authorization = await validateInviteCode({
      inviteCode,
      owner: owner.toBase58(),
      payer: payerPublicKey.toBase58(),
      collection: collectionPublicKey.toBase58(),
      loomIndex,
    });

    if (!authorization.allowed) {
      throw new Error(authorization.reason ?? "Invite code is not authorized for Loom minting.");
    }
    payerPublicKey = new PublicKey(
      authorization.payer ?? authorization.sponsorPayer ?? payerPublicKey.toBase58(),
    );
  }

  onStatus?.("preparing");
  const { asset, latestBlockhash, transaction } = await (
    buildCoreLoomMintTransaction ?? buildSelfFundedCoreLoomMintTransaction
  )({
    authorization,
    collection: collectionPublicKey,
    connection,
    loomIndex,
    name,
    owner,
    payer: payerPublicKey,
    uri,
  });

  const signature = await signAndSendWalletTransaction({
    wallet,
    connection,
    latestBlockhash,
    onStatus,
    transaction,
  });

  return {
    asset: asset.toBase58(),
    owner: owner.toBase58(),
    collection: collectionPublicKey.toBase58(),
    mintMode: authorization?.mode ?? "self_funded",
    signature,
    name,
    uri,
  };
}

export async function buildSelfFundedCoreLoomMintTransaction({
  authorization,
  collection,
  connection,
  name,
  owner,
  payer,
  uri,
}: BuildCoreLoomMintTransactionInput): Promise<BuildCoreLoomMintTransactionResult> {
  if (authorization?.sponsor) {
    throw new Error(
      "Sponsored Loom minting requires a backend-prepared Core transaction; the default builder only supports self-funded wallet-paid mints.",
    );
  }

  if (!payer.equals(owner)) {
    throw new Error(
      "The default Loom mint builder requires payer and owner to be the connected wallet. Use a backend-prepared builder for sponsored minting.",
    );
  }

  // Creating an asset inside a Core collection requires the collection authority
  // or a valid delegate. This devnet builder uses the connected wallet as that
  // authority, so production app flows should replace it with an Anky-prepared
  // transaction that includes the official collection authority signature.
  const umi = createUmi(connection).use(mplCore());
  const payerSigner = createNoopSigner(publicKey(payer.toBase58()));
  const assetKeypair = Keypair.generate();
  const asset = createSignerFromKeypair(umi, fromWeb3JsKeypair(assetKeypair));
  const coreCollection = await fetchCollection(umi, publicKey(collection.toBase58()));
  if (coreCollection.updateAuthority.toString() !== payerSigner.publicKey.toString()) {
    throw new Error(
      "This wallet is not the Core collection update authority. Official Loom mints for normal users need an Anky-prepared transaction that is signed by the collection authority and then signed/sent by the user's wallet.",
    );
  }

  const latestBlockhash = await connection.getLatestBlockhash("confirmed");

  const umiTransaction = await create(umi, {
    asset,
    authority: payerSigner,
    collection: coreCollection,
    name,
    owner: publicKey(owner.toBase58()),
    payer: payerSigner,
    uri,
  })
    .setFeePayer(payerSigner)
    .useLegacyVersion()
    .setBlockhash(latestBlockhash)
    .buildAndSign(umi);

  const transaction = toWeb3JsLegacyTransaction(umiTransaction);
  assertPartialSignature(transaction, assetKeypair.publicKey);

  return {
    asset: assetKeypair.publicKey,
    latestBlockhash,
    transaction,
  };
}

export function createBackendPreparedCoreLoomMintTransactionBuilder(
  prepareCoreLoomMintTransaction: PrepareCoreLoomMintTransaction,
): (input: BuildCoreLoomMintTransactionInput) => Promise<BuildCoreLoomMintTransactionResult> {
  return async ({
    authorization,
    collection,
    loomIndex,
    owner,
    payer,
    uri,
  }: BuildCoreLoomMintTransactionInput): Promise<BuildCoreLoomMintTransactionResult> => {
    if (authorization?.authorizationId == null) {
      throw new Error(
        "Backend-prepared Loom minting requires a mint authorization id from /api/mobile/looms/mint-authorizations.",
      );
    }

    const prepared = await prepareCoreLoomMintTransaction({
      authorizationId: authorization.authorizationId,
      collection: collection.toBase58(),
      loomIndex,
      metadataUri: uri,
      payer: payer.toBase58(),
      wallet: owner.toBase58(),
    });

    assertPreparedMintMatchesInput({ collection, owner, payer, prepared });

    return {
      asset: new PublicKey(prepared.asset),
      latestBlockhash: {
        blockhash: prepared.blockhash,
        lastValidBlockHeight: prepared.lastValidBlockHeight,
      },
      transaction: Transaction.from(Buffer.from(prepared.transactionBase64, "base64")),
    };
  };
}

function formatLoomNumber(loomIndex: number): string {
  if (!Number.isInteger(loomIndex) || loomIndex < 1) {
    throw new Error("loomIndex must be a positive integer.");
  }

  return loomIndex.toString().padStart(4, "0");
}

async function signAndSendWalletTransaction({
  wallet,
  connection,
  latestBlockhash,
  onStatus,
  transaction,
}: {
  wallet: AnkySolanaWallet;
  connection: Connection;
  latestBlockhash?: LatestBlockhash;
  onStatus?: (status: MintAnkyLoomStatus) => void;
  transaction: Transaction;
}): Promise<string> {
  const confirmationBlockhash =
    latestBlockhash ?? (transaction.recentBlockhash == null
      ? await connection.getLatestBlockhash("confirmed")
      : undefined);

  if (transaction.feePayer == null) {
    transaction.feePayer = new PublicKey(wallet.publicKey);
  }

  if (transaction.recentBlockhash == null && confirmationBlockhash != null) {
    transaction.recentBlockhash = confirmationBlockhash.blockhash;
  }

  if (wallet.signAndSendTransaction) {
    onStatus?.("signing");
    const signature = await wallet.signAndSendTransaction(transaction);
    onStatus?.("confirming");
    await confirmSentTransaction(connection, signature, confirmationBlockhash);
    return signature;
  }

  onStatus?.("signing");
  const signedTransaction = await wallet.signTransaction(transaction);
  const signature = await connection.sendRawTransaction(signedTransaction.serialize());
  onStatus?.("confirming");
  await confirmSentTransaction(connection, signature, confirmationBlockhash);

  return signature;
}

async function confirmSentTransaction(
  connection: Connection,
  signature: string,
  latestBlockhash?: LatestBlockhash,
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

function assertPartialSignature(transaction: Transaction, signer: PublicKey): void {
  const signature = transaction.signatures.find((item) => item.publicKey.equals(signer));

  if (signature?.signature == null) {
    throw new Error("Metaplex Core Loom mint transaction is missing the new asset signature.");
  }
}

function assertPreparedMintMatchesInput({
  collection,
  owner,
  payer,
  prepared,
}: {
  collection: PublicKey;
  owner: PublicKey;
  payer: PublicKey;
  prepared: PreparedCoreLoomMintTransactionResponse;
}): void {
  if (prepared.collection !== collection.toBase58()) {
    throw new Error("Prepared Loom mint collection does not match the requested collection.");
  }

  if (prepared.owner !== owner.toBase58()) {
    throw new Error("Prepared Loom mint owner does not match the connected wallet.");
  }

  if (prepared.payer !== payer.toBase58()) {
    throw new Error("Prepared Loom mint payer does not match the requested payer.");
  }
}
