import { getAnkyApiClient } from "../api/client";
import { ankySolanaConfig } from "./ankySolanaConfig";
import type { AnkySolanaConfig } from "./ankySolanaConfig";

export type MobileSolanaRuntimeConfig = AnkySolanaConfig & {
  source: "backend" | "local";
};

export async function loadMobileSolanaConfig(): Promise<MobileSolanaRuntimeConfig> {
  const api = getAnkyApiClient();

  if (api == null) {
    return {
      ...ankySolanaConfig,
      source: "local",
    };
  }

  try {
    const response = await api.getMobileSolanaConfig();

    return {
      cluster: response.cluster,
      collectionUri: response.collectionUri,
      coreCollection: response.coreCollection,
      coreProgramId: response.coreProgramId,
      loomMetadataBaseUrl: response.loomMetadataBaseUrl,
      network: response.network,
      proofVerifierAuthority: response.proofVerifierAuthority,
      rpcUrl: response.rpcUrl,
      sealProgramId: response.sealProgramId,
      sealVerification: response.sealVerification,
      source: "backend",
    };
  } catch (error) {
    console.warn("Falling back to local Solana config.", error);
    return {
      ...ankySolanaConfig,
      source: "local",
    };
  }
}
