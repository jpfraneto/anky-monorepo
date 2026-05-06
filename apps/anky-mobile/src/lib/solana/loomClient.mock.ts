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
const MS_PER_UTC_DAY = 86_400_000;

type MockLoomClientOptions = {
  now?: () => number;
  random?: () => number;
};

export function createMockLoomClient({
  now = Date.now,
  random = Math.random,
}: MockLoomClientOptions = {}): LoomClient {
  const looms = MOCK_LOOMS.map((loom) => ({ ...loom }));
  const sealedHashes = new Set<string>();
  const sealedUtcDays = new Set<number>();

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
      const currentUtcDay = Math.floor(timestamp / MS_PER_UTC_DAY);

      if (input.sessionUtcDay !== currentUtcDay) {
        throw new Error("Only an Anky from the current UTC day can be sealed.");
      }

      if (sealedUtcDays.has(input.sessionUtcDay)) {
        throw new Error("This writer has already sealed an Anky for this UTC day.");
      }

      if (sealedHashes.has(input.sessionHash)) {
        throw new Error("This writer has already sealed this session hash.");
      }

      const nonce = Math.floor(random() * 1_000_000_000)
        .toString(36)
        .padStart(6, "0");
      const txSignature = `mock_${timestamp.toString(36)}_${input.sessionHash.slice(0, 16)}_${nonce}`;
      const totalSeals = (loom.totalSeals ?? 0) + 1;

      sealedUtcDays.add(input.sessionUtcDay);
      sealedHashes.add(input.sessionHash);
      loom.totalSeals = totalSeals;
      loom.latestSessionHash = input.sessionHash;

      return {
        blockTime: Math.floor(timestamp / 1000),
        loomId: loom.id,
        sessionHash: input.sessionHash,
        slot: timestamp,
        txSignature,
        utcDay: input.sessionUtcDay,
        writer: MOCK_WRITER,
      };
    },
  };
}

export const mockLoomClient = createMockLoomClient();

export const getOwnedLooms = mockLoomClient.getOwnedLooms;
export const getSelectedLoom = mockLoomClient.getSelectedLoom;
export const sealAnky = mockLoomClient.sealAnky;
