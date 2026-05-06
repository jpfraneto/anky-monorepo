import * as Application from "expo-application";
import * as DocumentPicker from "expo-document-picker";
import * as FileSystem from "expo-file-system/legacy";
import * as Sharing from "expo-sharing";
import JSZip from "jszip";

import { computeSessionHashSync, parseAnky } from "./ankyProtocol";
import {
  rebuildAnkySessionIndex,
  mergeAnkySessionIndexFromRaw,
} from "./ankySessionIndex";
import { ensureAnkyDirectory, getAnkyDirectoryUri } from "./ankyStorage";
import {
  ANKY_BACKUP_MANIFEST_FILE,
  ANKY_SESSION_INDEX_FILE,
  AnkyBackupFileListing,
  AnkyBackupManifest,
  classifyBackupRelativePath,
  createAnkyBackupManifest,
  fromBackupArchivePath,
  getAnkyBackupFileName,
  isBackupEligibleRelativePath,
  parseAnkyBackupManifest,
  toBackupArchivePath,
} from "./ankyBackupManifest";

const ZIP_MIME_TYPES = [
  "application/zip",
  "application/x-zip-compressed",
  "application/octet-stream",
];
const HASH_NAMED_ANKY_FILE_PATTERN = /^[a-f0-9]{64}\.anky$/;

type LocalArchiveFile = AnkyBackupFileListing & {
  uri: string;
};

type RestoreEntryStatus =
  | "added"
  | "conflict"
  | "duplicate"
  | "overwritten"
  | "skipped_invalid"
  | "skipped_newer";

export type AnkyBackupExportResult = {
  fileCount: number;
  fileName: string;
  manifest: AnkyBackupManifest;
  uri: string;
};

export type AnkyBackupRestoreResult = {
  added: number;
  conflicts: number;
  duplicates: number;
  invalid: number;
  manifest: AnkyBackupManifest;
  mergedIndexEntries: number;
  rebuiltIndexEntries: number;
  skippedNewer: number;
  overwritten: number;
};

export async function exportAnkyBackupArchive(): Promise<AnkyBackupExportResult> {
  const backup = await createAnkyBackupArchive();
  const sharingAvailable = await Sharing.isAvailableAsync();

  if (!sharingAvailable) {
    throw new Error("System sharing is not available on this device.");
  }

  await Sharing.shareAsync(backup.uri, {
    dialogTitle: "Export Anky backup",
    mimeType: "application/zip",
    UTI: "com.pkware.zip-archive",
  });

  return backup;
}

export async function createAnkyBackupArchive(): Promise<AnkyBackupExportResult> {
  const files = await listLocalAnkyArchiveFiles();

  if (files.length === 0) {
    throw new Error("No local Anky archive exists yet.");
  }

  const exportedAt = new Date().toISOString();
  const manifest = createAnkyBackupManifest({
    appVersion: getAppVersion(),
    exportedAt,
    files,
  });
  const zip = new JSZip();

  zip.file(ANKY_BACKUP_MANIFEST_FILE, JSON.stringify(manifest, null, 2), {
    date: new Date(exportedAt),
  });

  for (const file of files) {
    const fileBase64 = await FileSystem.readAsStringAsync(file.uri, {
      encoding: FileSystem.EncodingType.Base64,
    });

    zip.file(toBackupArchivePath(file.path), fileBase64, {
      base64: true,
      createFolders: true,
      date:
        file.modificationTime == null
          ? undefined
          : new Date(file.modificationTime * 1000),
    });
  }

  const zipBase64 = await zip.generateAsync({
    compression: "DEFLATE",
    compressionOptions: { level: 6 },
    mimeType: "application/zip",
    type: "base64",
  });
  const outputDirectory = getBackupOutputDirectoryUri();
  const fileName = getAnkyBackupFileName(new Date(exportedAt));
  const uri = `${outputDirectory}${fileName}`;

  await FileSystem.makeDirectoryAsync(outputDirectory, { intermediates: true });
  await FileSystem.writeAsStringAsync(uri, zipBase64, {
    encoding: FileSystem.EncodingType.Base64,
  });

  return {
    fileCount: files.length,
    fileName,
    manifest,
    uri,
  };
}

export async function pickAndRestoreAnkyBackup(): Promise<AnkyBackupRestoreResult | null> {
  const picked = await DocumentPicker.getDocumentAsync({
    copyToCacheDirectory: true,
    multiple: false,
    type: ZIP_MIME_TYPES,
  });

  if (picked.canceled) {
    return null;
  }

  const asset = picked.assets[0];

  if (asset == null) {
    throw new Error("No backup file was selected.");
  }

  return restoreAnkyBackupFromUri(asset.uri);
}

