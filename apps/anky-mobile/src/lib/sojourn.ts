const DAY_MS = 24 * 60 * 60 * 1000;

export const SOJOURN_9_START_UTC = Date.UTC(2026, 2, 3, 0, 0, 0, 0);
export const SOJOURN_LENGTH_DAYS = 96;
export const DAYS_PER_KINGDOM = 12;

export const KINGDOM_COLORS = [
  "#FFFFFF",
  "#B98CFF",
  "#7A5CFF",
  "#3CA7FF",
  "#D85CFF",
  "#FF5CC8",
  "#FF8A33",
  "#FF2D2D",
] as const;

export type SojournDayStatus = "past" | "today" | "future";

export type SojournDay = {
  index: number;
  dayNumber: number;
  dateUtcMs: number;
  isoDate: string;
  kingdomIndex: number;
  dayInKingdom: number;
  status: SojournDayStatus;
};

export function startOfUtcDay(ms: number): number {
  const date = new Date(ms);

  return Date.UTC(
    date.getUTCFullYear(),
    date.getUTCMonth(),
    date.getUTCDate(),
    0,
    0,
    0,
    0,
  );
}

export function diffUtcDays(aUtcMs: number, bUtcMs: number): number {
  return Math.floor((startOfUtcDay(aUtcMs) - startOfUtcDay(bUtcMs)) / DAY_MS);
}

export function getSojournDayIndex(nowMs = Date.now()): number {
  return diffUtcDays(nowMs, SOJOURN_9_START_UTC);
}

export function getSojournDayNumber(nowMs = Date.now()): number {
  return getSojournDayIndex(nowMs) + 1;
}

export function isoDateFromUtcMs(ms: number): string {
  return new Date(ms).toISOString().slice(0, 10);
}

export function buildSojournDays(nowMs = Date.now()): SojournDay[] {
  const todayIndex = getSojournDayIndex(nowMs);

  return Array.from({ length: SOJOURN_LENGTH_DAYS }, (_, index) => {
    const dateUtcMs = SOJOURN_9_START_UTC + index * DAY_MS;
    let status: SojournDayStatus = "future";

    if (index < todayIndex) {
      status = "past";
    }

    if (index === todayIndex) {
      status = "today";
    }

    return {
      index,
      dayNumber: index + 1,
      dateUtcMs,
      isoDate: isoDateFromUtcMs(dateUtcMs),
      kingdomIndex: Math.floor(index / DAYS_PER_KINGDOM),
      dayInKingdom: (index % DAYS_PER_KINGDOM) + 1,
      status,
    };
  });
}
