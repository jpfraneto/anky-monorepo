import { create, fetchCollection, mplCore } from "@metaplex-foundation/mpl-core";
import {
  createSignerFromKeypair,
  generateSigner,
  keypairIdentity,
  publicKey,
} from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { base58 } from "@metaplex-foundation/umi/serializers";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type DevnetConfig = {
  coreCollection?: string;
  rpcUrl?: string;
};

const DEVNET_GENESIS_HASH = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";
const DEFAULT_RPC_URL = "https://api.devnet.solana.com";
const DEFAULT_METADATA_BASE_URL = "https://anky.app/devnet/metadata/looms";
const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const DEFAULT_CONFIG_PATH = resolve(SCRIPT_DIR, "devnetConfig.json");

function readFlag(name: string): string | undefined {
  const equalsPrefix = `${name}=`;
  const equalsValue = process.argv.find((arg) => arg.startsWith(equalsPrefix));
  if (equalsValue) {
    return equalsValue.slice(equalsPrefix.length);
  }

  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

function hasFlag(name: string): boolean {
  return process.argv.includes(name);
}

function expandHome(input: string): string {
  return input.startsWith("~/") ? resolve(homedir(), input.slice(2)) : input;
}

function loadConfig(path: string): DevnetConfig {
  if (!existsSync(path)) {
    return {};
  }

  return JSON.parse(readFileSync(path, "utf8")) as DevnetConfig;
}

function loadKeypairBytes(path: string): Uint8Array {
  if (!existsSync(path)) {
    throw new Error(`Keypair not found at ${path}. Pass --keypair or set KEYPAIR_PATH.`);
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

function assertPublicKeyLike(label: string, value: string): void {
  if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(value)) {
    throw new Error(`Invalid ${label}: ${value}`);
  }
}

function readPositiveIntegerFlag(name: string, fallback: number): number {
  const raw = readFlag(name);
  if (raw == null) {
    return fallback;
  }

  const value = Number.parseInt(raw, 10);
  if (!Number.isInteger(value) || value < 1) {
    throw new Error(`${name} must be a positive integer.`);
  }

  return value;
}

function formatLoomNumber(loomIndex: number): string {
  return loomIndex.toString().padStart(4, "0");
}

async function assertDevnetRpc(rpcUrl: string): Promise<void> {
  const response = await fetch(rpcUrl, {
    body: JSON.stringify({
      id: "anky-devnet-loom-genesis-check",
      jsonrpc: "2.0",
      method: "getGenesisHash",
    }),
    headers: { "content-type": "application/json" },
    method: "POST",
  });

  if (!response.ok) {
    throw new Error(`Could not verify Solana RPC genesis hash: HTTP ${response.status}`);
  }

  const payload = (await response.json()) as { error?: unknown; result?: string };
  if (payload.error != null || payload.result == null) {
    throw new Error(`Could not verify Solana RPC genesis hash: ${JSON.stringify(payload.error)}`);
  }

  if (payload.result !== DEVNET_GENESIS_HASH) {
    throw new Error(
      `Refusing to mint: RPC genesis hash ${payload.result} is not devnet ${DEVNET_GENESIS_HASH}.`,
    );
  }
}

function printHelp(): void {
  console.log(`Create a Metaplex Core Loom asset on devnet only.

Usage:
  npm run create-devnet-core-loom -- --keypair <path> --owner <writer-pubkey> [--loom-index <n>]

Options:
  --keypair <path>       Devnet collection authority and payer keypair path.
                         Can also be set with KEYPAIR_PATH.
  --owner <pubkey>       Wallet that will own the new Loom asset.
                         Defaults to the keypair public key.
  --collection <pubkey>  Core collection. Defaults to devnetConfig.json.
  --rpc-url <url>        RPC URL. Defaults to devnetConfig.json or public devnet.
  --loom-index <n>       Metadata index. Defaults to 1.
  --name <text>          Asset name. Defaults to Anky Sojourn 9 Loom #NNNN.
  --uri <url>            Asset metadata URI. Defaults to devnet Loom metadata.
  --asset-only           Print only the new Loom asset public key.
  --help                 Show this help.

This command refuses any RPC whose genesis hash is not Solana devnet.`);
}

async function main(): Promise<void> {
  if (hasFlag("--help")) {
    printHelp();
    return;
  }

  if (readFlag("--cluster") != null && readFlag("--cluster") !== "devnet") {
    throw new Error("This script is devnet-only. Use --cluster devnet or omit --cluster.");
  }

  const config = loadConfig(DEFAULT_CONFIG_PATH);
  const keypairPath = expandHome(readFlag("--keypair") ?? process.env.KEYPAIR_PATH ?? "");
  if (keypairPath === "") {
    throw new Error("Pass --keypair or set KEYPAIR_PATH.");
  }

  const rpcUrl = readFlag("--rpc-url") ?? process.env.SOLANA_RPC_URL ?? config.rpcUrl ?? DEFAULT_RPC_URL;
  const collection = readFlag("--collection") ?? config.coreCollection;
  if (collection == null) {
    throw new Error("Pass --collection or set coreCollection in devnetConfig.json.");
  }

  assertPublicKeyLike("collection public key", collection);
  await assertDevnetRpc(rpcUrl);

  const umi = createUmi(rpcUrl).use(mplCore());
  const authorityKeypair = umi.eddsa.createKeypairFromSecretKey(loadKeypairBytes(keypairPath));
  const authority = createSignerFromKeypair(umi, authorityKeypair);
  umi.use(keypairIdentity(authorityKeypair));

  const owner = readFlag("--owner") ?? authority.publicKey.toString();
  assertPublicKeyLike("owner public key", owner);

  const loomIndex = readPositiveIntegerFlag("--loom-index", 1);
  const loomNumber = formatLoomNumber(loomIndex);
  const name = readFlag("--name") ?? `Anky Sojourn 9 Loom #${loomNumber}`;
  const uri = readFlag("--uri") ?? `${DEFAULT_METADATA_BASE_URL}/${loomNumber}.json`;

  const coreCollection = await fetchCollection(umi, publicKey(collection));
  if (coreCollection.updateAuthority.toString() !== authority.publicKey.toString()) {
    throw new Error(
      `Keypair ${authority.publicKey} is not the Core collection update authority ${coreCollection.updateAuthority}.`,
    );
  }

  const asset = generateSigner(umi);
  const result = await create(umi, {
    asset,
    authority,
    collection: coreCollection,
    name,
    owner: publicKey(owner),
    payer: authority,
    uri,
  }).sendAndConfirm(umi, {
    confirm: { commitment: "finalized" },
  });

  const signature = base58.deserialize(result.signature)[0];
  if (hasFlag("--asset-only")) {
    console.log(asset.publicKey.toString());
    return;
  }

  console.log(
    JSON.stringify(
      {
        collection,
        createdAt: new Date().toISOString(),
        loomAsset: asset.publicKey.toString(),
        name,
        network: "devnet",
        owner,
        payer: authority.publicKey.toString(),
        rpcUrl,
        signature,
        uri,
      },
      null,
      2,
    ),
  );
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
