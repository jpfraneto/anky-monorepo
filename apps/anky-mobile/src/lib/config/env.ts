declare const process:
  | {
      env?: Record<string, string | undefined>;
    }
  | undefined;

export function getPublicEnv(name: string): string | undefined {
  if (typeof process === "undefined") {
    return undefined;
  }

  const value = process.env?.[name];

  return value == null || value.trim().length === 0 ? undefined : value;
}
