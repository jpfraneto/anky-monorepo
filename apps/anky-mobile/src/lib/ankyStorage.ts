import * as FileSystem from "expo-file-system/legacy";

import {
  computeSessionHash,
  parseAnky,
  reconstructText,
  verifyHash,
} from "./ankyProtocol";
import { resolveAnkyLocalState, AnkyLocalState } from "./ankyState";
import { ProcessingArtifact } from "./api/types";
import { LoomSeal } from "./solana/types";

const ANKY_DIRECTORY = "anky/";
const ACTIVE_DRAFT_FILE = "active.anky.draft";
const PENDING_REVEAL_FILE = "pending.anky";
const HASH_FILE_PATTERN = /^[a-f0-9]{64}\.anky$/;
const HASH_PATTERN = /^[a-f0-9]{64}$/;
const SEAL_FILE_PATTERN = /^[a-f0-9]{64}\.seals\.json$/;

type SidecarArtifactKind =
  | "conversation"
  | "deep_mirror"
  | "full_sojourn_archive"
  | "image"
  | "meta"
  | "reflection"
  | "title";

export type SavedAnkyFile = {
  artifactKinds: SidecarArtifactKind[];
  fileName: string;
  hash: string;
  hashMatches: boolean;
  latestSeal?: LoomSeal;
  localState: AnkyLocalState;
  preview: string;
  raw: string;
  sealCount: number;
  uri: string;
  valid: boolean;
};

export function getAnkyDirectoryUri(): string {
  if (FileSystem.documentDirectory == null) {
    throw new Error("Expo FileSystem documentDirectory is unavailable.");
  }

  return `${FileSystem.documentDirectory}${ANKY_DIRECTORY}`;
}

export function getActiveDraftUri(): string {
  return `${getAnkyDirectoryUri()}${ACTIVE_DRAFT_FILE}`;
}

export function getPendingRevealUri(): string {
  return `${getAnkyDirectoryUri()}${PENDING_REVEAL_FILE}`;
}

export async function ensureAnkyDirectory(): Promise<void> {
  await FileSystem.makeDirectoryAsync(getAnkyDirectoryUri(), { intermediates: true });
}

export async function readActiveDraft(): Promise<string | null> {
  await ensureAnkyDirectory();

  const uri = getActiveDraftUri();
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return null;
  }

  return FileSystem.readAsStringAsync(uri, { encoding: FileSystem.EncodingType.UTF8 });
}

export async function writeActiveDraft(raw: string): Promise<void> {
  await ensureAnkyDirectory();
  await FileSystem.writeAsStringAsync(getActiveDraftUri(), raw, {
    encoding: FileSystem.EncodingType.UTF8,
  });
}

export async function clearActiveDraft(): Promise<void> {
  const uri = getActiveDraftUri();
  const info = await FileSystem.getInfoAsync(uri);

  if (info.exists) {
    await FileSystem.deleteAsync(uri, { idempotent: true });
  }
}

export async function readPendingReveal(): Promise<string | null> {
  await ensureAnkyDirectory();

  const uri = getPendingRevealUri();
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return null;
  }

  return FileSystem.readAsStringAsync(uri, { encoding: FileSystem.EncodingType.UTF8 });
}

export async function writePendingReveal(raw: string): Promise<void> {
  const parsed = parseAnky(raw);

  if (!parsed.valid) {
    throw new Error(`Cannot write invalid pending .anky session: ${parsed.errors.join(" ")}`);
  }

  await ensureAnkyDirectory();
  await FileSystem.writeAsStringAsync(getPendingRevealUri(), raw, {
    encoding: FileSystem.EncodingType.UTF8,
  });
}

export async function clearPendingReveal(): Promise<void> {
  const uri = getPendingRevealUri();
  const info = await FileSystem.getInfoAsync(uri);

  if (info.exists) {
    await FileSystem.deleteAsync(uri, { idempotent: true });
  }
}

export async function saveClosedSession(raw: string): Promise<SavedAnkyFile> {
  const parsed = parseAnky(raw);

  if (!parsed.valid) {
    throw new Error(`Cannot save invalid .anky session: ${parsed.errors.join(" ")}`);
  }

  const hash = await computeSessionHash(raw);
  const fileName = `${hash}.anky`;
  const uri = `${getAnkyDirectoryUri()}${fileName}`;

  await ensureAnkyDirectory();

  const info = await FileSystem.getInfoAsync(uri);

  if (info.exists) {
    const existing = await FileSystem.readAsStringAsync(uri, {
      encoding: FileSystem.EncodingType.UTF8,
    });

    if (existing !== raw) {
      throw new Error("Hash collision or corrupted existing .anky file.");
    }
  } else {
    await FileSystem.writeAsStringAsync(uri, raw, {
      encoding: FileSystem.EncodingType.UTF8,
    });
  }

  return toSavedAnkyFile(fileName, raw, true);
}

