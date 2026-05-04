import { AnchorPoint, getAnchorByDayIndex, LoomGeometry, polar, SIDE_COLORS, SIDE_COUNT, TOTAL_ANCHORS } from "./geometry";
import { flowBucketForScore, type FlowBucket } from "./rhythm";

export const RENDER_VERSION = "motif-mandala-v6";
const ROUTE_VERSION = "motif-mandala-v4";

const MOTIFS = [
  [8, 24, -16, 40],
  [12, -24, 32, -8],
  [16, 32, -24, 48],
  [24, 8, -32, 16],
  [40, -16, 24, -8],
  [32, 12, -40, 24],
  [48, -24, 16, -8],
] as const;

type MovementKind = "local" | "medium" | "dramatic";

interface RoutePlan {
  route: number[];
  motifIds: number[];
  movementKinds: MovementKind[];
  diagnostics: RouteDiagnostics;
}

export interface RouteDiagnostics {
  longJumpCount: number;
  avgJumpDistance: number;
  tinyJumpCount: number;
}

export interface ThreadInput {
  sessionHash: string;
  rhythmSamples: readonly number[];
  dayIndex: number;
  durationMs: number;
  keystrokeCount: number;
  flowScore?: number;
}

export interface LoomThreadSegment {
  fromAnchorIndex: number;
  toAnchorIndex: number;
  fromDayIndex: number;
  toDayIndex: number;
  beat: number;
  path: string;
  strokeColor: string;
  strokeWidth: number;
  opacity: number;
}

export interface LoomThread {
  sessionHash: string;
  dayIndex: number;
  color: string;
  flowBucket: FlowBucket;
  route: number[];
  motifIds: number[];
  routeDiagnostics: RouteDiagnostics;
  segments: LoomThreadSegment[];
  strokeWidth: number;
  opacity: number;
}

export function createThread(input: ThreadInput, geometry: LoomGeometry): LoomThread {
  const start = getAnchorByDayIndex(geometry, input.dayIndex);
  const routePlan = routeForSession(input.dayIndex, input.sessionHash, input.rhythmSamples, geometry);
  const route = routePlan.route;
  const flowScore = clamp(input.flowScore ?? 0.35, 0.12, 1);
  const flowBucket = flowBucketForScore(flowScore);
  // Flow score is private line strength/coherence: it changes presence without exposing a number.
  const flowOpacityMultiplier = flowOpacityMultiplierForScore(flowScore);
  const bias = hashInt(`${RENDER_VERSION}:${input.sessionHash}`, 0, 2) === 0 ? 1 : -1;

  const segments = route.slice(0, -1).map((anchorIndex, index) => {
    const from = geometry.anchors[anchorIndex];
    const to = geometry.anchors[route[index + 1]];
    const beat = input.rhythmSamples[Math.min(index, input.rhythmSamples.length - 1)] ?? 0.5;
    const movementKind = routePlan.movementKinds[index] ?? classifyMovement(from, to, start);
    const strokeWidth = 0.86 + beat * 1.35 + flowScore * 0.3;
    const baseOpacity = 0.13 + beat * 0.26;
    const opacity = clamp(baseOpacity * flowOpacityMultiplier, 0.06, 0.42);

    return {
      fromAnchorIndex: from.anchorIndex,
      toAnchorIndex: to.anchorIndex,
      fromDayIndex: from.dayIndex,
      toDayIndex: to.dayIndex,
      beat,
      path: safeCurve(from, to, index % 2 === 0 ? bias : -bias, geometry, input.sessionHash, index),
      strokeColor: colorForMotion(start, to, movementKind, flowScore, index),
      strokeWidth: round(strokeWidth, 2),
      opacity: round(opacity, 3),
    };
  });

  return {
    sessionHash: input.sessionHash,
    dayIndex: input.dayIndex,
    color: start.color,
    flowBucket,
    route,
    motifIds: routePlan.motifIds,
    routeDiagnostics: routePlan.diagnostics,
    segments,
    strokeWidth: round(average(segments.map((segment) => segment.strokeWidth)), 3),
    opacity: round(average(segments.map((segment) => segment.opacity)), 3),
  };
}

