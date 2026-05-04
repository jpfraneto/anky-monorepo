import { createHash } from "node:crypto";

export interface ParsedAnkySession {
  sessionHash: string;
  deltas: number[];
  durationMs: number;
  keystrokeCount: number;
}

export class AnkyParseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AnkyParseError";
  }
}

export function parseAnky(raw: string): ParsedAnkySession {
  const sessionHash = createHash("sha256").update(raw, "utf8").digest("hex");
  const lines = raw.split("\n").filter(Boolean);

  const deltas: number[] = [];
  for (let index = 1; index < lines.length; index += 1) {
    const firstSpace = lines[index].indexOf(" ");
    if (firstSpace === -1) {
      continue;
    }

    const delta = Number(lines[index].slice(0, firstSpace));
    if (Number.isFinite(delta)) {
      deltas.push(Math.max(0, Math.min(7999, delta)));
    }
  }

  const durationMs = deltas.reduce((total, delta) => {
    const next = total + delta;
    if (!Number.isSafeInteger(next)) {
      throw new AnkyParseError("Session duration exceeds JavaScript's safe integer range.");
    }
    return next;
  }, 0);

  return {
    sessionHash,
    deltas,
    durationMs,
    keystrokeCount: lines.length,
  };
}
