import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const DEFAULT_INPUT = resolve(SCRIPT_DIR, "devnetConfig.json");
const EXAMPLE_INPUT = resolve(SCRIPT_DIR, "devnetConfig.example.json");
const DEFAULT_OUTPUT = resolve(SCRIPT_DIR, "devnetConfig.app.json");

type SolanaCluster = "devnet" | "mainnet-beta";

type CollectionOutput = {
  network?: string;
  rpcUrl?: string;
  coreCollection?: string;
  collection?: string;
  sealProgramId?: string;
  createdAt?: string;
};

function readFlag(name: string): string | undefined {
  const equalsPrefix = `${name}=`;
  const equalsValue = process.argv.find((arg) => arg.startsWith(equalsPrefix));
  if (equalsValue) {
    return equalsValue.slice(equalsPrefix.length);
  }

  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

function readJson(path: string): CollectionOutput {
  return JSON.parse(readFileSync(path, "utf8")) as CollectionOutput;
}

function main() {
  const cluster = readCluster();
  const inputPath = resolve(
    readFlag("--in") ??
      (cluster === "mainnet-beta"
        ? resolve(SCRIPT_DIR, "mainnetConfig.json")
        : DEFAULT_INPUT),
  );
  const outputPath = resolve(
    readFlag("--out") ??
      (cluster === "mainnet-beta"
        ? resolve(SCRIPT_DIR, "mainnetConfig.app.json")
        : DEFAULT_OUTPUT),
  );
  const examplePath =
    cluster === "mainnet-beta"
      ? resolve(SCRIPT_DIR, "mainnetConfig.example.json")
      : EXAMPLE_INPUT;
  const sourcePath = existsSync(inputPath) ? inputPath : examplePath;
  const source = readJson(sourcePath);

  const rpcUrl = readFlag("--rpc-url") ?? source.rpcUrl ?? defaultRpcUrl(cluster);
  const coreCollection =
    readFlag("--collection") ??
    readFlag("--core-collection") ??
    source.coreCollection ??
    source.collection ??
    "REPLACE_WITH_CORE_COLLECTION_PUBKEY";
  const sealProgramId =
    readFlag("--seal-program-id") ??
    source.sealProgramId ??
    "REPLACE_AFTER_ANCHOR_DEPLOY";

  const appConfig = {
    cluster,
    rpcUrl,
    coreCollection,
    sealProgramId,
    updatedAt: new Date().toISOString(),
    sourceCollectionCreatedAt: source.createdAt,
  };

  writeFileSync(outputPath, `${JSON.stringify(appConfig, null, 2)}\n`);
  console.log(`Wrote ${outputPath}`);
  console.log(JSON.stringify(appConfig, null, 2));
}

main();

function readCluster(): SolanaCluster {
  const value =
    readFlag("--cluster") ?? process.env.SOLANA_CLUSTER ?? process.env.ANKY_SOLANA_CLUSTER;

  return value === "mainnet-beta" ? "mainnet-beta" : "devnet";
}

function defaultRpcUrl(cluster: SolanaCluster): string {
  return cluster === "mainnet-beta"
    ? "https://api.mainnet-beta.solana.com"
    : "https://api.devnet.solana.com";
}
