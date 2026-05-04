const DAY_MS = 24 * 60 * 60 * 1000;

export const SOJOURN_9_START_UTC = "2026-03-03T00:00:00.000Z";
export const SOJOURN_9_START_UTC_MS = Date.parse(SOJOURN_9_START_UTC);
export const SOJOURN_LENGTH_DAYS = 96;
export const KINGDOM_COUNT = 8;
export const DAYS_PER_KINGDOM = 12;

export type Kingdom = {
  index: number;
  name: string;
  chakra: string;
  energy: string;
  lesson: string;
  startDay: number;
  endDay: number;
  accent: string;
};

export type AnkySessionSummary = {
  id: string;
  createdAt: string;
  sojournDay: number;
  kind: "daily_seal" | "extra_thread" | "fragment";
  sessionHash?: string;
  localFileUri?: string;
  wordCount?: number;
  characterCount?: number;
  sealedOnchain?: boolean;
  reflectionId?: string;
  hasThread?: boolean;
};

export type DayState = {
  day: number;
  dateUtc: string;
  status: "future" | "today_open" | "today_sealed" | "sealed" | "unwoven";
  kingdom: Kingdom;
  dailySeal?: AnkySessionSummary;
  extraThreads: AnkySessionSummary[];
  fragments: AnkySessionSummary[];
  threadCount: number;
  densityScore: number;
};

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

export const ANKY_KINGDOMS: Kingdom[] = [
  {
    index: 1,
    name: "Primordia",
    chakra: "Root",
    energy: "survival",
    lesson: "You are here.",
    startDay: 1,
    endDay: 12,
    accent: "#B84A3A",
  },
  {
    index: 2,
    name: "Emblazion",
    chakra: "Sacral",
    energy: "passion",
    lesson: "What do you want?",
    startDay: 13,
    endDay: 24,
    accent: "#E06A2E",
  },
  {
    index: 3,
    name: "Chryseos",
    chakra: "Solar Plexus",
    energy: "willpower",
    lesson: "You are not waiting for permission.",
    startDay: 25,
    endDay: 36,
    accent: "#D6A63A",
  },
  {
    index: 4,
    name: "Eleasis",
    chakra: "Heart",
    energy: "compassion",
    lesson: "Let the wall soften.",
    startDay: 37,
    endDay: 48,
    accent: "#5C9F68",
  },
  {
    index: 5,
    name: "Voxlumis",
    chakra: "Throat",
    energy: "communication",
    lesson: "Say the thing.",
    startDay: 49,
    endDay: 60,
    accent: "#4E9FCF",
  },
  {
    index: 6,
    name: "Insightia",
    chakra: "Third Eye",
    energy: "intuition",
    lesson: "You already know.",
    startDay: 61,
    endDay: 72,
    accent: "#5B63D6",
  },
  {
    index: 7,
    name: "Claridium",
    chakra: "Crown",
    energy: "enlightenment",
    lesson: "Who is asking?",
    startDay: 73,
    endDay: 84,
    accent: "#A678E2",
  },
  {
    index: 8,
    name: "Poiesis",
    chakra: "8th",
    energy: "creativity",
    lesson: "Get out of the way.",
    startDay: 85,
    endDay: 96,
    accent: "#C77AF2",
  },
];

export const KINGDOM_COLORS = ANKY_KINGDOMS.map((kingdom) => kingdom.accent) as [
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
];

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

export function isoDateFromUtcMs(ms: number): string {
  return new Date(ms).toISOString().slice(0, 10);
}

export function getCurrentSojournDay(now: Date = new Date()): number {
  const day = diffUtcDays(now.getTime(), SOJOURN_9_START_UTC_MS) + 1;

  if (day < 1) {
    return 1;
  }

  if (day > SOJOURN_LENGTH_DAYS) {
    // TODO: route post-day-96 users into Great Slumber or the next sojourn.
    return SOJOURN_LENGTH_DAYS;
  }

  return day;
}

export function getDateForSojournDay(day: number): string {
  const safeDay = clampDay(day);

  return new Date(SOJOURN_9_START_UTC_MS + (safeDay - 1) * DAY_MS).toISOString();
}

export function getKingdomForDay(day: number): Kingdom {
  const safeDay = clampDay(day);
  const kingdom = ANKY_KINGDOMS.find(
    (item) => safeDay >= item.startDay && safeDay <= item.endDay,
  );

  if (kingdom == null) {
    return ANKY_KINGDOMS[ANKY_KINGDOMS.length - 1];
  }

  return kingdom;
}

export function getDayState(
  day: number,
  sessions: AnkySessionSummary[],
  now: Date = new Date(),
): DayState {
  const safeDay = clampDay(day);
  const sortedSessions = sortSessions(
    sessions.filter((session) => session.sojournDay === safeDay),
  );
  const dailySeal = sortedSessions.find((session) => session.kind === "daily_seal");
  const extraThreads = sortedSessions.filter((session) => session.kind === "extra_thread");
  const fragments = sortedSessions.filter((session) => session.kind === "fragment");
  const threadCount = (dailySeal == null ? 0 : 1) + extraThreads.length;
  const currentDay = getCurrentSojournDay(now);
  const beforeStart = startOfUtcDay(now.getTime()) < SOJOURN_9_START_UTC_MS;
  let status: DayState["status"];

  if (beforeStart || safeDay > currentDay) {
    status = "future";
  } else if (safeDay === currentDay) {
    status = dailySeal == null ? "today_open" : "today_sealed";
  } else {
    status = dailySeal == null ? "unwoven" : "sealed";
  }

  return {
    day: safeDay,
    dateUtc: getDateForSojournDay(safeDay),
    status,
    kingdom: getKingdomForDay(safeDay),
    dailySeal,
    extraThreads,
    fragments,
    threadCount,
    densityScore: threadCount,
  };
}

export function buildSojournDays(
  sessions: AnkySessionSummary[] = [],
  now: Date = new Date(),
): DayState[] {
  return Array.from({ length: SOJOURN_LENGTH_DAYS }, (_, index) =>
    getDayState(index + 1, sessions, now),
  );
}

export function getTodaySealState(
  sessions: AnkySessionSummary[],
  now: Date = new Date(),
): "open" | "sealed" {
  return getDayState(getCurrentSojournDay(now), sessions, now).dailySeal == null
    ? "open"
    : "sealed";
}

export function getNextSessionKindForToday(
  sessions: AnkySessionSummary[],
  now: Date = new Date(),
): "daily_seal" | "extra_thread" {
  return getTodaySealState(sessions, now) === "open" ? "daily_seal" : "extra_thread";
}

export function getSojournDayIndex(nowMs = Date.now()): number {
  return diffUtcDays(nowMs, SOJOURN_9_START_UTC_MS);
}

export function getSojournDayNumber(nowMs = Date.now()): number {
  return getCurrentSojournDay(new Date(nowMs));
}

export function buildLegacySojournDays(nowMs = Date.now()): SojournDay[] {
  const todayIndex = getSojournDayIndex(nowMs);

  return Array.from({ length: SOJOURN_LENGTH_DAYS }, (_, index) => {
    const dateUtcMs = SOJOURN_9_START_UTC_MS + index * DAY_MS;
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

function clampDay(day: number): number {
  if (!Number.isFinite(day)) {
    return 1;
  }

  return Math.max(1, Math.min(SOJOURN_LENGTH_DAYS, Math.floor(day)));
}

function sortSessions(sessions: AnkySessionSummary[]): AnkySessionSummary[] {
  return [...sessions].sort(
    (left, right) => Date.parse(left.createdAt) - Date.parse(right.createdAt),
  );
}
