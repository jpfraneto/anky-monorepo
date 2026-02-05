import { createMiddleware } from "hono/factory";
import type { Context, Next } from "hono";
import { Logger } from "../lib/logger.js";
import {
  USDC_ADDRESS,
  ANKY_TOKEN_ADDRESS,
  TREASURY_ADDRESS,
  USD_PER_SESSION,
  ANKY_TOKENS_PER_SESSION,
} from "@anky/shared";

const logger = Logger("Payment");

// Extend Hono context with payment info
declare module "hono" {
  interface ContextVariableMap {
    paymentType: "free" | "usdc" | "anky_token";
    paymentProof: { txHash: string; chain: string; method: string } | null;
    paymentVerification: { valid: boolean; reason?: string } | null;
    freeSessionsRemaining: number;
  }
}

export const agentPaymentMiddleware = createMiddleware(async (c: Context, next: Next) => {
  const agent = c.get("agent");
  if (!agent) {
    return c.json({ error: "Agent not found in context" }, 500);
  }

  // Lazy-load DB operations to avoid circular dependency
  const dbOps = await import("../db/operations.js");
  const agentData = await dbOps.getAgentById(agent.agentId);

  if (!agentData) {
    return c.json({ error: "Agent not found" }, 404);
  }

  // Check free sessions first
  if (agentData.freeSessionsRemaining > 0) {
    await dbOps.decrementAgentFreeSessions(agent.agentId);
    c.set("paymentType", "free");
    c.set("paymentProof", null);
    c.set("paymentVerification", null);
    c.set("freeSessionsRemaining", agentData.freeSessionsRemaining - 1);
    logger.info(`Agent ${agent.agentName} using free session (${agentData.freeSessionsRemaining - 1} remaining)`);
    return next();
  }

  // No free sessions - check for payment proof
  let body: Record<string, unknown>;
  try {
    body = await c.req.json();
    // Re-set body on request for downstream handlers
    // Store parsed body for later use
    c.set("validatedBody", body);
  } catch {
    body = {};
  }

  const payment = body.payment as { txHash?: string; chain?: string; method?: string } | undefined;

  if (!payment || !payment.txHash) {
    // Return 402 with payment options
    return c.json({
      error: "Payment required",
      message: "Free sessions exhausted. Payment required to continue.",
      payment_options: [
        {
          method: "usdc",
          token: "USDC",
          amount: String(USD_PER_SESSION),
          recipient: TREASURY_ADDRESS,
          chain: "base",
          decimals: 6,
          token_address: USDC_ADDRESS,
        },
        {
          method: "anky_token",
          token: "$ANKY",
          amount: String(ANKY_TOKENS_PER_SESSION),
          recipient: TREASURY_ADDRESS,
          chain: "base",
          decimals: 18,
          token_address: ANKY_TOKEN_ADDRESS,
        },
      ],
      freeSessionsRemaining: 0,
    }, 402);
  }

  // Check for replay attack
  const existingPayment = await dbOps.getPaymentByTxHash(payment.txHash);
  if (existingPayment) {
    return c.json({ error: "Transaction hash already used" }, 409);
  }

  // Verify on-chain payment
  const { verifyBaseTransaction } = await import("../services/payment.js");
  const method = payment.method as "usdc" | "anky_token";
  const expectedAmount = method === "usdc"
    ? String(Math.round(USD_PER_SESSION * 1_000_000)) // USDC has 6 decimals
    : String(BigInt(ANKY_TOKENS_PER_SESSION) * BigInt(10 ** 18)); // ANKY has 18 decimals
  const tokenAddress = method === "usdc" ? USDC_ADDRESS : ANKY_TOKEN_ADDRESS;

  const verification = await verifyBaseTransaction({
    txHash: payment.txHash as `0x${string}`,
    expectedRecipient: TREASURY_ADDRESS,
    method,
    expectedAmount,
    tokenAddress: tokenAddress as `0x${string}`,
  });

  if (!verification.valid) {
    return c.json({
      error: "Payment verification failed",
      reason: verification.reason,
    }, 402);
  }

  c.set("paymentType", method);
  c.set("paymentProof", {
    txHash: payment.txHash,
    chain: payment.chain || "base",
    method,
  });
  c.set("paymentVerification", verification);
  c.set("freeSessionsRemaining", 0);

  logger.info(`Agent ${agent.agentName} paid with ${method}: ${payment.txHash.slice(0, 10)}...`);
  return next();
});
