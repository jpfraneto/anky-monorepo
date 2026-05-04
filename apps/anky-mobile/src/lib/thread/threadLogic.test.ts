import { describe, expect, it } from "vitest";

import { appendCharacter, appendFirstCharacter, closeSession, parseAnky } from "../ankyProtocol";
import {
  appendThreadMessagesToThread,
  createInitialThread,
  createThreadMessage,
  getRiteDurationMs,
  getThreadModeForRawAnky,
  hasReachedFreeThreadLimit,
  isCompleteRawAnky,
  MAX_FREE_THREAD_USER_MESSAGES,
} from "./threadLogic";

describe("thread logic", () => {
  it("classifies short sessions as fragments", () => {
    const raw = closeSession(
      appendCharacter(appendFirstCharacter("a", 1000), "b", 5000, 1000).raw,
    );

    expect(getThreadModeForRawAnky(raw)).toBe("fragment");
  });

  it("classifies full sessions as complete", () => {
    let raw = appendFirstCharacter("a", 1000);
    let previousAt = 1000;

    for (let index = 0; index < 60; index += 1) {
      const nextAt = previousAt + 7999;

      raw = appendCharacter(raw, "b", nextAt, previousAt).raw;
      previousAt = nextAt;
    }

    expect(getThreadModeForRawAnky(closeSession(raw))).toBe("complete");
  });

  it("uses reflection mode when a mirror already exists", () => {
    const raw = buildClosedSessionWithAcceptedDuration(472000);

    expect(getThreadModeForRawAnky(raw, true)).toBe("reflection");
  });

  it("counts the terminal silence in rite duration", () => {
    const raw = buildClosedSessionWithAcceptedDuration(472000);

    expect(getRiteDurationMs(parseAnky(raw))).toBe(480000);
    expect(isCompleteRawAnky(raw)).toBe(true);
  });

  it("keeps sessions below eight total minutes as fragments", () => {
    const raw = buildClosedSessionWithAcceptedDuration(471999);

    expect(isCompleteRawAnky(raw)).toBe(false);
    expect(getThreadModeForRawAnky(raw, true)).toBe("fragment");
  });

  it("rests after the free thread turn limit", () => {
    let thread = createInitialThread({
      mode: "complete",
      sessionHash: "a".repeat(64),
    });

    for (let index = 0; index < MAX_FREE_THREAD_USER_MESSAGES; index += 1) {
      thread = appendThreadMessagesToThread(thread, [
        createThreadMessage({ content: `message ${index}`, role: "user" }),
      ]);
    }

    expect(hasReachedFreeThreadLimit(thread)).toBe(true);
  });
});

function buildClosedSessionWithAcceptedDuration(durationMs: number): string {
  let raw = appendFirstCharacter("a", 1000);
  let previousAt = 1000;
  let remaining = durationMs;

  while (remaining > 0) {
    const delta = Math.min(remaining, 7999);
    const nextAt = previousAt + delta;

    raw = appendCharacter(raw, "b", nextAt, previousAt).raw;
    previousAt = nextAt;
    remaining -= delta;
  }

  return closeSession(raw);
}
