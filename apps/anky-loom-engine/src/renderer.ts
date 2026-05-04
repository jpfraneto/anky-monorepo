import { ANCHORS_PER_SIDE, LoomGeometry, SIDE_COUNT } from "./geometry";
import { buildPinStates } from "./pins";
import type { LoomPinState } from "./pins";
import { LoomThread, LoomThreadSegment } from "./thread";

export type RenderMode = "presence" | "weave" | "flow" | "time";

export interface RenderOptions {
  mode?: RenderMode;
}

interface OriginHaloStyle {
  outerRadius: number;
  outerFillOpacity: number;
  rippleRadius: number;
  rippleOpacity: number;
  bodyRadius: number;
  bodyFillOpacity: number;
  bodyStrokeOpacity: number;
  bodyStrokeWidth: number;
  coreRadius: number;
  coreOpacity: number;
}

const RENDER_MODES: readonly RenderMode[] = ["presence", "weave", "flow", "time"];

export function renderLoomSvg(
  geometry: LoomGeometry,
  threads: readonly LoomThread[],
  pinStates = buildPinStates(geometry, threads),
  options: RenderOptions = {},
): string {
  const mode = options.mode ?? "weave";
  const orderedThreads = sortThreadsByDay(threads);
  const timeAlphaByHash = buildTimeAlphaBySessionHash(orderedThreads, mode);
  const paths = orderedThreads
    .flatMap((thread) =>
      thread.segments.map((segment) =>
        renderThreadSegment(segment, thread, timeAlphaByHash.get(thread.sessionHash) ?? 1, mode),
      ),
    )
    .join("\n");

  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${geometry.width}" height="${geometry.height}" viewBox="0 0 ${geometry.width} ${geometry.height}">`,
    `  <defs>`,
    `    <radialGradient id="bg" cx="50%" cy="50%" r="58%">`,
    `      <stop offset="0%" stop-color="#151326"/>`,
    `      <stop offset="55%" stop-color="#090812"/>`,
    `      <stop offset="100%" stop-color="#020207"/>`,
    `    </radialGradient>`,
    `    <filter id="softGlow" x="-80%" y="-80%" width="260%" height="260%">`,
    `      <feGaussianBlur stdDeviation="7" result="blur"/>`,
    `      <feMerge>`,
    `        <feMergeNode in="blur"/>`,
    `        <feMergeNode in="SourceGraphic"/>`,
    `      </feMerge>`,
    `    </filter>`,
    `    <filter id="threadGlow" x="-40%" y="-40%" width="180%" height="180%">`,
    `      <feGaussianBlur stdDeviation="1.05" result="blur"/>`,
    `      <feMerge>`,
    `        <feMergeNode in="blur"/>`,
    `        <feMergeNode in="SourceGraphic"/>`,
    `      </feMerge>`,
    `    </filter>`,
    `  </defs>`,
    ``,
    `  <rect width="${geometry.width}" height="${geometry.height}" fill="url(#bg)"/>`,
    `  <circle cx="${formatNumber(geometry.center.x)}" cy="${formatNumber(geometry.center.y)}" r="${formatNumber(
      geometry.backgroundOuterRadius,
    )}" fill="none" stroke="#ffffff" stroke-opacity="0.035" stroke-width="1"/>`,
    `  <circle cx="${formatNumber(geometry.center.x)}" cy="${formatNumber(geometry.center.y)}" r="${formatNumber(
      geometry.backgroundInnerRadius,
    )}" fill="none" stroke="#ffffff" stroke-opacity="0.04" stroke-width="1"/>`,
    ``,
    `  <g id="cycle-guides">`,
    renderCycleGuides(geometry),
    `  </g>`,
    ``,
    `  <g id="threads" filter="url(#threadGlow)">`,
    paths,
    `  </g>`,
    ``,
    `  <g id="octagon-frame" filter="url(#softGlow)">`,
    `    <polygon points="${formatPoints(geometry.framePoints)}" fill="none" stroke="#ffffff" stroke-width="1.2" stroke-opacity="0.18"/>`,
    renderFrameLines(geometry),
    `  </g>`,
    ``,
    `  <g id="pins">`,
    renderPins(geometry, pinStates, timeAlphaByHash, mode),
    `  </g>`,
    ``,
    `  <g id="center-void">`,
    `    <circle cx="${formatNumber(geometry.center.x)}" cy="${formatNumber(geometry.center.y)}" r="${formatNumber(
      geometry.forbiddenRadius,
    )}" fill="#05050b" fill-opacity="0.86" stroke="#ffffff" stroke-opacity="0.10" stroke-width="1.3"/>`,
    `    <circle cx="${formatNumber(geometry.center.x)}" cy="${formatNumber(geometry.center.y)}" r="${formatNumber(
      geometry.centerRingRadius,
    )}" fill="none" stroke="#ffffff" stroke-opacity="0.05" stroke-width="1"/>`,
    `  </g>`,
    ``,
    `  <!-- Canonical mapping:`,
    `       day = (pin - 1) * 8 + side + 1`,
    `       side 0..7 = red, orange, yellow, green, blue, indigo, violet, white`,
    `       pin 1..12 = the 12 appearances of that resonance across the 96-day Sojourn.`,
    `  -->`,
    `</svg>`,
    "",
  ].join("\n");
}

