import { createClient, Errors } from "@farcaster/quick-auth";
import type { Context, Next } from "hono";

const client = createClient();

const domain = "miniapp.anky.app";

export const siwfAuthMiddleware = async (c: Context, next: Next) => {
  console.log("🔑 Checking authorization header...");
  const authorization = c.req.header("Authorization");

  if (!authorization || !authorization.startsWith("Bearer ")) {
    console.log("❌ Authorization header missing or invalid");
    return c.json(
      { error: "Unauthorized - Missing or invalid token format" },
      401
    );
  }

  try {
    console.log("🔍 Extracting token from header...");
    const token = authorization.split(" ")[1]!;
    console.log("🔐 Verifying JWT token...");
    const payload = await client.verifyJwt({
      token,
      domain,
    });

    console.log("✅ Token verified successfully!", payload);
    // Add verified user data to context for route handlers
    c.set("user", {
      fid: payload.sub,
      address: payload.address,
    });
    console.log("👤 User data added to context:", {
      fid: payload.sub,
      address: payload.address,
    });

    await next();
  } catch (e) {
    if (e instanceof Errors.InvalidTokenError) {
      console.log("🚫 Invalid token error:", e);
      return c.json({ error: "Unauthorized - Invalid token" }, 401);
    }
    console.log("💥 Internal server error:", e);
    return c.json({ error: "Internal server error" }, 500);
  }
};
