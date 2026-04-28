import { describe, expect, it } from "vitest";

import {
  buildSojournDays,
  diffUtcDays,
  getSojournDayIndex,
  getSojournDayNumber,
  isoDateFromUtcMs,
  SOJOURN_9_START_UTC,
  SOJOURN_LENGTH_DAYS,
  startOfUtcDay,
} from "./sojourn";

describe("9th sojourn UTC trail", () => {
  it("starts day 1 on 2026-03-03 UTC", () => {
    const days = buildSojournDays(Date.UTC(2026, 2, 3, 12));

    expect(days[0]).toMatchObject({
      dayNumber: 1,
      isoDate: "2026-03-03",
      status: "today",
    });
  });

  it("maps 2026-04-27 UTC to day 56", () => {
    const nowMs = Date.UTC(2026, 3, 27, 15, 30);

    expect(getSojournDayIndex(nowMs)).toBe(55);
    expect(getSojournDayNumber(nowMs)).toBe(56);

    const days = buildSojournDays(nowMs);
    expect(days[55]).toMatchObject({
      dayNumber: 56,
      isoDate: "2026-04-27",
      status: "today",
    });
  });

  it("sets day 96 to 2026-06-06 UTC", () => {
    const days = buildSojournDays(Date.UTC(2026, 5, 6, 1));

    expect(days).toHaveLength(SOJOURN_LENGTH_DAYS);
    expect(days[95]).toMatchObject({
      dayNumber: 96,
      isoDate: "2026-06-06",
      status: "today",
    });
  });

  it("uses UTC day boundaries", () => {
    const lateLocalRisk = Date.UTC(2026, 3, 27, 0, 0, 1);

    expect(startOfUtcDay(lateLocalRisk)).toBe(Date.UTC(2026, 3, 27));
    expect(diffUtcDays(lateLocalRisk, SOJOURN_9_START_UTC)).toBe(55);
  });

  it("marks every day future before the sojourn starts", () => {
    const days = buildSojournDays(Date.UTC(2026, 2, 2, 23, 59));

    expect(days.every((day) => day.status === "future")).toBe(true);
  });

  it("marks every day past after day 96 ends", () => {
    const days = buildSojournDays(Date.UTC(2026, 5, 7));

    expect(days.every((day) => day.status === "past")).toBe(true);
  });

  it("formats ISO dates from UTC milliseconds", () => {
    expect(isoDateFromUtcMs(Date.UTC(2026, 5, 6, 23, 59))).toBe("2026-06-06");
  });
});