export async function listSavedAnkyFiles(): Promise<SavedAnkyFile[]> {
  await ensureAnkyDirectory();

  const fileNames = await FileSystem.readDirectoryAsync(getAnkyDirectoryUri());
  const ankyFileNames = fileNames
    .filter((fileName) => HASH_FILE_PATTERN.test(fileName))
    .sort()
    .reverse();

  return Promise.all(
    ankyFileNames.map(async (fileName) => {
      const raw = await readAnkyFile(fileName);

      return toSavedAnkyFile(fileName, raw);
    }),
  );
}

export async function readAnkyFile(fileName: string): Promise<string> {
  if (!HASH_FILE_PATTERN.test(fileName)) {
    throw new Error("Invalid .anky file name.");
  }

  return FileSystem.readAsStringAsync(`${getAnkyDirectoryUri()}${fileName}`, {
    encoding: FileSystem.EncodingType.UTF8,
  });
}

export async function deleteSavedAnkyFile(fileName: string): Promise<void> {
  if (!HASH_FILE_PATTERN.test(fileName)) {
    throw new Error("Invalid .anky file name.");
  }

  const uri = `${getAnkyDirectoryUri()}${fileName}`;
  const info = await FileSystem.getInfoAsync(uri);

  if (info.exists) {
    await FileSystem.deleteAsync(uri, { idempotent: true });
  }
}

export async function readSavedAnkyFile(fileName: string): Promise<SavedAnkyFile> {
  const raw = await readAnkyFile(fileName);

  return toSavedAnkyFile(fileName, raw);
}

export async function appendLoomSeal(seal: LoomSeal): Promise<void> {
  validateHash(seal.sessionHash);

  await ensureAnkyDirectory();

  const existing = await readLoomSealsForHash(seal.sessionHash);
  const seals = [...existing, seal];

  await FileSystem.writeAsStringAsync(getSealSidecarUri(seal.sessionHash), JSON.stringify(seals), {
    encoding: FileSystem.EncodingType.UTF8,
  });
}

export async function readLoomSealsForHash(sessionHash: string): Promise<LoomSeal[]> {
  validateHash(sessionHash);
  await ensureAnkyDirectory();

  const uri = getSealSidecarUri(sessionHash);
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return [];
  }

  const raw = await FileSystem.readAsStringAsync(uri, {
    encoding: FileSystem.EncodingType.UTF8,
  });
  const parsed = JSON.parse(raw) as unknown;

  if (!Array.isArray(parsed)) {
    return [];
  }

  return parsed.filter(isLoomSeal);
}

export async function listLocalLoomSeals(): Promise<LoomSeal[]> {
  await ensureAnkyDirectory();

  const fileNames = await FileSystem.readDirectoryAsync(getAnkyDirectoryUri());
  const sealFiles = fileNames.filter((fileName) => SEAL_FILE_PATTERN.test(fileName));
  const nested = await Promise.all(
    sealFiles.map((fileName) => readLoomSealsForHash(fileName.replace(/\.seals\.json$/, ""))),
  );

  return nested.flat();
}

export async function writeProcessingArtifacts(
  artifacts: ProcessingArtifact[],
): Promise<string[]> {
  await ensureAnkyDirectory();

  const written: string[] = [];

  for (const artifact of artifacts) {
    switch (artifact.kind) {
      case "reflection": {
        validateHash(artifact.sessionHash);
        written.push(
          await writeUtf8Sidecar(`${artifact.sessionHash}.reflection.md`, artifact.markdown),
        );
        break;
      }

      case "title": {
        validateHash(artifact.sessionHash);
        written.push(await writeUtf8Sidecar(`${artifact.sessionHash}.title.txt`, artifact.title));
        break;
      }

      case "image": {
        validateHash(artifact.sessionHash);

        if (artifact.imageBase64 != null) {
          const extension = imageExtensionForMimeType(artifact.mimeType);
          const fileName = `${artifact.sessionHash}.image.${extension}`;
          const uri = `${getAnkyDirectoryUri()}${fileName}`;

          await FileSystem.writeAsStringAsync(uri, artifact.imageBase64, {
            encoding: FileSystem.EncodingType.Base64,
          });
          written.push(fileName);
          break;
        }

        if (artifact.imageUrl != null) {
          written.push(
            await writeUtf8Sidecar(
              `${artifact.sessionHash}.meta.json`,
              JSON.stringify({ imageUrl: artifact.imageUrl, mimeType: artifact.mimeType }),
            ),
          );
        }

        break;
      }

      case "deep_mirror": {
        validateHash(artifact.carpetHash);
        written.push(
          await writeUtf8Sidecar(`${artifact.carpetHash}.deep_mirror.md`, artifact.markdown),
        );
        break;
      }

      case "full_sojourn_archive": {
        validateHash(artifact.carpetHash);
        written.push(
          await writeUtf8Sidecar(
            `${artifact.carpetHash}.full_sojourn_archive.md`,
            artifact.markdown,
          ),
        );

        if (artifact.summaryJson !== undefined) {
          written.push(
            await writeUtf8Sidecar(
              `${artifact.carpetHash}.meta.json`,
              JSON.stringify(artifact.summaryJson),
            ),
          );
        }

        break;
      }
    }
  }

  return written;
}

