import { describe, expect, it } from "vitest";

import {
  ANKY_BACKUP_WARNING,
  createAnkyBackupManifest,
  fromBackupArchivePath,
  getAnkyBackupFileName,
  isAnkyBackupManifest,
  isSafeBackupRelativePath,
  parseAnkyBackupManifest,
  toBackupArchivePath,
} from "./ankyBackupManifest";

describe("Anky backup manifest", () => {
  it("creates a dated backup zip name", () => {
    expect(getAnkyBackupFileName(new Date("2026-05-04T18:30:00.000Z"))).toBe(
      "anky-backup-2026-05-04.zip",
    );
  });

  it("counts canonical files and sidecars", () => {
    const manifest = createAnkyBackupManifest({
      appVersion: "1.0.0",
      exportedAt: "2026-05-04T18:30:00.000Z",
      files: [
        { path: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.anky" },
        { path: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.reflection.md" },
        { path: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.image.png" },
        { path: "active.anky.draft" },
        { path: "sojourn9-session-index.json" },
      ],
    });

    expect(manifest.warning).toBe(ANKY_BACKUP_WARNING);
    expect(manifest.fileCounts).toEqual({
      ankyFiles: 1,
      drafts: 1,
      images: 1,
      sessionIndex: 1,
      sidecars: 2,
      total: 5,
    });
    expect(isAnkyBackupManifest(manifest)).toBe(true);
    expect(parseAnkyBackupManifest(JSON.stringify(manifest))).toEqual(manifest);
  });

  it("keeps archive file paths under the files prefix", () => {
    const archivePath = toBackupArchivePath(
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.conversation.json",
    );

    expect(archivePath).toBe(
      "files/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.conversation.json",
    );
    expect(fromBackupArchivePath(archivePath)).toBe(
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.conversation.json",
    );
  });

  it("rejects paths that could leave the restore root", () => {
    expect(isSafeBackupRelativePath("folder/meta.json")).toBe(true);
    expect(isSafeBackupRelativePath("../meta.json")).toBe(false);
    expect(isSafeBackupRelativePath("folder/../meta.json")).toBe(false);
    expect(isSafeBackupRelativePath("/meta.json")).toBe(false);
    expect(isSafeBackupRelativePath("folder\\meta.json")).toBe(false);
  });
});
