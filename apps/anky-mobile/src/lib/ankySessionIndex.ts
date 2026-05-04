import * as FileSystem from "expo-file-system/legacy";

import { parseAnky, reconstructText } from "./ankyProtocol";
import {
  ensureAnkyDirectory,
  getAnkyDirectoryUri,
  listSavedAnkyFiles,
  SavedAnkyFile,
} from "./ankyStorage";
import {
  AnkySessionSummary,
  getCurrentSojournDay,
  SOJOURN_LENGTH_DAYS,
} from "./sojourn";
import { isCompleteRawAnky } from "./thread/threadLogic";

const INDEX_FILE = "sojourn9-session-index.json";
const HASH_PATTERN = /^[a-f0-9]{64}$/;

type IndexedSummary = AnkySessionSummary & {
  storedKind?: true;
};

export async function listAnkySessionSummaries(): Promise<AnkySessionSummary[]> {
  const [stored, files] = await Promise.all([readStoredSummaries(), listSavedAnkyFiles()]);
  const storedByHash = new Map(
    stored
      .filter((summary) => summary.sessionHash != null)
      .map((summary) => [summary.sessionHash!, summary]),
  );
  const unknownIds = new Set<string>();
  const mergedById = new Map<string, IndexedSummary>();

  stored.forEach((summary) => {
    mergedById.set(summary.id, { ...summary, storedKind: true });
  });

  files.forEach((file) => {
    const existing = storedByHash.get(file.hash);

    if (existing != null) {
      mergedById.set(existing.id, {
        ...existing,
        characterCount: existing.characterCount ?? getCharacterCount(file),
        localFileUri: existing.localFileUri ?? file.uri,
        reflectionId: existing.reflectionId ?? getReflectionId(file),
        sealedOnchain: existing.sealedOnchain ?? file.sealCount > 0,
        sessionHash: existing.sessionHash ?? file.hash,
        hasThread: existing.hasThread ?? file.artifactKinds.includes("conversation"),
        wordCount: existing.wordCount ?? getWordCount(file),
        storedKind: true,
      });
      return;
    }

    const summary = deriveSummaryFromFile(file);

    if (summary == null) {
      return;
    }

    unknownIds.add(summary.id);
    mergedById.set(summary.id, summary);
  });

  return assignKinds([...mergedById.values()], unknownIds).map(stripInternalFields);
}

export async function addAnkySessionSummary(summary: AnkySessionSummary): Promise<void> {
  await ensureAnkyDirectory();

  const stored = await readStoredSummaries();
  const byId = new Map(stored.map((item) => [item.id, item]));
  const existingIdForHash =
    summary.sessionHash == null
      ? undefined
      : stored.find((item) => item.sessionHash === summary.sessionHash)?.id;
  const id = existingIdForHash ?? summary.id;
  const existing = byId.get(id);

  byId.set(id, normalizeSummary({ ...existing, ...summary, id }));

  await writeStoredSummaries([...byId.values()]);
}

export async function mergeAnkySessionIndexFromRaw(raw: string): Promise<number> {
  const restored = parseStoredSummaries(raw);

  if (restored.length === 0) {
    return 0;
  }

  const stored = await readStoredSummaries();
  const byId = new Map(stored.map((item) => [item.id, item]));
  const idByHash = new Map(
    stored
      .filter((summary) => summary.sessionHash != null)
      .map((summary) => [summary.sessionHash!, summary.id]),
  );
  let added = 0;

  restored.forEach((summary) => {
    const existingIdForHash =
      summary.sessionHash == null ? undefined : idByHash.get(summary.sessionHash);
    const id = existingIdForHash ?? summary.id;

    if (byId.has(id)) {
      return;
    }

    const normalized = normalizeSummary({ ...summary, id });

    byId.set(id, normalized);
    if (normalized.sessionHash != null) {
      idByHash.set(normalized.sessionHash, id);
    }
    added += 1;
  });

  await writeStoredSummaries([...byId.values()]);

  return added;
}

export async function rebuildAnkySessionIndex(): Promise<number> {
  const summaries = await listAnkySessionSummaries();

  await writeStoredSummaries(summaries);

  return summaries.length;
}

async function readStoredSummaries(): Promise<AnkySessionSummary[]> {
  await ensureAnkyDirectory();

  const uri = getIndexUri();
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return [];
  }

  const raw = await FileSystem.readAsStringAsync(uri, {
    encoding: FileSystem.EncodingType.UTF8,
  });

  if (raw.trim().length === 0) {
    return [];
  }

  return parseStoredSummaries(raw);
}

