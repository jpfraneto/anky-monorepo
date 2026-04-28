import { describe, expect, it } from "vitest";

import {
  appendCharacter,
  appendFirstCharacter,
  closeSession,
  computeSessionHash,
  getReplayWords,
  parseAnky,
  reconstructText,
  verifyHash,
} from "./ankyProtocol";

describe(".anky protocol", () => {
  it("appendFirstCharacter creates `{epoch_ms} {char}\\n`", () => {
    expect(appendFirstCharacter("a", 1700000000000)).toBe("1700000000000 a\n");
  });

  it("appendCharacter creates zero-padded delta lines", () => {
    const raw = appendFirstCharacter("a", 1000);
    const next = appendCharacter(raw, "b", 1042, 1000);

    expect(next).toEqual({
      raw: "1000 a\n0042 b\n",
      acceptedAt: 1042,
    });
  });

  it("caps delta at 7999", () => {
    const raw = appendFirstCharacter("a", 1000);
    const next = appendCharacter(raw, "b", 9000, 1000);

    expect(next.raw).toBe("1000 a\n7999 b\n");
  });

  it("stores literal space as separator space plus typed space", () => {
    expect(appendFirstCharacter(" ", 1000)).toBe("1000  \n");

    const raw = appendFirstCharacter("a", 1000);
    const next = appendCharacter(raw, " ", 1007, 1000);

    expect(next.raw).toBe("1000 a\n0007  \n");
  });

  it("closeSession appends terminal `8000` with no trailing text", () => {
    const raw = appendFirstCharacter("a", 1000);

    expect(closeSession(raw)).toBe("1000 a\n8000");
    expect(closeSession(raw).endsWith("\n8000")).toBe(true);
  });

  it("parseAnky accepts valid files", () => {
    const raw = closeSession("1000 a\n0042  \n0000 b\n");
    const parsed = parseAnky(raw);

    expect(parsed.valid).toBe(true);
    expect(parsed.closed).toBe(true);
    expect(parsed.events).toHaveLength(3);
  });

  it("parseAnky rejects files without terminal 8000", () => {
    const parsed = parseAnky("1000 a\n0042 b\n");

    expect(parsed.valid).toBe(false);
    expect(parsed.errors).toContain("Missing terminal 8000 line.");
  });

  it("parseAnky rejects files with anything after terminal 8000", () => {
    const parsed = parseAnky("1000 a\n8000\n0001 b\n");

    expect(parsed.valid).toBe(false);
  });

  it("reconstructText returns only typed characters", () => {
    const raw = closeSession("1000 a\n0042  \n0000 b\n");

    expect(reconstructText(raw)).toBe("a b");
  });

  it("computeSessionHash is stable", async () => {
    const raw = closeSession("1000 a\n0042 b\n");
    const first = await computeSessionHash(raw);
    const second = await computeSessionHash(raw);

    expect(first).toBe(second);
    expect(first).toMatch(/^[a-f0-9]{64}$/);
  });

  it("verifyHash returns true for matching hash and false otherwise", async () => {
    const raw = closeSession("1000 a\n0042 b\n");
    const hash = await computeSessionHash(raw);

    await expect(verifyHash(raw, hash)).resolves.toBe(true);
    await expect(verifyHash(raw, "0".repeat(64))).resolves.toBe(false);
  });

  it("produces plain text protocol lines, not JSON", () => {
    const raw = closeSession("1000 a\n0042 b\n");

    expect(raw).toBe("1000 a\n0042 b\n8000");
    expect(() => JSON.parse(raw)).toThrow();
  });

  it("rejects a trailing newline after terminal 8000", () => {
    const parsed = parseAnky("1000 a\n8000\n");

    expect(parsed.valid).toBe(false);
    expect(parsed.errors).toContain("Missing terminal 8000 line.");
  });

  it("rejects a UTF-8 BOM", () => {
    const parsed = parseAnky("\ufeff1000 a\n8000");

    expect(parsed.valid).toBe(false);
    expect(parsed.errors).toContain("File must not start with a BOM.");
  });

  it("derives replay words from the .anky string", () => {
    const raw = closeSession("1000 h\n0010 i\n0005  \n0020 a\n");

    expect(getReplayWords(raw)).toEqual([
      {
        endIndex: 1,
        endMs: 10,
        startIndex: 0,
        startMs: 0,
        word: "hi",
      },
      {
        endIndex: 3,
        endMs: 35,
        startIndex: 3,
        startMs: 35,
        word: "a",
      },
    ]);
  });
});