export function resonanceColor(dayIndex: number): string {
  if (!Number.isInteger(dayIndex) || dayIndex < 1 || dayIndex > TOTAL_ANCHORS) {
    throw new RangeError(`dayIndex must be between 1 and ${TOTAL_ANCHORS}. Received ${dayIndex}.`);
  }

  return SIDE_COLORS[(dayIndex - 1) % SIDE_COUNT];
}

function routeForSession(
  startDay: number,
  sessionHash: string,
  beats: readonly number[],
  geometry: LoomGeometry,
): RoutePlan {
  const seed = `${ROUTE_VERSION}:${sessionHash}:${startDay}`;
  const motifIds = chooseMotifIds(seed);
  const phrase = motifIds.flatMap((motifId) => [...MOTIFS[motifId]]);
  const start = getAnchorByDayIndex(geometry, startDay);
  // The day pin is the root note. Routes now travel in the visible 96-pin ring so motif intervals
  // become real loom tension instead of same-resonance edge scribbles.
  let current = start.anchorIndex;
  let direction = hashInt(seed, 10, 2) === 0 ? 1 : -1;
  const route = [current];
  const movementKinds: MovementKind[] = [];
  const tinyLimit = Math.max(1, Math.floor(beats.length * 0.15));
  const forcedLongIndexes = chooseForcedLongIndexes(seed, beats.length);
  const forcedCrossIndex = beats.length >= 14 ? chooseForcedCrossIndex(seed, beats.length, forcedLongIndexes) : -1;
  let tinyJumpCount = 0;
  let longJumpCount = 0;
  let crossJumpCount = 0;

  beats.forEach((beat, index) => {
    const previous = beats[Math.max(0, index - 1)] ?? beat;
    const jagged = Math.abs(beat - previous);
    const remainingSteps = beats.length - index;
    const missingLongJumps = Math.max(0, Math.min(2, beats.length) - longJumpCount);
    const missingCrossJumps = beats.length >= 14 ? Math.max(0, 1 - crossJumpCount) : 0;

    if (index > 0 && jagged > 0.28 + hashUnit(seed, 100 + index) * 0.18) {
      direction *= -1;
    }

    if (index > 0 && index % phrase.length === 0 && hashInt(seed, 200 + index, 4) === 0) {
      direction *= -1;
    }

    const mustMakeCrossJump = index === forcedCrossIndex || remainingSteps <= missingCrossJumps;
    const mustMakeLongJump =
      mustMakeCrossJump || forcedLongIndexes.has(index) || remainingSteps <= missingLongJumps;
    const movementKind = mustMakeLongJump ? "dramatic" : chooseMovementKind(seed, index, beat, jagged);
    const motifJump = phrase[index % phrase.length];
    const minimumDistance = mustMakeCrossJump ? 40 : mustMakeLongJump ? 32 : 8;
    const jump = jumpForMotion({
      motifJump,
      movementKind,
      direction,
      beat,
      jagged,
      seed,
      index,
      minimumDistance,
      allowTinyGrace: !mustMakeLongJump && tinyJumpCount < tinyLimit,
    });
    current = wrapAnchor(current + jump);
    const distance = ringDistance(route[route.length - 1], current);

    if (distance <= 3) {
      tinyJumpCount += 1;
    }

    if (distance >= 32) {
      longJumpCount += 1;
    }

    if (distance >= 40) {
      crossJumpCount += 1;
    }

    route.push(current);
    movementKinds.push(movementKind);
  });

  return {
    route,
    motifIds,
    movementKinds,
    diagnostics: calculateRouteDiagnostics(route),
  };
}

function chooseMotifIds(seed: string): number[] {
  const first = hashInt(seed, 20, MOTIFS.length);
  const useSecondMotif = hashInt(seed, 21, 2) === 0;

  if (!useSecondMotif) {
    return [first];
  }

  return [first, (first + 1 + hashInt(seed, 22, MOTIFS.length - 1)) % MOTIFS.length];
}

