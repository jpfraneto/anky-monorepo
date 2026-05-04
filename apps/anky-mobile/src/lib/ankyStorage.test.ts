import { beforeEach, describe, expect, it, vi } from "vitest";

const fsMock = vi.hoisted(() => ({
  files: new Map<string, string>(),
}));

vi.mock("expo-file-system/legacy", () => ({
  documentDirectory: "file:///documents/",
  EncodingType: {
    Base64: "base64",
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

import { closeSession, computeSessionHash } from "./ankyProtocol";
import {
  clearActiveDraft,
  clearPendingReveal,
  appendLoomSeal,
  listSavedAnkyFiles,
  readActiveDraft,
  readAnkyFile,
  readPendingReveal,
  saveClosedSession,
  stageTerminalDraftForReveal,
  writeActiveDraft,
  writePendingReveal,
  writeProcessingArtifacts,
} from "./ankyStorage";

describe(".anky storage", () => {
  beforeEach(() => {
    fsMock.files.clear();
  });

  it("writes, reads, and clears pending reveal as plain .anky text", async () => {
    const raw = closeSession("1000 a\n");

    await writePendingReveal(raw);

    await expect(readPendingReveal()).resolves.toBe(raw);

    await clearPendingReveal();

    await expect(readPendingReveal()).resolves.toBeNull();
  });

  it("release cleanup does not create a canonical file", async () => {
    const raw = closeSession("1000 a\n");

    await writePendingReveal(raw);
    await clearPendingReveal();
    await clearActiveDraft();

    await expect(listSavedAnkyFiles()).resolves.toEqual([]);
  });

  it("closed session saves the exact .anky file by hash", async () => {
    const raw = closeSession("1000 a\n0042 b\n");
    const hash = await computeSessionHash(raw);

    const saved = await saveClosedSession(raw);

    expect(saved.fileName).toBe(`${hash}.anky`);
    expect(saved.localState).toBe("verified");
    await expect(readAnkyFile(saved.fileName)).resolves.toBe(raw);
  });

  it("stages a terminal active draft for reveal without clearing the draft first", async () => {
    const raw = closeSession("1000 a\n0042 b\n");

    await writeActiveDraft(raw);

    const saved = await stageTerminalDraftForReveal(raw);

    await expect(readActiveDraft()).resolves.toBe(raw);
    await expect(readPendingReveal()).resolves.toBe(raw);
    await expect(readAnkyFile(saved.fileName)).resolves.toBe(raw);
  });

  it("marks saved .anky files as sealed when a loom seal sidecar exists", async () => {
    const raw = closeSession("1000 a\n");
    const saved = await saveClosedSession(raw);

    await appendLoomSeal({
      blockTime: 1700000000,
      createdAt: "2023-11-14T22:13:20.000Z",
      loomId: "loom-1",
      sessionHash: saved.hash,
      txSignature: "mock_sig",
      writer: "writer",
    });

    const [entry] = await listSavedAnkyFiles();

    expect(entry.localState).toBe("sealed");
    expect(entry.sealCount).toBe(1);
  });

  it("stores derived artifacts as sidecars", async () => {
    const raw = closeSession("1000 a\n");
    const saved = await saveClosedSession(raw);

    await writeProcessingArtifacts([
      {
        kind: "reflection",
        markdown: "# reflection",
        sessionHash: saved.hash,
      },
      {
        kind: "title",
        sessionHash: saved.hash,
        title: "A title",
      },
    ]);

    const [entry] = await listSavedAnkyFiles();

    expect(entry.localState).toBe("processed");
    expect(entry.artifactKinds).toEqual(["reflection", "title"]);
  });
});
