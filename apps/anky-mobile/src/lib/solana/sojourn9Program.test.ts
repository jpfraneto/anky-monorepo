import { describe, expect, it } from "vitest";

import {
  getSojourn9ProgramConfig,
  getSojourn9ProgramStatus,
  SOJOURN_9_CURRENT_SEAL_INSTRUCTION,
  SOJOURN_9_PROOF_VERIFIER_AUTHORITY,
  SOJOURN_9_PROGRAM_ID,
} from "./sojourn9Program";
import { createProgramLoomClient } from "./loomClient.program";

describe("Sojourn 9 program connection", () => {
  it("points at the checked-in Sojourn 9 program id", () => {
    expect(SOJOURN_9_PROGRAM_ID).toBe("4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX");
    expect(SOJOURN_9_PROOF_VERIFIER_AUTHORITY).toBe(
      "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP",
    );
    expect(SOJOURN_9_CURRENT_SEAL_INSTRUCTION).toBe("seal_anky");
  });

  it("marks the current program as devnet hash-only seal capable", () => {
    expect(getSojourn9ProgramConfig()).toMatchObject({
      cluster: "devnet",
      hashOnlyLoomSealSupported: true,
      programId: SOJOURN_9_PROGRAM_ID,
      proofVerifierAuthority: SOJOURN_9_PROOF_VERIFIER_AUTHORITY,
      sealAdapterMode: "mock",
      sealInstruction: "seal_anky",
    });
    expect(getSojourn9ProgramStatus()).toContain("Core ownership checked by seal program");
  });

  it("fails clearly if the legacy program LoomClient is used without wallet context", async () => {
    const client = createProgramLoomClient();

    await expect(
      client.sealAnky({
        loomId: "loom",
        sessionHash: "a".repeat(64),
        sessionUtcDay: Math.floor(Date.now() / 86_400_000),
      }),
    ).rejects.toThrow("Use src/lib/solana/sealAnky.ts");
  });
});
