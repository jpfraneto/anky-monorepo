import { describe, expect, it } from "vitest";
import { base58, base64 } from "@scure/base";

import { toPrivySiwsSignature } from "./siwsSignature";

describe("Privy SIWS signature normalization", () => {
  it("converts Phantom base58 signatures to Privy base64 signatures", () => {
    const signatureBytes = Uint8Array.from({ length: 64 }, (_, index) => index + 1);
    const phantomSignature = base58.encode(signatureBytes);

    expect(toPrivySiwsSignature(phantomSignature)).toBe(base64.encode(signatureBytes));
  });

  it("keeps base64 signatures normalized", () => {
    const signatureBytes = Uint8Array.from({ length: 64 }, (_, index) => 64 - index);
    const privySignature = base64.encode(signatureBytes);

    expect(toPrivySiwsSignature(privySignature)).toBe(privySignature);
  });

  it("rejects malformed signatures before calling Privy", () => {
    expect(() => toPrivySiwsSignature("not-a-solana-signature")).toThrow(
      "wallet returned an invalid Solana signature.",
    );
  });
});
