import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex, utf8ToBytes } from "@noble/hashes/utils.js";

const TERMINAL_LINE = "8000";
const TERMINAL_RECORD = `\n${TERMINAL_LINE}`;
const MAX_DELTA_MS = 7999;
const DELTA_WIDTH = 4;
const SPACE_TOKEN = "SPACE";

export type ParsedAnkyEvent = {
  acceptedAt: number;
  char: string;
  deltaMs: number | null;
  line: string;
};

export type ParseAnkyResult = {
  valid: boolean;
  closed: boolean;
  startedAt: number | null;
  events: ParsedAnkyEvent[];
  errors: string[];
};

export type ReplayWord = {
  endIndex: number;
  endMs: number;
  startIndex: number;
  startMs: number;
  word: string;
};

export function isAcceptedCharacter(input: string): boolean {
  const characters = Array.from(input);

  if (characters.length !== 1 || characters[0] !== input) {
    return false;
  }

  const codePoint = input.codePointAt(0);

  if (codePoint == null) {
    return false;
  }

  return codePoint > 31 && codePoint !== 127;
}

export function appendFirstCharacter(char: string, now: number): string {
  assertAcceptedCharacter(char);
  assertTimestamp(now, "now");

  return `${now} ${serializeCharacter(char)}\n`;
}

export function appendCharacter(
  raw: string,
  char: string,
  now: number,
  previousAt: number,
): { raw: string; acceptedAt: number } {
  assertAcceptedCharacter(char);
  assertTimestamp(now, "now");
  assertTimestamp(previousAt, "previousAt");

  if (raw.length === 0) {
    throw new Error("Cannot append a subsequent character to an empty .anky session.");
  }

  if (hasTerminalLine(raw)) {
    throw new Error("Cannot append to a closed .anky session.");
  }

  if (!raw.endsWith("\n")) {
    throw new Error("Cannot append to a malformed .anky draft.");
  }

  const elapsed = Math.max(0, now - previousAt);
  const capped = Math.min(elapsed, MAX_DELTA_MS);
  const padded = String(capped).padStart(DELTA_WIDTH, "0");

  return {
    raw: `${raw}${padded} ${serializeCharacter(char)}\n`,
    acceptedAt: now,
  };
}

export function closeSession(raw: string): string {
  if (hasTerminalLine(raw)) {
    return raw;
  }

  if (raw.length === 0) {
    throw new Error("Cannot close an empty .anky session.");
  }

  if (!raw.endsWith("\n")) {
    throw new Error("Cannot close a malformed .anky session.");
  }

  return `${raw}${TERMINAL_LINE}`;
}

export function parseAnky(raw: string): ParseAnkyResult {
  const errors: string[] = [];
  const events: ParsedAnkyEvent[] = [];

  if (raw.length === 0) {
    return {
      valid: false,
      closed: false,
      startedAt: null,
      events,
      errors: ["File is empty."],
    };
  }

  if (raw.charCodeAt(0) === 0xfeff) {
    errors.push("File must not start with a BOM.");
  }

  if (raw.includes("\r")) {
    errors.push("Line endings must be LF only.");
  }

  const closed = hasTerminalLine(raw);

  if (!closed) {
    errors.push("Missing terminal 8000 line.");
  }

  const eventRaw = closed ? raw.slice(0, -TERMINAL_RECORD.length) : raw;
  const eventLines = eventRaw.endsWith("\n")
    ? eventRaw.slice(0, -1).split("\n")
    : eventRaw.split("\n");

  if (eventLines.length === 0) {
    errors.push("Session must contain at least one accepted character.");
  }

  let startedAt: number | null = null;
  let acceptedAt: number | null = null;

  eventLines.forEach((line, index) => {
    if (line.length === 0) {
      errors.push(`Line ${index + 1} is empty.`);
      return;
    }

    if (index === 0) {
      const first = parseFirstLine(line);

      if (!first.ok) {
        errors.push(`Line 1: ${first.error}`);
        return;
      }

      startedAt = first.epochMs;
      acceptedAt = first.epochMs;
      events.push({
        acceptedAt: first.epochMs,
        char: first.char,
        deltaMs: null,
        line,
      });
      return;
    }

    const next = parseDeltaLine(line);

    if (!next.ok) {
      errors.push(`Line ${index + 1}: ${next.error}`);
      return;
    }

    acceptedAt = (acceptedAt ?? 0) + next.deltaMs;
    events.push({
      acceptedAt,
      char: next.char,
      deltaMs: next.deltaMs,
      line,
    });
  });

  return {
    valid: errors.length === 0,
    closed,
    startedAt,
    events,
    errors,
  };
}

export function reconstructText(raw: string): string {
  return getEventLines(raw)
    .map((line, index) => {
      const parsed = index === 0 ? parseFirstLine(line) : parseDeltaLine(line);

      return parsed.ok ? parsed.char : "";
    })
    .join("");
}

export async function computeSessionHash(raw: string): Promise<string> {
  return computeSessionHashSync(raw);
}