export function sortThreadsByDay(threads: readonly LoomThread[]): LoomThread[] {
  return [...threads].sort(
    (left, right) => left.dayIndex - right.dayIndex || left.sessionHash.localeCompare(right.sessionHash),
  );
}

export function isRenderMode(value: string): value is RenderMode {
  return RENDER_MODES.includes(value as RenderMode);
}

export function parseRenderMode(value: string): RenderMode {
  if (isRenderMode(value)) {
    return value;
  }

  throw new Error(`Unknown render mode: ${value}. Expected one of: ${renderModeList()}.`);
}

export function renderModeList(): string {
  return RENDER_MODES.join(", ");
}

function renderCycleGuides(geometry: LoomGeometry): string {
  return Array.from({ length: ANCHORS_PER_SIDE }, (_, pinIndex) => {
    const points = Array.from({ length: SIDE_COUNT }, (_, sideIndex) => {
      const anchor = geometry.anchors.find(
        (candidate) => candidate.sideIndex === sideIndex && candidate.sideAnchorIndex === pinIndex + 1,
      );

      if (!anchor) {
        throw new Error(`Missing anchor for side ${sideIndex} pin ${pinIndex + 1}.`);
      }

      return anchor;
    });

    const opacity = 0.035 + pinIndex * 0.004;
    return `    <polygon points="${formatPoints(points)}" fill="none" stroke="#ffffff" stroke-width="0.8" stroke-opacity="${formatDecimal(
      opacity,
      3,
    )}"/>`;
  }).join("\n");
}

function renderFrameLines(geometry: LoomGeometry): string {
  return geometry.framePoints
    .map((from, index) => {
      const to = geometry.framePoints[(index + 1) % geometry.framePoints.length];
      const color = geometry.anchors.find((anchor) => anchor.sideIndex === index)?.color ?? "#ffffff";

      return `<line x1="${formatDecimal(from.x, 2)}" y1="${formatDecimal(from.y, 2)}" x2="${formatDecimal(
        to.x,
        2,
      )}" y2="${formatDecimal(to.y, 2)}" stroke="${color}" stroke-width="5" stroke-opacity="0.52" stroke-linecap="round"/>`;
    })
    .join("\n");
}

function renderPins(
  geometry: LoomGeometry,
  pinStates: readonly LoomPinState[],
  timeAlphaByHash: ReadonlyMap<string, number>,
  mode: RenderMode,
): string {
  const pinStateByIndex = new Map(pinStates.map((pinState) => [pinState.pinIndex, pinState]));

  return geometry.anchors
    .map((anchor) => {
      const pinState = pinStateByIndex.get(anchor.anchorIndex) ?? {
        day: anchor.dayIndex,
        pinIndex: anchor.anchorIndex,
        state: "empty" as const,
        visitCount: 0,
      };

      return renderPin(anchor.x, anchor.y, anchor.color, pinState, timeAlphaByHash, mode);
    })
    .join("\n");
}

