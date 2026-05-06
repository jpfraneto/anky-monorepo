import { describe, expect, it, vi } from "vitest";

import { AnkyApiClient } from "./ankyApi";

describe("AnkyApiClient", () => {
  it("looks up the mobile seal score through the backend score route", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          finalizedOnly: true,
          formula:
            "score = unique_seal_days + verified_days + 2 * floor(each_consecutive_day_run / 7)",
          network: "devnet",
          proofVerifierAuthority: "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP",
          score: 5,
          sealedDays: [21000, 21001],
          streakBonus: 1,
          uniqueSealDays: 2,
          verifiedDays: [21000],
          verifiedSealDays: 1,
          wallet: "wallet one",
        }),
        {
          headers: { "content-type": "application/json" },
          status: 200,
        },
      ),
    );
    const fetchImpl = fetchMock as unknown as typeof fetch;
    const client = new AnkyApiClient({
      baseUrl: "https://anky.example/",
      fetchImpl,
    });

    const score = await client.lookupMobileSealScore("wallet one");

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [[url, init]] = fetchMock.mock.calls as unknown as Array<[string, RequestInit]>;
    expect(url).toBe("https://anky.example/api/mobile/seals/score?wallet=wallet+one");
    expect(init).toBeDefined();
    expect("body" in init).toBe(false);
    expect("method" in init).toBe(false);
    expect(score).toMatchObject({
      finalizedOnly: true,
      network: "devnet",
      score: 5,
      uniqueSealDays: 2,
      verifiedSealDays: 1,
      wallet: "wallet one",
    });
  });
});
