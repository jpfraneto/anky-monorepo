import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import type { Transaction } from "@solana/web3.js";
import {
  type UseDeeplinkWalletConnector,
  useBackpackDeeplinkWalletConnector,
  usePhantomDeeplinkWalletConnector,
} from "@privy-io/expo/connectors";

import { ANKY_APP_URL, PRIVY_WALLET_REDIRECT_URI } from "../auth/privyConfig";

export type ExternalWalletProviderName = "phantom" | "backpack";

export type ConnectedExternalSolanaWallet = {
  address: string;
  disconnect: () => Promise<void>;
  isConnected: true;
  label: string;
  provider: ExternalWalletProviderName;
  signMessage: (message: string) => Promise<{ signature: string }>;
  signTransaction: (transaction: Transaction) => Promise<ExternalSignTransactionResponse>;
};

export type ExternalSignTransactionResponse = {
  signature?: string;
  transaction?: string;
};

type ExternalWalletMap = Record<ExternalWalletProviderName, ConnectedExternalSolanaWallet | null>;

type ExternalSolanaWalletContextValue = {
  activeProvider: ExternalWalletProviderName | null;
  activeWallet: ConnectedExternalSolanaWallet | null;
  connectWallet: (provider: ExternalWalletProviderName) => Promise<void>;
  disconnectWallet: (provider: ExternalWalletProviderName) => Promise<void>;
  setActiveProvider: (provider: ExternalWalletProviderName | null) => void;
  wallets: ExternalWalletMap;
};

const ExternalSolanaWalletContext = createContext<ExternalSolanaWalletContextValue | null>(null);

type Props = {
  children: ReactNode;
};

export function ExternalSolanaWalletProvider({ children }: Props) {
  const phantom = usePhantomDeeplinkWalletConnector({
    appUrl: ANKY_APP_URL,
    redirectUri: PRIVY_WALLET_REDIRECT_URI,
  });
  const backpack = useBackpackDeeplinkWalletConnector({
    appUrl: ANKY_APP_URL,
    redirectUri: PRIVY_WALLET_REDIRECT_URI,
  });
  const [activeProvider, setActiveProvider] = useState<ExternalWalletProviderName | null>(null);

  const wallets = useMemo<ExternalWalletMap>(
    () => ({
      backpack: toConnectedWallet("backpack", "Backpack", backpack),
      phantom: toConnectedWallet("phantom", "Phantom", phantom),
    }),
    [backpack, phantom],
  );

  useEffect(() => {
    setActiveProvider((current) => {
      if (current === "phantom" && wallets.phantom != null) {
        return current;
      }

      if (current === "backpack" && wallets.backpack != null) {
        return current;
      }

      if (current != null) {
        return current;
      }

      if (wallets.phantom != null) {
        return "phantom";
      }

      if (wallets.backpack != null) {
        return "backpack";
      }

      return null;
    });
  }, [wallets.backpack, wallets.phantom]);

  const connectWallet = useCallback(
    async (provider: ExternalWalletProviderName) => {
      setActiveProvider(provider);

      try {
        if (provider === "phantom") {
          await phantom.connect();
        } else {
          await backpack.connect();
        }
      } catch (error) {
        setActiveProvider((current) => (current === provider ? null : current));
        throw error;
      }
    },
    [backpack, phantom],
  );

  const disconnectWallet = useCallback(
    async (provider: ExternalWalletProviderName) => {
      if (provider === "phantom") {
        await phantom.disconnect();
      } else {
        await backpack.disconnect();
      }

      setActiveProvider((current) => {
        if (current !== provider) {
          return current;
        }

        return provider === "phantom" && wallets.backpack != null
          ? "backpack"
          : provider === "backpack" && wallets.phantom != null
            ? "phantom"
            : null;
      });
    },
    [backpack, phantom, wallets.backpack, wallets.phantom],
  );

  const activeWallet =
    activeProvider == null ? null : (wallets[activeProvider] ?? null);

  const value = useMemo<ExternalSolanaWalletContextValue>(
    () => ({
      activeProvider,
      activeWallet,
      connectWallet,
      disconnectWallet,
      setActiveProvider,
      wallets,
    }),
    [activeProvider, activeWallet, connectWallet, disconnectWallet, wallets],
  );

  return (
    <ExternalSolanaWalletContext.Provider value={value}>
      {children}
    </ExternalSolanaWalletContext.Provider>
  );
}

export function useExternalSolanaWallet(): ExternalSolanaWalletContextValue {
  const context = useContext(ExternalSolanaWalletContext);

  if (context == null) {
    throw new Error("useExternalSolanaWallet must be used inside ExternalSolanaWalletProvider.");
  }

  return context;
}

function toConnectedWallet(
  provider: ExternalWalletProviderName,
  label: string,
  connector: UseDeeplinkWalletConnector,
): ConnectedExternalSolanaWallet | null {
  if (!connector.isConnected || connector.address == null) {
    return null;
  }

  return {
    address: connector.address,
    disconnect: connector.disconnect,
    isConnected: true,
    label,
    provider,
    signMessage: connector.signMessage,
    signTransaction: connector.signTransaction as unknown as (
      transaction: Transaction,
    ) => Promise<ExternalSignTransactionResponse>,
  };
}
