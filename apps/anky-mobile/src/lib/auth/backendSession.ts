import * as SecureStore from "expo-secure-store";

import { createAnkyApiClient } from "../api/ankyApi";
import type { BackendAuthResponse } from "../api/types";
import { getPublicEnv } from "../config/env";

const BACKEND_SESSION_KEY = "anky.backend.session.v1";

export const ANKY_API_BASE_URL = getPublicEnv("EXPO_PUBLIC_ANKY_API_URL") ?? "";

export type BackendAuthSession = {
  email?: string;
  sessionToken: string;
  userId: string;
  username?: string;
  walletAddress?: string;
};

export type BackendWalletAuthProof = {
  siwsMessage: string;
  siwsSignature: string;
  walletAddress: string;
};

export function hasConfiguredBackend(): boolean {
  return ANKY_API_BASE_URL.length > 0;
}

export async function exchangePrivyAccessTokenForBackendSession(
  accessToken: string,
  walletProof?: BackendWalletAuthProof,
): Promise<BackendAuthSession> {
  const api = createAnkyApiClient({ baseUrl: ANKY_API_BASE_URL });
  const response = await api.exchangePrivyAuthToken({
    auth_token: accessToken,
    siws_message: walletProof?.siwsMessage,
    siws_signature: walletProof?.siwsSignature,
    wallet_address: walletProof?.walletAddress,
  });
  const session = toBackendAuthSession(response);

  await saveBackendAuthSession(session);

  return session;
}

export async function getStoredBackendAuthSession(): Promise<BackendAuthSession | null> {
  const value = await SecureStore.getItemAsync(BACKEND_SESSION_KEY);

  if (value == null) {
    return null;
  }

  try {
    return JSON.parse(value) as BackendAuthSession;
  } catch {
    await clearBackendAuthSession();
    return null;
  }
}

export async function saveBackendAuthSession(session: BackendAuthSession): Promise<void> {
  await SecureStore.setItemAsync(BACKEND_SESSION_KEY, JSON.stringify(session));
}

export async function clearBackendAuthSession(): Promise<void> {
  await SecureStore.deleteItemAsync(BACKEND_SESSION_KEY);
}

function toBackendAuthSession(response: BackendAuthResponse): BackendAuthSession {
  if (!response.ok || response.session_token.length === 0 || response.user_id.length === 0) {
    throw new Error("Invalid backend auth response.");
  }

  return {
    email: response.email,
    sessionToken: response.session_token,
    userId: response.user_id,
    username: response.username,
    walletAddress: response.wallet_address,
  };
}
