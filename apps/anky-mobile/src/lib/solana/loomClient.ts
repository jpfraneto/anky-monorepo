// Swap this module to a real Solana adapter when devnet/mainnet sealing is implemented.
// Screens import from here so they do not know whether the loom client is mocked or real.
export { getOwnedLooms, getSelectedLoom, sealAnky } from "./loomClient.mock";
export type { LoomClient, SealAnkyInput, SealAnkyResult } from "./types";
