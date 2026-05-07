import { describe, expect, it, vi } from "vitest";

import { AnkyApiClient } from "./ankyApi";

describe("AnkyApiClient", () => {
  it("looks up the mobile seal score through the backend score route", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          finalizedOnly: true,
          formula:
            "score = unique_seal_days + (2 * verified_seal_days) + streak_bonus",
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

  it("looks up mobile seal points history", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          entries: [
            {
              loomId: "loom one",
              proofHash: "a".repeat(64),
              proofPoints: 2,
              proofStatus: "finalized",
              proofTxSignature: "proof tx",
              provedAt: "2026-05-06T00:02:00Z",
              sealPoints: 1,
              sealSignature: "seal tx",
              sealStatus: "finalized",
              sealedAt: "2026-05-06T00:01:00Z",
              sessionHash: "b".repeat(64),
              totalPoints: 3,
              utcDay: 20579,
            },
          ],
          formula: "score = unique_seal_days + (2 * verified_seal_days) + streak_bonus",
          network: "devnet",
          score: 3,
          streakBonus: 0,
          uniqueSealDays: 1,
          verifiedSealDays: 1,
          wallet: "wallet one",
        }),
        {
          headers: { "content-type": "application/json" },
          status: 200,
        },
      ),
    );
    const client = new AnkyApiClient({
      baseUrl: "https://anky.example/",
      fetchImpl: fetchMock as unknown as typeof fetch,
    });

    const history = await client.lookupMobileSealPoints("wallet one");

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [[url]] = fetchMock.mock.calls as unknown as Array<[string, RequestInit]>;
    expect(url).toBe("https://anky.example/api/mobile/seals/points?wallet=wallet+one");
    expect(history.score).toBe(3);
    expect(history.entries[0]).toMatchObject({
      proofPoints: 2,
      proofStatus: "finalized",
      sealPoints: 1,
      totalPoints: 3,
    });
  });

  it("requests a mobile seal proof and parses the accepted job response", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          jobId: "job-1",
          pollAfterMs: 4000,
          sessionHash: "b".repeat(64),
          status: "proving",
          utcDay: 20579,
          wallet: "wallet one",
        }),
        {
          headers: { "content-type": "application/json" },
          status: 202,
        },
      ),
    );
    const client = new AnkyApiClient({
      baseUrl: "https://anky.example/",
      fetchImpl: fetchMock as unknown as typeof fetch,
    });

    const response = await client.requestMobileSealProof({
      rawAnky: "1710000000000 a\n8000",
      sealSignature: "seal tx",
      sessionHash: "b".repeat(64),
      utcDay: 20579,
      wallet: "wallet one",
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [[url, init]] = fetchMock.mock.calls as unknown as Array<[string, RequestInit]>;
    expect(url).toBe("https://anky.example/api/mobile/seals/prove");
    expect(init.method).toBe("POST");
    expect(response).toMatchObject({
      jobId: "job-1",
      status: "proving",
    });
  });

  it("parses already-finalized mobile seal proof responses", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          proofHash: "a".repeat(64),
          proofTxSignature: "proof tx",
          sessionHash: "b".repeat(64),
          status: "finalized",
          utcDay: 20579,
          wallet: "wallet one",
        }),
        {
          headers: { "content-type": "application/json" },
          status: 200,
        },
      ),
    );
    const client = new AnkyApiClient({
      baseUrl: "https://anky.example/",
      fetchImpl: fetchMock as unknown as typeof fetch,
    });

    const response = await client.requestMobileSealProof({
      rawAnky: "1710000000000 a\n8000",
      sealSignature: "seal tx",
      sessionHash: "b".repeat(64),
      utcDay: 20579,
      wallet: "wallet one",
    });

    expect(response).toMatchObject({
      proofHash: "a".repeat(64),
      status: "finalized",
    });
  });

  it("parses on-chain-syncing mobile seal proof responses", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          message: "verified on-chain · syncing",
          pollAfterMs: 4000,
          proofHash: "a".repeat(64),
          sessionHash: "b".repeat(64),
          status: "backfill_required",
          utcDay: 20579,
          verifiedSeal: "verified pda",
          wallet: "wallet one",
        }),
        {
          headers: { "content-type": "application/json" },
          status: 202,
        },
      ),
    );
    const client = new AnkyApiClient({
      baseUrl: "https://anky.example/",
      fetchImpl: fetchMock as unknown as typeof fetch,
    });

    const response = await client.requestMobileSealProof({
      rawAnky: "1710000000000 a\n8000",
      sealSignature: "seal tx",
      sessionHash: "b".repeat(64),
      utcDay: 20579,
      wallet: "wallet one",
    });

    expect(response).toMatchObject({
      message: "verified on-chain · syncing",
      proofHash: "a".repeat(64),
      status: "backfill_required",
    });
  });

  it("parses unavailable mobile seal proof responses from HTTP 503", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          message: "proof prover is not configured",
          status: "unavailable",
        }),
        {
          headers: { "content-type": "application/json" },
          status: 503,
        },
      ),
    );
    const client = new AnkyApiClient({
      baseUrl: "https://anky.example/",
      fetchImpl: fetchMock as unknown as typeof fetch,
    });

    const response = await client.requestMobileSealProof({
      rawAnky: "1710000000000 a\n8000",
      sealSignature: "seal tx",
      sessionHash: "b".repeat(64),
      utcDay: 20579,
      wallet: "wallet one",
    });

    expect(response).toEqual({
      message: "proof prover is not configured",
      status: "unavailable",
    });
  });

  it("looks up mobile proof job status", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          jobId: "job-1",
          proofHash: "a".repeat(64),
          proofTxSignature: "proof tx",
          sessionHash: "b".repeat(64),
          status: "finalized",
          utcDay: 20579,
          wallet: "wallet one",
        }),
        {
          headers: { "content-type": "application/json" },
          status: 200,
        },
      ),
    );
    const client = new AnkyApiClient({
      baseUrl: "https://anky.example/",
      fetchImpl: fetchMock as unknown as typeof fetch,
    });

    const job = await client.getMobileSealProofJob("job 1");

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [[url]] = fetchMock.mock.calls as unknown as Array<[string, RequestInit]>;
    expect(url).toBe("https://anky.example/api/mobile/seals/prove/job%201");
    expect(job.status).toBe("finalized");
  });

  it("looks up syncing mobile proof job status", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          jobId: "job-1",
          message: "verified on-chain · syncing",
          proofHash: "a".repeat(64),
          sessionHash: "b".repeat(64),
          status: "syncing",
          utcDay: 20579,
          wallet: "wallet one",
        }),
        {
          headers: { "content-type": "application/json" },
          status: 200,
        },
      ),
    );
    const client = new AnkyApiClient({
      baseUrl: "https://anky.example/",
      fetchImpl: fetchMock as unknown as typeof fetch,
    });

    const job = await client.getMobileSealProofJob("job 1");

    expect(job.status).toBe("syncing");
    expect(job.message).toBe("verified on-chain · syncing");
  });
});
