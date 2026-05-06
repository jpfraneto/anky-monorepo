#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { redactSecretValues } from "../sojourn9/redactSecrets.mjs";

const DEFAULT_PROGRAM_ID = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const DEFAULT_CLUSTER = "devnet";
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BOOLEAN_FLAGS = new Set(["--allow-http-localhost"]);
const VALUE_FLAGS = new Set([
  "--cluster",
  "--out",
  "--program-id",
  "--transaction-types",
  "--webhook-url",
]);

main();

function main() {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help === true) {
      printUsage();
      return;
    }

    const cluster = resolveCluster(firstNonempty(args.cluster, process.env.ANKY_SOLANA_CLUSTER, DEFAULT_CLUSTER));
    const programId = resolvePublicKey(
      firstNonempty(args.programId, process.env.ANKY_SEAL_PROGRAM_ID, DEFAULT_PROGRAM_ID),
      "program ID",
    );
    const webhookUrl = validateWebhookUrl(requiredArg(args, "webhookUrl"), {
      allowHttpLocalhost: args.allowHttpLocalhost === true,
    });
    const transactionTypes = parseTransactionTypes(args.transactionTypes ?? "ANY");
    const payload = {
      webhookURL: webhookUrl,
      webhookType: heliusWebhookType(cluster),
      accountAddresses: [programId],
      transactionTypes,
      authHeader: "Bearer $ANKY_INDEXER_WRITE_SECRET",
    };
    const output = {
      cluster,
      createEndpoint: heliusWebhookEndpoint(cluster),
      notes: [
        "Dry-run only: this script does not call Helius and does not read Helius API keys.",
        "Create the webhook from the Helius dashboard or with the shown endpoint using HELIUS_API_KEY outside Codex.",
        "Set authHeader to Bearer plus the real ANKY_INDEXER_WRITE_SECRET value outside Codex.",
        "The receiver must return HTTP 200 and dedupe by transaction signature.",
        "Helius retries failed deliveries with exponential backoff for up to 24 hours; monitor webhook logs and re-enable disabled webhooks from the Helius dashboard or API after fixing receiver failures.",
        "Helius may auto-disable webhooks with very high delivery failure rates; fix the receiver before re-enabling to preserve the post-reenable grace period.",
        "Helius cannot deliver to private localhost; use an HTTPS public tunnel such as ngrok for live delivery tests.",
      ],
      payload,
    };

    if (typeof args.out === "string") {
      const outPath = path.resolve(args.out);
      fs.mkdirSync(path.dirname(outPath), { recursive: true });
      fs.writeFileSync(outPath, `${JSON.stringify(output, null, 2)}\n`);
      console.log(`wrote ${outPath}`);
      return;
    }

    console.log(JSON.stringify(output, null, 2));
  } catch (error) {
    console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
    process.exit(1);
  }
}

function heliusWebhookEndpoint(cluster) {
  const subdomain = cluster === "mainnet-beta" ? "api-mainnet" : "api-devnet";

  return `https://${subdomain}.helius-rpc.com/v0/webhooks?api-key=$HELIUS_API_KEY`;
}

function heliusWebhookType(cluster) {
  return cluster === "mainnet-beta" ? "enhanced" : "enhancedDevnet";
}

function validateWebhookUrl(value, { allowHttpLocalhost }) {
  let url;
  try {
    url = new URL(value);
  } catch (_error) {
    throw new Error("--webhook-url must be a valid absolute URL.");
  }

  if (url.username !== "" || url.password !== "") {
    throw new Error("--webhook-url must not contain credentials.");
  }
  if (url.search !== "" || url.hash !== "") {
    throw new Error("--webhook-url must not contain query strings or fragments.");
  }
  if (url.protocol === "https:") {
    return url.toString();
  }
  if (allowHttpLocalhost && url.protocol === "http:" && isLocalhost(url.hostname)) {
    return url.toString();
  }

  throw new Error("--webhook-url must use https. Use --allow-http-localhost only for local tunnel smoke tests.");
}

function isLocalhost(hostname) {
  return hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1";
}

function parseTransactionTypes(value) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error("--transaction-types must be a comma-separated list.");
  }
  const types = value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
  if (types.length === 0) {
    throw new Error("--transaction-types must include at least one transaction type.");
  }
  for (const type of types) {
    if (!/^[A-Z][A-Z0-9_]*$/.test(type)) {
      throw new Error(`Invalid Helius transaction type: ${type}`);
    }
  }

  return types;
}

function resolveCluster(value) {
  if (value === "devnet" || value === "mainnet-beta") {
    return value;
  }

  throw new Error("--cluster must be devnet or mainnet-beta.");
}

function resolvePublicKey(value, label) {
  if (!isBase58PublicKey(value)) {
    throw new Error(`${label} must be a base58 Solana public key.`);
  }

  return value;
}

function isBase58PublicKey(value) {
  if (typeof value !== "string") {
    return false;
  }

  try {
    return base58Decode(value).length === 32;
  } catch (_error) {
    return false;
  }
}

function base58Decode(value) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error("invalid base58 string");
  }

  let decoded = 0n;
  for (const character of value) {
    const digit = BASE58_ALPHABET.indexOf(character);
    if (digit < 0) {
      throw new Error("invalid base58 character");
    }
    decoded = decoded * 58n + BigInt(digit);
  }

  let hex = decoded.toString(16);
  if (hex.length % 2 === 1) {
    hex = `0${hex}`;
  }
  const bytes = decoded === 0n ? [] : [...Buffer.from(hex, "hex")];
  for (const character of value) {
    if (character === "1") {
      bytes.unshift(0);
    } else {
      break;
    }
  }

  return Buffer.from(bytes);
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
    if (arg === "--allow-http-localhost") {
      args.allowHttpLocalhost = true;
      continue;
    }

    const value = argv[index + 1];
    if (value == null || value.startsWith("--")) {
      throw new Error(`${arg} requires a value.`);
    }
    const key = arg.slice(2).replace(/-([a-z])/g, (_match, letter) => letter.toUpperCase());
    args[key] = value;
    index += 1;
  }

  return args;
}

function requiredArg(args, name) {
  const value = args[name];
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`--${name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} is required.`);
  }

  return value.trim();
}

function firstNonempty(...values) {
  for (const value of values) {
    if (typeof value === "string" && value.trim().length > 0) {
      return value.trim();
    }
  }

  return null;
}

function printUsage() {
  console.log(`Usage:
  node solana/scripts/indexer/heliusWebhookManifest.mjs \\
    --webhook-url https://your-domain.example/api/helius/anky-seal

Options:
  --webhook-url <url>         Required HTTPS endpoint for Helius delivery.
  --program-id <pubkey>       Defaults to ANKY_SEAL_PROGRAM_ID or Sojourn 9 devnet program.
  --cluster <cluster>         devnet or mainnet-beta. Defaults to ANKY_SOLANA_CLUSTER or devnet.
  --transaction-types <csv>   Defaults to ANY.
  --allow-http-localhost      Permit http://localhost URLs only for local receiver tests, not Helius delivery.
  --out <path>                Write JSON manifest to a file.

This prints the public Helius enhanced webhook creation payload. It does not call
Helius, does not read HELIUS_API_KEY, and does not create a paid webhook.`);
}
