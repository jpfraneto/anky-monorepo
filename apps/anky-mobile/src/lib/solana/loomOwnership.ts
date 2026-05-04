import { getSelectedLoom, SelectedLoom } from "./loomStorage";

export type LoomOwnershipState =
  | {
      kind: "guest";
      selectedLoom?: SelectedLoom;
    }
  | {
      kind: "wallet_not_ready";
      selectedLoom?: SelectedLoom;
    }
  | {
      kind: "no_loom";
      wallet: string;
    }
  | {
      kind: "ready";
      selectedLoom: SelectedLoom;
      wallet: string;
    };

export async function resolveLocalLoomOwnershipState({
  authenticated,
  wallet,
}: {
  authenticated: boolean;
  wallet?: string;
}): Promise<LoomOwnershipState> {
  const selectedLoom = await getSelectedLoom();

  if (!authenticated) {
    return { kind: "guest", selectedLoom: selectedLoom ?? undefined };
  }

  if (wallet == null) {
    return { kind: "wallet_not_ready", selectedLoom: selectedLoom ?? undefined };
  }

  if (selectedLoom == null) {
    return { kind: "no_loom", wallet };
  }

  return {
    kind: "ready",
    selectedLoom,
    wallet,
  };
}
