import * as SecureStore from "expo-secure-store";

import { getStoredBackendAuthSession } from "./backendSession";

const MOBILE_IDENTITY_KEY = "anky.mobile.identity.v1";

export async function getMobileApiIdentityId(): Promise<string> {
  const session = await getStoredBackendAuthSession();

  if (session?.userId != null && session.userId.length > 0) {
    return `user:${session.userId}`;
  }

  const existing = await SecureStore.getItemAsync(MOBILE_IDENTITY_KEY);

  if (existing != null && existing.length > 0) {
    return existing;
  }

  const identityId = `device:${Date.now().toString(36)}-${Math.random()
    .toString(36)
    .slice(2)}`;
  await SecureStore.setItemAsync(MOBILE_IDENTITY_KEY, identityId);

  return identityId;
}
