import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));

const PROGRAM_PATH = resolve(
  SCRIPT_DIR,
  "../../anky-seal-program/programs/anky-seal-program/src/lib.rs",
);
const DEFAULT_MAINNET_CONFIG = resolve(SCRIPT_DIR, "mainnetConfig.json");
const COLLECTION_PATTERN =
  /pub const OFFICIAL_COLLECTION: Pubkey = pubkey!\("[1-9A-HJ-NP-Za-km-z]+"\);/;

type CollectionConfig = {
  coreCollection?: string;
  collection?: string;
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

function readCollectionFromConfig(path: string): string | undefined {
  if (!existsSync(path)) {
    return undefined;
  }

  const parsed = JSON.parse(readFileSync(path, "utf8")) as CollectionConfig;

  return parsed.coreCollection ?? parsed.collection;
}

function assertPublicKeyLike(value: string): void {
  if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(value)) {
    throw new Error(`Invalid Solana public key: ${value}`);
  }
}

function main() {
  const configPath = resolve(readFlag("--config") ?? DEFAULT_MAINNET_CONFIG);
  const collection = readFlag("--collection") ?? readCollectionFromConfig(configPath);

  if (collection == null) {
    throw new Error("Pass --collection or create mainnetConfig.json first.");
  }

  assertPublicKeyLike(collection);

  const source = readFileSync(PROGRAM_PATH, "utf8");
  if (!COLLECTION_PATTERN.test(source)) {
    throw new Error(`Could not find OFFICIAL_COLLECTION declaration in ${PROGRAM_PATH}`);
  }

  const next = source.replace(
    COLLECTION_PATTERN,
    `pub const OFFICIAL_COLLECTION: Pubkey = pubkey!("${collection}");`,
  );

  writeFileSync(PROGRAM_PATH, next);
  console.log(`Updated OFFICIAL_COLLECTION in ${PROGRAM_PATH}`);
  console.log(`Collection: ${collection}`);
}

main();
