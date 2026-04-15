/**
 * Anky Sojourn 9 — Merkle Tree + Collection NFT Setup
 *
 * Creates:
 * 1. A Bubblegum Merkle Tree (maxDepth=12 → 4,096 leaf capacity)
 * 2. A Collection NFT that all 3,456 mirror cNFTs belong to
 *
 * Usage:
 *   npm run create-tree:devnet   (SOLANA_NETWORK=devnet)
 *   npm run create-tree:mainnet  (SOLANA_NETWORK=mainnet-beta)
 *
 * Required env:
 *   HELIUS_API_KEY — Helius RPC key
 *   AUTHORITY_KEYPAIR — (optional) base58-encoded keypair; generated if absent
 */

import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  createTree,
  mplBubblegum,
} from "@metaplex-foundation/mpl-bubblegum";
import {
  createNft,
  mplTokenMetadata,
} from "@metaplex-foundation/mpl-token-metadata";
import {
  generateSigner,
  keypairIdentity,
  percentAmount,
  publicKey,
} from "@metaplex-foundation/umi";
import { readFileSync, writeFileSync, existsSync } from "fs";
import bs58 from "bs58";

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const network = process.env.SOLANA_NETWORK || "devnet";
const heliusKey = process.env.HELIUS_API_KEY;
if (!heliusKey) {
  console.error("HELIUS_API_KEY is required");
  process.exit(1);
}

const rpcUrl =
  network === "mainnet-beta"
    ? `https://mainnet.helius-rpc.com/?api-key=${heliusKey}`
    : `https://devnet.helius-rpc.com/?api-key=${heliusKey}`;

console.log(`Network: ${network}`);
console.log(`RPC: ${rpcUrl.replace(heliusKey, "***")}`);

// ---------------------------------------------------------------------------
// Authority keypair — load or generate
// ---------------------------------------------------------------------------

const KEYPAIR_PATH = "./authority.json";

function loadOrGenerateKeypair(umi: ReturnType<typeof createUmi>) {
  if (process.env.AUTHORITY_KEYPAIR) {
    const raw = bs58.decode(process.env.AUTHORITY_KEYPAIR);
    return umi.eddsa.createKeypairFromSecretKey(raw);
  }

  if (existsSync(KEYPAIR_PATH)) {
    const bytes = JSON.parse(readFileSync(KEYPAIR_PATH, "utf-8")) as number[];
    return umi.eddsa.createKeypairFromSecretKey(new Uint8Array(bytes));
  }

  const kp = umi.eddsa.generateKeypair();
  writeFileSync(KEYPAIR_PATH, JSON.stringify(Array.from(kp.secretKey)));
  console.log("Generated new authority keypair → authority.json");
  return kp;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const umi = createUmi(rpcUrl).use(mplBubblegum()).use(mplTokenMetadata());

  const authority = loadOrGenerateKeypair(umi);
  umi.use(keypairIdentity(authority));

  console.log(`Authority: ${authority.publicKey}`);

  // Check balance
  const balance = await umi.rpc.getBalance(authority.publicKey);
  const solBalance = Number(balance.basisPoints) / 1e9;
  console.log(`Balance: ${solBalance} SOL`);

  if (solBalance < 0.05) {
    console.error(
      "Insufficient balance. Need at least 0.05 SOL for tree + collection creation."
    );
    if (network === "devnet") {
      console.log("Request an airdrop: solana airdrop 2 " + authority.publicKey);
    }
    process.exit(1);
  }

  // -------------------------------------------------------------------------
  // 1. Create Merkle Tree
  // -------------------------------------------------------------------------

  // Valid pairs: (12,32), (14,64), (20,256), etc.
  // maxDepth=12 → 4,096 leaves (enough for 3,456 mirrors)
  // maxBufferSize=32 is the valid buffer size for depth 12
  console.log("\nCreating Merkle Tree (maxDepth=12, maxBufferSize=32)...");

  const merkleTree = generateSigner(umi);

  const treeTx = await createTree(umi, {
    merkleTree,
    maxDepth: 12,
    maxBufferSize: 32,
  });

  const treeResult = await treeTx.sendAndConfirm(umi, {
    confirm: { commitment: "finalized" },
  });

  console.log(`Merkle Tree: ${merkleTree.publicKey}`);
  console.log(
    `Tree tx: ${bs58.encode(treeResult.signature)}`
  );

  // -------------------------------------------------------------------------
  // 2. Create Collection NFT
  // -------------------------------------------------------------------------

  console.log("\nCreating Collection NFT...");

  const collectionMint = generateSigner(umi);

  const collectionTx = await createNft(umi, {
    mint: collectionMint,
    name: "Anky Sojourn 9",
    symbol: "ANKY",
    uri: "https://ankycoin.com/api/mirror/collection-metadata",
    sellerFeeBasisPoints: percentAmount(0),
    isCollection: true,
  });

  const collectionResult = await collectionTx.sendAndConfirm(umi, {
    confirm: { commitment: "finalized" },
  });

  console.log(`Collection Mint: ${collectionMint.publicKey}`);
  console.log(
    `Collection tx: ${bs58.encode(collectionResult.signature)}`
  );

  // -------------------------------------------------------------------------
  // Output
  // -------------------------------------------------------------------------

  const output = {
    network,
    authority: authority.publicKey.toString(),
    merkleTree: merkleTree.publicKey.toString(),
    collectionMint: collectionMint.publicKey.toString(),
    maxDepth: 12,
    maxBufferSize: 32,
    capacity: 4096,
    targetSupply: 3456,
  };

  const outputPath = `./tree-${network}.json`;
  writeFileSync(outputPath, JSON.stringify(output, null, 2));

  console.log("\n========================================");
  console.log("Setup complete!");
  console.log("========================================");
  console.log(JSON.stringify(output, null, 2));
  console.log(`\nSaved to ${outputPath}`);
  console.log("\nNext steps:");
  console.log("1. Set these env vars in your Cloudflare Worker:");
  console.log(`   MERKLE_TREE=${merkleTree.publicKey}`);
  console.log(`   COLLECTION_MINT=${collectionMint.publicKey}`);
  console.log(
    `   AUTHORITY_KEYPAIR=${bs58.encode(authority.secretKey)}`
  );
  console.log("2. Set these env vars in the Axum backend:");
  console.log(`   SOLANA_MERKLE_TREE=${merkleTree.publicKey}`);
  console.log(`   SOLANA_COLLECTION_MINT=${collectionMint.publicKey}`);
}

main().catch((err) => {
  console.error("Setup failed:", err);
  process.exit(1);
});
