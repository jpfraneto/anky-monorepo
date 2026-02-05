import { createMiddleware } from "hono/factory";
import { PrivyClient } from "@privy-io/server-auth";
import type { Context, Next } from "hono";
import { Logger } from "../lib/logger";

const logger = Logger("Auth");

// Initialize Privy client
const privyAppId = process.env.PRIVY_APP_ID;
const privyAppSecret = process.env.PRIVY_APP_SECRET;

let privyClient: PrivyClient | null = null;

if (privyAppId && privyAppSecret) {
  privyClient = new PrivyClient(privyAppId, privyAppSecret);
  logger.info("Privy authentication initialized");
} else {
  logger.warn("PRIVY_APP_ID or PRIVY_APP_SECRET not set - auth middleware disabled");
}

// Type for authenticated context
export interface AuthContext {
  userId: string; // Privy user ID (did:privy:xxx)
  walletAddress?: string;
}

// Extend Hono context with auth
declare module "hono" {
  interface ContextVariableMap {
    auth: AuthContext;
  }
}

/**
 * Authentication middleware that validates Privy access tokens.
 *
 * The frontend should send the token in the Authorization header:
 * Authorization: Bearer <privy_access_token>
 *
 * On success, sets c.var.auth with { userId, walletAddress }
 */
export const authMiddleware = createMiddleware(async (c: Context, next: Next) => {
  // Skip auth if Privy is not configured (for development)
  if (!privyClient) {
    logger.debug("Auth skipped - Privy not configured");
    // Set a placeholder auth context for development
    c.set("auth", {
      userId: "dev-user",
      walletAddress: undefined,
    });
    return next();
  }

  const authHeader = c.req.header("Authorization");

  if (!authHeader || !authHeader.startsWith("Bearer ")) {
    logger.debug("Missing or invalid Authorization header");
    return c.json({ error: "Missing or invalid Authorization header" }, 401);
  }

  const token = authHeader.replace("Bearer ", "");

  try {
    // Verify the access token with Privy
    const verifiedClaims = await privyClient.verifyAuthToken(token);

    // Get user details to extract wallet address
    let walletAddress: string | undefined;

    try {
      const user = await privyClient.getUser(verifiedClaims.userId);

      // Find the first linked wallet
      const wallet = user.linkedAccounts?.find(
        (account) => account.type === "wallet"
      );

      if (wallet && "address" in wallet) {
        walletAddress = wallet.address.toLowerCase();
      }
    } catch (userError) {
      // User fetch failed, continue without wallet address
      logger.warn("Could not fetch user details", userError);
    }

    // Set auth context
    c.set("auth", {
      userId: verifiedClaims.userId,
      walletAddress,
    });

    logger.debug(`Authenticated user: ${verifiedClaims.userId.slice(0, 20)}...`);
    return next();
  } catch (error) {
    logger.error("Auth token verification failed", error);
    return c.json({ error: "Invalid or expired token" }, 401);
  }
});

/**
 * Optional auth middleware - doesn't fail if no token provided.
 * Useful for endpoints that work for both authenticated and anonymous users.
 */
export const optionalAuthMiddleware = createMiddleware(async (c: Context, next: Next) => {
  // Skip auth if Privy is not configured
  if (!privyClient) {
    c.set("auth", {
      userId: "dev-user",
      walletAddress: undefined,
    });
    return next();
  }

  const authHeader = c.req.header("Authorization");

  // No token provided - continue without auth
  if (!authHeader || !authHeader.startsWith("Bearer ")) {
    return next();
  }

  const token = authHeader.replace("Bearer ", "");

  try {
    const verifiedClaims = await privyClient.verifyAuthToken(token);

    let walletAddress: string | undefined;

    try {
      const user = await privyClient.getUser(verifiedClaims.userId);
      const wallet = user.linkedAccounts?.find(
        (account) => account.type === "wallet"
      );
      if (wallet && "address" in wallet) {
        walletAddress = wallet.address.toLowerCase();
      }
    } catch {
      // Continue without wallet
    }

    c.set("auth", {
      userId: verifiedClaims.userId,
      walletAddress,
    });

    logger.debug(`Optional auth: user ${verifiedClaims.userId.slice(0, 20)}...`);
  } catch {
    // Token invalid, continue without auth (don't fail)
    logger.debug("Optional auth: invalid token, continuing without auth");
  }

  return next();
});

/**
 * Helper to get the current user's wallet address from the auth context.
 * Returns undefined if not authenticated or no wallet linked.
 */
export function getAuthWallet(c: Context): string | undefined {
  return c.get("auth")?.walletAddress;
}

/**
 * Helper to get the current user's Privy ID from the auth context.
 * Returns undefined if not authenticated.
 */
export function getAuthUserId(c: Context): string | undefined {
  return c.get("auth")?.userId;
}
