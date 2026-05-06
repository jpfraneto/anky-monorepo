#!/usr/bin/env node

import * as anchor from "@coral-xyz/anchor";
import { redactSecretValues } from "../../scripts/sojourn9/redactSecrets.mjs";

const { Connection, PublicKey } = anchor.web3;

const DEFAULT_PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const DEFAULT_CORE_PROGRAM_ID = "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d";
const DEFAULT_CORE_COLLECTION = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const DEFAULT_PROOF_VERIFIER_AUTHORITY = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const DEFAULT_DEVNET_RPC_URL = "https://api.devnet.solana.com";
const DEFAULT_MAINNET_RPC_URL = "https://api.mainnet-beta.solana.com";
const CORE_ASSET_V1_KEY = 1;
const CORE_COLLECTION_V1_KEY = 5;
const CORE_UPDATE_AUTHORITY_COLLECTION = 2;
const BOOLEAN_FLAGS = new Set(["--allow-mainnet-read"]);
const VALUE_FLAGS = new Set([
  "--cluster",
  "--core-collection",
  "--core-program-id",
  "--loom-asset",
  "--loom-owner",
  "--program-id",
  "--proof-verifier",
  "--rpc-url",
]);

main().catch((error) => {
  console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
  process.exit(1);
});

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help === true) {
    printUsage();
    return;
  }

  const cluster = resolveCluster(args.cluster ?? process.env.ANKY_SOLANA_CLUSTER);
  if (
    cluster === "mainnet-beta" &&
    args.allowMainnetRead !== true &&
    process.env.ANKY_ALLOW_MAINNET_READINESS_CHECK !== "true"
  ) {
    throw new Error(
      "Refusing mainnet readiness check without --allow-mainnet-read or ANKY_ALLOW_MAINNET_READINESS_CHECK=true.",
    );
  }

  const programId = readPublicKey(
    args.programId ?? process.env.ANKY_SEAL_PROGRAM_ID ?? DEFAULT_PROGRAM_ID,
    "seal program ID",
  );
  const coreProgramId = readPublicKey(
    args.coreProgramId ?? process.env.ANKY_CORE_PROGRAM_ID ?? DEFAULT_CORE_PROGRAM_ID,
    "Core program ID",
  );
  const coreCollection = readPublicKey(
    args.coreCollection ?? process.env.ANKY_CORE_COLLECTION ?? DEFAULT_CORE_COLLECTION,
    "Core collection",
  );
  const proofVerifier = readPublicKey(
    args.proofVerifier ??
      process.env.ANKY_PROOF_VERIFIER_AUTHORITY ??
      DEFAULT_PROOF_VERIFIER_AUTHORITY,
    "proof verifier authority",
  );
  const loomAsset =
    args.loomAsset == null ? null : readPublicKey(args.loomAsset, "Core Loom asset");
  const expectedLoomOwner =
    args.loomOwner == null ? null : readPublicKey(args.loomOwner, "Core Loom owner");
  const connection = new Connection(args.rpcUrl ?? resolveRpcUrl(cluster), "confirmed");

  const [programAccount, collectionAccount, loomAssetAccount] = await Promise.all([
    connection.getAccountInfo(programId, "confirmed"),
    connection.getAccountInfo(coreCollection, "confirmed"),
    loomAsset == null ? Promise.resolve(null) : connection.getAccountInfo(loomAsset, "confirmed"),
  ]);
  const parsedLoomAsset =
    loomAssetAccount == null ? null : parseCoreAssetBase(loomAssetAccount.data);

  const checks = {
    coreCollectionExists: collectionAccount != null,
    coreCollectionHasCollectionV1Discriminator:
      collectionAccount?.data?.[0] === CORE_COLLECTION_V1_KEY,
    coreCollectionOwnedByCore:
      collectionAccount != null && collectionAccount.owner.equals(coreProgramId),
    ...(loomAsset == null
      ? {}
      : {
          coreLoomAssetCollectionMatchesConfig:
            parsedLoomAsset?.collection === coreCollection.toBase58(),
          coreLoomAssetExists: loomAssetAccount != null,
          coreLoomAssetHasAssetV1Discriminator:
            loomAssetAccount?.data?.[0] === CORE_ASSET_V1_KEY,
          coreLoomAssetOwnedByCore:
            loomAssetAccount != null && loomAssetAccount.owner.equals(coreProgramId),
          coreLoomAssetUsesCollectionUpdateAuthority:
            parsedLoomAsset?.updateAuthorityKind === CORE_UPDATE_AUTHORITY_COLLECTION,
          ...(expectedLoomOwner == null
            ? {}
            : {
                coreLoomAssetOwnerMatchesExpected:
                  parsedLoomAsset?.owner === expectedLoomOwner.toBase58(),
              }),
        }),
    proofVerifierIsPublicKey: proofVerifier instanceof PublicKey,
    sealProgramExecutable: programAccount?.executable === true,
    sealProgramExists: programAccount != null,
  };
  const ok = Object.values(checks).every(Boolean);
  const summary = {
    checks,
    cluster,
    coreCollection: coreCollection.toBase58(),
    coreProgramId: coreProgramId.toBase58(),
    ok,
    programId: programId.toBase58(),
    proofVerifier: proofVerifier.toBase58(),
    rpcUrl: redactRpcUrl(connection.rpcEndpoint),
    ...(loomAsset == null
      ? {}
      : {
          loomAsset: {
            address: loomAsset.toBase58(),
            collection: parsedLoomAsset?.collection ?? null,
            owner: parsedLoomAsset?.owner ?? null,
            updateAuthorityKind: parsedLoomAsset?.updateAuthorityKind ?? null,
          },
        }),
  };

  console.log(JSON.stringify(summary, null, 2));
  if (!ok) {
    process.exit(1);
  }
}

