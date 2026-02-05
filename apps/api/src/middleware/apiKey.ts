import { createMiddleware } from "hono/factory";
import type { Context, Next } from "hono";
import { Logger } from "../lib/logger";

const logger = Logger("ApiKey");

// Type for agent context
export interface AgentContext {
  agentId: string;
  agentName: string;
}

// Extend Hono context with agent
declare module "hono" {
  interface ContextVariableMap {
    agent: AgentContext;
  }
}

// Lazy import to avoid circular dependency
let getAgentByApiKeyHash: ((hash: string) => Promise<{
  id: string;
  name: string;
  isActive: boolean;
} | null>) | null = null;

async function loadDbOps() {
  if (!getAgentByApiKeyHash) {
    const dbOps = await import("../db/operations.js");
    getAgentByApiKeyHash = dbOps.getAgentByApiKeyHash;
  }
  return getAgentByApiKeyHash;
}

/**
 * Hash an API key using SHA-256 for comparison
 */
async function hashApiKey(apiKey: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(apiKey);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, "0")).join("");
}

/**
 * API Key middleware that validates X-API-Key header against hashed keys.
 *
 * The agent should send the API key in the X-API-Key header:
 * X-API-Key: <api_key>
 *
 * On success, sets c.var.agent with { agentId, agentName }
 */
export const apiKeyMiddleware = createMiddleware(async (c: Context, next: Next) => {
  const apiKey = c.req.header("X-API-Key");

  if (!apiKey) {
    logger.debug("Missing X-API-Key header");
    return c.json({ error: "Missing X-API-Key header" }, 401);
  }

  try {
    const getAgent = await loadDbOps();
    if (!getAgent) {
      logger.error("Database operations not available");
      return c.json({ error: "Service unavailable" }, 503);
    }

    // Hash the provided API key
    const apiKeyHash = await hashApiKey(apiKey);

    // Look up agent by hashed key
    const agent = await getAgent(apiKeyHash);

    if (!agent) {
      logger.debug("Invalid API key");
      return c.json({ error: "Invalid API key" }, 401);
    }

    if (!agent.isActive) {
      logger.debug(`Agent ${agent.name} is deactivated`);
      return c.json({ error: "Agent is deactivated" }, 403);
    }

    // Set agent context
    c.set("agent", {
      agentId: agent.id,
      agentName: agent.name,
    });

    logger.debug(`Authenticated agent: ${agent.name}`);
    return next();
  } catch (error) {
    logger.error("API key validation failed", error);
    return c.json({ error: "Authentication failed" }, 500);
  }
});

/**
 * Helper to get the current agent's ID from the context.
 * Returns undefined if not authenticated as an agent.
 */
export function getAgentId(c: Context): string | undefined {
  return c.get("agent")?.agentId;
}

/**
 * Helper to get the current agent's name from the context.
 * Returns undefined if not authenticated as an agent.
 */
export function getAgentName(c: Context): string | undefined {
  return c.get("agent")?.agentName;
}

/**
 * Generate a secure random API key
 */
export function generateApiKey(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Hash an API key for storage
 */
export { hashApiKey };
