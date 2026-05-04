import { ANKY_API_BASE_URL, hasConfiguredBackend } from "../auth/backendSession";
import { AnkyApiClient, createAnkyApiClient } from "./ankyApi";

export function getAnkyApiClient(): AnkyApiClient | null {
  if (!hasConfiguredBackend()) {
    return null;
  }

  return createAnkyApiClient({ baseUrl: ANKY_API_BASE_URL });
}

export function requireAnkyApiClient(): AnkyApiClient {
  const api = getAnkyApiClient();

  if (api == null) {
    throw new Error("Anky API URL is not configured.");
  }

  return api;
}
