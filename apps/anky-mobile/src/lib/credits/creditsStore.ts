import * as SecureStore from "expo-secure-store";

import { CREDIT_COSTS, ProcessingType } from "../api/types";

const DEV_CREDIT_BALANCE_KEY = "anky.devCredits.balance.v1";
export const DEV_STARTING_CREDITS = 8;

export async function getDevCreditBalance(): Promise<number> {
  const value = await SecureStore.getItemAsync(DEV_CREDIT_BALANCE_KEY);

  if (value == null) {
    await setDevCreditBalance(DEV_STARTING_CREDITS);
    return DEV_STARTING_CREDITS;
  }

  const parsed = Number(value);

  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    await setDevCreditBalance(DEV_STARTING_CREDITS);
    return DEV_STARTING_CREDITS;
  }

  return parsed;
}

export async function setDevCreditBalance(balance: number): Promise<void> {
  if (!Number.isSafeInteger(balance) || balance < 0) {
    throw new Error("Credit balance must be a non-negative integer.");
  }

  await SecureStore.setItemAsync(DEV_CREDIT_BALANCE_KEY, String(balance));
}

export async function spendDevCredits(processingType: ProcessingType): Promise<{
  creditsRemaining: number;
  creditsSpent: number;
}> {
  const creditsSpent = CREDIT_COSTS[processingType];
  const currentBalance = await getDevCreditBalance();

  if (currentBalance < creditsSpent) {
    throw new Error("Not enough credits. Credits pay for mirrors, not for truth.");
  }

  const creditsRemaining = currentBalance - creditsSpent;
  await setDevCreditBalance(creditsRemaining);

  return {
    creditsRemaining,
    creditsSpent,
  };
}

export async function resetDevCredits(): Promise<void> {
  await setDevCreditBalance(DEV_STARTING_CREDITS);
}
