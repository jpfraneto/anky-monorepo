export const RHYTHM_SAMPLE_COUNT = 16;

export type FlowBucket = "low" | "medium" | "high";

export interface RhythmPattern {
  samples: number[];
}

export function createRhythmPattern(deltas: readonly number[], sampleCount = RHYTHM_SAMPLE_COUNT): RhythmPattern {
  if (!Number.isInteger(sampleCount) || sampleCount <= 0) {
    throw new RangeError("sampleCount must be a positive integer.");
  }

  if (deltas.length === 0) {
    return { samples: Array.from({ length: sampleCount }, () => 0.5) };
  }

  const samples = Array.from({ length: sampleCount }, (_, sampleIndex) => {
    const start = Math.floor((sampleIndex / sampleCount) * deltas.length);
    const end = Math.floor(((sampleIndex + 1) / sampleCount) * deltas.length);
    const chunk = deltas.slice(start, Math.max(start + 1, end));
    const avg = chunk.reduce((total, delta) => total + delta, 0) / chunk.length;

    return round(clamp(avg / 1800, 0, 1), 6);
  });

  return { samples };
}

export function calculateFlowScore(deltas: readonly number[]): number {
  if (deltas.length < 5) {
    return 0.35;
  }

  const velocities = deltas.map((delta) => clamp(delta, 40, 7999)).map((delta) => 1000 / delta);
  const mean = velocities.reduce((total, velocity) => total + velocity, 0) / velocities.length;
  const variance =
    velocities.reduce((total, velocity) => total + Math.pow(velocity - mean, 2), 0) / velocities.length;
  const standardDeviation = Math.sqrt(variance);
  const coefficientOfVariation = standardDeviation / Math.max(mean, 0.0001);

  // Flow is private rhythm coherence: steadier keystroke velocity makes the line stronger.
  const score = 1 - Math.min(1, coefficientOfVariation / 1.25);

  return clamp(score, 0.12, 1);
}

export function flowBucketForScore(score: number): FlowBucket {
  if (score < 0.38) {
    return "low";
  }

  if (score < 0.68) {
    return "medium";
  }

  return "high";
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function round(value: number, decimals: number): number {
  const scale = 10 ** decimals;
  return Math.round(value * scale) / scale;
}
