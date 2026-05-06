import { useCallback } from "react";
import { useEmbeddedSolanaWallet, usePrivy } from "@privy-io/expo";
import { Transaction } from "@solana/web3.js";
import { base58 } from "@scure/base";
import { Buffer } from "buffer";
import { AppState } from "react-native";

import type { AnkySolanaWallet } from "../solana/walletTypes";
import { useExternalSolanaWallet } from "./ExternalSolanaWalletProvider";

type PrivySolanaProvider = {
  request<T extends Transaction>(request: {
    method: "signTransaction";
    params: {
      transaction: T;
    };
  }): Promise<{ signedTransaction: T }>;
};

type ConnectedPrivySolanaWallet = {
  address: string;
  getProvider: () => Promise<unknown>;
};

const EXTERNAL_WALLET_RETURN_GRACE_MS = 2400;
const EXTERNAL_WALLET_SIGN_TIMEOUT_MS = 90000;

export type AnkyPrivyWalletState = {
  authenticated: boolean;
  canCreateEmbeddedWallet: boolean;
  createWallet: () => Promise<void>;
  embeddedPublicKey?: string;
  externalPublicKey?: string;
  getWallet: () => Promise<AnkySolanaWallet>;
  hasEmbeddedWallet: boolean;
  hasExternalWallet: boolean;
  hasWallet: boolean;
  publicKey?: string;
  ready: boolean;
  status: string;
  walletKind?: "embedded" | "external";
  walletLabel?: string;
};

export function useAnkyPrivyWallet(): AnkyPrivyWalletState {
  const { isReady, user } = usePrivy();
  const externalWallet = useExternalSolanaWallet();
  const solanaWallet = useEmbeddedSolanaWallet();
  const wallets = getConnectedWallets(solanaWallet);
  const embeddedWallet = wallets[0] ?? null;
  const activeExternalWallet = externalWallet.activeWallet;
  const primaryWallet = activeExternalWallet ?? embeddedWallet;
  const publicKey =
    primaryWallet?.address ??
    ("publicKey" in solanaWallet && typeof solanaWallet.publicKey === "string"
      ? solanaWallet.publicKey
      : undefined);

  const createWallet = useCallback(async () => {
    if (user == null) {
      throw new Error("Connect with Apple, Google, or email before creating an embedded wallet.");
    }

    if (!("create" in solanaWallet) || typeof solanaWallet.create !== "function") {
      throw new Error("Embedded Solana wallet creation is not available in this session.");
    }

    await solanaWallet.create();
  }, [solanaWallet, user]);

  const getWallet = useCallback(async (): Promise<AnkySolanaWallet> => {
    const external = externalWallet.activeWallet;

    if (external != null) {
      return {
        publicKey: external.address,
        async signTransaction(transaction: Transaction) {
          const response = await waitForExternalWalletResult(
            Promise.resolve().then(() => external.signTransaction(transaction)),
            external.label,
          );

          if (response.transaction != null) {
            return Transaction.from(Buffer.from(base58.decode(response.transaction)));
          }

          throw new Error(
            `${external.label.toLowerCase()} did not return a signed transaction. swipe again or update the wallet app.`,
          );
        },
      };
    }

    if (user == null) {
      throw new Error("Connect a Solana wallet before minting or sealing.");
    }

    const wallet = getConnectedWallets(solanaWallet)[0] ?? null;

    if (wallet == null) {
      throw new Error("Create an embedded Solana wallet before minting or sealing.");
    }

    const provider = (await wallet.getProvider()) as PrivySolanaProvider;

    return {
      publicKey: wallet.address,
      async signTransaction(transaction: Transaction) {
        const { signedTransaction } = await provider.request({
          method: "signTransaction",
          params: { transaction },
        });

        return signedTransaction;
      },
    };
  }, [externalWallet.activeWallet, solanaWallet, user]);

  return {
    authenticated: user != null,
    canCreateEmbeddedWallet: user != null,
    createWallet,
    embeddedPublicKey: embeddedWallet?.address,
    externalPublicKey: activeExternalWallet?.address,
    getWallet,
    hasEmbeddedWallet: embeddedWallet != null,
    hasExternalWallet: activeExternalWallet != null,
    hasWallet: primaryWallet != null,
    publicKey,
    ready: isReady || activeExternalWallet != null,
    status: activeExternalWallet == null ? solanaWallet.status : "connected",
    walletKind: activeExternalWallet == null && embeddedWallet != null ? "embedded" : primaryWallet == null ? undefined : "external",
    walletLabel: activeExternalWallet?.label ?? (embeddedWallet == null ? undefined : "Embedded"),
  };
}

async function waitForExternalWalletResult<T>(request: Promise<T>, walletLabel: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    let settled = false;
    let sawWalletHandoff = AppState.currentState !== "active";
    let returnTimer: ReturnType<typeof setTimeout> | null = null;
    let timeoutTimer: ReturnType<typeof setTimeout> | null = null;

    function cleanup() {
      if (returnTimer != null) {
        clearTimeout(returnTimer);
      }

      if (timeoutTimer != null) {
        clearTimeout(timeoutTimer);
      }

      subscription.remove();
    }

    function settleWithError(error: Error) {
      if (settled) {
        return;
      }

      settled = true;
      cleanup();
      reject(error);
    }

    const subscription = AppState.addEventListener("change", (state) => {
      if (state === "background" || state === "inactive") {
        sawWalletHandoff = true;
        return;
      }

      if (state !== "active" || !sawWalletHandoff || settled) {
        return;
      }

      if (returnTimer != null) {
        clearTimeout(returnTimer);
      }

      returnTimer = setTimeout(() => {
        settleWithError(
          new Error(`${walletLabel.toLowerCase()} did not approve the seal. swipe again when ready.`),
        );
      }, EXTERNAL_WALLET_RETURN_GRACE_MS);
    });

    timeoutTimer = setTimeout(() => {
      settleWithError(
        new Error(`${walletLabel.toLowerCase()} did not respond. swipe again when ready.`),
      );
    }, EXTERNAL_WALLET_SIGN_TIMEOUT_MS);

    request.then(
      (value) => {
        if (settled) {
          return;
        }

        settled = true;
        cleanup();
        resolve(value);
      },
      (error: unknown) => {
        settleWithError(normalizeExternalWalletError(error, walletLabel));
      },
    );
  });
}

function normalizeExternalWalletError(error: unknown, walletLabel: string): Error {
  const fallback = `${walletLabel.toLowerCase()} did not approve the seal. swipe again when ready.`;

  if (!(error instanceof Error)) {
    return new Error(fallback);
  }

  if (/reject|cancel|declin|denied|user/i.test(error.message)) {
    return new Error(fallback);
  }

  return error;
}

function getConnectedWallets(value: unknown): ConnectedPrivySolanaWallet[] {
  if (typeof value !== "object" || value == null || !("wallets" in value)) {
    return [];
  }

  const wallets = (value as { wallets?: unknown }).wallets;

  return Array.isArray(wallets) ? (wallets as ConnectedPrivySolanaWallet[]) : [];
}
