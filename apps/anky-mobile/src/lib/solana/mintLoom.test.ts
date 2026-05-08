import { PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { Buffer } from "buffer";
import { describe, expect, it, vi } from "vitest";

import {
  buildSelfFundedCoreLoomMintTransaction,
  createBackendPreparedCoreLoomMintTransactionBuilder,
} from "./mintLoom";
import type { PreparedCoreLoomMintTransactionResponse } from "./mintLoom";

const OWNER = "11111111111111111111111111111112";
const SPONSOR = "So11111111111111111111111111111111111111112";
const COLLECTION = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const ASSET = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
const BLOCKHASH = "11111111111111111111111111111111";

describe("Core Loom mint transaction builders", () => {
  it("requests a backend-prepared sponsored mint with the user as owner and sponsor as payer", async () => {
    const prepareCoreLoomMintTransaction = vi.fn(async () => preparedMintResponse());
    const builder = createBackendPreparedCoreLoomMintTransactionBuilder(
      prepareCoreLoomMintTransaction,
    );

    const result = await builder({
      authorization: {
        allowed: true,
        authorizationId: "auth-sponsored-1",
        mode: "self_funded",
        payer: SPONSOR,
        sponsor: true,
        sponsorPayer: SPONSOR,
      },
      collection: new PublicKey(COLLECTION),
      connection: {} as never,
      loomIndex: 9,
      name: "Anky Sojourn 9 Loom #0009",
      owner: new PublicKey(OWNER),
      payer: new PublicKey(SPONSOR),
      uri: "https://anky.app/devnet/metadata/looms/0009.json",
    });

    expect(prepareCoreLoomMintTransaction).toHaveBeenCalledWith({
      authorizationId: "auth-sponsored-1",
      collection: COLLECTION,
      loomIndex: 9,
      metadataUri: "https://anky.app/devnet/metadata/looms/0009.json",
      payer: SPONSOR,
      wallet: OWNER,
    });
    expect(result.asset.toBase58()).toBe(ASSET);
    expect(result.latestBlockhash).toEqual({
      blockhash: BLOCKHASH,
      lastValidBlockHeight: 123,
    });
    expect(result.transaction.feePayer?.toBase58()).toBe(SPONSOR);
  });

  it("rejects backend-prepared mints that change the owner, payer, or collection", async () => {
    const baseInput = {
      authorization: {
        allowed: true,
        authorizationId: "auth-sponsored-1",
      },
      collection: new PublicKey(COLLECTION),
      connection: {} as never,
      loomIndex: 9,
      name: "Anky Sojourn 9 Loom #0009",
      owner: new PublicKey(OWNER),
      payer: new PublicKey(SPONSOR),
      uri: "https://anky.app/devnet/metadata/looms/0009.json",
    };

    await expect(
      createBackendPreparedCoreLoomMintTransactionBuilder(async () =>
        preparedMintResponse({ owner: SPONSOR }),
      )(baseInput),
    ).rejects.toThrow("owner does not match");

    await expect(
      createBackendPreparedCoreLoomMintTransactionBuilder(async () =>
        preparedMintResponse({ payer: OWNER }),
      )(baseInput),
    ).rejects.toThrow("payer does not match");

    await expect(
      createBackendPreparedCoreLoomMintTransactionBuilder(async () =>
        preparedMintResponse({ collection: ASSET }),
      )(baseInput),
    ).rejects.toThrow("collection does not match");
  });

  it("keeps the default mint builder self-funded only", async () => {
    await expect(
      buildSelfFundedCoreLoomMintTransaction({
        authorization: {
          allowed: true,
          authorizationId: "auth-sponsored-1",
          sponsor: true,
        },
        collection: new PublicKey(COLLECTION),
        connection: {} as never,
        loomIndex: 9,
        name: "Anky Sojourn 9 Loom #0009",
        owner: new PublicKey(OWNER),
        payer: new PublicKey(SPONSOR),
        uri: "https://anky.app/devnet/metadata/looms/0009.json",
      }),
    ).rejects.toThrow("backend-prepared Core transaction");

    await expect(
      buildSelfFundedCoreLoomMintTransaction({
        collection: new PublicKey(COLLECTION),
        connection: {} as never,
        loomIndex: 9,
        name: "Anky Sojourn 9 Loom #0009",
        owner: new PublicKey(OWNER),
        payer: new PublicKey(SPONSOR),
        uri: "https://anky.app/devnet/metadata/looms/0009.json",
      }),
    ).rejects.toThrow("requires payer and owner to be the connected wallet");
  });
});

function preparedMintResponse(
  overrides: Partial<PreparedCoreLoomMintTransactionResponse> = {},
): PreparedCoreLoomMintTransactionResponse {
  return {
    asset: ASSET,
    authorizationId: "auth-sponsored-1",
    blockhash: BLOCKHASH,
    collection: COLLECTION,
    collectionAuthority: "Auth111111111111111111111111111111111111111",
    lastValidBlockHeight: 123,
    loomIndex: 9,
    mode: "self_funded",
    name: "Anky Sojourn 9 Loom #0009",
    owner: OWNER,
    payer: SPONSOR,
    transactionBase64: preparedTransactionBase64(),
    uri: "https://anky.app/devnet/metadata/looms/0009.json",
    ...overrides,
  };
}

function preparedTransactionBase64(): string {
  const owner = new PublicKey(OWNER);
  const transaction = new Transaction({
    feePayer: new PublicKey(SPONSOR),
    recentBlockhash: BLOCKHASH,
  });
  transaction.add(
    SystemProgram.transfer({
      fromPubkey: owner,
      lamports: 0,
      toPubkey: owner,
    }),
  );

  return Buffer.from(
    transaction.serialize({
      requireAllSignatures: false,
      verifySignatures: false,
    }),
  ).toString("base64");
}
