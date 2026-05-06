export type AnkyRevenueCatPackageId = "regular" | "sojourner" | "starter";

export type CreditProduct = {
  androidProductId: string;
  baseCredits: number;
  bonusCredits: number;
  description: string;
  fallbackPriceLabel: string;
  id: string;
  iosProductId: string;
  kind: "consumable";
  recommended?: boolean;
  revenueCatPackageId: AnkyRevenueCatPackageId;
  title: string;
  totalCredits: number;
};

export const CREDIT_PRODUCTS: CreditProduct[] = [
  {
    androidProductId: "credits_22",
    baseCredits: 22,
    bonusCredits: 0,
    description: "try the mirror.",
    fallbackPriceLabel: "$2.22",
    id: "credits_22",
    iosProductId: "inc.anky.credits.22",
    kind: "consumable",
    revenueCatPackageId: "starter",
    title: "22 credits",
    totalCredits: 22,
  },
  {
    androidProductId: "credits_88_bonus_11",
    baseCredits: 88,
    bonusCredits: 11,
    description: "best for regular writing.",
    fallbackPriceLabel: "$8.88",
    id: "credits_88_bonus_11",
    iosProductId: "inc.anky.credits.88_bonus_11",
    kind: "consumable",
    recommended: true,
    revenueCatPackageId: "regular",
    title: "88 + 11 bonus",
    totalCredits: 99,
  },
  {
    androidProductId: "credits_333_bonus_88",
    baseCredits: 333,
    bonusCredits: 88,
    description: "for the full sojourn.",
    fallbackPriceLabel: "$33.33",
    id: "credits_333_bonus_88",
    iosProductId: "inc.anky.credits.333_bonus_88",
    kind: "consumable",
    revenueCatPackageId: "sojourner",
    title: "333 + 88 bonus",
    totalCredits: 421,
  },
];
