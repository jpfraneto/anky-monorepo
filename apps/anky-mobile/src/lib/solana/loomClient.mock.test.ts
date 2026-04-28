import { describe, expect, it } from "vitest";

import { createMockLoomClient } from "./loomClient.mock";

describe("mock loom client", () => {
  it("seals a hash through an owned Anky Sojourn 9 Loom", async () => {
    const client = createMockLoomClient({
      now: () => 1700000000000,
      random: () => 0.123,
    });
    const [loom] = await client.getOwnedLooms();
    const sessionHash = "a".repeat(64);

    await expect(
      client.sealAnky({
        loomId: loom.id,
        sessionHash,
      }),
    ).resolves.toMatchObject({
      blockTime: 1700000000,
      loomId: loom.id,
      sessionHash,
      writer: loom.ownerWallet,
    });
  });

  it("rejects malformed hashes before sealing", async () => {
    const client = createMockLoomClient();
    const [loom] = await client.getOwnedLooms();

    await expect(
      client.sealAnky({
        loomId: loom.id,
        sessionHash: "not-a-hash",
      }),
    ).rejects.toThrow("sessionHash");
  });
});
