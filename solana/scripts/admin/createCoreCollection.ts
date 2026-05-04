import { createCollection, mplCore } from "@metaplex-foundation/mpl-core";
import { generateSigner, keypairIdentity } from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { base58 } from "@metaplex-foundation/umi/serializers";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type SolanaCluster = "devnet" | "mainnet-beta";

const DEFAULT_CLUSTER: SolanaCluster = "devnet";
const COLLECTION_NAME = "Anky Sojourn 9 Looms";
const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));

function clusterFromEnv(): SolanaCluster {
  return process.env.SOLANA_CLUSTER === "mainnet-beta" ? "mainnet-beta" : DEFAULT_CLUSTER;
}

function defaultRpcUrl(cluster: SolanaCluster): string {
  return cluster === "mainnet-beta"
    ? "https://api.mainnet-beta.solana.com"
    : "https://api.devnet.solana.com";
}

function defaultCollectionUri(cluster: SolanaCluster): string {
  return cluster === "mainnet-beta"
    ? "https://anky.app/mainnet/metadata/sojourn-9-looms.json"
    : "https://anky.app/devnet/metadata/sojourn-9-looms.json";
}

function defaultOutputPath(cluster: SolanaCluster): string {
  return resolve(SCRIPT_DIR, cluster === "mainnet-beta" ? "mainnetConfig.json" : "devnetConfig.json");
}

function expandHome(input: string): string {
  return input.startsWith("~/") ? resolve(homedir(), input.slice(2)) : input;
}

function readKeypairPath(): string {
  return expandHome(process.env.KEYPAIR_PATH ?? "~/.config/solana/id.json");
}

function loadKeypairBytes(path: string): Uint8Array {
  if (!existsSync(path)) {
    throw new Error(
      `Keypair not found at ${path}. Set KEYPAIR_PATH or create ~/.config/solana/id.json.`,
    );
  }

  const parsed = JSON.parse(readFileSync(path, "utf8")) as unknown;
  if (
    !Array.isArray(parsed) ||
    parsed.length !== 64 ||
    !parsed.every((value) => Number.isInteger(value) && value >= 0 && value <= 255)
  ) {
    throw new Error(`Expected ${path} to contain a Solana 64-byte keypair array.`);
  }

  return new Uint8Array(parsed);
}

async function main() {
  const cluster = clusterFromEnv();
  const rpcUrl =
    process.env.SOLANA_RPC_URL ??
    process.env.ANKY_SOLANA_RPC_URL ??
    process.env.DEVNET_RPC_URL ??
    defaultRpcUrl(cluster);
  const collectionUri = process.env.COLLECTION_URI ?? defaultCollectionUri(cluster);
  const keypairPath = readKeypairPath();
  const outputPath = process.env.OUTPUT_PATH
    ? resolve(process.env.OUTPUT_PATH)
    : defaultOutputPath(cluster);

  const umi = createUmi(rpcUrl).use(mplCore());
  const authority = umi.eddsa.createKeypairFromSecretKey(
    loadKeypairBytes(keypairPath),
  );
  umi.use(keypairIdentity(authority));

  console.log(`Network: ${cluster}`);
  console.log(`RPC: ${rpcUrl}`);
  console.log(`Authority: ${authority.publicKey}`);
  console.log(`Creating Core collection: ${COLLECTION_NAME}`);

  const collection = generateSigner(umi);
  const result = await createCollection(umi, {
    collection,
    name: COLLECTION_NAME,
    uri: collectionUri,
  }).sendAndConfirm(umi, {
    confirm: { commitment: "finalized" },
  });

  const signature = base58.deserialize(result.signature)[0];
  const output = {
    network: cluster,
    rpcUrl,
    coreCollection: collection.publicKey.toString(),
    collectionName: COLLECTION_NAME,
    collectionUri,
    sealProgramId: "REPLACE_AFTER_ANCHOR_DEPLOY",
    createdAt: new Date().toISOString(),
    signature,
  };

  writeFileSync(outputPath, `${JSON.stringify(output, null, 2)}\n`);

  console.log(`Core collection: ${output.coreCollection}`);
  console.log(`Signature: ${signature}`);
  console.log(`Saved ${outputPath}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
