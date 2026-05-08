import { sha256 } from "@noble/hashes/sha2.js";
import { utf8ToBytes } from "@noble/hashes/utils.js";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { Buffer } from "buffer";
import { describe, expect, it } from "vitest";

import {
  buildSealAnkyInstruction,
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

  it("orders writer and sponsor payer accounts for the sponsored Anchor instruction", () => {
    const writer = new PublicKey("11111111111111111111111111111111");
    const payer = new PublicKey("So11111111111111111111111111111111111111112");
    const loomAsset = new PublicKey("4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9");
    const coreCollection = new PublicKey("F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u");
    const programId = new PublicKey("4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX");
    const loomState = new PublicKey("H3jY1khbKv7n8qah1hRheXNjGA8GSctK2TqN69ZN2Eyf");
    const dailySeal = new PublicKey("9LqQGfGpCFpfMw2wWZ5WzHTkV1ov87XJadBvXyxWDFcZ");
    const hashSeal = new PublicKey("6sB6cCdbN3JbE8QS9LMC8XLucBa42i2CCdojS4jCyv8P");
    const sessionHashBytes = Buffer.from("02".repeat(32), "hex");

    const instruction = buildSealAnkyInstruction({
      coreCollection,
      dailySeal,
      hashSeal,
      loomAsset,
      loomState,
      payer,
      programId,
      sessionHashBytes,
      utcDay: 20_580,
      writer,
    });

    expect(instruction.programId.toBase58()).toBe(programId.toBase58());
    expect(instruction.keys.map((key) => key.pubkey.toBase58())).toEqual([
      writer.toBase58(),
      payer.toBase58(),
      loomAsset.toBase58(),
      coreCollection.toBase58(),
      loomState.toBase58(),
      dailySeal.toBase58(),
      hashSeal.toBase58(),
      SystemProgram.programId.toBase58(),
    ]);
    expect(instruction.keys[0]).toMatchObject({ isSigner: true, isWritable: false });
    expect(instruction.keys[1]).toMatchObject({ isSigner: true, isWritable: true });
    expect(instruction.keys[4]).toMatchObject({ isSigner: false, isWritable: true });
    expect(instruction.keys[5]).toMatchObject({ isSigner: false, isWritable: true });
    expect(instruction.keys[6]).toMatchObject({ isSigner: false, isWritable: true });
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
