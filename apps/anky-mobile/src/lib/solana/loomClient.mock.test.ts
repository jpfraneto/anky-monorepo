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
    const sessionUtcDay = Math.floor(1700000000000 / 86_400_000);

    await expect(
      client.sealAnky({
        loomId: loom.id,
        sessionHash,
        sessionUtcDay,
      }),
    ).resolves.toMatchObject({
      blockTime: 1700000000,
      loomId: loom.id,
      sessionHash,
      utcDay: sessionUtcDay,
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
        sessionUtcDay: Math.floor(Date.now() / 86_400_000),
      }),
    ).rejects.toThrow("sessionHash");
  });

  it("rejects duplicate daily seals", async () => {
    const client = createMockLoomClient({
      now: () => 1700000000000,
      random: () => 0.123,
    });
    const [loom] = await client.getOwnedLooms();
    const sessionUtcDay = Math.floor(1700000000000 / 86_400_000);

    await client.sealAnky({
      loomId: loom.id,
      sessionHash: "a".repeat(64),
      sessionUtcDay,
    });

    await expect(
      client.sealAnky({
        loomId: loom.id,
        sessionHash: "b".repeat(64),
        sessionUtcDay,
      }),
    ).rejects.toThrow("UTC day");
  });
});
