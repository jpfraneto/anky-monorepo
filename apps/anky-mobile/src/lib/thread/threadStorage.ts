import * as FileSystem from "expo-file-system/legacy";

import { ensureAnkyDirectory, getAnkyDirectoryUri } from "../ankyStorage";
import { countUserMessages } from "./threadLogic";
import type { AnkyThread, ThreadMessage, ThreadMode } from "./types";

const HASH_PATTERN = /^[a-f0-9]{64}$/;
const THREAD_FILE_PATTERN = /^[a-f0-9]{64}\.conversation\.json$/;

export async function getThread(sessionHash: string): Promise<AnkyThread | null> {
  validateHash(sessionHash);
  await ensureAnkyDirectory();

  const uri = getThreadSidecarUri(sessionHash);
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return null;
  }

  try {
    const raw = await FileSystem.readAsStringAsync(uri, {
      encoding: FileSystem.EncodingType.UTF8,
    });
    const parsed = JSON.parse(raw) as unknown;

    return isAnkyThread(parsed) ? normalizeThread(parsed) : null;
  } catch {
    return null;
  }
}

export async function saveThread(thread: AnkyThread): Promise<void> {
  const normalized = normalizeThread(thread);

  validateHash(normalized.sessionHash);
  await ensureAnkyDirectory();
  await FileSystem.writeAsStringAsync(
    getThreadSidecarUri(normalized.sessionHash),
    JSON.stringify(normalized, null, 2),
    {
      encoding: FileSystem.EncodingType.UTF8,
    },
  );
}

export async function appendThreadMessage(
  sessionHash: string,
  message: ThreadMessage,
  mode: ThreadMode = "complete",
): Promise<AnkyThread> {
  const existing = await getThread(sessionHash);
  const base =
    existing ??
    normalizeThread({
      version: 1,
      createdAt: message.createdAt,
      messages: [],
      mode,
      sessionHash,
      updatedAt: message.createdAt,
      userMessageCount: 0,
    });
  const next = normalizeThread({
    ...base,
    messages: [...base.messages, message],
    updatedAt: message.createdAt,
  });

  await saveThread(next);

  return next;
}

export async function listThreads(): Promise<AnkyThread[]> {
  await ensureAnkyDirectory();

  const fileNames = await FileSystem.readDirectoryAsync(getAnkyDirectoryUri());
  const threads = await Promise.all(
    fileNames
      .filter((fileName) => THREAD_FILE_PATTERN.test(fileName))
      .map((fileName) => getThread(fileName.replace(/\.conversation\.json$/, ""))),
  );

  return threads.filter((thread): thread is AnkyThread => thread != null);
}

export async function deleteThread(sessionHash: string): Promise<void> {
  validateHash(sessionHash);

  const uri = getThreadSidecarUri(sessionHash);
  const info = await FileSystem.getInfoAsync(uri);

  if (info.exists) {
    await FileSystem.deleteAsync(uri, { idempotent: true });
  }
}

export function getThreadSidecarUri(sessionHash: string): string {
  validateHash(sessionHash);

  return `${getAnkyDirectoryUri()}${sessionHash}.conversation.json`;
}

function normalizeThread(thread: AnkyThread): AnkyThread {
  return {
    ...thread,
    userMessageCount: countUserMessages(thread.messages),
    version: 1,
  };
}

function validateHash(value: string): void {
  if (!HASH_PATTERN.test(value)) {
    throw new Error("Invalid session hash.");
  }
}

function isAnkyThread(value: unknown): value is AnkyThread {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const thread = value as Partial<AnkyThread>;

  return (
    thread.version === 1 &&
    typeof thread.sessionHash === "string" &&
    HASH_PATTERN.test(thread.sessionHash) &&
    isThreadMode(thread.mode) &&
    typeof thread.createdAt === "string" &&
    typeof thread.updatedAt === "string" &&
    Array.isArray(thread.messages) &&
    thread.messages.every(isThreadMessage)
  );
}

function isThreadMessage(value: unknown): value is ThreadMessage {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const message = value as Partial<ThreadMessage>;

  return (
    typeof message.id === "string" &&
    (message.role === "anky" || message.role === "user") &&
    typeof message.content === "string" &&
    typeof message.createdAt === "string"
  );
}

function isThreadMode(value: unknown): value is ThreadMode {
  return value === "complete" || value === "fragment" || value === "reflection";
}
