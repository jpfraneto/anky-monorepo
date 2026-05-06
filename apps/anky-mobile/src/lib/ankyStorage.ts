import * as FileSystem from "expo-file-system/legacy";

import {
  computeSessionHash,
  hasTerminalLine,
  parseAnky,
  reconstructText,
  verifyHash,
} from "./ankyProtocol";
import { resolveAnkyLocalState } from "./ankyState";
import type { AnkyLocalState } from "./ankyState";
import type { ProcessingArtifact } from "./api/types";
import type { AnkySolanaCluster } from "./solana/ankySolanaConfig";
import type { LoomSeal } from "./solana/types";

const ANKY_DIRECTORY = "anky/";
const ACTIVE_DRAFT_FILE = "active.anky.draft";
const PENDING_REVEAL_FILE = "pending.anky";
const HASH_FILE_PATTERN = /^[a-f0-9]{64}\.anky$/;
const HASH_PATTERN = /^[a-f0-9]{64}$/;
const SEAL_FILE_PATTERN = /^[a-f0-9]{64}\.seals\.json$/;
const SEAL_RECEIPT_FILE_PATTERN = /^[a-f0-9]{64}\.seal\.json$/;

type SidecarArtifactKind =
  | "conversation"
  | "deep_mirror"
  | "full_sojourn_archive"
  | "image"
  | "meta"
  | "processing"
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

export type AnkySealSidecar = {
  created_at: string;
  loom_asset: string;
  network: AnkySolanaCluster;
  session_hash: string;
  signature: string;
  status: "confirmed";
  utc_day?: number;
  version: 1;
  writer: string;
};

export type ProcessingReceiptSidecar = {
  created_at: string;
  credits_remaining: number;
  credits_spent: number;
  engine: "anky-backend-dev-placeholder" | "local-dev-placeholder";
  processing_type: "full_anky" | "reflection";
  version: 1;
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

export async function stageTerminalDraftForReveal(raw: string): Promise<SavedAnkyFile> {
  if (!hasTerminalLine(raw)) {
    throw new Error("Cannot stage a non-terminal active draft for reveal.");
  }

  const parsed = parseAnky(raw);

  if (!parsed.valid) {
    throw new Error(`Cannot stage invalid terminal active draft: ${parsed.errors.join(" ")}`);
  }

  const saved = await saveClosedSession(raw);

  await writePendingReveal(raw);

  return saved;
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
  const bySignature = new Map(existing.map((item) => [item.txSignature, item]));

  bySignature.set(seal.txSignature, seal);
  const seals = [...bySignature.values()];

  await FileSystem.writeAsStringAsync(getSealSidecarUri(seal.sessionHash), JSON.stringify(seals), {
    encoding: FileSystem.EncodingType.UTF8,
  });
}

export async function writeSealSidecar(seal: AnkySealSidecar): Promise<void> {
  validateHash(seal.session_hash);
  await ensureAnkyDirectory();
  await writeUtf8Sidecar(`${seal.session_hash}.seal.json`, JSON.stringify(seal, null, 2));
}

export async function readSealSidecar(sessionHash: string): Promise<AnkySealSidecar | null> {
  validateHash(sessionHash);
  await ensureAnkyDirectory();

  const uri = getSingleSealSidecarUri(sessionHash);
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return null;
  }

  const raw = await FileSystem.readAsStringAsync(uri, {
    encoding: FileSystem.EncodingType.UTF8,
  });
  const parsed = JSON.parse(raw) as unknown;

  return isAnkySealSidecar(parsed) ? parsed : null;
}

export async function readLoomSealsForHash(sessionHash: string): Promise<LoomSeal[]> {
  validateHash(sessionHash);
  await ensureAnkyDirectory();

  const singleSeal = await readSealSidecar(sessionHash);
  const uri = getSealSidecarUri(sessionHash);
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return singleSeal == null ? [] : [toLoomSeal(singleSeal)];
  }

  const raw = await FileSystem.readAsStringAsync(uri, {
    encoding: FileSystem.EncodingType.UTF8,
  });
  const parsed = JSON.parse(raw) as unknown;

  if (!Array.isArray(parsed)) {
    return [];
  }

  const legacySeals = parsed.filter(isLoomSeal);

  if (singleSeal == null) {
    return legacySeals;
  }

  return [...legacySeals, toLoomSeal(singleSeal)];
}

