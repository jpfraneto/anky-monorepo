import { ReactNode } from "react";
import { PrivyProvider as ExpoPrivyProvider } from "@privy-io/expo";

import { PRIVY_APP_ID, PRIVY_CLIENT_ID } from "../auth/privyConfig";
import { ExternalSolanaWalletProvider } from "./ExternalSolanaWalletProvider";

type Props = {
  children: ReactNode;
};

export function AnkyPrivyProvider({ children }: Props) {
  return (
    <ExpoPrivyProvider appId={PRIVY_APP_ID} clientId={PRIVY_CLIENT_ID}>
      <ExternalSolanaWalletProvider>{children}</ExternalSolanaWalletProvider>
    </ExpoPrivyProvider>
  );
}
