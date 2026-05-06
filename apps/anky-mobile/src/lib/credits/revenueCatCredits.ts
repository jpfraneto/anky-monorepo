import { Platform } from "react-native";
import type { PurchasesPackage } from "react-native-purchases";

import { getMobileApiIdentityId } from "../auth/mobileIdentity";
import { getPublicEnv } from "../config/env";
import {
  CREDIT_PRODUCTS,
  type AnkyRevenueCatPackageId,
  type CreditProduct,
} from "./products";

export type { AnkyRevenueCatPackageId };

export type RevenueCatCreditStatus = "available" | "pending" | "unavailable";

export type AnkyCreditStorePackage = {
  bonusCredits: number;
  description: string;
  packageId: AnkyRevenueCatPackageId;
  priceLabel: string;
  productId: string;
  recommended?: boolean;
  revenueCatPackage: PurchasesPackage;
  title: string;
  totalCredits: number;
};

export type RevenueCatCreditPurchaseResult =
  | { message: string; status: "cancelled" }
  | { message: string; status: "completed" }
  | { message: string; status: "failed" };

type PurchasesModule = typeof import("react-native-purchases");

const CREDITS_OFFERING_ID = "credits";
const CREDITS_VIRTUAL_CURRENCY_CODE = "CREDITS";

let purchasesModule: PurchasesModule | null = null;
let configuredAppUserId: string | null = null;
let configurePromise: Promise<void> | null = null;
let status: RevenueCatCreditStatus =
  Platform.OS === "ios" || Platform.OS === "android" ? "pending" : "unavailable";
const packageCache = new Map<AnkyRevenueCatPackageId, AnkyCreditStorePackage>();

export async function configureRevenueCat(): Promise<void> {
  if (Platform.OS !== "ios" && Platform.OS !== "android") {
    status = "unavailable";
    return;
  }

  if (configurePromise != null) {
    return configurePromise;
  }

  configurePromise = configureRevenueCatOnce().finally(() => {
    configurePromise = null;
  });

  return configurePromise;
}

export async function getCreditsOfferingPackages(): Promise<AnkyCreditStorePackage[]> {
  await configureRevenueCat();

  if (status !== "available" || purchasesModule == null) {
    throw new Error("revenuecat is not configured in this build.");
  }

  const Purchases = purchasesModule.default;
  const offerings = await Purchases.getOfferings();
  const offering =
    offerings.all[CREDITS_OFFERING_ID] ??
    (offeringHasExpectedPackages(offerings.current) ? offerings.current : null);

  if (offering == null) {
    throw new Error("credits offering is unavailable.");
  }

  const byPackageId = new Map<AnkyRevenueCatPackageId, PurchasesPackage>();

  for (const revenueCatPackage of offering.availablePackages) {
    const packageId = getKnownPackageId(revenueCatPackage);

    if (packageId != null) {
      byPackageId.set(packageId, revenueCatPackage);
    }
  }

  const packages = CREDIT_PRODUCTS.map((product) => {
    const revenueCatPackage = byPackageId.get(product.revenueCatPackageId);

    if (revenueCatPackage == null) {
      throw new Error(`revenuecat package ${product.revenueCatPackageId} is missing.`);
    }

    return toStorePackage(product, revenueCatPackage);
  });

  packageCache.clear();
  for (const storePackage of packages) {
    packageCache.set(storePackage.packageId, storePackage);
  }

  return packages;
}

export async function purchaseCreditsPackage(
  packageId: AnkyRevenueCatPackageId,
): Promise<RevenueCatCreditPurchaseResult> {
  try {
    await configureRevenueCat();

    if (status !== "available" || purchasesModule == null) {
      return {
        message: "iap unavailable in this build.",
        status: "failed",
      };
    }

    let storePackage = packageCache.get(packageId);

    if (storePackage == null) {
      const packages = await getCreditsOfferingPackages();
      storePackage = packages.find((candidate) => candidate.packageId === packageId);
    }

    if (storePackage == null) {
      return {
        message: "credits package is unavailable.",
        status: "failed",
      };
    }

    await purchasesModule.default.purchasePackage(storePackage.revenueCatPackage);
    await purchasesModule.default.invalidateVirtualCurrenciesCache().catch(() => undefined);
    await purchasesModule.default.getVirtualCurrencies().catch(() => undefined);

    return {
      message: "credits added.",
      status: "completed",
    };
  } catch (error) {
    if (isRevenueCatPurchaseCancelled(error)) {
      return {
        message: "purchase cancelled.",
        status: "cancelled",
      };
    }

    return {
      message: getRevenueCatErrorMessage(error),
      status: "failed",
    };
  }
}