export async function listLocalLoomSeals(): Promise<LoomSeal[]> {
  await ensureAnkyDirectory();

  const fileNames = await FileSystem.readDirectoryAsync(getAnkyDirectoryUri());
  const sealFiles = fileNames.filter((fileName) => SEAL_FILE_PATTERN.test(fileName));
  const sealReceiptFiles = fileNames.filter((fileName) => SEAL_RECEIPT_FILE_PATTERN.test(fileName));
  const nested = await Promise.all(
    sealFiles.map((fileName) => readLoomSealsForHash(fileName.replace(/\.seals\.json$/, ""))),
  );
  const singleSeals = await Promise.all(
    sealReceiptFiles.map((fileName) => readSealSidecar(fileName.replace(/\.seal\.json$/, ""))),
  );

  const allSeals = [
    ...nested.flat(),
    ...singleSeals.filter((seal): seal is AnkySealSidecar => seal != null).map(toLoomSeal),
  ];
  const unique = new Map<string, LoomSeal>();

  allSeals.forEach((seal) => {
    unique.set(seal.txSignature, seal);
  });

  return [...unique.values()];
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

export async function writeLocalReflectionSidecars({
  creditsRemaining,
  creditsSpent,
  engine = "local-dev-placeholder",
  markdown,
  processingType = "reflection",
  sessionHash,
}: {
  creditsRemaining: number;
  creditsSpent: number;
  engine?: ProcessingReceiptSidecar["engine"];
  markdown: string;
  processingType?: ProcessingReceiptSidecar["processing_type"];
  sessionHash: string;
}): Promise<void> {
  validateHash(sessionHash);
  await ensureAnkyDirectory();

  await writeUtf8Sidecar(`${sessionHash}.reflection.md`, markdown);
  await writeUtf8Sidecar(
    `${sessionHash}.processing.json`,
    JSON.stringify(
      {
        created_at: new Date().toISOString(),
        credits_remaining: creditsRemaining,
        credits_spent: creditsSpent,
        engine,
        processing_type: processingType,
        version: 1,
      } satisfies ProcessingReceiptSidecar,
      null,
      2,
    ),
  );
}

export async function deleteAllLocalAnkyData(): Promise<void> {
  const uri = getAnkyDirectoryUri();
  const info = await FileSystem.getInfoAsync(uri);

  if (info.exists) {
    await FileSystem.deleteAsync(uri, { idempotent: true });
  }

  await ensureAnkyDirectory();
}

export async function readReflectionSidecar(sessionHash: string): Promise<string | null> {
  validateHash(sessionHash);
  await ensureAnkyDirectory();

  return readOptionalUtf8Sidecar(`${sessionHash}.reflection.md`);
}

export async function readProcessingReceipt(
  sessionHash: string,
): Promise<ProcessingReceiptSidecar | null> {
  validateHash(sessionHash);
  await ensureAnkyDirectory();

  const raw = await readOptionalUtf8Sidecar(`${sessionHash}.processing.json`);

  if (raw == null) {
    return null;
  }

  const parsed = JSON.parse(raw) as unknown;

  return isProcessingReceiptSidecar(parsed) ? parsed : null;
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

function getSingleSealSidecarUri(sessionHash: string): string {
  return `${getAnkyDirectoryUri()}${sessionHash}.seal.json`;
}

async function writeUtf8Sidecar(fileName: string, value: string): Promise<string> {
  await FileSystem.writeAsStringAsync(`${getAnkyDirectoryUri()}${fileName}`, value, {
    encoding: FileSystem.EncodingType.UTF8,
  });

  return fileName;
}

async function readOptionalUtf8Sidecar(fileName: string): Promise<string | null> {
  const uri = `${getAnkyDirectoryUri()}${fileName}`;
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return null;
  }

  return FileSystem.readAsStringAsync(uri, {
    encoding: FileSystem.EncodingType.UTF8,
  });
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

function isAnkySealSidecar(value: unknown): value is AnkySealSidecar {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const seal = value as Partial<AnkySealSidecar>;

  return (
    seal.version === 1 &&
    (seal.network === "devnet" || seal.network === "mainnet-beta") &&
    seal.status === "confirmed" &&
    typeof seal.session_hash === "string" &&
    HASH_PATTERN.test(seal.session_hash) &&
    typeof seal.loom_asset === "string" &&
    typeof seal.writer === "string" &&
    typeof seal.signature === "string" &&
    typeof seal.created_at === "string" &&
    (seal.utc_day == null || Number.isSafeInteger(seal.utc_day))
  );
}

function isProcessingReceiptSidecar(value: unknown): value is ProcessingReceiptSidecar {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const receipt = value as Partial<ProcessingReceiptSidecar>;

  return (
    receipt.version === 1 &&
    (receipt.processing_type === "reflection" || receipt.processing_type === "full_anky") &&
    (receipt.engine === "local-dev-placeholder" ||
      receipt.engine === "anky-backend-dev-placeholder") &&
    typeof receipt.credits_spent === "number" &&
    typeof receipt.credits_remaining === "number" &&
    typeof receipt.created_at === "string"
  );
}

function toLoomSeal(seal: AnkySealSidecar): LoomSeal {
  return {
    createdAt: seal.created_at,
    loomId: seal.loom_asset,
    network: seal.network,
    sessionHash: seal.session_hash,
    txSignature: seal.signature,
    writer: seal.writer,
  };
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
    ["processing", fileNames.includes(`${sessionHash}.processing.json`)],
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
