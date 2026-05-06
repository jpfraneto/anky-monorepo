export const ANKY_BACKUP_EXPORT_VERSION = 1;
export const ANKY_BACKUP_MANIFEST_FILE = "manifest.json";
export const ANKY_BACKUP_FILES_PREFIX = "files/";
export const ANKY_SESSION_INDEX_FILE = "sojourn9-session-index.json";
export const ANKY_BACKUP_WARNING =
  "This backup may include plaintext writing, reflections, keep-writing conversations, images, and local metadata. Save it only somewhere you trust.";

const HASH_NAMED_ANKY_FILE_PATTERN = /^[a-f0-9]{64}\.anky$/;
const IMAGE_SIDECAR_PATTERN = /^[a-f0-9]{64}\.image\.[a-z0-9]+$/;
const TRANSIENT_PROOF_ARTIFACT_FILE_NAMES = new Set([
  "handoff-manifest.json",
  "proof-with-public-values.bin",
  "receipt.json",
  "verified-receipt.json",
]);
const TRANSIENT_PROOF_ARTIFACT_PATTERN =
  /(?:^|[._-])(?:proof|sp1|witness|handoff)(?:[._-]|$)/i;

export type AnkyBackupFileKind = "anky" | "draft" | "image" | "session_index" | "sidecar";

export type AnkyBackupFileListing = {
  kind?: AnkyBackupFileKind;
  modificationTime?: number;
  path: string;
  size?: number;
};

export type AnkyBackupFileCounts = {
  ankyFiles: number;
  drafts: number;
  images: number;
  sessionIndex: number;
  sidecars: number;
  total: number;
};

export type AnkyBackupManifest = {
  appVersion?: string;
  exportedAt: string;
  exportVersion: typeof ANKY_BACKUP_EXPORT_VERSION;
  fileCounts: AnkyBackupFileCounts;
  files: Array<Required<Pick<AnkyBackupFileListing, "kind" | "path">> &
    Pick<AnkyBackupFileListing, "modificationTime" | "size">>;
  warning: string;
};

export function createAnkyBackupManifest({
  appVersion,
  exportedAt,
  files,
}: {
  appVersion?: string;
  exportedAt: string;
  files: AnkyBackupFileListing[];
}): AnkyBackupManifest {
  const manifestFiles = files
    .filter((file) => isBackupEligibleRelativePath(file.path))
    .map((file) => ({
      kind: file.kind ?? classifyBackupRelativePath(file.path),
      modificationTime: file.modificationTime,
      path: file.path,
      size: file.size,
    }))
    .sort((left, right) => left.path.localeCompare(right.path));

  return {
    appVersion,
    exportedAt,
    exportVersion: ANKY_BACKUP_EXPORT_VERSION,
    fileCounts: countBackupFiles(manifestFiles),
    files: manifestFiles,
    warning: ANKY_BACKUP_WARNING,
  };
}

export function countBackupFiles(files: AnkyBackupFileListing[]): AnkyBackupFileCounts {
  const kinds = files.map((file) => file.kind ?? classifyBackupRelativePath(file.path));

  return {
    ankyFiles: kinds.filter((kind) => kind === "anky").length,
    drafts: kinds.filter((kind) => kind === "draft").length,
    images: kinds.filter((kind) => kind === "image").length,
    sessionIndex: kinds.filter((kind) => kind === "session_index").length,
    sidecars: kinds.filter((kind) => kind === "image" || kind === "sidecar").length,
    total: kinds.length,
  };
}

export function classifyBackupRelativePath(path: string): AnkyBackupFileKind {
  const fileName = path.split("/").at(-1) ?? path;

  if (HASH_NAMED_ANKY_FILE_PATTERN.test(fileName)) {
    return "anky";
  }

  if (fileName === "active.anky.draft" || fileName === "pending.anky") {
    return "draft";
  }

  if (fileName === ANKY_SESSION_INDEX_FILE) {
    return "session_index";
  }

  if (IMAGE_SIDECAR_PATTERN.test(fileName)) {
    return "image";
  }

  return "sidecar";
}

