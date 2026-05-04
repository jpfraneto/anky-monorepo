export const SIDE_COUNT = 8;
export const ANCHORS_PER_SIDE = 12;
export const TOTAL_ANCHORS = SIDE_COUNT * ANCHORS_PER_SIDE;

export const DEFAULT_WIDTH = 1200;
export const DEFAULT_HEIGHT = 1200;
export const OUTER_RADIUS = 430;
export const BACKGROUND_OUTER_RADIUS = 500;
export const BACKGROUND_INNER_RADIUS = 360;
export const FORBIDDEN_RADIUS = 136;
export const CENTER_RING_RADIUS = 112;
export const BEND_RADIUS = 180;

export const SIDE_COLORS = [
  "#ff3b30",
  "#ff8a00",
  "#ffd60a",
  "#34c759",
  "#0a84ff",
  "#5e5ce6",
  "#bf5af2",
  "#f2f2f7",
] as const;

export interface Point {
  x: number;
  y: number;
}

export interface AnchorPoint extends Point {
  anchorIndex: number;
  dayIndex: number;
  sideIndex: number;
  sideAnchorIndex: number;
  color: string;
  angleRadians: number;
}

export interface LoomGeometry {
  width: number;
  height: number;
  center: Point;
  outerRadius: number;
  backgroundOuterRadius: number;
  backgroundInnerRadius: number;
  forbiddenRadius: number;
  centerRingRadius: number;
  bendRadius: number;
  framePoints: Point[];
  anchors: AnchorPoint[];
}

export function createOctagonalLoom(width = DEFAULT_WIDTH, height = DEFAULT_HEIGHT): LoomGeometry {
  const center = { x: width / 2, y: height / 2 };
  const scale = Math.min(width / DEFAULT_WIDTH, height / DEFAULT_HEIGHT);
  const outerRadius = OUTER_RADIUS * scale;
  const backgroundOuterRadius = BACKGROUND_OUTER_RADIUS * scale;
  const backgroundInnerRadius = BACKGROUND_INNER_RADIUS * scale;
  const forbiddenRadius = FORBIDDEN_RADIUS * scale;
  const centerRingRadius = CENTER_RING_RADIUS * scale;
  const bendRadius = BEND_RADIUS * scale;
  const sideAngles = Array.from({ length: SIDE_COUNT }, (_, index) => -112.5 + index * 45);

  const framePoints = sideAngles.map((angle) => polar(center, angle, outerRadius));

  const anchors: AnchorPoint[] = [];
  for (let sideIndex = 0; sideIndex < SIDE_COUNT; sideIndex += 1) {
    const from = framePoints[sideIndex];
    const to = framePoints[(sideIndex + 1) % SIDE_COUNT];

    for (let sideAnchorIndex = 0; sideAnchorIndex < ANCHORS_PER_SIDE; sideAnchorIndex += 1) {
      const t = (sideAnchorIndex + 1) / (ANCHORS_PER_SIDE + 1);
      const point = {
        x: lerp(from.x, to.x, t),
        y: lerp(from.y, to.y, t),
      };
      const dayIndex = sideAnchorIndex * SIDE_COUNT + sideIndex + 1;

      anchors.push({
        anchorIndex: anchors.length,
        x: point.x,
        y: point.y,
        dayIndex,
        sideIndex,
        sideAnchorIndex: sideAnchorIndex + 1,
        color: SIDE_COLORS[sideIndex],
        angleRadians: Math.atan2(point.y - center.y, point.x - center.x),
      });
    }
  }

  return {
    width,
    height,
    center,
    outerRadius,
    backgroundOuterRadius,
    backgroundInnerRadius,
    forbiddenRadius,
    centerRingRadius,
    bendRadius,
    framePoints,
    anchors,
  };
}

export function getAnchorByDayIndex(geometry: LoomGeometry, dayIndex: number): AnchorPoint {
  const anchor = geometry.anchors.find((candidate) => candidate.dayIndex === dayIndex);
  if (!anchor) {
    throw new RangeError(`dayIndex must be between 1 and ${TOTAL_ANCHORS}. Received ${dayIndex}.`);
  }

  return anchor;
}

export function polar(center: Point, angleDegrees: number, radius: number): Point {
  const angleRadians = (angleDegrees * Math.PI) / 180;

  return {
    x: center.x + radius * Math.cos(angleRadians),
    y: center.y + radius * Math.sin(angleRadians),
  };
}

function lerp(from: number, to: number, t: number): number {
  return from + (to - from) * t;
}
