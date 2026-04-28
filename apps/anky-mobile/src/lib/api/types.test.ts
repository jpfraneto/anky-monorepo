import { describe, expect, it } from "vitest";

import {
  assertCreditReceipt,
  CREDIT_COSTS,
  isCreditReceipt,
  isProcessingType,
} from "./types";

describe("credit and processing types", () => {
  it("maps processing products to canonical credit costs", () => {
    expect(CREDIT_COSTS).toEqual({
      deep_mirror: 8,
      full_anky: 5,
      full_sojourn_archive: 88,
      image: 3,
      reflection: 1,
    });
  });

  it("validates processing types", () => {
    expect(isProcessingType("full_anky")).toBe(true);
    expect(isProcessingType("journal_prompt")).toBe(false);
  });

  it("validates signed credit receipts", () => {
    const receipt = {
      creditsRemaining: 37,
      creditsSpent: 5,
      expiresAt: 1700000600000,
      issuedAt: 1700000000000,
      nonce: "nonce",
      processingType: "full_anky",
      receiptVersion: 1,
      signature: "sig",
      ticketId: "ticket",
    };

    expect(isCreditReceipt(receipt)).toBe(true);
    expect(() => assertCreditReceipt(receipt)).not.toThrow();
    expect(isCreditReceipt({ ...receipt, processingType: "unknown" })).toBe(false);
  });
});