function chooseForcedLongIndexes(seed: string, beatCount: number): Set<number> {
  if (beatCount <= 0) {
    return new Set();
  }

  if (beatCount === 1) {
    return new Set([0]);
  }

  const firstWindow = Math.max(1, Math.floor(beatCount / 2));
  const first = hashInt(seed, 80, firstWindow);
  const secondWindowStart = firstWindow;
  const second = secondWindowStart + hashInt(seed, 81, beatCount - secondWindowStart);

  return new Set([first, second]);
}

function chooseForcedCrossIndex(seed: string, beatCount: number, forcedLongIndexes: ReadonlySet<number>): number {
  const candidates = [...forcedLongIndexes];

  if (candidates.length > 0 && hashInt(seed, 82, 2) === 0) {
    return candidates[hashInt(seed, 83, candidates.length)];
  }

  return hashInt(seed, 84, beatCount);
}

function chooseMovementKind(seed: string, index: number, beat: number, jagged: number): MovementKind {
  let local = 0.35;
  let medium = 0.4;
  let dramatic = 0.25;

  if (beat < 0.35 && jagged < 0.22) {
    local += 0.08;
    medium += 0.02;
    dramatic -= 0.1;
  }

  if (beat > 0.72) {
    local -= 0.12;
    medium -= 0.08;
    dramatic += 0.2;
  }

  if (jagged > 0.32) {
    local -= 0.08;
    medium += 0.02;
    dramatic += 0.06;
  }

  local = clamp(local, 0.2, 0.48);
  medium = clamp(medium, 0.26, 0.52);
  dramatic = clamp(dramatic, 0.16, 0.42);

  const total = local + medium + dramatic;
  const roll = hashUnit(seed, 300 + index);

  if (roll < local / total) {
    return "local";
  }

  if (roll < (local + medium) / total) {
    return "medium";
  }

  return "dramatic";
}

interface JumpOptions {
  motifJump: number;
  movementKind: MovementKind;
  direction: number;
  beat: number;
  jagged: number;
  seed: string;
  index: number;
  minimumDistance: number;
  allowTinyGrace: boolean;
}

function jumpForMotion(options: JumpOptions): number {
  const { motifJump, movementKind, direction, beat, jagged, seed, index, minimumDistance, allowTinyGrace } = options;
  const signedMotif = motifJump * direction;
  const sign = signedMotif < 0 ? -1 : 1;
  const motifDistance = Math.min(48, Math.max(1, Math.abs(signedMotif)));

  // The day pin is the root note. Harmonic motifs are phrases, and each beat is a rhythm event
  // that nudges the phrase toward local memory, medium harmonic travel, or a dramatic interval.
  if (movementKind === "local") {
    const graceNote = allowTinyGrace && hashInt(seed, 620 + index, 8) === 0;
    if (graceNote) {
      return sign * (1 + hashInt(seed, 640 + index, 3));
    }

    const distance = rhythmVariedDistance(seed, index, motifDistance, 8, beat > 0.72 ? 24 : 16, beat, jagged);

    return sign * enforceMinimumDistance(distance, minimumDistance, seed, index);
  }

  if (movementKind === "medium") {
    const distance = rhythmVariedDistance(seed, index, motifDistance, 16, 32, beat, jagged);

    return sign * enforceMinimumDistance(distance, minimumDistance, seed, index);
  }

  const distance = rhythmVariedDistance(seed, index, motifDistance, 32, 48, beat, jagged);

  return sign * enforceMinimumDistance(distance, minimumDistance, seed, index);
}

function rhythmVariedDistance(
  seed: string,
  index: number,
  motifDistance: number,
  minDistance: number,
  maxDistance: number,
  beat: number,
  jagged: number,
): number {
  let distance = clampInt(motifDistance, minDistance, maxDistance);

  if (beat < 0.28 && jagged < 0.2) {
    distance -= 4;
  }

  if ((beat > 0.78 || jagged > 0.42) && hashInt(seed, 500 + index, 3) === 0) {
    distance += 4;
  }

  return clampInt(distance, minDistance, maxDistance);
}