export function getAnkyBackupFileName(now = new Date()): string {
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");

  return `anky-backup-${year}-${month}-${day}.zip`;
}

export function toBackupArchivePath(relativePath: string): string {
  if (!isBackupEligibleRelativePath(relativePath)) {
    throw new Error("Unsafe backup file path.");
  }

  return `${ANKY_BACKUP_FILES_PREFIX}${relativePath}`;
}

export function fromBackupArchivePath(archivePath: string): string | null {
  if (!archivePath.startsWith(ANKY_BACKUP_FILES_PREFIX)) {
    return null;
  }

  const relativePath = archivePath.slice(ANKY_BACKUP_FILES_PREFIX.length);

  return isBackupEligibleRelativePath(relativePath) ? relativePath : null;
}

export function parseAnkyBackupManifest(raw: string): AnkyBackupManifest {
  const parsed = JSON.parse(raw) as unknown;

  if (!isAnkyBackupManifest(parsed)) {
    throw new Error("The selected file is not a valid Anky backup.");
  }

  return parsed;
}

export function isAnkyBackupManifest(value: unknown): value is AnkyBackupManifest {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const manifest = value as Partial<AnkyBackupManifest>;

  return (
    manifest.exportVersion === ANKY_BACKUP_EXPORT_VERSION &&
    typeof manifest.exportedAt === "string" &&
    Number.isFinite(Date.parse(manifest.exportedAt)) &&
    typeof manifest.warning === "string" &&
    manifest.warning.toLowerCase().includes("plaintext") &&
    (manifest.appVersion == null || typeof manifest.appVersion === "string") &&
    isFileCounts(manifest.fileCounts) &&
    Array.isArray(manifest.files) &&
    manifest.files.length === manifest.fileCounts.total &&
    manifest.files.every(isManifestFile)
  );
}

export function isSafeBackupRelativePath(path: string): boolean {
  return (
    path.length > 0 &&
    !path.startsWith("/") &&
    !path.startsWith("\\") &&
    !path.endsWith("/") &&
    !path.includes("\\") &&
    path
      .split("/")
      .every((part) => part.length > 0 && part !== "." && part !== "..")
  );
}

export function isBackupEligibleRelativePath(path: string): boolean {
  if (!isSafeBackupRelativePath(path)) {
    return false;
  }

  const parts = path.split("/");
  const fileName = parts.at(-1) ?? path;

  if (parts.some((part) => part.startsWith("."))) {
    return false;
  }

  if (
    fileName.endsWith(".anky") &&
    fileName !== "pending.anky" &&
    !HASH_NAMED_ANKY_FILE_PATTERN.test(fileName)
  ) {
    return false;
  }

  if (TRANSIENT_PROOF_ARTIFACT_FILE_NAMES.has(fileName)) {
    return false;
  }

  return !TRANSIENT_PROOF_ARTIFACT_PATTERN.test(fileName);
}

function isFileCounts(value: unknown): value is AnkyBackupFileCounts {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const counts = value as Partial<AnkyBackupFileCounts>;

  return (
    isNonNegativeInteger(counts.ankyFiles) &&
    isNonNegativeInteger(counts.drafts) &&
    isNonNegativeInteger(counts.images) &&
    isNonNegativeInteger(counts.sessionIndex) &&
    isNonNegativeInteger(counts.sidecars) &&
    isNonNegativeInteger(counts.total)
  );
}

function isManifestFile(value: unknown): value is AnkyBackupManifest["files"][number] {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const file = value as Partial<AnkyBackupManifest["files"][number]>;

  return (
    typeof file.path === "string" &&
    isBackupEligibleRelativePath(file.path) &&
    (file.kind === "anky" ||
      file.kind === "draft" ||
      file.kind === "image" ||
      file.kind === "session_index" ||
      file.kind === "sidecar") &&
    (file.modificationTime == null || typeof file.modificationTime === "number") &&
    (file.size == null || typeof file.size === "number")
  );
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}
