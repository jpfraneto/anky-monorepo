import { createMockLoomClient } from "./loomClient.mock";
import { createProgramLoomClient } from "./loomClient.program";
import { getSojourn9ProgramConfig } from "./sojourn9Program";

const loomClient =
  getSojourn9ProgramConfig().sealAdapterMode === "program"
    ? createProgramLoomClient()
    : createMockLoomClient();

export const getOwnedLooms = loomClient.getOwnedLooms;
export const getSelectedLoom = loomClient.getSelectedLoom;
export const sealAnky = loomClient.sealAnky;

export type { LoomClient, SealAnkyInput, SealAnkyResult } from "./types";
