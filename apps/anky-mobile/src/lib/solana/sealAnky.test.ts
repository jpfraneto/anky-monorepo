import { sha256 } from "@noble/hashes/sha2.js";
import { utf8ToBytes } from "@noble/hashes/utils.js";
import { Buffer } from "buffer";
import { describe, expect, it } from "vitest";

import {
  buildSealAnkyInstructionData,
  getUtcDayFromUnixMs,
  isCurrentUtcDay,
  normalizeSessionHash,
} from "./sealAnky";

describe("sealAnky instruction encoding", () => {
  it("encodes the Anchor seal_anky discriminator, exact hash bytes, and UTC day", () => {
    const sessionHash = "01".repeat(32);
    const utcDay = 19_999;
    const data = buildSealAnkyInstructionData(Buffer.from(sessionHash, "hex"), utcDay);
    const expectedDiscriminator = sha256(utf8ToBytes("global:seal_anky")).slice(0, 8);
    const expectedUtcDay = Buffer.alloc(8);
    expectedUtcDay.writeBigInt64LE(BigInt(utcDay));

    expect(data).toHaveLength(48);
    expect(data.subarray(0, 8)).toEqual(Buffer.from(expectedDiscriminator));
    expect(data.subarray(8, 40).toString("hex")).toBe(sessionHash);
    expect(data.subarray(40, 48)).toEqual(expectedUtcDay);
  });

  it("normalizes session hash casing without changing bytes", () => {
    expect(normalizeSessionHash("AA".repeat(32))).toBe("aa".repeat(32));
    expect(() => normalizeSessionHash("aa")).toThrow("64 hex");
  });

  it("derives UTC days consistently at boundaries", () => {
    expect(getUtcDayFromUnixMs(0)).toBe(0);
    expect(getUtcDayFromUnixMs(86_399_999)).toBe(0);
    expect(getUtcDayFromUnixMs(86_400_000)).toBe(1);
    expect(getUtcDayFromUnixMs(-1)).toBe(-1);
  });

  it("checks the current UTC day using an injected clock", () => {
    expect(isCurrentUtcDay(10, 10 * 86_400_000)).toBe(true);
    expect(isCurrentUtcDay(9, 10 * 86_400_000)).toBe(false);
  });
});
