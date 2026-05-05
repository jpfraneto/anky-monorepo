declare const process:
  | {
      env?: Record<string, string | undefined>;
    }
  | undefined;

declare const require:
  | ((name: string) => unknown)
  | undefined;

type PublicEnvName =
  | "EXPO_PUBLIC_PRIVY_APP_ID"
  | "EXPO_PUBLIC_PRIVY_CLIENT_ID"
  | "EXPO_PUBLIC_ANKY_API_URL"
  | "EXPO_PUBLIC_APP_URL"
  | "EXPO_PUBLIC_SOLANA_RPC_URL"
  | "EXPO_PUBLIC_SOLANA_CLUSTER"
  | "EXPO_PUBLIC_SOLANA_SEAL_ADAPTER"
  | "EXPO_PUBLIC_ANKY_CORE_PROGRAM_ID"
  | "EXPO_PUBLIC_ANKY_CORE_COLLECTION"
  | "EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID"
  | "EXPO_PUBLIC_PRIVY_WALLET_EXPORT_URL"
  | "EXPO_PUBLIC_IAP_CREDITS_8_ID"
  | "EXPO_PUBLIC_IAP_CREDITS_24_ID"
  | "EXPO_PUBLIC_IAP_CREDITS_88_ID"
  | "EXPO_PUBLIC_IAP_PREMIUM_MONTHLY_ID";

const bundledPublicEnv: Partial<Record<PublicEnvName, string | undefined>> =
  typeof process === "undefined" || process.env == null
    ? {}
    : {
        EXPO_PUBLIC_PRIVY_APP_ID: process.env.EXPO_PUBLIC_PRIVY_APP_ID,
        EXPO_PUBLIC_PRIVY_CLIENT_ID: process.env.EXPO_PUBLIC_PRIVY_CLIENT_ID,
        EXPO_PUBLIC_ANKY_API_URL: process.env.EXPO_PUBLIC_ANKY_API_URL,
        EXPO_PUBLIC_APP_URL: process.env.EXPO_PUBLIC_APP_URL,
        EXPO_PUBLIC_SOLANA_RPC_URL: process.env.EXPO_PUBLIC_SOLANA_RPC_URL,
        EXPO_PUBLIC_SOLANA_CLUSTER: process.env.EXPO_PUBLIC_SOLANA_CLUSTER,
        EXPO_PUBLIC_SOLANA_SEAL_ADAPTER:
          process.env.EXPO_PUBLIC_SOLANA_SEAL_ADAPTER,
        EXPO_PUBLIC_ANKY_CORE_PROGRAM_ID:
          process.env.EXPO_PUBLIC_ANKY_CORE_PROGRAM_ID,
        EXPO_PUBLIC_ANKY_CORE_COLLECTION:
          process.env.EXPO_PUBLIC_ANKY_CORE_COLLECTION,
        EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID:
          process.env.EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID,
        EXPO_PUBLIC_PRIVY_WALLET_EXPORT_URL:
          process.env.EXPO_PUBLIC_PRIVY_WALLET_EXPORT_URL,
        EXPO_PUBLIC_IAP_CREDITS_8_ID: process.env.EXPO_PUBLIC_IAP_CREDITS_8_ID,
        EXPO_PUBLIC_IAP_CREDITS_24_ID:
          process.env.EXPO_PUBLIC_IAP_CREDITS_24_ID,
        EXPO_PUBLIC_IAP_CREDITS_88_ID:
          process.env.EXPO_PUBLIC_IAP_CREDITS_88_ID,
        EXPO_PUBLIC_IAP_PREMIUM_MONTHLY_ID:
          process.env.EXPO_PUBLIC_IAP_PREMIUM_MONTHLY_ID,
      };

type ExpoConstants = {
  expoConfig?: {
    extra?: {
      publicEnv?: Record<string, string | undefined>;
    };
  };
};

type ExpoConstantsModule = ExpoConstants & {
  default?: ExpoConstants;
};

let expoExtraPublicEnv: Record<string, string | undefined> | undefined;

function getExpoExtraPublicEnv(name: string): string | undefined {
  if (expoExtraPublicEnv == null) {
    expoExtraPublicEnv = {};

    if (
      typeof process !== "undefined" &&
      (process.env?.VITEST != null || process.env?.NODE_ENV === "test")
    ) {
      return undefined;
    }

    try {
      const constantsModule =
        typeof require === "function"
          ? (require("expo-constants") as ExpoConstantsModule)
          : undefined;
      const constants = constantsModule?.default ?? constantsModule;

      expoExtraPublicEnv = constants?.expoConfig?.extra?.publicEnv ?? {};
    } catch {
      expoExtraPublicEnv = {};
    }
  }

  return expoExtraPublicEnv[name];
}

export function getPublicEnv(name: string): string | undefined {
  if (typeof process === "undefined") {
    const extraValue = getExpoExtraPublicEnv(name);
    return extraValue == null || extraValue.trim().length === 0
      ? undefined
      : extraValue;
  }

  const value =
    bundledPublicEnv[name as PublicEnvName] ??
    getExpoExtraPublicEnv(name) ??
    process.env?.[name];

  return value == null || value.trim().length === 0 ? undefined : value;
}
