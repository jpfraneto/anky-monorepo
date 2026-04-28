export type AnkyLocalState = "drafting" | "closed" | "verified" | "sealed" | "processed";

type LocalStateInput = {
  artifactCount?: number;
  closed: boolean;
  hashMatches: boolean;
  sealCount?: number;
  valid: boolean;
};

export function resolveAnkyLocalState({
  artifactCount = 0,
  closed,
  hashMatches,
  sealCount = 0,
  valid,
}: LocalStateInput): AnkyLocalState {
  if (!closed) {
    return "drafting";
  }

  if (artifactCount > 0) {
    return "processed";
  }

  if (sealCount > 0) {
    return "sealed";
  }

  if (valid && hashMatches) {
    return "verified";
  }

  return "closed";
}
