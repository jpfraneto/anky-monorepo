import * as SecureStore from "expo-secure-store";

const THREAD_PROCESSING_CONSENT_KEY = "thread_processing_consent_v1";

export async function hasThreadProcessingConsent(): Promise<boolean> {
  return (await SecureStore.getItemAsync(THREAD_PROCESSING_CONSENT_KEY)) === "true";
}

export async function markThreadProcessingConsent(): Promise<void> {
  await SecureStore.setItemAsync(THREAD_PROCESSING_CONSENT_KEY, "true");
}

export async function resetThreadProcessingConsent(): Promise<void> {
  await SecureStore.deleteItemAsync(THREAD_PROCESSING_CONSENT_KEY);
}
