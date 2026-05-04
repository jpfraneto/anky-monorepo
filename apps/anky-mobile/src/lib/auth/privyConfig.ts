import { getPublicEnv } from "../config/env";

export const PRIVY_APP_ID =
  getPublicEnv("EXPO_PUBLIC_PRIVY_APP_ID") ?? "cmivv85zt00ftla0cjpaw155h";

export const PRIVY_CLIENT_ID = getPublicEnv("EXPO_PUBLIC_PRIVY_CLIENT_ID");

export const ANKY_APP_URL = getPublicEnv("EXPO_PUBLIC_APP_URL") ?? "https://anky.app";

export const PRIVY_OAUTH_REDIRECT_PATH = "auth";

export const PRIVY_WALLET_REDIRECT_URI = "anky://auth";

export function getPrivySignInDomain(): string {
  try {
    return new URL(ANKY_APP_URL).host;
  } catch {
    return "anky.app";
  }
}