function renderPin(
  x: number,
  y: number,
  color: string,
  pinState: LoomPinState,
  timeAlphaByHash: ReadonlyMap<string, number>,
  mode: RenderMode,
): string {
  const cx = formatDecimal(x, 2);
  const cy = formatDecimal(y, 2);

  if (pinState.state === "origin") {
    const timeAlpha = pinState.originSessionHash ? timeAlphaByHash.get(pinState.originSessionHash) ?? 1 : 1;
    const halo = originHaloForBucket(pinState.originFlowBucket, mode, timeAlpha);

    return [
      `    <g class="pin pin-origin" filter="url(#softGlow)">`,
      `      <circle cx="${cx}" cy="${cy}" r="${formatDecimal(halo.outerRadius, 2)}" fill="${color}" fill-opacity="${formatDecimal(
        halo.outerFillOpacity,
        3,
      )}"/>`,
      `      <circle cx="${cx}" cy="${cy}" r="${formatDecimal(halo.rippleRadius, 2)}" fill="none" stroke="${color}" stroke-opacity="${formatDecimal(
        halo.rippleOpacity,
        3,
      )}" stroke-width="1.1"/>`,
      `      <circle cx="${cx}" cy="${cy}" r="${formatDecimal(halo.bodyRadius, 2)}" fill="${color}" fill-opacity="${formatDecimal(
        halo.bodyFillOpacity,
        3,
      )}" stroke="${color}" stroke-opacity="${formatDecimal(halo.bodyStrokeOpacity, 3)}" stroke-width="${formatDecimal(
        halo.bodyStrokeWidth,
        2,
      )}"/>`,
      `      <circle cx="${cx}" cy="${cy}" r="${formatDecimal(halo.coreRadius, 2)}" fill="${color}" fill-opacity="${formatDecimal(
        halo.coreOpacity,
        3,
      )}"/>`,
      `    </g>`,
    ].join("\n");
  }

  if (pinState.state === "visited") {
    const visitWeight = clamp(pinState.visitCount, 1, 6) / 6;
    const visitedOpacityMultiplier = mode === "presence" ? 0.72 : mode === "time" ? 0.88 : 1;

    return [
      `    <g class="pin pin-visited">`,
      `      <circle cx="${cx}" cy="${cy}" r="${formatDecimal(5.4 + visitWeight * 1.4, 2)}" fill="${color}" fill-opacity="${formatDecimal(
        (0.08 + visitWeight * 0.08) * visitedOpacityMultiplier,
        3,
      )}" stroke="${color}" stroke-opacity="${formatDecimal(
        (0.28 + visitWeight * 0.18) * visitedOpacityMultiplier,
        3,
      )}" stroke-width="1.05"/>`,
      `      <circle cx="${cx}" cy="${cy}" r="${formatDecimal(2.2 + visitWeight * 0.6, 2)}" fill="${color}" fill-opacity="${formatDecimal(
        (0.34 + visitWeight * 0.18) * visitedOpacityMultiplier,
        3,
      )}"/>`,
      `    </g>`,
    ].join("\n");
  }

  return [
    `    <g class="pin pin-empty">`,
    `      <circle cx="${cx}" cy="${cy}" r="3.6" fill="none" stroke="${color}" stroke-opacity="0.18" stroke-width="0.95"/>`,
    `      <circle cx="${cx}" cy="${cy}" r="1.35" fill="${color}" fill-opacity="0.10"/>`,
    `    </g>`,
  ].join("\n");
}