function enforceMinimumDistance(distance: number, minimumDistance: number, seed: string, index: number): number {
  if (distance >= minimumDistance) {
    return clampInt(distance, 1, 48);
  }

  const extra = hashInt(seed, 700 + index, 3) * 4;

  return clampInt(minimumDistance + extra, minimumDistance, 48);
}

export function calculateRouteDiagnostics(route: readonly number[]): RouteDiagnostics {
  const distances = route.slice(0, -1).map((anchorIndex, index) => ringDistance(anchorIndex, route[index + 1]));

  if (distances.length === 0) {
    return {
      longJumpCount: 0,
      avgJumpDistance: 0,
      tinyJumpCount: 0,
    };
  }

  return {
    longJumpCount: distances.filter((distance) => distance >= 32).length,
    avgJumpDistance: round(average(distances), 2),
    tinyJumpCount: distances.filter((distance) => distance <= 3).length,
  };
}

function classifyMovement(from: AnchorPoint, to: AnchorPoint, home: AnchorPoint): MovementKind {
  const distance = ringDistance(from.anchorIndex, to.anchorIndex);

  if (distance <= 16) {
    return "local";
  }

  if (distance < 32 || circularDistance(home.sideIndex, to.sideIndex, SIDE_COUNT) <= 2) {
    return "medium";
  }

  return "dramatic";
}

function colorForMotion(
  home: AnchorPoint,
  to: AnchorPoint,
  movementKind: MovementKind,
  flowScore: number,
  index: number,
): string {
  const bleed = 1 - flowScore;

  if (movementKind === "local") {
    const neighborOffset = index % 2 === 0 ? 1 : -1;
    const softNeighbor = SIDE_COLORS[(home.sideIndex + neighborOffset + SIDE_COUNT) % SIDE_COUNT];

    return mixHex(home.color, softNeighbor, 0.04 + bleed * 0.12);
  }

  if (movementKind === "medium") {
    return mixHex(home.color, to.color, 0.3 + bleed * 0.26);
  }

  const complementaryColor = SIDE_COLORS[(home.sideIndex + SIDE_COUNT / 2) % SIDE_COUNT];

  return mixHex(home.color, complementaryColor, 0.34 + bleed * 0.3);
}

function safeCurve(
  a: AnchorPoint,
  b: AnchorPoint,
  bias: number,
  geometry: LoomGeometry,
  sessionHash: string,
  segmentIndex: number,
): string {
  const distance = distanceFromCenterToSegment(a, b, geometry);

  if (distance > geometry.forbiddenRadius + 30) {
    const mx = (a.x + b.x) / 2;
    const my = (a.y + b.y) / 2;
    const qx = mx + (geometry.center.x - mx) * 0.03;
    const qy = my + (geometry.center.y - my) * 0.03;

    return `M ${formatPoint(a)} Q ${round(qx, 2)} ${round(qy, 2)} ${formatPoint(b)}`;
  }

  const aa = angleOf(a, geometry);
  const ba = angleOf(b, geometry);
  const mid = (aa + ba) / 2 + bias * 70;
  const bendRadius = bendRadiusForSegment(a, b, geometry, sessionHash, segmentIndex);
  const bendRadiusMax = bendRadiusRange(geometry).max;
  const bendCrownRadius = clamp(bendRadius + 24 * geometryScale(geometry), bendRadius, bendRadiusMax);

  // The center void is silence: any chord that would cross the forbidden radius bends
  // around a deterministic safety radius so the black center never hides a broken path.
  const c1 = polar(geometry.center, aa + bias * 24, bendRadius);
  const c2 = polar(geometry.center, mid, bendCrownRadius);
  const c3 = polar(geometry.center, ba - bias * 24, bendRadius);

  return `M ${formatPoint(a)} C ${formatPoint(c1)}, ${formatPoint(c2)}, ${formatPoint(c3)} S ${formatPoint(
    b,
  )}, ${formatPoint(b)}`;
}