export async function restoreAnkyBackupFromUri(uri: string): Promise<AnkyBackupRestoreResult> {
  const zipBase64 = await FileSystem.readAsStringAsync(uri, {
    encoding: FileSystem.EncodingType.Base64,
  });
  const zip = await JSZip.loadAsync(zipBase64, { base64: true, checkCRC32: true });
  const manifestEntry = zip.file(ANKY_BACKUP_MANIFEST_FILE);

  if (manifestEntry == null) {
    throw new Error("This zip is missing an Anky backup manifest.");
  }

  const manifest = parseAnkyBackupManifest(await manifestEntry.async("string"));
  const entries = getRestorableZipEntries(zip);

  validateArchiveEntriesAgainstManifest(entries, manifest);
  await ensureAnkyDirectory();

  const result: AnkyBackupRestoreResult = {
    added: 0,
    conflicts: 0,
    duplicates: 0,
    invalid: 0,
    manifest,
    mergedIndexEntries: 0,
    rebuiltIndexEntries: 0,
    skippedNewer: 0,
    overwritten: 0,
  };
  let sessionIndexRaw: string | null = null;

  for (const entry of entries) {
    if (entry.relativePath === ANKY_SESSION_INDEX_FILE) {
      sessionIndexRaw = await entry.zipEntry.async("string");
      continue;
    }

    const status = await restoreZipEntry({
      manifest,
      relativePath: entry.relativePath,
      zipEntry: entry.zipEntry,
    });

    addRestoreStatus(result, status);
  }

  if (sessionIndexRaw != null) {
    result.mergedIndexEntries = await mergeAnkySessionIndexFromRaw(sessionIndexRaw);
  }

  result.rebuiltIndexEntries = await rebuildAnkySessionIndex();

  return result;
}

async function listLocalAnkyArchiveFiles(): Promise<LocalArchiveFile[]> {
  await ensureAnkyDirectory();

  return listArchiveFilesInDirectory(getAnkyDirectoryUri(), "");
}

async function listArchiveFilesInDirectory(
  directoryUri: string,
  relativeDirectory: string,
): Promise<LocalArchiveFile[]> {
  const fileNames = await FileSystem.readDirectoryAsync(directoryUri);
  const nested = await Promise.all(
    fileNames.map(async (fileName) => {
      const relativePath = `${relativeDirectory}${fileName}`;

      if (!isBackupEligibleRelativePath(relativePath)) {
        return [];
      }

      const uri = joinFileUri(directoryUri, fileName);
      const info = await FileSystem.getInfoAsync(uri);

      if (!info.exists) {
        return [];
      }

      if (info.isDirectory) {
        return listArchiveFilesInDirectory(`${uri}/`, `${relativePath}/`);
      }

      return [
        {
          kind: classifyBackupRelativePath(relativePath),
          modificationTime: info.modificationTime,
          path: relativePath,
          size: info.size,
          uri,
        },
      ];
    }),
  );

  return nested.flat().sort((left, right) => left.path.localeCompare(right.path));
}

async function restoreZipEntry({
  manifest,
  relativePath,
  zipEntry,
}: {
  manifest: AnkyBackupManifest;
  relativePath: string;
  zipEntry: JSZip.JSZipObject;
}): Promise<RestoreEntryStatus> {
  const manifestFile = manifest.files.find((file) => file.path === relativePath);
  const kind = manifestFile?.kind ?? classifyBackupRelativePath(relativePath);

  if (kind === "anky") {
    await validateCanonicalAnkyEntry(relativePath, zipEntry);
  }

  if (kind === "draft" && (await isInvalidDraftEntry(zipEntry))) {
    return "skipped_invalid";
  }

  const targetUri = `${getAnkyDirectoryUri()}${relativePath}`;
  const incomingBase64 = await zipEntry.async("base64");
  const localInfo = await FileSystem.getInfoAsync(targetUri);

  if (localInfo.exists) {
    if (localInfo.isDirectory) {
      return "conflict";
    }

    const existingBase64 = await FileSystem.readAsStringAsync(targetUri, {
      encoding: FileSystem.EncodingType.Base64,
    });

    if (existingBase64 === incomingBase64) {
      return "duplicate";
    }

    if (kind === "anky") {
      return "conflict";
    }

    if (isLocalFileNewer(localInfo.modificationTime, manifestFile?.modificationTime, zipEntry.date)) {
      return "skipped_newer";
    }
  }

  await ensureParentDirectory(targetUri);
  await FileSystem.writeAsStringAsync(targetUri, incomingBase64, {
    encoding: FileSystem.EncodingType.Base64,
  });

  return localInfo.exists ? "overwritten" : "added";
}

