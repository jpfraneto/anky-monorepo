import { parseAnky } from "../../lib/ankyProtocol";
import type { SavedAnkyFile } from "../../lib/ankyStorage";
import type { AnkySessionSummary, DayState } from "../../lib/sojourn";
import {
  getRiteDurationMs,
  isCompleteRawAnky,
} from "../../lib/thread/threadLogic";
import type { SojournMapAnky, SojournMapDay } from "./SojournMap.types";

const avatarDefault = require("../../../assets/sojourn-map/avatars/anky-default.png");
const avatarBegin = require("../../../assets/sojourn-map/avatars/begin-again-softly.png");
const avatarBreathe = require("../../../assets/sojourn-map/avatars/room-to-breathe.png");
const avatarCalm = require("../../../assets/sojourn-map/avatars/carry-this-calm.png");
const AVATARS = [avatarDefault, avatarBegin, avatarBreathe, avatarCalm] as const;

export function buildSojournMapDays({
  currentDay,
  days,
  files,
}: {
  currentDay: number;
  days: DayState[];
  files: SavedAnkyFile[];
}): SojournMapDay[] {
  const filesByHash = new Map(files.map((file) => [file.hash, file]));

  return days.map((day) => {
    const completeSessions = [day.dailySeal, ...day.extraThreads].filter(
      (session): session is AnkySessionSummary => session != null,
    );
    const ankys = completeSessions
      .map((session, index) =>
        mapSessionToAnky({
          day: day.day,
          file: session.sessionHash == null ? undefined : filesByHash.get(session.sessionHash),
          index,
          session,
        }),
      )
      .filter((anky): anky is SojournMapAnky => anky != null);

    return {
      ankyCount: ankys.length,
      ankys,
      day: day.day,
      isCurrent: day.day === currentDay,
      isFuture: day.day > currentDay,
    };
  });
}

function mapSessionToAnky({
  day,
  file,
  index,
  session,
}: {
  day: number;
  file?: SavedAnkyFile;
  index: number;
  session: AnkySessionSummary;
}): SojournMapAnky | null {
  if (session.kind === "fragment") {
    return null;
  }

  const durationMs =
    file == null ? undefined : getRiteDurationMs(parseAnky(file.raw)) ?? undefined;

  if (file != null && !isCompleteRawAnky(file.raw)) {
    return null;
  }

  const sessionHash = session.sessionHash ?? file?.hash;
  const firstLine = firstVisibleLine(file?.preview) ?? "open this anky to remember what was written.";
  const durationLabel = buildDurationLabel({ durationMs, session });

  return {
    avatar: AVATARS[(day + index) % AVATARS.length],
    day,
    durationLabel,
    fileName: file?.fileName,
    firstLine,
    id: sessionHash ?? session.id,
    sessionHash,
    title: buildTitle({ firstLine, index, session }),
  };
}

function buildDurationLabel({
  durationMs,
  session,
}: {
  durationMs?: number;
  session: AnkySessionSummary;
}): string {
  const minutes =
    durationMs == null ? 8 : Math.max(1, Math.round(durationMs / 60000));
  const status = [
    `${minutes} min`,
    session.sealedOnchain === true ? "sealed" : "local",
    session.reflectionId != null ? "reflected" : null,
  ]
    .filter(Boolean)
    .join(" • ");

  return status.length === 0 ? "8 min • local" : status;
}

function buildTitle({
  firstLine,
  index,
  session,
}: {
  firstLine: string;
  index: number;
  session: AnkySessionSummary;
}): string {
  if (session.reflectionId != null) {
    return "reflected anky";
  }

  const words = firstLine
    .replace(/[^\p{L}\p{N}\s'-]/gu, "")
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 4);

  if (words.length >= 2) {
    return words.join(" ");
  }

  return index === 0 ? "daily anky" : `extra anky ${index + 1}`;
}

function firstVisibleLine(value?: string): string | null {
  if (value == null) {
    return null;
  }

  const line = value
    .split("\n")
    .map((item) => item.trim())
    .find((item) => item.length > 0);

  return line == null ? null : line;
}
