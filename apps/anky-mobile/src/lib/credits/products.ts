import { getPublicEnv } from "../config/env";

export type CreditProductKind = "consumable" | "subscription";

export type CreditProduct = {
  credits?: number;
  description: string;
  id: string;
  kind: CreditProductKind;
  priceLabel: string;
  title: string;
};

export const CREDIT_PRODUCTS: CreditProduct[] = [
  {
    credits: 8,
    description: "a small bundle for a few reflections.",
    id: getPublicEnv("EXPO_PUBLIC_IAP_CREDITS_8_ID") ?? "credits_8",
    kind: "consumable",
    priceLabel: "$2.99",
    title: "8 credits",
  },
  {
    credits: 24,
    description: "for a deeper stretch of reflections.",
    id: getPublicEnv("EXPO_PUBLIC_IAP_CREDITS_24_ID") ?? "credits_24",
    kind: "consumable",
    priceLabel: "$6.99",
    title: "24 credits",
  },
  {
    credits: 88,
    description: "for a long season of mirrors.",
    id: getPublicEnv("EXPO_PUBLIC_IAP_CREDITS_88_ID") ?? "credits_88",
    kind: "consumable",
    priceLabel: "$19.99",
    title: "88 credits",
  },
  {
    description: "8 credits every day.",
    id:
      getPublicEnv("EXPO_PUBLIC_IAP_PREMIUM_MONTHLY_ID") ??
      "premium_monthly_8_per_day",
    kind: "subscription",
    priceLabel: "$8 / month",
    title: "premium",
  },
];
