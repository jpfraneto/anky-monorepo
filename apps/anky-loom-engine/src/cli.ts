#!/usr/bin/env node
import path from "node:path";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { createOctagonalLoom, TOTAL_ANCHORS } from "./geometry";
import { parseAnky } from "./parser";
import { buildPinStates, LoomPinState } from "./pins";
import { parseRenderMode, renderLoomSvg, renderModeList, type RenderMode } from "./renderer";
import { calculateFlowScore, createRhythmPattern, type FlowBucket, RHYTHM_SAMPLE_COUNT } from "./rhythm";
import { createThread, LoomThread, RENDER_VERSION } from "./thread";

interface GeneratedSessionState {
  day: number;
  sessionHash: string;
  route: number[];
  motifIds: number[];
  flowBucket: FlowBucket;
  beatCount: number;
  longJumpCount: number;
  avgJumpDistance: number;
  tinyJumpCount: number;
  renderVersion: string;
}

interface GeneratedPinState {
  day: number;
  pinIndex: number;
  state: "empty" | "visited" | "origin";
  visitCount: number;
  originSessionHash?: string;
}

interface AnkySourceFile {
  name: string;
  relativePath: string;
  absolutePath: string;
}

interface CliOptions {
  inputArg: string;
  beatCount: number;
  renderMode: RenderMode;
}

async function main(): Promise<void> {
  const { inputArg, beatCount, renderMode } = parseCliArgs(process.argv.slice(2));

  const inputDir = path.resolve(process.cwd(), inputArg);
  const files = await listAnkyFiles(inputDir);
  if (files.length === 0) {
    throw new Error(`No .anky files found in ${inputDir}.`);
  }

  const geometry = createOctagonalLoom();
  const sessions: GeneratedSessionState[] = [];
  const threads: LoomThread[] = [];

  for (let index = 0; index < files.length; index += 1) {
    const file = files[index];
    const raw = await readFile(file.absolutePath, "utf8");
    const parsed = parseAnky(raw);
    const dayIndex = dayIndexForFile(file, index);
    const rhythm = createRhythmPattern(parsed.deltas, beatCount);
    const flowScore = calculateFlowScore(parsed.deltas);
    const thread = createThread(
      {
        sessionHash: parsed.sessionHash,
        rhythmSamples: rhythm.samples,
        dayIndex,
        durationMs: parsed.durationMs,
        keystrokeCount: parsed.keystrokeCount,
        flowScore,
      },
      geometry,
    );

    sessions.push({
      day: dayIndex,
      sessionHash: parsed.sessionHash,
      route: thread.route,
      motifIds: thread.motifIds,
      flowBucket: thread.flowBucket,
      beatCount,
      longJumpCount: thread.routeDiagnostics.longJumpCount,
      avgJumpDistance: thread.routeDiagnostics.avgJumpDistance,
      tinyJumpCount: thread.routeDiagnostics.tinyJumpCount,
      renderVersion: RENDER_VERSION,
    });
    threads.push(thread);
  }

  const orderedPairs = threads
    .map((thread, index) => ({ thread, session: sessions[index] }))
    .sort(
      (left, right) =>
        left.thread.dayIndex - right.thread.dayIndex ||
        left.thread.sessionHash.localeCompare(right.thread.sessionHash),
    );
  const orderedThreads = orderedPairs.map((pair) => pair.thread);
  const orderedSessions = orderedPairs.map((pair) => pair.session);
  const pinStates = buildPinStates(geometry, orderedThreads);
  const outputDir = path.resolve(process.cwd(), "output");
  await mkdir(outputDir, { recursive: true });
  await writeFile(path.join(outputDir, "loom.svg"), renderLoomSvg(geometry, orderedThreads, pinStates, { mode: renderMode }), "utf8");
  await writeFile(
    path.join(outputDir, "loom-state.json"),
    JSON.stringify(buildState(geometry, orderedSessions, pinStates, renderMode), null, 2) + "\n",
    "utf8",
  );

  console.log(
    `Generated output/loom.svg and output/loom-state.json from ${files.length} .anky file(s) with ${beatCount} beats in ${renderMode} mode.`,
  );
}

function parseCliArgs(args: readonly string[]): CliOptions {
  let inputArg: string | undefined;
  let beatCount = RHYTHM_SAMPLE_COUNT;
  let renderMode: RenderMode = "weave";

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];

    if (arg === "--beats") {
      const value = args[index + 1];
      if (!value) {
        throw new Error(`Missing value for --beats.\n${usage()}`);
      }

      beatCount = parseBeatCount(value);
      index += 1;
      continue;
    }

    if (arg.startsWith("--beats=")) {
      beatCount = parseBeatCount(arg.slice("--beats=".length));
      continue;
    }

    if (arg === "--mode") {
      const value = args[index + 1];
      if (!value) {
        throw new Error(`Missing value for --mode.\n${usage()}`);
      }

      renderMode = parseRenderMode(value);
      index += 1;
      continue;
    }

    if (arg.startsWith("--mode=")) {
      renderMode = parseRenderMode(arg.slice("--mode=".length));
      continue;
    }

    if (arg.startsWith("--")) {
      throw new Error(`Unknown option: ${arg}\n${usage()}`);
    }

    if (inputArg) {
      throw new Error(`Unexpected argument: ${arg}\n${usage()}`);
    }

    inputArg = arg;
  }

  if (!inputArg) {
    throw new Error(usage());
  }

  return {
    inputArg,
    beatCount,
    renderMode,
  };
}

