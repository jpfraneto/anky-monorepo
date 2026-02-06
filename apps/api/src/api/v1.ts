import { Hono } from "hono";
import { isDatabaseAvailable } from "../db/index.js";
import * as dbOps from "../db/operations.js";
import { apiKeyMiddleware, getAgentId, getAgentName, generateApiKey, hashApiKey } from "../middleware/apiKey.js";
import { agentRegisterLimiter, sessionSubmitLimiter } from "../middleware/rateLimit.js";
import { validate } from "../middleware/validate.js";
import { agentPaymentMiddleware } from "../middleware/payment.js";
import { agentRegisterSchema } from "@anky/shared";
import { generatePrompt, generateReflection, generateImage, generateTitle } from "../lib/ai.js";
import { uploadImageToIPFS, uploadWritingToIPFS, uploadMetadataToIPFS, getPinataGatewayUrl } from "../services/ipfs.js";
import { Logger } from "../lib/logger.js";

const APP_BASE_URL = process.env.APP_BASE_URL || "https://anky.app";

const logger = Logger("API-v1");

const app = new Hono();

// Request logging middleware
app.use("*", async (c, next) => {
  const start = Date.now();
  const method = c.req.method;
  const path = c.req.path;

  logger.info(`>>> v1 REQUEST: ${method} ${path}`);

  await next();

  const duration = Date.now() - start;
  const status = c.res.status;

  logger.info(`<<< v1 RESPONSE: ${method} ${path} ${status} ${duration}ms`);
});

// ============================================================================
// AGENT REGISTRATION (Public, rate limited, validated)
// ============================================================================

app.post("/agents/register", agentRegisterLimiter, validate(agentRegisterSchema), async (c) => {
  logger.info("Registering new agent");

  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const body = c.get("validatedBody") as { name: string; description?: string; model?: string };

  // Check if name is already taken
  const existing = await dbOps.getAgentByName(body.name);
  if (existing) {
    return c.json({ error: "Agent name already taken" }, 409);
  }

  // Generate API key
  const apiKey = generateApiKey();
  const apiKeyHash = await hashApiKey(apiKey);

  // Create agent
  const agent = await dbOps.createAgent({
    name: body.name,
    description: body.description,
    model: body.model,
    apiKeyHash,
  });

  if (!agent) {
    return c.json({ error: "Failed to create agent" }, 500);
  }

  logger.info(`Agent registered: ${agent.name} (${agent.id})`);

  return c.json({
    agent: {
      id: agent.id,
      name: agent.name,
      description: agent.description,
      createdAt: agent.createdAt,
    },
    apiKey,
  });
});

// ============================================================================
// AGENT PROFILE (Authenticated)
// ============================================================================

app.get("/agents/me", apiKeyMiddleware, async (c) => {
  const agentId = getAgentId(c);

  if (!agentId) {
    return c.json({ error: "Agent not found in context" }, 500);
  }

  const agent = await dbOps.getAgentById(agentId);

  if (!agent) {
    return c.json({ error: "Agent not found" }, 404);
  }

  return c.json({
    agent: {
      id: agent.id,
      name: agent.name,
      description: agent.description,
      model: agent.model,
      sessionCount: agent.sessionCount,
      freeSessionsRemaining: agent.freeSessionsRemaining,
      totalPaidSessions: agent.totalPaidSessions,
      lastActiveAt: agent.lastActiveAt,
      createdAt: agent.createdAt,
    },
  });
});

// ============================================================================
// SESSION SUBMISSION (Authenticated, rate limited, payment gated)
// ============================================================================

