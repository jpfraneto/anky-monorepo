import { describe, expect, it } from "vitest";

import { getLoomSealProofState, type LoomSeal } from "./types";

const EXPECTED_VERIFIER = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";

describe("getLoomSealProofState", () => {
  it("marks finalized proof metadata verified only when all public proof fields match", () => {
    expect(getLoomSealProofState(validSeal(), EXPECTED_VERIFIER)).toBe("verified");
  });

  it("fails finalized proof metadata when the verifier is missing or unexpected", () => {
    expect(getLoomSealProofState(validSeal({ proofVerifier: undefined }), EXPECTED_VERIFIER)).toBe(
      "failed",
    );
    expect(
      getLoomSealProofState(
        validSeal({ proofVerifier: "11111111111111111111111111111111" }),
        EXPECTED_VERIFIER,
      ),
    ).toBe("failed");
    expect(getLoomSealProofState(validSeal(), undefined)).toBe("failed");
  });

  it("fails finalized proof metadata when protocol, proof hash, or proof signature is missing", () => {
    expect(getLoomSealProofState(validSeal({ proofProtocolVersion: 2 }), EXPECTED_VERIFIER)).toBe(
      "failed",
    );
    expect(getLoomSealProofState(validSeal({ proofHash: undefined }), EXPECTED_VERIFIER)).toBe(
      "failed",
    );
    expect(
      getLoomSealProofState(validSeal({ proofTxSignature: undefined }), EXPECTED_VERIFIER),
    ).toBe("failed");
  });

  it("fails finalized proof metadata when proof UTC day is missing or mismatched", () => {
    expect(getLoomSealProofState(validSeal({ proofUtcDay: undefined }), EXPECTED_VERIFIER)).toBe(
      "failed",
    );
    expect(getLoomSealProofState(validSeal({ proofUtcDay: 20_000 }), EXPECTED_VERIFIER)).toBe(
      "failed",
    );
  });

  it("preserves proving and failed states without requiring verifier metadata", () => {
    expect(getLoomSealProofState(validSeal({ proofStatus: "pending" }), EXPECTED_VERIFIER)).toBe(
      "proving",
    );
    expect(getLoomSealProofState(validSeal({ proofStatus: "processed" }), EXPECTED_VERIFIER)).toBe(
      "proving",
    );
    expect(getLoomSealProofState(validSeal({ proofStatus: "failed" }), EXPECTED_VERIFIER)).toBe(
      "failed",
    );
    expect(getLoomSealProofState(null, EXPECTED_VERIFIER)).toBe("none");
  });
});

function validSeal(overrides: Partial<LoomSeal> = {}): LoomSeal {
  return {
    loomId: "loom-1",
    proofHash: "b".repeat(64),
    proofProtocolVersion: 1,
    proofStatus: "finalized",
    proofTxSignature: "mock_verified_sig",
    proofUtcDay: 19_999,
    proofVerifier: EXPECTED_VERIFIER,
    sessionHash: "a".repeat(64),
    txSignature: "mock_seal_sig",
    utcDay: 19_999,
    writer: "writer",
    ...overrides,
  };
}
