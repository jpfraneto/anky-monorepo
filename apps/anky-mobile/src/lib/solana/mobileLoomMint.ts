import { Connection } from "@solana/web3.js";

import type { AnkyApiClient } from "../api/ankyApi";
import type { MobileLoomMint } from "../api/types";
import {
  createBackendPreparedCoreLoomMintTransactionBuilder,
  mintAnkyLoom,
  type MintAnkyLoomStatus,
} from "./mintLoom";
import {
  clearSelectedLoom,
  createDevnetLoomRecord,
  getSelectedLoomForWallet,
  saveSelectedLoom,
} from "./loomStorage";
import type { SelectedLoom } from "./loomStorage";
import type { MobileSolanaRuntimeConfig } from "./mobileSolanaConfig";
import type { AnkySolanaWallet } from "./walletTypes";

export type MintAndSaveLoomInput = {
  api: AnkyApiClient;
  config: MobileSolanaRuntimeConfig;
  connection: Connection;
  inviteCode?: string;
  loomIndex?: number;
  onStatus?: (status: MintAndSaveLoomStatus) => void;
  wallet: AnkySolanaWallet;
};

export type MintAndSaveLoomStatus = MintAnkyLoomStatus | "recording";

export type MintAndSaveLoomResult = {
  recordError?: unknown;
  recordStatus: "confirmed" | "pending_record";
  selectedLoom: SelectedLoom;
};

export async function mintAndSaveLoom({
  api,
  config,
  connection,
  inviteCode,
  loomIndex = 1,
  onStatus,
  wallet,
}: MintAndSaveLoomInput): Promise<MintAndSaveLoomResult> {
  const trimmedInviteCode = inviteCode?.trim();
  const mint = await mintAnkyLoom({
    buildCoreLoomMintTransaction: createBackendPreparedCoreLoomMintTransactionBuilder(
      api.prepareMobileLoomMint,
    ),
    collection: config.coreCollection,
    connection,
    createMintAuthorization: (input) =>
      api.createMobileMintAuthorization({
        collection: input.collection,
        inviteCode: input.inviteCode,
        loomIndex: input.loomIndex,
        payer: input.payer,
        wallet: input.owner,
      }),
    inviteCode: trimmedInviteCode == null || trimmedInviteCode.length === 0 ? undefined : trimmedInviteCode,
    loomIndex,
    metadataUri: `${config.loomMetadataBaseUrl ?? defaultLoomMetadataBaseUrl(config.network)}/${formatLoomNumber(loomIndex)}.json`,
    onStatus,
    wallet,
  });

  const confirmedLoom = createDevnetLoomRecord({
    asset: mint.asset,
    collection: mint.collection,
    loomIndex,
    mintMode: mint.mintMode,
    name: mint.name,
    network: config.network,
    owner: mint.owner,
    recordStatus: "confirmed",
    signature: mint.signature,
    uri: mint.uri,
  });

  try {
    onStatus?.("recording");
    const record = await api.recordMobileLoomMint({
      coreCollection: mint.collection,
      loomAsset: mint.asset,
      loomIndex,
      metadataUri: mint.uri,
      mintMode: mint.mintMode,
      signature: mint.signature,
      status: "confirmed",
      wallet: mint.owner,
    });
    const selectedLoom = toSelectedLoom(record.loom);

    await saveSelectedLoom(selectedLoom);

    return {
      recordStatus: "confirmed",
      selectedLoom,
    };
  } catch (recordError) {
    const pendingLoom = {
      ...confirmedLoom,
      recordStatus: "pending_record" as const,
    };

    await saveSelectedLoom(pendingLoom);

    return {
      recordError,
      recordStatus: "pending_record",
      selectedLoom: pendingLoom,
    };
  }
}

export async function retrySelectedLoomRecord({
  api,
  loom,
  wallet,
}: {
  api: AnkyApiClient;
  loom: SelectedLoom;
  wallet?: string;
}): Promise<SelectedLoom> {
  if (loom.signature == null || loom.signature.length === 0) {
    throw new Error("Selected Loom does not have a mint transaction signature to record.");
  }

  const owner = wallet ?? loom.owner;

  if (owner == null || owner.length === 0) {
    throw new Error("Selected Loom does not have an owner wallet to record.");
  }

  const record = await api.recordMobileLoomMint({
    coreCollection: loom.collection,
    loomAsset: loom.asset,
    loomIndex: loom.loomIndex ?? 1,
    metadataUri: loom.uri,
    mintMode: loom.mintMode ?? "self_funded",
    signature: loom.signature,
    status: "confirmed",
    wallet: owner,
  });
  const selectedLoom = toSelectedLoom(record.loom);

  await saveSelectedLoom(selectedLoom);

  return selectedLoom;
}

export async function restoreRecordedLoomSelection({
  api,
  wallet,
}: {
  api: AnkyApiClient;
  wallet: string;
}): Promise<{
  looms: MobileLoomMint[];
  selectedLoom: SelectedLoom | null;
}> {
  const response = await api.lookupMobileLooms(wallet);
  const selectedLoom = await getSelectedLoomForWallet(wallet);
  const confirmed = newestConfirmedLoom(response.looms);

  if (confirmed != null) {
    const restored = toSelectedLoom(confirmed);

    if (selectedLoom?.asset !== restored.asset) {
      await saveSelectedLoom(restored);
    }

    return {
      looms: response.looms,
      selectedLoom: restored,
    };
  }

  if (selectedLoom != null) {
    await clearSelectedLoom();
  }

  return {
    looms: response.looms,
    selectedLoom: null,
  };
}

export function toSelectedLoom(loom: MobileLoomMint): SelectedLoom {
  return createDevnetLoomRecord({
    asset: loom.loomAsset,
    collection: loom.coreCollection,
    loomIndex: loom.loomIndex ?? 1,
    mintMode: loom.mintMode === "invite_code" ? "invite_code" : "self_funded",
    network: loom.network,
    owner: loom.wallet,
    recordStatus: loom.status === "confirmed" || loom.status === "finalized" ? "confirmed" : "pending_record",
    recordedAt: loom.createdAt,
    signature: loom.signature,
    uri: loom.metadataUri,
  });
}

function newestConfirmedLoom(looms: MobileLoomMint[]): MobileLoomMint | null {
  const confirmed = looms.filter(
    (loom) => loom.status === "confirmed" || loom.status === "finalized",
  );

  confirmed.sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt));

  return confirmed[0] ?? null;
}

function formatLoomNumber(loomIndex: number): string {
  return loomIndex.toString().padStart(4, "0");
}

function defaultLoomMetadataBaseUrl(network: MobileSolanaRuntimeConfig["network"]): string {
  return network === "mainnet-beta"
    ? "https://anky.app/mainnet/metadata/looms"
    : "https://anky.app/devnet/metadata/looms";
}