async function validateCanonicalAnkyEntry(
  relativePath: string,
  zipEntry: JSZip.JSZipObject,
): Promise<void> {
  const fileName = relativePath.split("/").at(-1) ?? relativePath;

  if (!HASH_NAMED_ANKY_FILE_PATTERN.test(fileName)) {
    throw new Error(`Backup contains an invalid .anky file name: ${relativePath}`);
  }

  const expectedHash = fileName.replace(/\.anky$/, "");
  const raw = await zipEntry.async("string");
  const parsed = parseAnky(raw);

  if (!parsed.valid) {
    throw new Error(`Backup contains an invalid .anky file: ${relativePath}`);
  }

  if (computeSessionHashSync(raw) !== expectedHash) {
    throw new Error(`Backup contains a .anky hash mismatch: ${relativePath}`);
  }
}

async function isInvalidDraftEntry(zipEntry: JSZip.JSZipObject): Promise<boolean> {
  try {
    return !parseAnky(await zipEntry.async("string")).valid;
  } catch {
    return true;
  }
}

function getRestorableZipEntries(
  zip: JSZip,
): Array<{ relativePath: string; zipEntry: JSZip.JSZipObject }> {
  return Object.values(zip.files)
    .filter((entry) => !entry.dir)
    .map((entry) => ({
      relativePath: fromBackupArchivePath(entry.name),
      zipEntry: entry,
    }))
    .filter(
      (entry): entry is { relativePath: string; zipEntry: JSZip.JSZipObject } =>
        entry.relativePath != null,
    )
    .sort((left, right) => left.relativePath.localeCompare(right.relativePath));
}

function validateArchiveEntriesAgainstManifest(
  entries: Array<{ relativePath: string }>,
  manifest: AnkyBackupManifest,
): void {
  const archivePaths = new Set(entries.map((entry) => entry.relativePath));
  const manifestPaths = new Set(manifest.files.map((file) => file.path));

  if (archivePaths.size !== manifest.fileCounts.total) {
    throw new Error("The backup manifest does not match the zip contents.");
  }

  for (const manifestPath of manifestPaths) {
    if (!archivePaths.has(manifestPath)) {
      throw new Error("The backup is missing files listed in its manifest.");
    }
  }

  for (const archivePath of archivePaths) {
    if (!manifestPaths.has(archivePath)) {
      throw new Error("The backup contains files not listed in its manifest.");
    }
  }
}

function addRestoreStatus(result: AnkyBackupRestoreResult, status: RestoreEntryStatus): void {
  switch (status) {
    case "added":
      result.added += 1;
      break;
    case "conflict":
      result.conflicts += 1;
      break;
    case "duplicate":
      result.duplicates += 1;
      break;
    case "overwritten":
      result.overwritten += 1;
      break;
    case "skipped_invalid":
      result.invalid += 1;
      break;
    case "skipped_newer":
      result.skippedNewer += 1;
      break;
  }
}

function isLocalFileNewer(
  localModificationTime: number,
  manifestModificationTime: number | undefined,
  zipDate: Date,
): boolean {
  const backupModificationTime = manifestModificationTime ?? zipDate.getTime() / 1000;

  return localModificationTime > backupModificationTime + 1;
}

async function ensureParentDirectory(uri: string): Promise<void> {
  const rootUri = getAnkyDirectoryUri();

  if (!uri.startsWith(rootUri)) {
    throw new Error("Restore target is outside the Anky archive.");
  }

  const relativePath = uri.slice(rootUri.length);
  const directoryParts = relativePath.split("/").slice(0, -1);

  if (directoryParts.length === 0) {
    return;
  }

  await FileSystem.makeDirectoryAsync(`${rootUri}${directoryParts.join("/")}/`, {
    intermediates: true,
  });
}

function getBackupOutputDirectoryUri(): string {
  const rootUri = FileSystem.cacheDirectory ?? FileSystem.documentDirectory;

  if (rootUri == null) {
    throw new Error("Expo FileSystem storage is unavailable.");
  }

  return `${rootUri}anky-backups/`;
}

function getAppVersion(): string | undefined {
  const version = Application.nativeApplicationVersion;
  const build = Application.nativeBuildVersion;

  if (version != null && build != null) {
    return `${version} (${build})`;
  }

  return version ?? build ?? undefined;
}

function joinFileUri(directoryUri: string, fileName: string): string {
  return `${directoryUri.endsWith("/") ? directoryUri : `${directoryUri}/`}${fileName}`;
}
