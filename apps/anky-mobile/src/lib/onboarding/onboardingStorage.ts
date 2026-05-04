import * as SecureStore from "expo-secure-store";

const ONBOARDING_COMPLETE_KEY = "anky.onboarding.complete.v1";

export async function hasCompletedOnboarding(): Promise<boolean> {
  return (await SecureStore.getItemAsync(ONBOARDING_COMPLETE_KEY)) === "true";
}

export async function markOnboardingComplete(): Promise<void> {
  await SecureStore.setItemAsync(ONBOARDING_COMPLETE_KEY, "true");
}

export async function resetOnboardingForDev(): Promise<void> {
  await SecureStore.deleteItemAsync(ONBOARDING_COMPLETE_KEY);
}