app.post("/sessions", apiKeyMiddleware, sessionSubmitLimiter, agentPaymentMiddleware, async (c) => {
  logger.info("Agent submitting session");

  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const agentId = getAgentId(c);
  const agentName = getAgentName(c);

  if (!agentId) {
    return c.json({ error: "Agent not found in context" }, 500);
  }

  // Get body - may be pre-parsed by payment middleware or needs parsing
  let body = c.get("validatedBody") as Record<string, unknown> | undefined;
  if (!body) {
    try {
      body = await c.req.json();
    } catch {
      return c.json({ error: "Invalid JSON body" }, 400);
    }
  }

  const { content, durationSeconds, wordCount, wordsPerMinute } = body as {
    content?: string;
    durationSeconds?: number;
    wordCount?: number;
    wordsPerMinute?: number;
  };

  if (!content || durationSeconds === undefined || wordCount === undefined) {
    return c.json({ error: "content, durationSeconds, wordCount required" }, 400);
  }

  // Create session
  const session = await dbOps.createWritingSessionForAgent({
    agentId,
    content,
    durationSeconds,
    wordCount,
    wordsPerMinute,
    isPublic: true,
  });

  if (!session) {
    return c.json({ error: "Failed to create session" }, 500);
  }

  // Record payment if applicable
  const paymentType = c.get("paymentType");
  const paymentProof = c.get("paymentProof");

  if (paymentType !== "free" && paymentProof) {
    try {
      await dbOps.createAgentPayment({
        agentId,
        sessionId: session.id,
        txHash: paymentProof.txHash,
        chain: paymentProof.chain,
        paymentMethod: paymentProof.method,
        amount: paymentProof.method === "usdc" ? "0.333" : "100",
        verified: true,
      });
      await dbOps.updateAgentPaymentStats(
        agentId,
        paymentProof.method as "usdc" | "anky_token",
        paymentProof.method === "usdc" ? "0.333" : "100"
      );
    } catch (e) {
      logger.error("Failed to record payment:", e);
    }
  }

  const isAnky = durationSeconds >= 480;
  logger.info(`Agent ${agentName} created session: ${session.id} (${wordCount} words, ${Math.floor(durationSeconds / 60)}min, isAnky=${isAnky})`);

  // If this is a full Anky session, generate the Anky
  let anky = null;
  if (isAnky) {
    try {
      const [promptResult, reflectionResult] = await Promise.all([
        generatePrompt(content),
        generateReflection(content),
      ]);

      const imageResult = await generateImage(promptResult.prompt);

      const titleResult = await generateTitle(
        content,
        promptResult.prompt,
        reflectionResult.reflection
      );

      // Auto-upload to IPFS (non-fatal)
      let imageIpfsHash: string | undefined;
      let writingIpfsHash: string | undefined;
      let metadataIpfsHash: string | undefined;
      let ipfsImageUrl: string | undefined;

      try {
        const [imageIpfs, writingIpfs] = await Promise.all([
          uploadImageToIPFS(imageResult.base64),
          uploadWritingToIPFS(content),
        ]);

        imageIpfsHash = imageIpfs.ipfsHash;
        writingIpfsHash = writingIpfs.ipfsHash;
        ipfsImageUrl = imageIpfs.gatewayUrl;

        const metadataIpfs = await uploadMetadataToIPFS({
          title: titleResult.title,
          reflection: reflectionResult.reflection,
          imageIpfsHash: imageIpfs.ipfsHash,
          writingIpfsHash: writingIpfs.ipfsHash,
          imagePrompt: promptResult.prompt,
        });
        metadataIpfsHash = metadataIpfs.ipfsHash;

        logger.info(`IPFS upload complete for agent ${agentName}: image=${imageIpfsHash}, writing=${writingIpfsHash}, metadata=${metadataIpfsHash}`);
      } catch (ipfsErr) {
        logger.warn("IPFS upload failed (non-fatal), falling back to data URL:", ipfsErr);
      }

      anky = await dbOps.createAnky({
        writingSessionId: session.id,
        imagePrompt: promptResult.prompt,
        reflection: reflectionResult.reflection,
        title: titleResult.title,
        imageUrl: ipfsImageUrl || imageResult.url,
        imageIpfsHash,
        writingIpfsHash,
        metadataIpfsHash,
      });

      logger.info(`Anky generated for agent ${agentName}: ${anky?.title}`);
    } catch (e) {
      logger.error("Failed to generate Anky for agent session:", e);
    }
  }

  return c.json({
    session: {
      id: session.id,
      shareId: session.shareId,
      isAnky: session.isAnky,
      wordCount: session.wordCount,
      durationSeconds: session.durationSeconds,
      createdAt: session.createdAt,
    },
    anky: anky ? {
      id: anky.id,
      title: anky.title,
      imageUrl: anky.imageUrl,
      reflection: anky.reflection,
    } : null,
    shareUrl: `${APP_BASE_URL}/session/${session.shareId}`,
    payment: {
      type: paymentType,
      freeSessionsRemaining: c.get("freeSessionsRemaining") ?? 0,
    },
  });
});

// ============================================================================
// GET AGENT'S SESSIONS (Authenticated)
// ============================================================================

app.get("/sessions/me", apiKeyMiddleware, async (c) => {
  const agentId = getAgentId(c);

  if (!agentId) {
    return c.json({ error: "Agent not found in context" }, 500);
  }

  const limit = parseInt(c.req.query("limit") || "50");
  const sessions = await dbOps.getAgentSessions(agentId, limit);

  return c.json({
    sessions: sessions.map((s) => ({
      id: s.id,
      shareId: s.shareId,
      content: s.content,
      isAnky: s.isAnky,
      wordCount: s.wordCount,
      durationSeconds: s.durationSeconds,
      createdAt: s.createdAt,
      shareUrl: `${APP_BASE_URL}/session/${s.shareId}`,
      anky: s.anky ? {
        id: s.anky.id,
        title: s.anky.title,
        imageUrl: s.anky.imageUrl,
        reflection: s.anky.reflection,
      } : null,
    })),
  });
});

export default app;
