export type AnkyLocalState =
  | "drafting"
  | "closed"
  | "verified"
  | "sealed"
  | "proving"
  | "proof_verified"
  | "proof_failed"
  | "processed";

type LocalStateInput = {
  artifactCount?: number;
  closed: boolean;
  hashMatches: boolean;
  proofStatus?: "failed" | "none" | "proving" | "syncing" | "unavailable" | "verified";
  sealCount?: number;
  valid: boolean;
};

export function resolveAnkyLocalState({
  artifactCount = 0,
  closed,
  hashMatches,
  proofStatus = "none",
  sealCount = 0,
  valid,
}: LocalStateInput): AnkyLocalState {
  if (!closed) {
    return "drafting";
  }

  if (proofStatus === "verified") {
    return "proof_verified";
  }

  if (proofStatus === "proving" || proofStatus === "syncing") {
    return "proving";
  }

  if (proofStatus === "failed") {
    return "proof_failed";
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
