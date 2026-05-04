import {
  readSavedAnkyFile,
  writeLocalReflectionSidecars,
  writeProcessingArtifacts,
} from "../ankyStorage";
import { AnkyApiError } from "../api/ankyApi";
import { requireAnkyApiClient } from "../api/client";
import type { ProcessingType } from "../api/types";
import { hasConfiguredBackend } from "../auth/backendSession";
import { getMobileApiIdentityId } from "../auth/mobileIdentity";

export type LocalReflectionResult = {
  creditsRemaining: number;
  creditsSpent: number;
  markdown: string;
};

export type ReflectionMode = "full" | "simple";

export async function processLocalReflection(fileName: string): Promise<LocalReflectionResult> {
  await readSavedAnkyFile(fileName);
  throw new Error(
    "local reflection generation is not available. your writing is still saved.",
  );
}

export async function getReflectionCreditBalance(): Promise<number> {
  if (!hasConfiguredBackend()) {
    return 0;
  }

  const identityId = await getMobileApiIdentityId();
  const api = requireAnkyApiClient();
  try {
    const response = await api.getMobileCreditBalance(identityId);

    return response.account.creditsRemaining;
  } catch (error) {
    console.warn("Backend credits unavailable.", error);
    return 0;
  }
}

export async function processReflection(fileName: string): Promise<LocalReflectionResult> {
  return processReflectionWithMode(fileName, "simple");
}

export async function processReflectionWithMode(
  fileName: string,
  mode: ReflectionMode = "simple",
): Promise<LocalReflectionResult> {
  if (!hasConfiguredBackend()) {
    throw new Error(
      "anky cannot respond because the backend is not configured. your writing is still saved.",
    );
  }

  try {
    return await processBackendReflection(fileName, mode);
  } catch (error) {
    throw toReflectionError(error);
  }
}

export async function processBackendReflection(
  fileName: string,
  mode: ReflectionMode = "simple",
): Promise<LocalReflectionResult> {
  const saved = await readSavedAnkyFile(fileName);

  if (!saved.valid || !saved.hashMatches) {
    throw new Error("Cannot reflect an invalid .anky file.");
  }

  const identityId = await getMobileApiIdentityId();
  const api = requireAnkyApiClient();
  const response = await api.createMobileReflection({
    anky: saved.raw,
    identityId,
    processingType: mode === "full" ? "full_anky" : "reflection",
    sessionHash: saved.hash,
  });
  const reflection = response.artifacts.find((artifact) => artifact.kind === "reflection");

  if (reflection == null || reflection.kind !== "reflection") {
    throw new Error("Backend did not return a reflection artifact.");
  }

  await writeProcessingArtifacts(response.artifacts);
  await writeLocalReflectionSidecars({
    creditsRemaining: response.creditsRemaining,
    creditsSpent: response.creditsSpent,
    engine: "anky-backend-dev-placeholder",
    markdown: reflection.markdown,
    processingType: mode === "full" ? "full_anky" : "reflection",
    sessionHash: saved.hash,
  });

  return {
    creditsRemaining: response.creditsRemaining,
    creditsSpent: response.creditsSpent,
    markdown: reflection.markdown,
  };
}

export function isLocalProcessingImplemented(processingType: ProcessingType): boolean {
  return false;
}

function toReflectionError(error: unknown): Error {
  if (error instanceof AnkyApiError) {
    if (error.status === 402) {
      return new Error("not enough credits for this reflection. your writing is still saved.");
    }

    if (error.status >= 500) {
      return new Error("anky could not respond right now. your writing is still saved.");
    }

    if (error.body != null && error.body.trim().length > 0) {
      return new Error(`reflection was refused: ${error.body}`);
    }

    return new Error("reflection was refused. your writing is still saved.");
  }

  if (error instanceof Error) {
    if (error.name === "AbortError") {
      return new Error("anky took too long to respond. your writing is still saved.");
    }

    return new Error(`${error.message} your writing is still saved.`);
  }

  return new Error("reflection failed. your writing is still saved.");
}
