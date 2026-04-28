import {
  assertSessionHash,
  Loom,
  LoomClient,
  SealAnkyInput,
  SealAnkyResult,
} from "./types";

const MOCK_WRITER = "MockWriter1111111111111111111111111111111111";

const MOCK_LOOMS: Loom[] = [
  {
    id: "mock-loom-sojourn-9-777",
    name: "Anky Sojourn 9 Loom #777",
    ownerWallet: MOCK_WRITER,
    totalSeals: 0,
  },
];

type MockLoomClientOptions = {
  now?: () => number;
  random?: () => number;
};

export function createMockLoomClient({
  now = Date.now,
  random = Math.random,
}: MockLoomClientOptions = {}): LoomClient {
  const looms = MOCK_LOOMS.map((loom) => ({ ...loom }));

  return {
    async getOwnedLooms() {
      return looms.map((loom) => ({ ...loom }));
    },

    async getSelectedLoom() {
      return looms[0] == null ? null : { ...looms[0] };
    },

    async sealAnky(input: SealAnkyInput): Promise<SealAnkyResult> {
      assertSessionHash(input.sessionHash);

      const loom = looms.find((candidate) => candidate.id === input.loomId);

      if (loom == null || loom.ownerWallet !== MOCK_WRITER) {
        throw new Error("A valid Anky Sojourn 9 Loom is required to seal this hash.");
      }

      const timestamp = now();
      const nonce = Math.floor(random() * 1_000_000_000)
        .toString(36)
        .padStart(6, "0");
      const txSignature = `mock_${timestamp.toString(36)}_${input.sessionHash.slice(0, 16)}_${nonce}`;
      const totalSeals = (loom.totalSeals ?? 0) + 1;

      loom.totalSeals = totalSeals;
      loom.latestSessionHash = input.sessionHash;

      return {
        blockTime: Math.floor(timestamp / 1000),
        loomId: loom.id,
        sessionHash: input.sessionHash,
        slot: timestamp,
        txSignature,
        writer: MOCK_WRITER,
      };
    },
  };
}

export const mockLoomClient = createMockLoomClient();

export const getOwnedLooms = mockLoomClient.getOwnedLooms;
export const getSelectedLoom = mockLoomClient.getSelectedLoom;
export const sealAnky = mockLoomClient.sealAnky;