export function computeSessionHashSync(raw: string): string {
  return bytesToHex(sha256(utf8ToBytes(raw)));
}

export async function verifyHash(raw: string, expectedHash: string): Promise<boolean> {
  const actualHash = computeSessionHashSync(raw);

  return actualHash === expectedHash.toLowerCase();
}

export function hasTerminalLine(raw: string): boolean {
  return raw.endsWith(TERMINAL_RECORD);
}

export function getLastAcceptedAt(raw: string): number | null {
  const eventLines = getEventLines(raw);

  if (eventLines.length === 0) {
    return null;
  }

  const first = parseFirstLine(eventLines[0]);

  if (!first.ok) {
    return null;
  }

  let acceptedAt = first.epochMs;

  for (let index = 1; index < eventLines.length; index += 1) {
    const parsed = parseDeltaLine(eventLines[index]);

    if (!parsed.ok) {
      return null;
    }

    acceptedAt += parsed.deltaMs;
  }

  return acceptedAt;
}

export function getReplayWords(raw: string): ReplayWord[] {
  const parsed = parseAnky(raw);

  if (!parsed.valid || parsed.startedAt == null) {
    return [];
  }

  const words: ReplayWord[] = [];
  let activeWord = "";
  let startIndex = 0;
  let startMs = 0;
  let endIndex = 0;
  let endMs = 0;

  parsed.events.forEach((event, index) => {
    const eventMs = event.acceptedAt - parsed.startedAt!;

    if (event.char === " ") {
      if (activeWord.length > 0) {
        words.push({
          endIndex,
          endMs,
          startIndex,
          startMs,
          word: activeWord,
        });
      }

      activeWord = "";
      return;
    }

    if (activeWord.length === 0) {
      startIndex = index;
      startMs = eventMs;
    }

    activeWord = `${activeWord}${event.char}`;
    endIndex = index;
    endMs = eventMs;
  });

  if (activeWord.length > 0) {
    words.push({
      endIndex,
      endMs,
      startIndex,
      startMs,
      word: activeWord,
    });
  }

  return words;
}

function getEventLines(raw: string): string[] {
  const eventRaw = hasTerminalLine(raw) ? raw.slice(0, -TERMINAL_RECORD.length) : raw;
  const eventLines = eventRaw.endsWith("\n")
    ? eventRaw.slice(0, -1).split("\n")
    : eventRaw.split("\n");

  return eventLines.filter((line) => line.length > 0);
}

function parseFirstLine(
  line: string,
): { ok: true; epochMs: number; char: string } | { ok: false; error: string } {
  const separatorIndex = line.indexOf(" ");

  if (separatorIndex <= 0) {
    return { ok: false, error: "First line must be `{epoch_ms} {character}`." };
  }

  const epoch = line.slice(0, separatorIndex);
  const token = line.slice(separatorIndex + 1);

  if (!/^\d+$/.test(epoch)) {
    return { ok: false, error: "Epoch must contain only digits." };
  }

  const parsedChar = parseCharacterToken(token);

  if (!parsedChar.ok) {
    return { ok: false, error: parsedChar.error };
  }

  const epochMs = Number(epoch);

  if (!Number.isSafeInteger(epochMs)) {
    return { ok: false, error: "Epoch is not a safe integer." };
  }

  return { ok: true, epochMs, char: parsedChar.char };
}

function parseDeltaLine(
  line: string,
): { ok: true; deltaMs: number; char: string } | { ok: false; error: string } {
  if (line.length < DELTA_WIDTH + 2 || line[DELTA_WIDTH] !== " ") {
    return { ok: false, error: "Delta line must be `{delta_ms} {character}`." };
  }

  const delta = line.slice(0, DELTA_WIDTH);
  const token = line.slice(DELTA_WIDTH + 1);

  if (!/^\d{4}$/.test(delta)) {
    return { ok: false, error: "Delta must be exactly four digits." };
  }

  const deltaMs = Number(delta);

  if (deltaMs > MAX_DELTA_MS) {
    return { ok: false, error: "Delta must be capped at 7999." };
  }

  const parsedChar = parseCharacterToken(token);

  if (!parsedChar.ok) {
    return { ok: false, error: parsedChar.error };
  }

  return { ok: true, deltaMs, char: parsedChar.char };
}

function serializeCharacter(char: string): string {
  return char === " " ? SPACE_TOKEN : char;
}

function parseCharacterToken(
  token: string,
): { ok: true; char: string } | { ok: false; error: string } {
  if (token === SPACE_TOKEN) {
    return { ok: true, char: " " };
  }

  if (token === " ") {
    return { ok: false, error: "Space must be encoded as SPACE." };
  }

  if (!isAcceptedCharacter(token)) {
    return { ok: false, error: "Character is not an accepted single character or SPACE token." };
  }

  return { ok: true, char: token };
}

function assertAcceptedCharacter(char: string): void {
  if (!isAcceptedCharacter(char)) {
    throw new Error("Input is not an accepted .anky character.");
  }
}

function assertTimestamp(value: number, label: string): void {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${label} must be a non-negative safe integer.`);
  }
}
