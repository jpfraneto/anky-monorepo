import { getAnkyApiClient } from "../api/client";
import { appendLoomSeal } from "../ankyStorage";

export async function hydrateMobileSealReceiptsForHashes(
  sessionHashes: string[],
): Promise<number> {
  const api = getAnkyApiClient();

  if (api == null) {
    return 0;
  }

  let hydratedCount = 0;

  for (const sessionHash of uniqueHashes(sessionHashes)) {
    try {
      const response = await api.lookupMobileSeals({ sessionHash });

      for (const seal of response.seals) {
        await appendLoomSeal(seal);
        hydratedCount += 1;
      }
    } catch (error) {
      console.warn("Could not hydrate mobile seal receipt.", error);
    }
  }

  return hydratedCount;
}

function uniqueHashes(sessionHashes: string[]): string[] {
  return [...new Set(sessionHashes.filter((hash) => /^[a-f0-9]{64}$/.test(hash)))];
}