async function writeStoredSummaries(summaries: AnkySessionSummary[]): Promise<void> {
  await ensureAnkyDirectory();

  const ordered = summaries
    .map(normalizeSummary)
    .sort((left, right) => Date.parse(left.createdAt) - Date.parse(right.createdAt));

  await FileSystem.writeAsStringAsync(getIndexUri(), JSON.stringify(ordered, null, 2), {
    encoding: FileSystem.EncodingType.UTF8,
  });
}

function parseStoredSummaries(raw: string): AnkySessionSummary[] {
  if (raw.trim().length === 0) {
    return [];
  }

  try {
    const parsed = JSON.parse(raw) as unknown;

    return Array.isArray(parsed) ? parsed.filter(isAnkySessionSummary).map(normalizeSummary) : [];
  } catch {
    return [];
  }
}

function deriveSummaryFromFile(file: SavedAnkyFile): IndexedSummary | null {
  const parsed = parseAnky(file.raw);

  if (parsed.startedAt == null) {
    return null;
  }

  const createdAt = new Date(parsed.startedAt).toISOString();

  return {
    id: file.hash,
    characterCount: getCharacterCount(file),
    createdAt,
    kind: isCompleteRawAnky(file.raw) ? "extra_thread" : "fragment",
    localFileUri: file.uri,
    reflectionId: getReflectionId(file),
    sealedOnchain: file.sealCount > 0,
    hasThread: file.artifactKinds.includes("conversation"),
    sessionHash: file.hash,
    sojournDay: getCurrentSojournDay(new Date(createdAt)),
    wordCount: getWordCount(file),
  };
}

function assignKinds(
  summaries: IndexedSummary[],
  unknownIds: Set<string>,
): IndexedSummary[] {
  const byDay = new Map<number, IndexedSummary[]>();

  summaries.forEach((summary) => {
    const daySummaries = byDay.get(summary.sojournDay) ?? [];

    daySummaries.push(summary);
    byDay.set(summary.sojournDay, daySummaries);
  });

  byDay.forEach((daySummaries) => {
    const sorted = daySummaries.sort(
      (left, right) => Date.parse(left.createdAt) - Date.parse(right.createdAt),
    );
    let hasDailySeal = sorted.some(
      (summary) => !unknownIds.has(summary.id) && summary.kind === "daily_seal",
    );

    sorted.forEach((summary) => {
      if (!unknownIds.has(summary.id)) {
        return;
      }

      if (summary.kind === "fragment") {
        return;
      }

      if (!hasDailySeal) {
        summary.kind = "daily_seal";
        hasDailySeal = true;
        return;
      }

      summary.kind = "extra_thread";
    });
  });

  return summaries.sort((left, right) => Date.parse(left.createdAt) - Date.parse(right.createdAt));
}

function getWordCount(file: SavedAnkyFile): number {
  return reconstructText(file.raw).trim().split(/\s+/).filter(Boolean).length;
}

function getCharacterCount(file: SavedAnkyFile): number {
  return reconstructText(file.raw).length;
}

function getReflectionId(file: SavedAnkyFile): string | undefined {
  return file.artifactKinds.includes("reflection") ? file.hash : undefined;
}

function normalizeSummary(summary: AnkySessionSummary): AnkySessionSummary {
  return {
    ...summary,
    sojournDay: Math.max(1, Math.min(SOJOURN_LENGTH_DAYS, Math.floor(summary.sojournDay))),
  };
}

function stripInternalFields(summary: IndexedSummary): AnkySessionSummary {
  const { storedKind: _storedKind, ...publicSummary } = summary;

  return publicSummary;
}

function getIndexUri(): string {
  return `${getAnkyDirectoryUri()}${INDEX_FILE}`;
}

function isAnkySessionSummary(value: unknown): value is AnkySessionSummary {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const summary = value as Partial<AnkySessionSummary>;

  return (
    typeof summary.id === "string" &&
    typeof summary.createdAt === "string" &&
    Number.isFinite(Date.parse(summary.createdAt)) &&
    typeof summary.sojournDay === "number" &&
    (summary.kind === "daily_seal" ||
      summary.kind === "extra_thread" ||
      summary.kind === "fragment") &&
    (summary.sessionHash == null ||
      (typeof summary.sessionHash === "string" && HASH_PATTERN.test(summary.sessionHash))) &&
    (summary.localFileUri == null || typeof summary.localFileUri === "string") &&
    (summary.wordCount == null || typeof summary.wordCount === "number") &&
    (summary.characterCount == null || typeof summary.characterCount === "number") &&
    (summary.sealedOnchain == null || typeof summary.sealedOnchain === "boolean") &&
    (summary.reflectionId == null || typeof summary.reflectionId === "string") &&
    (summary.hasThread == null || typeof summary.hasThread === "boolean")
  );
}
