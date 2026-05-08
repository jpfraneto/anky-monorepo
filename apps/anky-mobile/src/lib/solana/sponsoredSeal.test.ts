import { describe, expect, it, vi, beforeEach } from "vitest";

import type { AnkyApiClient } from "../api/ankyApi";
import { AnkyApiError } from "../api/ankyApi";
import { sealAnky } from "./sealAnky";
import { needsSolanaFunding, sealAnkyWithPayerPolicy } from "./sponsoredSeal";
import type { AnkySolanaWallet } from "./walletTypes";

vi.mock("./sealAnky", () => ({
  sealAnky: vi.fn(),
}));

const WRITER = "11111111111111111111111111111111";
const SPONSOR = "So11111111111111111111111111111111111111112";
const PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const CORE_COLLECTION = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const LOOM_ASSET = "4ENNjitn7223tyNAyzdhZ4QWo4iQD5j5DiM3fDz2wLS9";
const SESSION_HASH = "ab".repeat(32);
const UTC_DAY = 20_580;

describe("sealAnkyWithPayerPolicy", () => {
  beforeEach(() => {
    vi.mocked(sealAnky).mockReset();
  });

  it("uses the writer-paid path when the wallet has enough SOL", async () => {
    const api = mockApi();
    vi.mocked(sealAnky).mockResolvedValueOnce(mockSealResult({ payer: WRITER }));

    const result = await sealAnkyWithPayerPolicy({
      ...baseInput(),
      api,
      connection: mockConnection(6_000_000),
    });

    expect(result.payer).toBe(WRITER);
    expect(result.sponsored).toBe(false);
    expect(api.prepareMobileSeal).not.toHaveBeenCalled();
    const [sealInput] = vi.mocked(sealAnky).mock.calls[0];
    expect(sealInput.wallet.publicKey).toBe(WRITER);
    expect(sealInput).not.toHaveProperty("payer");
    expect(sealInput).not.toHaveProperty("preparedTransactionBase64");
  });

  it("asks the backend for a sponsored seal when the wallet lacks SOL", async () => {
    const api = mockApi({
      blockhash: "blockhash",
      estimatedLamports: 8_000_000,
      idempotencyKey: `seal:${WRITER}:${UTC_DAY}:${SESSION_HASH}`,
      lastValidBlockHeight: 123,
      payer: SPONSOR,
      sponsor: true,
      sponsorPayer: SPONSOR,
      transactionBase64: "prepared-tx",
    });
    vi.mocked(sealAnky).mockResolvedValueOnce(mockSealResult({ payer: SPONSOR }));

    const result = await sealAnkyWithPayerPolicy({
      ...baseInput(),
      api,
      connection: mockConnection(0),
    });

    expect(result.payer).toBe(SPONSOR);
    expect(result.sponsored).toBe(true);
    expect(api.prepareMobileSeal).toHaveBeenCalledWith({
      canonical: true,
      coreCollection: CORE_COLLECTION,
      loomAsset: LOOM_ASSET,
      sessionHash: SESSION_HASH,
      utcDay: UTC_DAY,
      wallet: WRITER,
    });
    expect(vi.mocked(sealAnky)).toHaveBeenCalledWith(
      expect.objectContaining({
        payer: SPONSOR,
        preparedBlockhash: {
          blockhash: "blockhash",
          lastValidBlockHeight: 123,
        },
        preparedTransactionBase64: "prepared-tx",
      }),
    );
  });

  it("falls back to sponsorship when a funded-looking wallet hits a Solana funding error", async () => {
    const api = mockApi({
      blockhash: "blockhash",
      estimatedLamports: 8_000_000,
      idempotencyKey: `seal:${WRITER}:${UTC_DAY}:${SESSION_HASH}`,
      lastValidBlockHeight: 123,
      payer: SPONSOR,
      sponsor: true,
      sponsorPayer: SPONSOR,
      transactionBase64: "prepared-tx",
    });
    vi.mocked(sealAnky)
      .mockRejectedValueOnce(new Error("attempted to debit an account but found no record of a prior credit"))
      .mockResolvedValueOnce(mockSealResult({ payer: SPONSOR }));

    const result = await sealAnkyWithPayerPolicy({
      ...baseInput(),
      api,
      connection: mockConnection(6_000_000),
    });

    expect(result.sponsored).toBe(true);
    expect(api.prepareMobileSeal).toHaveBeenCalledTimes(1);
    expect(vi.mocked(sealAnky)).toHaveBeenCalledTimes(2);
  });

  it("does not ask the backend for sponsorship after non-funding seal errors", async () => {
    const api = mockApi();
    vi.mocked(sealAnky).mockRejectedValueOnce(new Error("wallet rejected signing"));

    await expect(
      sealAnkyWithPayerPolicy({
        ...baseInput(),
        api,
        connection: mockConnection(6_000_000),
      }),
    ).rejects.toThrow("wallet rejected signing");

    expect(api.prepareMobileSeal).not.toHaveBeenCalled();
  });

  it("maps unavailable sponsorship responses to a gas-friendly error", async () => {
    const api = mockApi();
    api.prepareMobileSeal.mockRejectedValueOnce(
      new AnkyApiError({
        path: "/api/mobile/seals/prepare",
        status: 503,
      }),
    );

    await expect(
      sealAnkyWithPayerPolicy({
        ...baseInput(),
        api,
        connection: mockConnection(0),
      }),
    ).rejects.toThrow("this wallet needs SOL for gas and seal sponsorship is not available right now.");
  });
});

describe("needsSolanaFunding", () => {
  it("matches common Solana funding failures", () => {
    expect(needsSolanaFunding(new Error("attempted to debit an account but found no record of a prior credit"))).toBe(true);
    expect(needsSolanaFunding(new Error("insufficient lamports for rent"))).toBe(true);
    expect(needsSolanaFunding(new Error("wallet rejected signing"))).toBe(false);
  });
});

function baseInput() {
  return {
    api: null,
    canonical: true,
    connection: mockConnection(6_000_000),
    coreCollection: CORE_COLLECTION,
    loomAsset: LOOM_ASSET,
    network: "devnet" as const,
    programId: PROGRAM_ID,
    sessionHashHex: SESSION_HASH,
    sessionUtcDay: UTC_DAY,
    wallet: { publicKey: WRITER } as AnkySolanaWallet,
  };
}

function mockConnection(lamports: number) {
  return {
    getBalance: vi.fn(async () => lamports),
  } as never;
}

function mockApi(response?: Awaited<ReturnType<AnkyApiClient["prepareMobileSeal"]>>) {
  return {
    prepareMobileSeal: vi.fn(async () => {
      if (response == null) {
        throw new Error("unexpected prepareMobileSeal call");
      }

      return response;
    }),
  } as unknown as AnkyApiClient & {
    prepareMobileSeal: ReturnType<typeof vi.fn>;
  };
}

function mockSealResult({ payer }: { payer: string }) {
  return {
    created_at: "2026-05-07T00:00:00.000Z",
    loom_asset: LOOM_ASSET,
    network: "devnet" as const,
    payer,
    session_hash: SESSION_HASH,
    signature: "mock-signature",
    sponsored: payer !== WRITER,
    status: "confirmed" as const,
    utc_day: UTC_DAY,
    version: 1 as const,
    writer: WRITER,
  };
}
