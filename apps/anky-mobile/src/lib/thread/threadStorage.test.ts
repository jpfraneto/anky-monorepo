import { beforeEach, describe, expect, it, vi } from "vitest";

const fsMock = vi.hoisted(() => ({
  files: new Map<string, string>(),
}));

vi.mock("expo-file-system/legacy", () => ({
  documentDirectory: "file:///documents/",
  EncodingType: {
    UTF8: "utf8",
  },
  deleteAsync: vi.fn(async (uri: string) => {
    fsMock.files.delete(uri);
  }),
  getInfoAsync: vi.fn(async (uri: string) => ({
    exists: fsMock.files.has(uri),
    isDirectory: false,
  })),
  makeDirectoryAsync: vi.fn(async () => undefined),
  readAsStringAsync: vi.fn(async (uri: string) => {
    const value = fsMock.files.get(uri);

    if (value == null) {
      throw new Error(`Missing mocked file: ${uri}`);
    }

    return value;
  }),
  readDirectoryAsync: vi.fn(async (uri: string) =>
    Array.from(fsMock.files.keys())
      .filter((fileUri) => fileUri.startsWith(uri))
      .map((fileUri) => fileUri.slice(uri.length))
      .filter((fileName) => fileName.length > 0 && !fileName.includes("/")),
  ),
  writeAsStringAsync: vi.fn(async (uri: string, value: string) => {
    fsMock.files.set(uri, value);
  }),
}));

import { createInitialThread, createThreadMessage } from "./threadLogic";
import { appendThreadMessage, getThread, listThreads, saveThread } from "./threadStorage";

describe("thread storage", () => {
  beforeEach(() => {
    fsMock.files.clear();
  });

  it("stores thread conversations as local sidecars by session hash", async () => {
    const sessionHash = "a".repeat(64);
    const thread = createInitialThread({ mode: "fragment", sessionHash });

    await saveThread(thread);

    const saved = await getThread(sessionHash);

    expect(saved?.sessionHash).toBe(sessionHash);
    expect(saved?.mode).toBe("fragment");
    expect(saved?.messages[0]?.role).toBe("anky");
  });

  it("appends messages and tracks user message count", async () => {
    const sessionHash = "b".repeat(64);
    const message = createThreadMessage({ content: "still alive", role: "user" });

    const thread = await appendThreadMessage(sessionHash, message, "complete");

    expect(thread.userMessageCount).toBe(1);
    await expect(getThread(sessionHash)).resolves.toMatchObject({
      userMessageCount: 1,
    });
  });

  it("lists stored thread sidecars", async () => {
    await saveThread(createInitialThread({ mode: "complete", sessionHash: "c".repeat(64) }));
    await saveThread(createInitialThread({ mode: "reflection", sessionHash: "d".repeat(64) }));

    await expect(listThreads()).resolves.toHaveLength(2);
  });
});
