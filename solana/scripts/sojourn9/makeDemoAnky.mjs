#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { redactSecretValues } from "./redactSecrets.mjs";

const MS_PER_UTC_DAY = 86_400_000;
const DEFAULT_DELTA_MS = 7_999;
const DEFAULT_EVENT_COUNT = 61;
const TERMINAL_SILENCE_MS = 8_000;
const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const BOOLEAN_FLAGS = new Set(["--force"]);
const VALUE_FLAGS = new Set([
  "--character",
  "--event-count",
  "--out",
  "--started-at-ms",
]);

try {
  main();
} catch (error) {
  console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
  process.exit(1);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printUsage();
    return;
  }

  if (typeof args.out !== "string") {
    throw new Error("--out <path> is required.");
  }

  const outputPath = path.resolve(process.cwd(), args.out);
  if (isInsideRepo(outputPath)) {
    throw new Error("Refusing to write demo .anky plaintext inside this git worktree. Use a temp path such as /tmp/anky-sojourn9-demo.anky.");
  }
  if (fs.existsSync(outputPath) && args.force !== true) {
    throw new Error(`${outputPath} already exists. Pass --force to overwrite this demo witness.`);
  }

  const startedAtMs =
    args.startedAtMs == null ? Date.now() : parseInteger(args.startedAtMs, "started-at-ms");
  const character = parseCharacter(args.character ?? "a");
  const eventCount =
    args.eventCount == null ? DEFAULT_EVENT_COUNT : parseInteger(args.eventCount, "event-count");
  if (eventCount < 2) {
    throw new Error("--event-count must be at least 2.");
  }

  const raw = buildDemoAnky({ character, eventCount, startedAtMs });
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, raw, { encoding: "utf8", mode: 0o600 });

  const acceptedDurationMs = (eventCount - 1) * DEFAULT_DELTA_MS;
  const metadata = {
    witnessPath: outputPath,
    sessionHash: sha256Hex(raw),
    utcDay: Math.floor(startedAtMs / MS_PER_UTC_DAY),
    startedAtMs,
    acceptedDurationMs,
    riteDurationMs: acceptedDurationMs + TERMINAL_SILENCE_MS,
    eventCount,
    note: "Demo .anky witness written to the requested temp path. Do not commit or upload it; use the public sessionHash and utcDay for devnet seal/proof preflight.",
  };

  console.log(JSON.stringify(metadata, null, 2));
}

function buildDemoAnky({ character, eventCount, startedAtMs }) {
  const lines = [`${startedAtMs} ${character}`];
  for (let index = 1; index < eventCount; index += 1) {
    lines.push(`${String(DEFAULT_DELTA_MS).padStart(4, "0")} ${character}`);
  }
  lines.push("8000");

  return lines.join("\n");
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
    if (arg === "--force") {
      args.force = true;
      continue;
    }
    if (!arg.startsWith("--")) {
      throw new Error(`Unexpected argument: ${arg}`);
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

function parseInteger(input, label) {
  if (!/^\d+$/.test(String(input))) {
    throw new Error(`--${label} must be a non-negative integer.`);
  }

  const value = Number(input);
  if (!Number.isSafeInteger(value)) {
    throw new Error(`--${label} must be a safe integer.`);
  }

  return value;
}

function parseCharacter(input) {
  if (input === "SPACE") {
    return input;
  }
  if (input === " ") {
    throw new Error("Use --character SPACE for a space token.");
  }
  if (typeof input !== "string" || [...input].length !== 1 || input === "\n" || input === "\r") {
    throw new Error("--character must be one accepted character or SPACE.");
  }
  const codePoint = input.codePointAt(0);
  if (codePoint == null || codePoint <= 31 || codePoint === 127) {
    throw new Error("--character must be one accepted character or SPACE.");
  }

  return input;
}

function isInsideRepo(candidatePath) {
  const relative = path.relative(REPO_ROOT, candidatePath);

  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function sha256Hex(raw) {
  return crypto.createHash("sha256").update(Buffer.from(raw, "utf8")).digest("hex");
}

function printUsage() {
  console.log(`Usage:
  node solana/scripts/sojourn9/makeDemoAnky.mjs --out /tmp/anky-sojourn9-demo.anky

Options:
  --out <path>               Required output path. Must be outside the git worktree.
  --started-at-ms <epoch_ms> Optional deterministic start time. Defaults to Date.now().
  --character <char|SPACE>   Accepted character token to repeat. Defaults to a.
  --event-count <n>          Number of accepted events before terminal 8000. Defaults to ${DEFAULT_EVENT_COUNT}.
  --force                    Overwrite an existing demo witness.

This creates demo .anky plaintext for a devnet E2E run and prints only public metadata.`);
}
