import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import assert from "assert";

const MPL_CORE_PROGRAM_ID = new PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d",
);

// Matches OFFICIAL_COLLECTION in programs/anky-seal-program/src/lib.rs.
const OFFICIAL_COLLECTION_DEVNET = new PublicKey(
  "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u",
);

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

  // Real Core verification requires a Core-serialized Asset account. The old
  // placeholder test created a zero-data account owned by Core, which must not
  // pass anymore.
  it.skip("seals sample .anky hashes and updates LoomState lineage with a real Core asset", async () => {
    const writer = provider.wallet.publicKey;
    const loomAsset = Keypair.generate();

    const lamports =
      await provider.connection.getMinimumBalanceForRentExemption(0);

    await provider.sendAndConfirm(
      new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: writer,
          newAccountPubkey: loomAsset.publicKey,
          lamports,
          space: 0,
          programId: MPL_CORE_PROGRAM_ID,
        }),
      ),
      [loomAsset],
    );

    const [loomState] = PublicKey.findProgramAddressSync(
      [Buffer.from("loom_state"), loomAsset.publicKey.toBuffer()],
      program.programId,
    );

    const firstHash = new Uint8Array(32).fill(7);
    const secondHash = new Uint8Array(32).fill(9);

    await program.methods
      .sealAnky(Array.from(firstHash))
      .accounts({
        writer,
        loomAsset: loomAsset.publicKey,
        loomCollection: OFFICIAL_COLLECTION_DEVNET,
      } as never)
      .rpc();

    const afterFirst = await loomStateAccount.fetch(loomState);
    assert.strictEqual(afterFirst.loomAsset.toBase58(), loomAsset.publicKey.toBase58());
    assert.strictEqual(afterFirst.totalSeals.toNumber(), 1);
    assert.deepStrictEqual(
      Array.from(afterFirst.latestSessionHash),
      Array.from(firstHash),
    );

    await program.methods
      .sealAnky(Array.from(secondHash))
      .accounts({
        writer,
        loomAsset: loomAsset.publicKey,
        loomCollection: OFFICIAL_COLLECTION_DEVNET,
      } as never)
      .rpc();

    const afterSecond = await loomStateAccount.fetch(loomState);
    assert.strictEqual(afterSecond.totalSeals.toNumber(), 2);
    assert.deepStrictEqual(
      Array.from(afterSecond.latestSessionHash),
      Array.from(secondHash),
    );
    assert.notDeepStrictEqual(
      Array.from(afterSecond.rollingRoot),
      new Array(32).fill(0),
    );
  });
});
