/**
 * Anky Sojourn 9 — Ankys Collection Tree
 *
 * Creates:
 * 1. A Bubblegum Merkle Tree (maxDepth=10 → 1,024 leaf capacity)
 * 2. A Collection NFT for the ankys (separate from mirrors)
 *
 * Uses the same authority keypair as the mirrors tree.
 */

import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { createTree, mplBubblegum } from "@metaplex-foundation/mpl-bubblegum";
import { createNft, mplTokenMetadata } from "@metaplex-foundation/mpl-token-metadata";
import { generateSigner, keypairIdentity, percentAmount } from "@metaplex-foundation/umi";
import { readFileSync, writeFileSync, existsSync } from "fs";
import bs58 from "bs58";

const network = process.env.SOLANA_NETWORK || "devnet";
const heliusKey = process.env.HELIUS_API_KEY;
const rpcUrl = heliusKey
  ? (network === "mainnet-beta"
    ? `https://mainnet.helius-rpc.com/?api-key=${heliusKey}`
    : `https://devnet.helius-rpc.com/?api-key=${heliusKey}`)
  : "https://api.devnet.solana.com";

console.log(`Network: ${network}`);

// Load authority from existing keypair
const KEYPAIR_PATH = "./authority.json";
if (!existsSync(KEYPAIR_PATH)) {
  console.error("authority.json not found — run create-tree.ts first");
  process.exit(1);
}

async function main() {
  const umi = createUmi(rpcUrl).use(mplBubblegum()).use(mplTokenMetadata());

  const bytes = JSON.parse(readFileSync(KEYPAIR_PATH, "utf-8")) as number[];
  const authority = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(bytes));
  umi.use(keypairIdentity(authority));

  console.log(`Authority: ${authority.publicKey}`);

  const balance = await umi.rpc.getBalance(authority.publicKey);
  const solBalance = Number(balance.basisPoints) / 1e9;
  console.log(`Balance: ${solBalance} SOL`);

  if (solBalance < 0.05) {
    console.error("Need at least 0.05 SOL");
    process.exit(1);
  }

  // 1. Create Ankys Merkle Tree (depth 10 = 1,024 capacity)
  // Valid buffer size for depth 10 is 32
  console.log("\nCreating Ankys Merkle Tree (maxDepth=10, maxBufferSize=32)...");

  const merkleTree = generateSigner(umi);
  const treeTx = await createTree(umi, {
    merkleTree,
    maxDepth: 10,
    maxBufferSize: 32,
  });
  const treeResult = await treeTx.sendAndConfirm(umi, {
    confirm: { commitment: "finalized" },
  });
  console.log(`Ankys Tree: ${merkleTree.publicKey}`);
  console.log(`Tree tx: ${bs58.encode(treeResult.signature)}`);

  // 2. Create Ankys Collection NFT
  console.log("\nCreating Ankys Collection NFT...");
  const collectionMint = generateSigner(umi);
  const collectionTx = await createNft(umi, {
    mint: collectionMint,
    name: "Ankys — Sojourn 9",
    symbol: "ANKY",
    uri: "https://anky.app/api/ankys/collection-metadata",
    sellerFeeBasisPoints: percentAmount(0),
    isCollection: true,
  });
  const collectionResult = await collectionTx.sendAndConfirm(umi, {
    confirm: { commitment: "finalized" },
  });
  console.log(`Collection Mint: ${collectionMint.publicKey}`);
  console.log(`Collection tx: ${bs58.encode(collectionResult.signature)}`);

  const output = {
    network,
    authority: authority.publicKey.toString(),
    merkleTree: merkleTree.publicKey.toString(),
    collectionMint: collectionMint.publicKey.toString(),
    maxDepth: 10,
    maxBufferSize: 8,
    capacity: 1024,
    purpose: "ankys — stories written during sojourn 9",
  };

  writeFileSync(`./ankys-tree-${network}.json`, JSON.stringify(output, null, 2));
  console.log("\n" + JSON.stringify(output, null, 2));
}

main().catch((e) => { console.error(e); process.exit(1); });
