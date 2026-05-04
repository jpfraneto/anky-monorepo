import { describe, expect, it } from "vitest";

import {
  ANKY_KINGDOMS,
  AnkySessionSummary,
  buildSojournDays,
  DAYS_PER_KINGDOM,
  getCurrentSojournDay,
  getDayState,
  getKingdomForDay,
  getNextSessionKindForToday,
  KINGDOM_COUNT,
  SOJOURN_LENGTH_DAYS,
} from "./sojourn";

const START = new Date("2026-03-03T00:00:00.000Z");

describe("Sojourn 9 date model", () => {
  it("uses UTC boundaries for the current sojourn day", () => {
    expect(getCurrentSojournDay(new Date("2026-03-02T23:59:59.999Z"))).toBe(1);
    expect(getCurrentSojournDay(START)).toBe(1);
    expect(getCurrentSojournDay(new Date("2026-03-03T23:59:59.999Z"))).toBe(1);
    expect(getCurrentSojournDay(new Date("2026-03-04T00:00:00.000Z"))).toBe(2);
    expect(getCurrentSojournDay(new Date("2026-06-06T23:59:59.999Z"))).toBe(96);
    expect(getCurrentSojournDay(new Date("2026-06-07T00:00:00.000Z"))).toBe(96);
  });

  it.each([
    [1, "Primordia"],
    [12, "Primordia"],
    [13, "Emblazion"],
    [24, "Emblazion"],
    [25, "Chryseos"],
    [36, "Chryseos"],
    [37, "Eleasis"],
    [48, "Eleasis"],
    [49, "Voxlumis"],
    [60, "Voxlumis"],
    [61, "Insightia"],
    [72, "Insightia"],
    [73, "Claridium"],
    [84, "Claridium"],
    [85, "Poiesis"],
    [96, "Poiesis"],
  ])("maps day %i to %s", (day, kingdomName) => {
    expect(getKingdomForDay(day).name).toBe(kingdomName);
  });

  it("builds exactly 96 days", () => {
    expect(buildSojournDays([], START)).toHaveLength(SOJOURN_LENGTH_DAYS);
  });

  it("marks all days future before the sojourn starts", () => {
    const days = buildSojournDays([], new Date("2026-03-02T23:59:59.999Z"));

    expect(days.every((day) => day.status === "future")).toBe(true);
  });

  it("assigns exactly 12 days to each kingdom", () => {
    const days = buildSojournDays([], START);

    ANKY_KINGDOMS.forEach((kingdom) => {
      expect(days.filter((day) => day.kingdom.index === kingdom.index)).toHaveLength(
        DAYS_PER_KINGDOM,
      );
    });
    expect(ANKY_KINGDOMS).toHaveLength(KINGDOM_COUNT);
  });

  it("marks a day with one daily seal as sealed", () => {
    const sessions: AnkySessionSummary[] = [
      makeSession({ createdAt: "2026-03-03T08:00:00.000Z", kind: "daily_seal", sojournDay: 1 }),
    ];

    expect(getDayState(1, sessions, new Date("2026-03-04T00:00:00.000Z"))).toMatchObject({
      status: "sealed",
      threadCount: 1,
      densityScore: 1,
    });
  });

  it("counts daily seals and extra threads as day density", () => {
    const sessions: AnkySessionSummary[] = [
      makeSession({ createdAt: "2026-03-03T08:00:00.000Z", kind: "daily_seal", sojournDay: 1 }),
      makeSession({ createdAt: "2026-03-03T10:00:00.000Z", kind: "extra_thread", sojournDay: 1 }),
    ];
    const day = getDayState(1, sessions, new Date("2026-03-04T00:00:00.000Z"));

    expect(day.threadCount).toBe(2);
    expect(day.densityScore).toBe(2);
    expect(day.extraThreads).toHaveLength(1);
  });

  it("returns daily_seal before today's seal and extra_thread after it", () => {
    const now = new Date("2026-03-03T18:00:00.000Z");
    const sessions: AnkySessionSummary[] = [
      makeSession({ createdAt: "2026-03-03T08:00:00.000Z", kind: "daily_seal", sojournDay: 1 }),
    ];

    expect(getNextSessionKindForToday([], now)).toBe("daily_seal");
    expect(getNextSessionKindForToday(sessions, now)).toBe("extra_thread");
  });

  it("does not count fragments as completed daily ankys", () => {
    const now = new Date("2026-03-03T18:00:00.000Z");
    const sessions: AnkySessionSummary[] = [
      makeSession({ createdAt: "2026-03-03T08:00:00.000Z", kind: "fragment", sojournDay: 1 }),
    ];
    const day = getDayState(1, sessions, now);

    expect(day.status).toBe("today_open");
    expect(day.threadCount).toBe(0);
    expect(getNextSessionKindForToday(sessions, now)).toBe("daily_seal");
  });
});

function makeSession(
  overrides: Pick<AnkySessionSummary, "createdAt" | "kind" | "sojournDay">,
): AnkySessionSummary {
  return {
    id: `${overrides.sojournDay}-${overrides.kind}-${overrides.createdAt}`,
    sessionHash: "a".repeat(64),
    ...overrides,
  };
}
