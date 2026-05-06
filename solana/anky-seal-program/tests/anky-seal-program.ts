import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import assert from "assert";
import crypto from "crypto";

const MPL_CORE_PROGRAM_ID = new PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d",
);

// Matches OFFICIAL_COLLECTION in programs/anky-seal-program/src/lib.rs.
const OFFICIAL_COLLECTION_DEVNET = new PublicKey(
  "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
);
const CORE_INTEGRATION_LOOM_ASSET_ENV = "ANKY_CORE_INTEGRATION_LOOM_ASSET";
const CORE_INTEGRATION_COLLECTION_ENV = "ANKY_CORE_INTEGRATION_COLLECTION";
const ALLOW_MAINNET_CORE_INTEGRATION_ENV =
  "ANKY_ALLOW_MAINNET_CORE_INTEGRATION_TEST";

type FetchedLoomState = {
  loomAsset: PublicKey;
  totalSeals: anchor.BN;
  latestSessionHash: number[] | Uint8Array;
  rollingRoot: number[] | Uint8Array;
};

type LoomStateAccountClient = {
  fetch: (address: PublicKey) => Promise<FetchedLoomState>;
};

describe("anky-seal-program", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = anchor.workspace.AnkySealProgram as Program;
  const loomStateAccount = (program.account as unknown as {
    loomState: LoomStateAccountClient;
  }).loomState;

  it("seals one current-day .anky hash and updates LoomState lineage with an owned real Core asset", async function () {
    this.timeout(120_000);

    const loomAssetValue = process.env[CORE_INTEGRATION_LOOM_ASSET_ENV];
    if (loomAssetValue == null || loomAssetValue.trim() === "") {
      this.skip();
      return;
    }

    if (isMainnetEndpoint(provider.connection.rpcEndpoint)) {
      assert.strictEqual(
        process.env[ALLOW_MAINNET_CORE_INTEGRATION_ENV],
        "true",
        `refusing mainnet Core integration test without ${ALLOW_MAINNET_CORE_INTEGRATION_ENV}=true`,
      );
    }

    const writer = provider.wallet.publicKey;
    const loomAsset = new PublicKey(loomAssetValue.trim());
    const loomCollection = new PublicKey(
      process.env[CORE_INTEGRATION_COLLECTION_ENV]?.trim() ||
        OFFICIAL_COLLECTION_DEVNET.toBase58(),
    );

    const [loomState] = PublicKey.findProgramAddressSync(
      [Buffer.from("loom_state"), loomAsset.toBuffer()],
      program.programId,
    );

    const firstHash = crypto.randomBytes(32);
    const utcDay = Math.floor(Date.now() / 86_400_000);
    const utcDayBytes = Buffer.alloc(8);
    utcDayBytes.writeBigInt64LE(BigInt(utcDay));
    const [dailySeal] = PublicKey.findProgramAddressSync(
      [Buffer.from("daily_seal"), writer.toBuffer(), utcDayBytes],
      program.programId,
    );
    const [hashSeal] = PublicKey.findProgramAddressSync(
      [Buffer.from("hash_seal"), writer.toBuffer(), Buffer.from(firstHash)],
      program.programId,
    );

    await program.methods
      .sealAnky(Array.from(firstHash), new anchor.BN(utcDay))
      .accounts({
        writer,
        loomAsset,
        loomCollection,
        dailySeal,
        hashSeal,
      } as never)
      .rpc();

    const afterFirst = await loomStateAccount.fetch(loomState);
    assert.strictEqual(afterFirst.loomAsset.toBase58(), loomAsset.toBase58());
    assert.strictEqual(afterFirst.totalSeals.toNumber(), 1);
    assert.deepStrictEqual(
      Array.from(afterFirst.latestSessionHash),
      Array.from(firstHash),
    );
    assert.notDeepStrictEqual(Array.from(afterFirst.rollingRoot), new Array(32).fill(0));
  });
});

function isMainnetEndpoint(endpoint: string): boolean {
  return endpoint.toLowerCase().includes("mainnet");
}
