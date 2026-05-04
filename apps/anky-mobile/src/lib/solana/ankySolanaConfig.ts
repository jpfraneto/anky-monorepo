import { getPublicEnv } from "../config/env";

export type AnkySolanaCluster = "devnet" | "mainnet-beta";

export type AnkySolanaConfig = {
  cluster: AnkySolanaCluster;
  collectionUri?: string;
  coreProgramId: string;
  rpcUrl: string;
  coreCollection: string;
  loomMetadataBaseUrl?: string;
  network: AnkySolanaCluster;
  sealVerification?: string;
  sealProgramId: string;
};

const cluster = readCluster();

export const ankySolanaConfig: AnkySolanaConfig = {
  cluster,
  collectionUri: defaultCollectionUri(cluster),
  coreProgramId:
    getPublicEnv("EXPO_PUBLIC_ANKY_CORE_PROGRAM_ID") ??
    "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d",
  rpcUrl: getPublicEnv("EXPO_PUBLIC_SOLANA_RPC_URL") ?? defaultRpcUrl(cluster),
  coreCollection:
    getPublicEnv("EXPO_PUBLIC_ANKY_CORE_COLLECTION") ??
    "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
  loomMetadataBaseUrl: defaultLoomMetadataBaseUrl(cluster),
  network: cluster,
  sealVerification:
    cluster === "mainnet-beta"
      ? "mainnet_core_base_account_verification"
      : "devnet_core_base_account_verification",
  sealProgramId:
    getPublicEnv("EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID") ??
    "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX",
};

function readCluster(): AnkySolanaCluster {
  return getPublicEnv("EXPO_PUBLIC_SOLANA_CLUSTER") === "mainnet-beta"
    ? "mainnet-beta"
    : "devnet";
}

function defaultRpcUrl(value: AnkySolanaCluster): string {
  return value === "mainnet-beta"
    ? "https://api.mainnet-beta.solana.com"
    : "https://api.devnet.solana.com";
}

function defaultCollectionUri(value: AnkySolanaCluster): string {
  return value === "mainnet-beta"
    ? "https://anky.app/mainnet/metadata/sojourn-9-looms.json"
    : "https://anky.app/devnet/metadata/sojourn-9-looms.json";
}

function defaultLoomMetadataBaseUrl(value: AnkySolanaCluster): string {
  return value === "mainnet-beta"
    ? "https://anky.app/mainnet/metadata/looms"
    : "https://anky.app/devnet/metadata/looms";
}
