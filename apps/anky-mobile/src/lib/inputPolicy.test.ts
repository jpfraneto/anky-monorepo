import { describe, expect, it } from "vitest";

import { getAcceptedInputCharacter } from "./inputPolicy";

describe("anky input policy", () => {
  it("accepts one visible character appended to the hidden input", () => {
    expect(getAcceptedInputCharacter("ab", "abc")).toEqual({ accepted: true, char: "c" });
    expect(getAcceptedInputCharacter("ab", "ab ")).toEqual({ accepted: true, char: " " });
  });

  it("rejects paste and autocomplete insertions", () => {
    expect(getAcceptedInputCharacter("a", "abc")).toMatchObject({
      accepted: false,
      reason: "multi_character",
    });
  });

  it("rejects deletion and replacement edits", () => {
    expect(getAcceptedInputCharacter("abc", "ab")).toMatchObject({
      accepted: false,
      reason: "deletion",
    });
    expect(getAcceptedInputCharacter("abc", "axc")).toMatchObject({
      accepted: false,
      reason: "replacement",
    });
  });

  it("rejects newline and unsupported multi-codepoint insertions", () => {
    expect(getAcceptedInputCharacter("a", "a\n")).toMatchObject({
      accepted: false,
      reason: "unsupported_character",
    });
    expect(getAcceptedInputCharacter("a", "a👩‍💻")).toMatchObject({
      accepted: false,
      reason: "multi_character",
    });
  });
});