export async function getRevenueCatCreditBalance(): Promise<number> {
  await configureRevenueCat();

  if (status !== "available" || purchasesModule == null) {
    throw new Error("revenuecat is not configured in this build.");
  }

  const currencies = await purchasesModule.default.getVirtualCurrencies();
  const credits = currencies.all[CREDITS_VIRTUAL_CURRENCY_CODE];

  return credits?.balance ?? 0;
}

export function getRevenueCatCreditStatus(): RevenueCatCreditStatus {
  return status;
}

async function configureRevenueCatOnce(): Promise<void> {
  const apiKey = getRevenueCatApiKey();

  if (apiKey == null) {
    status = "unavailable";
    return;
  }

  status = "pending";

  try {
    const nextModule = await import("react-native-purchases");
    const Purchases = nextModule.default;
    const appUserID = await getMobileApiIdentityId();
    const alreadyConfigured = await Purchases.isConfigured().catch(() => false);

    if (!alreadyConfigured) {
      Purchases.configure({ apiKey, appUserID });
      configuredAppUserId = appUserID;
    } else {
      const currentAppUserId = await Purchases.getAppUserID().catch(() => configuredAppUserId);

      if (currentAppUserId != null && currentAppUserId !== appUserID) {
        await Purchases.logIn(appUserID);
      }

      configuredAppUserId = appUserID;
    }

    purchasesModule = nextModule;
    status = "available";
  } catch (error) {
    status = "unavailable";
    throw error;
  }
}

function getRevenueCatApiKey(): string | null {
  const envName =
    Platform.OS === "ios"
      ? "EXPO_PUBLIC_REVENUECAT_IOS_API_KEY"
      : Platform.OS === "android"
        ? "EXPO_PUBLIC_REVENUECAT_ANDROID_API_KEY"
        : null;

  if (envName == null) {
    return null;
  }

  return getPublicEnv(envName) ?? null;
}

function offeringHasExpectedPackages(
  offering: { availablePackages: PurchasesPackage[] } | null,
): offering is { availablePackages: PurchasesPackage[] } {
  if (offering == null) {
    return false;
  }

  const packageIds = new Set(
    offering.availablePackages
      .map(getKnownPackageId)
      .filter((packageId): packageId is AnkyRevenueCatPackageId => packageId != null),
  );

  return CREDIT_PRODUCTS.every((product) => packageIds.has(product.revenueCatPackageId));
}

function getKnownPackageId(
  revenueCatPackage: PurchasesPackage,
): AnkyRevenueCatPackageId | null {
  const byIdentifier = normalizeRevenueCatPackageIdentifier(revenueCatPackage.identifier);

  if (byIdentifier != null) {
    return byIdentifier;
  }

  const product = CREDIT_PRODUCTS.find(
    (candidate) =>
      candidate.iosProductId === revenueCatPackage.product.identifier ||
      candidate.androidProductId === revenueCatPackage.product.identifier,
  );

  return product?.revenueCatPackageId ?? null;
}

function normalizeRevenueCatPackageIdentifier(
  identifier: string,
): AnkyRevenueCatPackageId | null {
  const candidates = [
    identifier,
    identifier.replace(/^\$rc_/, ""),
    identifier.replace(/^\$rc_custom:/, ""),
    identifier.replace(/^\$rc_custom_/, ""),
  ];

  for (const candidate of candidates) {
    if (candidate === "starter" || candidate === "regular" || candidate === "sojourner") {
      return candidate;
    }
  }

  return null;
}

function toStorePackage(
  product: CreditProduct,
  revenueCatPackage: PurchasesPackage,
): AnkyCreditStorePackage {
  return {
    bonusCredits: product.bonusCredits,
    description: product.description,
    packageId: product.revenueCatPackageId,
    priceLabel: revenueCatPackage.product.priceString || product.fallbackPriceLabel,
    productId: revenueCatPackage.product.identifier,
    recommended: product.recommended,
    revenueCatPackage,
    title: product.title,
    totalCredits: product.totalCredits,
  };
}

function isRevenueCatPurchaseCancelled(error: unknown): boolean {
  if (typeof error !== "object" || error == null) {
    return false;
  }

  const userCancelled =
    "userCancelled" in error && (error as { userCancelled?: unknown }).userCancelled === true;
  const code = "code" in error ? String((error as { code?: unknown }).code) : "";
  const message =
    "message" in error ? String((error as { message?: unknown }).message).toLowerCase() : "";

  return userCancelled || code === "1" || message.includes("cancel");
}

function getRevenueCatErrorMessage(error: unknown): string {
  if (typeof error === "object" && error != null && "message" in error) {
    const message = String((error as { message?: unknown }).message);

    if (message.trim().length > 0) {
      return message;
    }
  }

  return "purchase failed.";
}
