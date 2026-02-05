import { createMiddleware } from "hono/factory";
import type { Context, Next } from "hono";

interface RateLimitEntry {
  count: number;
  resetAt: number;
}

const stores = new Map<string, Map<string, RateLimitEntry>>();

// Cleanup stale entries every 5 minutes
setInterval(() => {
  const now = Date.now();
  for (const [, store] of stores) {
    for (const [key, entry] of store) {
      if (entry.resetAt <= now) {
        store.delete(key);
      }
    }
  }
}, 5 * 60 * 1000);

function createRateLimiter(config: {
  name: string;
  max: number;
  windowMs: number;
  keyFn: (c: Context) => string;
}) {
  const store = new Map<string, RateLimitEntry>();
  stores.set(config.name, store);

  return createMiddleware(async (c: Context, next: Next) => {
    const key = config.keyFn(c);
    const now = Date.now();

    let entry = store.get(key);
    if (!entry || entry.resetAt <= now) {
      entry = { count: 0, resetAt: now + config.windowMs };
      store.set(key, entry);
    }

    entry.count++;

    const remaining = Math.max(0, config.max - entry.count);
    const resetSeconds = Math.ceil((entry.resetAt - now) / 1000);

    c.res.headers.set("X-RateLimit-Limit", String(config.max));
    c.res.headers.set("X-RateLimit-Remaining", String(remaining));
    c.res.headers.set("X-RateLimit-Reset", String(resetSeconds));

    if (entry.count > config.max) {
      return c.json(
        { error: "Too many requests", retryAfter: resetSeconds },
        429,
      );
    }

    await next();

    // Set headers on actual response too
    c.res.headers.set("X-RateLimit-Limit", String(config.max));
    c.res.headers.set("X-RateLimit-Remaining", String(Math.max(0, config.max - entry.count)));
    c.res.headers.set("X-RateLimit-Reset", String(Math.ceil((entry.resetAt - Date.now()) / 1000)));
  });
}

function getClientIp(c: Context): string {
  return (
    c.req.header("x-forwarded-for")?.split(",")[0]?.trim() ||
    c.req.header("x-real-ip") ||
    "unknown"
  );
}

// 5 registrations per hour per IP
export const agentRegisterLimiter = createRateLimiter({
  name: "agent-register",
  max: 5,
  windowMs: 60 * 60 * 1000,
  keyFn: getClientIp,
});

// 20 session submissions per hour per agent
export const sessionSubmitLimiter = createRateLimiter({
  name: "session-submit",
  max: 20,
  windowMs: 60 * 60 * 1000,
  keyFn: (c) => c.get("agent")?.agentId || getClientIp(c),
});

// 100 reads per minute per IP
export const publicReadLimiter = createRateLimiter({
  name: "public-read",
  max: 100,
  windowMs: 60 * 1000,
  keyFn: getClientIp,
});

// 30 AI generation requests per hour per IP (for /prompt, /reflection, /image, /title, /chat)
export const aiGenerationLimiter = createRateLimiter({
  name: "ai-generation",
  max: 30,
  windowMs: 60 * 60 * 1000,
  keyFn: getClientIp,
});
