import { createMiddleware } from "hono/factory";
import type { Context, Next } from "hono";
import { Logger } from "../lib/logger.js";

const logger = Logger("Subscription");

export const subscriptionMiddleware = createMiddleware(async (c: Context, next: Next) => {
  const auth = c.get("auth");
  if (!auth?.walletAddress) {
    // Anonymous user - allow for now (free session logic)
    return next();
  }

  const dbOps = await import("../db/operations.js");
  const user = await dbOps.getUserByWallet(auth.walletAddress);

  if (!user) {
    return next();
  }

  // Check if user has used free session
  if (!user.freeSessionUsed) {
    // First session is free - mark it as used
    await dbOps.markFreeSessionUsed(user.id);
    logger.info(`User ${user.walletAddress.slice(0, 10)}... using free session`);
    return next();
  }

  // Check active subscription
  const hasSubscription = await dbOps.hasActiveSubscription(user.id);
  if (hasSubscription) {
    return next();
  }

  // No free session and no subscription
  return c.json({
    error: "Subscription required",
    message: "Your free session has been used. Subscribe to continue writing.",
    checkoutUrl: "/api/checkout",
  }, 402);
});