function parseCoreAssetBase(data) {
  if (!Buffer.isBuffer(data) || data.length < 1 + 32 + 1) {
    return null;
  }
  if (data[0] !== CORE_ASSET_V1_KEY) {
    return null;
  }

  const owner = new PublicKey(data.subarray(1, 33)).toBase58();
  const updateAuthorityKind = data[33];
  const collection =
    updateAuthorityKind === CORE_UPDATE_AUTHORITY_COLLECTION && data.length >= 66
      ? new PublicKey(data.subarray(34, 66)).toBase58()
      : null;

  return {
    collection,
    owner,
    updateAuthorityKind,
  };
}

function readPublicKey(value, label) {
  try {
    return new PublicKey(value);
  } catch (_error) {
    throw new Error(`${label} must be a base58 Solana public key.`);
  }
}

function resolveCluster(value) {
  if (value == null || value === "" || value === "devnet") {
    return "devnet";
  }
  if (value === "mainnet-beta") {
    return "mainnet-beta";
  }

  throw new Error("--cluster must be devnet or mainnet-beta.");
}

function resolveRpcUrl(cluster) {
  if (process.env.ANKY_SOLANA_RPC_URL != null && process.env.ANKY_SOLANA_RPC_URL.trim() !== "") {
    return process.env.ANKY_SOLANA_RPC_URL.trim();
  }

  if (process.env.HELIUS_API_KEY != null && process.env.HELIUS_API_KEY.trim() !== "") {
    const host = cluster === "mainnet-beta" ? "mainnet" : "devnet";
    return `https://${host}.helius-rpc.com/?api-key=${process.env.HELIUS_API_KEY.trim()}`;
  }

  return cluster === "mainnet-beta" ? DEFAULT_MAINNET_RPC_URL : DEFAULT_DEVNET_RPC_URL;
}

function redactRpcUrl(value) {
  return value.replace(/([?&]api-key=)[^&]+/i, "$1<redacted>");
}

function parseArgs(argv) {
  const args = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (!BOOLEAN_FLAGS.has(arg) && !VALUE_FLAGS.has(arg)) {
      if (!arg.startsWith("--")) {
        throw new Error(`Unexpected argument: ${arg}`);
      }
      throw new Error(`Unknown option: ${arg}`);
    }
    if (arg === "--allow-mainnet-read") {
      args.allowMainnetRead = true;
      continue;
    }
    if (!arg.startsWith("--")) {
      throw new Error(`Unexpected argument: ${arg}`);
    }

    const key = arg.slice(2).replace(/-([a-z])/g, (_match, letter) => letter.toUpperCase());
    const value = argv[index + 1];
    if (value == null || value.startsWith("--")) {
      throw new Error(`${arg} requires a value.`);
    }
    args[key] = value;
    index += 1;
  }

  return args;
}

function printUsage() {
  console.log(`Usage:
  npm run check-config -- --cluster devnet

Options:
  --cluster <cluster>          devnet or mainnet-beta. Defaults to ANKY_SOLANA_CLUSTER or devnet.
  --rpc-url <url>              RPC URL. Defaults to ANKY_SOLANA_RPC_URL, Helius RPC, or public RPC.
  --program-id <pubkey>        Seal program. Defaults to ANKY_SEAL_PROGRAM_ID or Sojourn 9 devnet.
  --core-program-id <pubkey>   Metaplex Core program. Defaults to ANKY_CORE_PROGRAM_ID.
  --core-collection <pubkey>   Core collection. Defaults to ANKY_CORE_COLLECTION.
  --loom-asset <pubkey>        Optional real Core AssetV1 Loom account to check read-only.
  --loom-owner <pubkey>        Optional expected owner for --loom-asset.
  --proof-verifier <pubkey>    Proof verifier authority. Defaults to ANKY_PROOF_VERIFIER_AUTHORITY.
  --allow-mainnet-read         Allow a read-only mainnet readiness check.

This script only reads public account state. It never reads keypairs, signs transactions, or prints API keys.`);
}
