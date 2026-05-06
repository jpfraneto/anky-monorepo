import { describe, expect, it } from "vitest";

import {
  ANKY_BACKUP_WARNING,
  createAnkyBackupManifest,
  fromBackupArchivePath,
  getAnkyBackupFileName,
  isBackupEligibleRelativePath,
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
    const sessionHash = "a".repeat(64);
    const manifest = createAnkyBackupManifest({
      appVersion: "1.0.0",
      exportedAt: "2026-05-04T18:30:00.000Z",
      files: [
        { path: `${sessionHash}.anky` },
        { path: `${sessionHash}.reflection.md` },
        { path: `${sessionHash}.image.png` },
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

  it("excludes transient proof artifacts and generic .anky witnesses", () => {
    const sessionHash = "b".repeat(64);
    const manifest = createAnkyBackupManifest({
      exportedAt: "2026-05-04T18:30:00.000Z",
      files: [
        { path: `${sessionHash}.anky` },
        { path: `${sessionHash}.seal.json` },
        { path: `${sessionHash}.seals.json` },
        { path: `${sessionHash}.processing.json` },
        { path: "pending.anky" },
        { path: "demo.anky" },
        { path: "receipt.json" },
        { path: "verified-receipt.json" },
        { path: "proof-with-public-values.bin" },
        { path: "handoff-manifest.json" },
        { path: "private-witness.txt" },
        { path: ".shadow.json" },
      ],
    });

    expect(manifest.files.map((file) => file.path)).toEqual([
      `${sessionHash}.anky`,
      `${sessionHash}.processing.json`,
      `${sessionHash}.seal.json`,
      `${sessionHash}.seals.json`,
      "pending.anky",
    ]);
    expect(manifest.fileCounts.total).toBe(5);

    expect(isBackupEligibleRelativePath("demo.anky")).toBe(false);
    expect(isBackupEligibleRelativePath("proof-with-public-values.bin")).toBe(false);
    expect(isBackupEligibleRelativePath(`${sessionHash}.seal.json`)).toBe(true);
    expect(fromBackupArchivePath("files/proof-with-public-values.bin")).toBeNull();
    expect(() => toBackupArchivePath("receipt.json")).toThrow("Unsafe backup file path.");
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
