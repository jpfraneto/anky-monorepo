import * as FileSystem from "expo-file-system/legacy";
import { PublicKey } from "@solana/web3.js";

import { ensureAnkyDirectory, getAnkyDirectoryUri } from "../ankyStorage";
import { ankySolanaConfig } from "./ankySolanaConfig";
import type { AnkySolanaCluster } from "./ankySolanaConfig";

const SELECTED_LOOM_FILE = "selectedLoom.json";

export type SelectedLoom = {
  asset: string;
  collection: string;
  loomIndex?: number;
  mintMode?: "self_funded" | "invite_code";
  name: string;
  network: AnkySolanaCluster;
  owner?: string;
  recordStatus?: "confirmed" | "pending_record";
  recordedAt?: string;
  signature?: string;
  uri: string;
};

export function createDevnetLoomRecord({
  asset,
  collection = ankySolanaConfig.coreCollection,
  loomIndex = 1,
  mintMode,
  name,
  network = ankySolanaConfig.network,
  owner,
  recordStatus = "confirmed",
  recordedAt,
  signature,
  uri,
}: {
  asset: string;
  collection?: string;
  loomIndex?: number;
  mintMode?: SelectedLoom["mintMode"];
  name?: string;
  network?: AnkySolanaCluster;
  owner?: string;
  recordStatus?: SelectedLoom["recordStatus"];
  recordedAt?: string;
  signature?: string;
  uri?: string;
}): SelectedLoom {
  assertPublicKey(asset, "loom asset");
  assertPublicKey(collection, "collection");

  if (owner != null && owner.length > 0) {
    assertPublicKey(owner, "owner");
  }

  if (signature != null && signature.length === 0) {
    throw new Error("Invalid transaction signature.");
  }

  return {
    asset,
    collection,
    loomIndex,
    mintMode,
    name: name ?? `Anky Sojourn 9 Loom #${formatLoomNumber(loomIndex)}`,
    network,
    owner,
    recordStatus,
    recordedAt,
    signature,
    uri:
      uri ??
      `${ankySolanaConfig.loomMetadataBaseUrl ?? defaultLoomMetadataBaseUrl(network)}/${formatLoomNumber(
        loomIndex,
      )}.json`,
  };
}

export async function getSelectedLoom(): Promise<SelectedLoom | null> {
  await ensureAnkyDirectory();

  const uri = getSelectedLoomUri();
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return null;
  }

  try {
    const value = await FileSystem.readAsStringAsync(uri, {
      encoding: FileSystem.EncodingType.UTF8,
    });

    return JSON.parse(value) as SelectedLoom;
  } catch {
    await clearSelectedLoom();
    return null;
  }
}

export async function saveSelectedLoom(loom: SelectedLoom): Promise<void> {
  assertPublicKey(loom.asset, "loom asset");
  assertPublicKey(loom.collection, "collection");

  if (loom.owner != null && loom.owner.length > 0) {
    assertPublicKey(loom.owner, "owner");
  }

  await ensureAnkyDirectory();

  await FileSystem.writeAsStringAsync(getSelectedLoomUri(), JSON.stringify(loom, null, 2), {
    encoding: FileSystem.EncodingType.UTF8,
  });
}

export async function clearSelectedLoom(): Promise<void> {
  await FileSystem.deleteAsync(getSelectedLoomUri(), { idempotent: true });
}

export function shortAddress(value: string, size = 4): string {
  if (value.length <= size * 2 + 3) {
    return value;
  }

  return `${value.slice(0, size)}...${value.slice(-size)}`;
}

function assertPublicKey(value: string, label: string): void {
  try {
    new PublicKey(value);
  } catch {
    throw new Error(`Invalid ${label} public key.`);
  }
}

function formatLoomNumber(loomIndex: number): string {
  if (!Number.isInteger(loomIndex) || loomIndex < 1) {
    throw new Error("loomIndex must be a positive integer.");
  }

  return loomIndex.toString().padStart(4, "0");
}

function defaultLoomMetadataBaseUrl(network: AnkySolanaCluster): string {
  return network === "mainnet-beta"
    ? "https://anky.app/mainnet/metadata/looms"
    : "https://anky.app/devnet/metadata/looms";
}

function getSelectedLoomUri(): string {
  return `${getAnkyDirectoryUri()}${SELECTED_LOOM_FILE}`;
}
