import { getPublicEnv } from "../config/env";

export type SolanaCluster = "devnet" | "mainnet-beta";
export type SolanaSealAdapterMode = "mock" | "program";

export const SOJOURN_9_PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
export const SOJOURN_9_PROGRAM_NAME = "anky_seal_program";
export const SOJOURN_9_CURRENT_SEAL_INSTRUCTION = "seal_anky";

export type Sojourn9ProgramConfig = {
  cluster: SolanaCluster;
  hashOnlyLoomSealSupported: boolean;
  programId: string;
  rpcUrl?: string;
  sealAdapterMode: SolanaSealAdapterMode;
  sealInstruction: typeof SOJOURN_9_CURRENT_SEAL_INSTRUCTION;
};

export function getSojourn9ProgramConfig(): Sojourn9ProgramConfig {
  return {
    cluster: readCluster(),
    hashOnlyLoomSealSupported: true,
    programId: getPublicEnv("EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID") ?? SOJOURN_9_PROGRAM_ID,
    rpcUrl: getPublicEnv("EXPO_PUBLIC_SOLANA_RPC_URL"),
    sealAdapterMode: readSealAdapterMode(),
    sealInstruction: SOJOURN_9_CURRENT_SEAL_INSTRUCTION,
  };
}

export function getSojourn9ProgramStatus(): string {
  const config = getSojourn9ProgramConfig();
  const shortProgramId = `${config.programId.slice(0, 4)}...${config.programId.slice(-4)}`;

  if (config.hashOnlyLoomSealSupported) {
    return `${config.cluster} program ${shortProgramId}; hash-only Loom seal enabled; Core ownership proof is incomplete`;
  }

  return `${config.cluster} configured id ${shortProgramId}; hash-only Loom seal disabled`;
}

function readCluster(): SolanaCluster {
  const value = getPublicEnv("EXPO_PUBLIC_SOLANA_CLUSTER");

  return value === "mainnet-beta" ? "mainnet-beta" : "devnet";
}

function readSealAdapterMode(): SolanaSealAdapterMode {
  return getPublicEnv("EXPO_PUBLIC_SOLANA_SEAL_ADAPTER") === "program" ? "program" : "mock";
}