function originHaloForBucket(
  bucket: LoomPinState["originFlowBucket"],
  mode: RenderMode,
  timeAlpha: number,
): OriginHaloStyle {
  const flowRadiusMultiplier = bucket === "high" ? 1.14 : bucket === "low" ? 0.9 : 1;
  const flowOpacityMultiplier = bucket === "high" ? 1.32 : bucket === "low" ? 0.72 : 1;
  const modeRadiusMultiplier = mode === "presence" ? 1.22 : mode === "flow" ? 1.08 : mode === "time" ? 0.96 : 1;
  const modeOpacityMultiplier = mode === "presence" ? 1.24 : mode === "flow" ? 1.18 : mode === "time" ? 0.94 : 1;
  const timeRadiusMultiplier = mode === "time" ? 0.88 + timeAlpha * 0.16 : 1;
  const timeOpacityMultiplier = mode === "time" ? timeAlpha : 1;
  const radiusMultiplier = flowRadiusMultiplier * modeRadiusMultiplier * timeRadiusMultiplier;
  const opacityMultiplier = flowOpacityMultiplier * modeOpacityMultiplier * timeOpacityMultiplier;
  const base =
    bucket === "high"
      ? { outerRadius: 17.5, outerFillOpacity: 0.17, rippleRadius: 14.2, rippleOpacity: 0.5 }
      : bucket === "low"
        ? { outerRadius: 13.5, outerFillOpacity: 0.09, rippleRadius: 11.4, rippleOpacity: 0.28 }
        : { outerRadius: 15.5, outerFillOpacity: 0.13, rippleRadius: 12.8, rippleOpacity: 0.4 };

  return {
    outerRadius: base.outerRadius * radiusMultiplier,
    outerFillOpacity: clamp(base.outerFillOpacity * opacityMultiplier, 0.045, 0.34),
    rippleRadius: base.rippleRadius * radiusMultiplier,
    rippleOpacity: clamp(base.rippleOpacity * opacityMultiplier, 0.16, 0.86),
    bodyRadius: 9.4 * radiusMultiplier,
    bodyFillOpacity: clamp(0.34 * opacityMultiplier, 0.18, 0.62),
    bodyStrokeOpacity: clamp(0.86 * opacityMultiplier, 0.42, 1),
    bodyStrokeWidth: mode === "presence" ? 1.75 : mode === "flow" ? 1.6 : 1.45,
    coreRadius: 4.9 * radiusMultiplier,
    coreOpacity: clamp(0.98 * opacityMultiplier, 0.58, 1),
  };
}

function renderThreadSegment(
  segment: LoomThreadSegment,
  thread: LoomThread,
  timeAlpha: number,
  mode: RenderMode,
): string {
  const opacity = clamp(segment.opacity * threadOpacityMultiplier(thread, mode) * timeAlpha, 0.035, 0.42);
  const strokeWidth = segment.strokeWidth * threadStrokeWidthMultiplier(thread, mode);

  return `<path d="${segment.path}" fill="none" stroke="${segment.strokeColor}" stroke-width="${formatDecimal(
    strokeWidth,
    2,
  )}" stroke-opacity="${formatDecimal(opacity, 3)}" stroke-linecap="round" stroke-linejoin="round"/>`;
}

function threadOpacityMultiplier(thread: LoomThread, mode: RenderMode): number {
  const flowMultiplier = flowOpacityMultiplier(thread.flowBucket, mode);

  if (mode === "presence") {
    return 0.42 * flowMultiplier;
  }

  if (mode === "flow") {
    return 0.88 * flowMultiplier;
  }

  if (mode === "time") {
    return 0.9 * flowMultiplier;
  }

  return flowMultiplier;
}

function threadStrokeWidthMultiplier(thread: LoomThread, mode: RenderMode): number {
  if (mode === "presence") {
    return 0.82;
  }

  if (mode === "flow") {
    return thread.flowBucket === "high" ? 1.16 : thread.flowBucket === "low" ? 0.88 : 1;
  }

  return 1;
}

function flowOpacityMultiplier(bucket: LoomThread["flowBucket"], mode: RenderMode): number {
  if (mode === "flow") {
    if (bucket === "high") return 1.42;
    if (bucket === "low") return 0.58;
    return 1;
  }

  if (bucket === "high") return 1.08;
  if (bucket === "low") return 0.9;
  return 1;
}

function buildTimeAlphaBySessionHash(threads: readonly LoomThread[], mode: RenderMode): Map<string, number> {
  const alphaByHash = new Map<string, number>();

  threads.forEach((thread, index) => {
    alphaByHash.set(thread.sessionHash, timeAlphaForIndex(index, threads.length, mode));
  });

  return alphaByHash;
}

function timeAlphaForIndex(index: number, count: number, mode: RenderMode): number {
  if (count <= 1) {
    return 1;
  }

  const t = index / (count - 1);
  const min = mode === "time" ? 0.62 : 0.84;
  const max = mode === "time" ? 1.34 : 1.12;

  return min + (max - min) * t;
}

function formatPoints(points: readonly { x: number; y: number }[]): string {
  return points.map((point) => `${formatDecimal(point.x, 2)},${formatDecimal(point.y, 2)}`).join(" ");
}

function formatNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : formatDecimal(value, 2);
}

function formatDecimal(value: number, decimals: number): string {
  return value.toFixed(decimals);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
