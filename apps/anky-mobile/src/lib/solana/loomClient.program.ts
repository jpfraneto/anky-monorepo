import { getSojourn9ProgramConfig } from "./sojourn9Program";
import { assertSessionHash, Loom, LoomClient, SealAnkyInput, SealAnkyResult } from "./types";

export function createProgramLoomClient(): LoomClient {
  return {
    async getOwnedLooms(): Promise<Loom[]> {
      return [];
    },

    async getSelectedLoom(): Promise<Loom | null> {
      return null;
    },

    async sealAnky(input: SealAnkyInput): Promise<SealAnkyResult> {
      assertSessionHash(input.sessionHash);

      const config = getSojourn9ProgramConfig();

      throw new Error(
        `Sojourn 9 program ${config.programId} is configured for ${config.sealInstruction}, but this legacy LoomClient does not have wallet, connection, or Loom asset context. Use src/lib/solana/sealAnky.ts with a Privy Solana wallet adapter.`,
      );
    },
  };
}
