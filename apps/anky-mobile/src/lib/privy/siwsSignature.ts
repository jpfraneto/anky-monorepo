import { base58, base64 } from "@scure/base";

const SOLANA_SIGNATURE_BYTES = 64;

export function toPrivySiwsSignature(signature: string): string {
  const trimmed = signature.trim();

  if (trimmed.length === 0) {
    throw new Error("wallet returned an empty signature.");
  }

  const base58Signature = decodeSignature(trimmed, "base58");

  if (base58Signature != null) {
    return base64.encode(base58Signature);
  }

  const base64Signature = decodeSignature(trimmed, "base64");

  if (base64Signature != null) {
    return base64.encode(base64Signature);
  }

  throw new Error("wallet returned an invalid Solana signature.");
}

function decodeSignature(value: string, encoding: "base58" | "base64"): Uint8Array | null {
  try {
    const decoded = encoding === "base58" ? base58.decode(value) : base64.decode(value);

    return decoded.length === SOLANA_SIGNATURE_BYTES ? decoded : null;
  } catch {
    return null;
  }
}