function distanceFromCenterToSegment(a: AnchorPoint, b: AnchorPoint, geometry: LoomGeometry): number {
  const vx = b.x - a.x;
  const vy = b.y - a.y;
  const wx = geometry.center.x - a.x;
  const wy = geometry.center.y - a.y;
  const len2 = vx * vx + vy * vy;
  const t = len2 === 0 ? 0 : clamp((wx * vx + wy * vy) / len2, 0, 1);
  const px = a.x + t * vx;
  const py = a.y + t * vy;

  return Math.hypot(geometry.center.x - px, geometry.center.y - py);
}

function bendRadiusForSegment(
  a: AnchorPoint,
  b: AnchorPoint,
  geometry: LoomGeometry,
  sessionHash: string,
  segmentIndex: number,
): number {
  const { min, max } = bendRadiusRange(geometry);
  const travelFactor = ringDistance(a.anchorIndex, b.anchorIndex) / (TOTAL_ANCHORS / 2);
  const seededVariation = hashUnit(`${RENDER_VERSION}:bend:${sessionHash}`, segmentIndex);
  const radius = min + travelFactor * 74 * geometryScale(geometry) + seededVariation * 31 * geometryScale(geometry);

  return round(clamp(radius, min, max), 2);
}

function bendRadiusRange(geometry: LoomGeometry): { min: number; max: number } {
  const scale = geometryScale(geometry);

  return {
    min: 180 * scale,
    max: 285 * scale,
  };
}

function geometryScale(geometry: LoomGeometry): number {
  return geometry.bendRadius / 180;
}

function angleOf(point: AnchorPoint, geometry: LoomGeometry): number {
  return (Math.atan2(point.y - geometry.center.y, point.x - geometry.center.x) * 180) / Math.PI;
}

function hashInt(hash: string, offset: number, mod: number): number {
  if (mod <= 0) {
    throw new RangeError("mod must be positive.");
  }

  return stableHash(`${hash}:${offset}`) % mod;
}

function hashUnit(hash: string, offset: number): number {
  return hashInt(hash, offset, 1_000_000) / 1_000_000;
}

function stableHash(value: string): number {
  let hash = 2166136261;

  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }

  return hash >>> 0;
}

function flowOpacityMultiplierForScore(score: number): number {
  if (score < 0.38) {
    return 0.55;
  }

  if (score < 0.68) {
    return 0.85;
  }

  return 1.25;
}

function average(values: readonly number[]): number {
  if (values.length === 0) {
    return 0;
  }

  return values.reduce((total, value) => total + value, 0) / values.length;
}

function formatPoint(point: { x: number; y: number }): string {
  return `${round(point.x, 2)} ${round(point.y, 2)}`;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function clampInt(value: number, min: number, max: number): number {
  return Math.round(clamp(value, min, max));
}

function circularDistance(from: number, to: number, count: number): number {
  const distance = Math.abs(from - to) % count;

  return Math.min(distance, count - distance);
}

function ringDistance(from: number, to: number): number {
  return circularDistance(from, to, TOTAL_ANCHORS);
}

function wrapAnchor(value: number): number {
  return ((value % TOTAL_ANCHORS) + TOTAL_ANCHORS) % TOTAL_ANCHORS;
}

function mixHex(from: string, to: string, amount: number): string {
  const fromRgb = hexToRgb(from);
  const toRgb = hexToRgb(to);
  const mixAmount = clamp(amount, 0, 1);
  const mixed = fromRgb.map((channel, index) => Math.round(channel + (toRgb[index] - channel) * mixAmount));

  return `#${mixed.map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
}

function hexToRgb(hex: string): [number, number, number] {
  const normalized = hex.replace(/^#/, "");

  if (!/^[0-9a-f]{6}$/i.test(normalized)) {
    throw new Error(`Unsupported color value: ${hex}`);
  }

  return [
    Number.parseInt(normalized.slice(0, 2), 16),
    Number.parseInt(normalized.slice(2, 4), 16),
    Number.parseInt(normalized.slice(4, 6), 16),
  ];
}

function round(value: number, decimals: number): number {
  const scale = 10 ** decimals;
  return Math.round(value * scale) / scale;
}