export async function listArtifactKindsForHash(
  sessionHash: string,
): Promise<SidecarArtifactKind[]> {
  validateHash(sessionHash);
  await ensureAnkyDirectory();

  const fileNames = await FileSystem.readDirectoryAsync(getAnkyDirectoryUri());

  return getArtifactKindsFromFileNames(sessionHash, fileNames);
}

async function toSavedAnkyFile(
  fileName: string,
  raw: string,
  knownHashMatch?: boolean,
): Promise<SavedAnkyFile> {
  const hash = fileName.replace(/\.anky$/, "");
  const parsed = parseAnky(raw);
  const hashMatches = knownHashMatch ?? (await verifyHash(raw, hash));
  const preview = reconstructText(raw).slice(0, 96);
  const seals = await readLoomSealsForHash(hash);
  const artifactKinds = await listArtifactKindsForHash(hash);

  return {
    artifactKinds,
    fileName,
    hash,
    hashMatches,
    latestSeal: seals.at(-1),
    localState: resolveAnkyLocalState({
      artifactCount: artifactKinds.length,
      closed: parsed.closed,
      hashMatches,
      sealCount: seals.length,
      valid: parsed.valid,
    }),
    preview,
    raw,
    sealCount: seals.length,
    uri: `${getAnkyDirectoryUri()}${fileName}`,
    valid: parsed.valid,
  };
}

function getSealSidecarUri(sessionHash: string): string {
  return `${getAnkyDirectoryUri()}${sessionHash}.seals.json`;
}

async function writeUtf8Sidecar(fileName: string, value: string): Promise<string> {
  await FileSystem.writeAsStringAsync(`${getAnkyDirectoryUri()}${fileName}`, value, {
    encoding: FileSystem.EncodingType.UTF8,
  });

  return fileName;
}

function validateHash(value: string): void {
  if (!HASH_PATTERN.test(value)) {
    throw new Error("Invalid session hash.");
  }
}

function isLoomSeal(value: unknown): value is LoomSeal {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const seal = value as Partial<LoomSeal>;

  return (
    typeof seal.txSignature === "string" &&
    typeof seal.writer === "string" &&
    typeof seal.loomId === "string" &&
    typeof seal.sessionHash === "string" &&
    HASH_PATTERN.test(seal.sessionHash)
  );
}

function getArtifactKindsFromFileNames(
  sessionHash: string,
  fileNames: string[],
): SidecarArtifactKind[] {
  const matches: Array<[SidecarArtifactKind, boolean]> = [
    ["reflection", fileNames.includes(`${sessionHash}.reflection.md`)],
    ["title", fileNames.includes(`${sessionHash}.title.txt`)],
    [
      "image",
      fileNames.some((fileName) =>
        /^image\.(png|jpe?g|webp)$/.test(fileName.replace(`${sessionHash}.`, "")),
      ),
    ],
    ["meta", fileNames.includes(`${sessionHash}.meta.json`)],
    ["conversation", fileNames.includes(`${sessionHash}.conversation.json`)],
    ["deep_mirror", fileNames.includes(`${sessionHash}.deep_mirror.md`)],
    ["full_sojourn_archive", fileNames.includes(`${sessionHash}.full_sojourn_archive.md`)],
  ];

  return matches
    .filter(([, present]) => present)
    .map(([kind]) => kind);
}

function imageExtensionForMimeType(mimeType: "image/jpeg" | "image/png" | "image/webp"): string {
  switch (mimeType) {
    case "image/jpeg":
      return "jpg";
    case "image/png":
      return "png";
    case "image/webp":
      return "webp";
  }
}