function parseBeatCount(value: string): number {
  const beatCount = Number(value);

  if (!Number.isInteger(beatCount) || beatCount < 1 || beatCount > 96) {
    throw new RangeError("--beats must be an integer between 1 and 96.");
  }

  return beatCount;
}

function usage(): string {
  return `Usage: npm run generate -- ./input [--beats 16] [--mode ${renderModeList()}]`;
}

async function listAnkyFiles(inputDir: string): Promise<AnkySourceFile[]> {
  const files: AnkySourceFile[] = [];

  await collectAnkyFiles(inputDir, inputDir, files);

  return files.sort((left, right) =>
    left.relativePath.localeCompare(right.relativePath, undefined, { numeric: true, sensitivity: "base" }),
  );
}

async function collectAnkyFiles(rootDir: string, currentDir: string, files: AnkySourceFile[]): Promise<void> {
  const entries = await readdir(currentDir, { withFileTypes: true });
  const sortedEntries = entries.sort((left, right) =>
    left.name.localeCompare(right.name, undefined, { numeric: true, sensitivity: "base" }),
  );

  for (const entry of sortedEntries) {
    const absolutePath = path.join(currentDir, entry.name);

    if (entry.isDirectory()) {
      await collectAnkyFiles(rootDir, absolutePath, files);
      continue;
    }

    if (!entry.isFile() || !entry.name.toLowerCase().endsWith(".anky")) {
      continue;
    }

    files.push({
      name: entry.name,
      relativePath: path.relative(rootDir, absolutePath).split(path.sep).join("/"),
      absolutePath,
    });
  }
}

function dayIndexForFile(file: AnkySourceFile, sortedIndex: number): number {
  const pathSegments = file.relativePath.split("/");
  const directories = pathSegments.slice(0, -1);
  const explicitFileDay = file.name.match(/day[-_ ]?(\d{1,3})/i)?.[1];
  const explicitDirectoryDay = [...directories].reverse().find((segment) => /^day[-_ ]?\d{1,3}$/i.test(segment));
  const explicitDirectoryValue = explicitDirectoryDay?.match(/\d{1,3}/)?.[0];
  const dateStyleDay = dayFromDateStylePath(directories);
  const nearestNumericDirectory = [...directories].reverse().find((segment) => /^\d{1,3}$/.test(segment));
  const dayIndex = Number(
    explicitFileDay ?? explicitDirectoryValue ?? dateStyleDay ?? nearestNumericDirectory ?? sortedIndex + 1,
  );

  if (!Number.isInteger(dayIndex) || dayIndex < 1 || dayIndex > TOTAL_ANCHORS) {
    throw new RangeError(`${file.relativePath} resolves to dayIndex ${dayIndex}; expected 1-${TOTAL_ANCHORS}.`);
  }

  return dayIndex;
}

function dayFromDateStylePath(directories: readonly string[]): string | undefined {
  const numericDirectories = directories.filter((segment) => /^\d{1,3}$/.test(segment));
  if (numericDirectories.length < 2) {
    return undefined;
  }

  const month = Number(numericDirectories[0]);
  const day = Number(numericDirectories[1]);

  if (month >= 1 && month <= 12 && day >= 1 && day <= 31) {
    return numericDirectories[1];
  }

  return undefined;
}

function buildState(
  geometry: ReturnType<typeof createOctagonalLoom>,
  sessions: GeneratedSessionState[],
  pinStates: readonly LoomPinState[],
  renderMode: RenderMode,
) {
  return {
    version: 6,
    renderVersion: RENDER_VERSION,
    renderMode,
    canvas: {
      width: geometry.width,
      height: geometry.height,
    },
    loom: {
      sides: 8,
      anchorsPerSide: 12,
      totalAnchors: TOTAL_ANCHORS,
      outerRadius: geometry.outerRadius,
      backgroundOuterRadius: geometry.backgroundOuterRadius,
      backgroundInnerRadius: geometry.backgroundInnerRadius,
      forbiddenRadius: geometry.forbiddenRadius,
      centerRingRadius: geometry.centerRingRadius,
      bendRadius: geometry.bendRadius,
    },
    pins: pinStates.map(formatPinStateForJson),
    sessions,
  };
}

function formatPinStateForJson(pinState: LoomPinState): GeneratedPinState {
  const generatedPinState: GeneratedPinState = {
    day: pinState.day,
    pinIndex: pinState.pinIndex,
    state: pinState.state,
    visitCount: pinState.visitCount,
  };

  if (pinState.state === "origin" && pinState.originSessionHash) {
    generatedPinState.originSessionHash = pinState.originSessionHash;
  }

  return generatedPinState;
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
