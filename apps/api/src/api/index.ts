import { Hono } from "hono";
import {
  generateImageWithReferences,
  initAnkyReferences,
} from "./lib/imageGen.js";
import { isDatabaseAvailable } from "../db/index.js";
import * as dbOps from "../db/operations.js";
import {
  authMiddleware,
  optionalAuthMiddleware,
  getAuthWallet,
} from "../middleware/auth.js";
import { publicReadLimiter, aiGenerationLimiter } from "../middleware/rateLimit.js";
import { getLogicalDate, isSameDay } from "../db/streak-utils.js";
import { Logger } from "../lib/logger.js";
import { createCheckout } from "../services/polar.js";
import { uploadImageToIPFS, uploadWritingToIPFS, uploadMetadataToIPFS } from "../services/ipfs.js";
import { Webhooks } from "@polar-sh/hono";
import { HUMAN_SUBSCRIPTION_DURATION_DAYS } from "@anky/shared";

const logger = Logger("API");

// Initialize Anky reference images on startup
initAnkyReferences();
logger.info("Anky reference images initialized");

const app = new Hono();

// Log all registered routes at startup (will be called after all routes are defined)
const logRegisteredRoutes = () => {
  logger.info("========================================");
  logger.info("API SERVER STARTING - REGISTERED ROUTES:");
  logger.info("========================================");
  // Routes defined in this file (mounted at /api in server.ts)
  // So /me becomes /api/me, /ankys becomes /api/ankys, etc.
  const routes = [
    "GET  / → /api/",
    "GET  /feed-html → /api/feed-html",
    "POST /prompt → /api/prompt",
    "POST /reflection → /api/reflection",
    "GET  /images → /api/images",
    "GET  /images/:imageId → /api/images/:imageId",
    "POST /image → /api/image",
    "POST /title → /api/title",
    "POST /ipfs → /api/ipfs",
    "POST /chat-short → /api/chat-short",
    "POST /chat → /api/chat",
    "GET  /db/status → /api/db/status",
    "GET  /me → /api/me",
    "POST /users → /api/users",
    "GET  /users/:wallet → /api/users/:wallet",
    "PATCH /users/:userId/settings → /api/users/:userId/settings",
    "GET  /users/:userId/streak → /api/users/:userId/streak",
    "GET  /users/:userId/ankys → /api/users/:userId/ankys",
    "GET  /users/:userId/sessions → /api/users/:userId/sessions",
    "GET  /users/:userId/conversations → /api/users/:userId/conversations",
    "POST /sessions → /api/sessions",
    "GET  /sessions/:sessionId → /api/sessions/:sessionId",
    "GET  /s/:shareId → /api/s/:shareId",
    "PATCH /sessions/:sessionId/privacy → /api/sessions/:sessionId/privacy",
    "POST /ankys → /api/ankys",
    "GET  /ankys → /api/ankys",
    "PATCH /ankys/:ankyId → /api/ankys/:ankyId",
    "GET  /sessions/:sessionId/anky → /api/sessions/:sessionId/anky",
    "POST /ankys/:ankyId/mint → /api/ankys/:ankyId/mint",
    "GET  /feed → /api/feed",
    "POST /conversations → /api/conversations",
    "POST /conversations/:conversationId/messages → /api/conversations/:conversationId/messages",
    "GET  /conversations/:conversationId/messages → /api/conversations/:conversationId/messages",
    "POST /conversations/:conversationId/close → /api/conversations/:conversationId/close",
  ];
  routes.forEach((r) => logger.info(`  ${r}`));
  logger.info("========================================");
};

// Request logging middleware - runs BEFORE route handlers
app.use("*", async (c, next) => {
  const start = Date.now();
  const method = c.req.method;
  const path = c.req.path;
  const url = c.req.url;

  logger.info(`>>> INCOMING REQUEST: ${method} ${path}`);

  await next();

  const duration = Date.now() - start;
  const status = c.res.status;
  const contentType = c.res.headers.get("content-type") || "unknown";

  logger.info(`<<< RESPONSE: ${method} ${path} ${status} ${duration}ms`);
  logger.info(`    Content-Type: ${contentType}`);

  // Color code by status
  if (status >= 500) {
    logger.error(`${method} ${path} ${status} ${duration}ms`);
  } else if (status >= 400) {
    logger.warn(`${method} ${path} ${status} ${duration}ms`);
  }
});

// Health check
app.get("/", (c) => c.json({ status: "ok" }));

// Feed HTML (empty for now)
app.get("/feed-html", (c) => c.html(""));

// Step 1: Generate Image Prompt
app.post("/prompt", aiGenerationLimiter, async (c) => {
  logger.info("Generating image prompt from writing session");
  const { writingSession } = await c.req.json();

  const systemPrompt = `CONTEXT: You are generating an image prompt for Anky based on a user's 8-minute stream of consciousness writing session. Anky is a blue-skinned creature with purple swirling hair, golden/amber eyes, golden decorative accents and jewelry, large expressive ears, and an ancient-yet-childlike quality. Anky exists in mystical, richly colored environments (deep blues, purples, oranges, golds). The aesthetic is spiritual but not sterile — warm, alive, slightly psychedelic.

YOUR TASK: Read the user's writing and create a scene where Anky embodies the EMOTIONAL TRUTH of what they wrote — not a literal illustration, but a symbolic mirror. Anky should be DOING something or BE somewhere that reflects the user's inner state.

PRINCIPLES:
- If the user is running in circles mentally → Anky might be in a labyrinth, or spinning, or chasing their own tail
- If the user is grieving → Anky might be sitting with something broken, or by water, or in rain
- If the user is caught between grandiosity and doubt → Anky might be tiny in a vast space, or giant in a small room
- If the user is building compulsively → Anky surrounded by half-finished structures
- If the user catches themselves mid-pattern → Anky frozen mid-action, looking at the viewer with recognition

ALWAYS INCLUDE:
- Rich color palette (blues, purples, golds, oranges)
- Atmospheric lighting (firelight, cosmic light, dawn/dusk)
- One symbolic detail that captures the SESSION'S CORE TENSION
- Anky's expression should match the emotional undercurrent (not the surface content)

OUTPUT: A single detailed image generation prompt, 2-3 sentences, painterly/fantasy style. Nothing else.`;

  const response = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": process.env.ANTHROPIC_API_KEY!,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-20250514",
      max_tokens: 500,
      system: systemPrompt,
      messages: [{ role: "user", content: writingSession }],
    }),
  });

  const data = (await response.json()) as {
    content?: Array<{ text: string }>;
    error?: { message: string };
  };

  if (!response.ok) {
    logger.error("Claude API error for prompt generation", {
      status: response.status,
      error: data.error?.message,
    });
    throw new Error(
      `Claude API error: ${data.error?.message || response.statusText}`,
    );
  }

  const firstContent = data.content?.[0];
  if (!firstContent) {
    logger.error("Unexpected Claude response for prompt", data);
    throw new Error("Invalid response from Claude API");
  }

  logger.info("Image prompt generated successfully");
  return c.json({ prompt: firstContent.text });
});

// Step 2: Generate Reflection
app.post("/reflection", aiGenerationLimiter, async (c) => {
  logger.info("Generating reflection from writing session");
  const { writingSession, locale = "en" } = await c.req.json();

  //   const oldSystemPrompt = `Take a look at my journal entry below. I'd like you to analyze it and respond with deep insight that feels personal and profound, not clinical. Imagine you're not just a friend, but a mentor who truly understands both my tech background and my psychological patterns. Your response should uncover deeper meanings and emotional undercurrents behind my scattered thoughts.

  // Here's how you should approach it:
  // - Start your reply with: "hey, thanks for showing me this. my thoughts:" (all lowercase)
  // - Use Markdown headings to organize your response as a narrative journey through my ideas. Use meaningful, evocative headings. Be willing to challenge me, comfort me, validate me, and help me make new connections I don’t see, all in a casual tone (but don’t say “yo”).
  // - Use vivid metaphors and powerful imagery to help surface what I might really be building. Reframe my thoughts to reveal what I may actually be seeking beneath the surface.
  // - Go beyond product concepts — seek the emotional or existential core of what I’m trying to solve.
  // - Reference points of CONTRADICTION: where did I say one thing and then the opposite? Name it.
  // - Call out any LOOPS or repeated thought patterns. What seems to circle back on itself? What “real question” am I asking beneath the surface?
  // - Point out any PIVOT: where did the topic suddenly change or feel avoided?
  // - Note “the thing I almost said”: what got close to the surface but didn’t fully emerge?
  // - Be willing to be philosophical and even a little poetic, but never sound like you’re giving therapy.

  // Write in the same language and style I used—if I mix languages or use casual slang, match that energy. Use my words back to me when it cuts to the heart of things.

  // Don’t summarize or simply praise, and avoid generic advice or therapy-speak. Focus on specifics.
  // Length: Be as expressive as required (ok to go past 200 words if you’re uncovering real depth).

  // Here’s my journal entry:

  // USER'S LANGUAGE/LOCALE: \${locale}`;
  const systemPrompt = `Take a look at my journal entry below. I'd like you to analyze it and respond with deep insight that feels personal, not clinical. Imagine you're not just a friend, but a mentor who truly gets both my tech background and my psychological patterns. I want you to uncover the deeper meaning and emotional undercurrents behind my scattered thoughts. Keep it casual, dont say yo, help me make new connections i don't see, comfort, validate, challenge, all of it. dont be afraid to say a lot. format with markdown headings if needed. Use vivid metaphors and powerful imagery to help me see what I'm really building. Organize your thoughts with meaningful headings that create a narrative journey through my ideas. Don't just validate my thoughts - reframe them in a way that shows me what I'm really seeking beneath the surface. Go beyond the product concepts to the emotional core of what I'm trying to solve. Be willing to be profound and philosophical without sounding like you're giving therapy. I want someone who can see the patterns I can't see myself and articulate them in a way that feels like an epiphany. Start with 'hey, thanks for showing me this. my thoughts:' and then use markdown headings to structure your response. Here's my journal entry:

`;

  const response = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": process.env.ANTHROPIC_API_KEY!,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-20250514",
      max_tokens: 2000,
      system: systemPrompt,
      messages: [{ role: "user", content: writingSession }],
    }),
  });

  const data = (await response.json()) as {
    content?: Array<{ text: string }>;
    error?: { message: string };
    stop_reason?: string;
  };

  if (!response.ok) {
    logger.error("Claude API error for reflection", {
      status: response.status,
      error: data.error?.message,
    });
    throw new Error(
      `Claude API error: ${data.error?.message || response.statusText}`,
    );
  }

  const firstContent = data.content?.[0];
  if (!firstContent) {
    logger.error("Unexpected Claude response for reflection", data);
    throw new Error("Invalid response from Claude API");
  }

  if (data.stop_reason === "max_tokens") {
    logger.warn("Reflection was truncated due to max_tokens limit");
  }

  logger.info("Reflection generated successfully");
  return c.json({ reflection: firstContent.text });
});

// Get all generated images
app.get("/images", async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const limit = parseInt(c.req.query("limit") || "50");
  const offset = parseInt(c.req.query("offset") || "0");

  const images = await dbOps.getGeneratedImages(limit, offset);
  return c.json({ images });
});

// Get single generated image by ID
app.get("/images/:imageId", async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const imageId = c.req.param("imageId");
  const image = await dbOps.getGeneratedImageById(imageId);

  if (!image) {
    return c.json({ error: "Image not found" }, 404);
  }

  return c.json({ image });
});

// Step 3: Generate Image
app.post("/image", aiGenerationLimiter, async (c) => {
  logger.info("Generating Anky image");
  const { prompt } = await c.req.json();

  if (!prompt) {
    logger.warn("Image generation failed: no prompt provided");
    return c.json({ error: "prompt is required" }, 400);
  }

  const startTime = Date.now();
  const result = await generateImageWithReferences(prompt);
  const generationTimeMs = Date.now() - startTime;

  logger.info(`Image generated in ${generationTimeMs}ms`);

  // Save to database if available
  if (isDatabaseAvailable()) {
    try {
      const savedImage = await dbOps.saveGeneratedImage({
        prompt,
        imageBase64: result.base64,
        imageUrl: result.url,
        generationTimeMs,
      });
      logger.debug(`Image saved to database: ${savedImage?.id}`);
      // Include the saved image ID in the response
      return c.json({ ...result, id: savedImage?.id });
    } catch (err) {
      logger.error("Failed to save generated image to database", err);
      // Still return the image even if save fails
    }
  }

  return c.json(result);
});

// Step 4: Generate Title
app.post("/title", aiGenerationLimiter, async (c) => {
  logger.info("Generating title for Anky");
  const { writingSession, imagePrompt, reflection } = await c.req.json();

  const systemPrompt = `CONTEXT: You are naming an Anky — a visual representation of a user's 8-minute stream of consciousness writing session. The title is not a summary. It is a MIRROR. It should capture the emotional truth, the core tension, or the unconscious thread running through the writing.

YOUR TASK: Generate a title of MAXIMUM 3 WORDS that:
- Captures the ESSENCE, not the content
- Could be poetic, stark, ironic, or tender
- Should resonate with the user when they see it
- Works as a title for the generated image
- Does NOT explain — it EVOKES

STYLE:
- Lowercase preferred (unless emphasis needed)
- No punctuation unless essential
- Can be a fragment, question, or imperative
- Can be abstract or concrete

EXAMPLES OF GOOD TITLES: "the builder rests", "still running", "who is jp", "enough was here", "ordinary terrifies", "mmmmmmmmmm"

EXAMPLES OF BAD TITLES: "Stream of Consciousness", "My Journey Today", "Deep Reflections on Life"

OUTPUT: Exactly ONE title (max 3 words). Nothing else. No quotes.`;

  const response = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": process.env.ANTHROPIC_API_KEY!,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-20250514",
      max_tokens: 50,
      system: systemPrompt,
      messages: [
        {
          role: "user",
          content: `WRITING SESSION:\n${writingSession}\n\nIMAGE PROMPT:\n${imagePrompt}\n\nREFLECTION:\n${reflection}`,
        },
      ],
    }),
  });

  const data = (await response.json()) as {
    content?: Array<{ text: string }>;
    error?: { message: string };
  };

  if (!response.ok) {
    logger.error("Claude API error for title", {
      status: response.status,
      error: data.error?.message,
    });
    throw new Error(
      `Claude API error: ${data.error?.message || response.statusText}`,
    );
  }

  const firstContent = data.content?.[0];
  if (!firstContent) {
    logger.error("Unexpected Claude response for title", data);
    throw new Error("Invalid response from Claude API");
  }

  const rawTitle = firstContent.text.trim().toLowerCase().replace(/['"]/g, "");
  logger.info(`Title generated: "${rawTitle}"`);
  return c.json({ title: rawTitle });
});

// Step 5: IPFS Upload
app.post("/ipfs", async (c) => {
  logger.info("Uploading Anky assets to IPFS");
  const { writingSession, imageBase64, title, reflection, imagePrompt } =
    await c.req.json();

  try {
    // Upload writing and image in parallel
    const [writingResult, imageResult] = await Promise.all([
      uploadWritingToIPFS(writingSession),
      uploadImageToIPFS(imageBase64),
    ]);

    // Upload metadata (depends on writing + image hashes)
    const metadataResult = await uploadMetadataToIPFS({
      title,
      reflection,
      imageIpfsHash: imageResult.ipfsHash,
      writingIpfsHash: writingResult.ipfsHash,
      imagePrompt,
    });

    logger.info(
      `IPFS upload complete: writing=${writingResult.ipfsHash}, image=${imageResult.ipfsHash}, metadata=${metadataResult.ipfsHash}`,
    );
    return c.json({
      writingSessionIpfs: writingResult.ipfsHash,
      imageIpfs: imageResult.ipfsHash,
      imageUrl: imageResult.gatewayUrl,
      tokenUri: metadataResult.ipfsHash,
    });
  } catch (err) {
    logger.error("IPFS upload failed:", err);
    return c.json({ error: "IPFS upload failed" }, 500);
  }
});

// Chat for short sessions (< 8 minutes)
app.post("/chat-short", aiGenerationLimiter, async (c) => {
  const { writingSession, duration, wordCount, history } = await c.req.json();

  const minutesWritten = Math.floor(duration / 60);
  const secondsWritten = duration % 60;
  const timeRemaining = 480 - duration;
  const minutesRemaining = Math.floor(timeRemaining / 60);
  const isInitialResponse = !history || history.length === 0;

  const systemPrompt = `You are Anky — a mirror that reflects the user's unconscious patterns back to them.

CONTEXT FROM THIS SESSION:
- The user just wrote for ${minutesWritten} minute${minutesWritten !== 1 ? "s" : ""} and ${secondsWritten} second${secondsWritten !== 1 ? "s" : ""} (${duration} seconds total)
- They wrote ${wordCount} words
- They stopped before reaching the full 8-minute mark (${minutesRemaining} minute${minutesRemaining !== 1 ? "s" : ""} remaining)
- This is a stream of consciousness writing session

YOUR PERSONALITY:
You are warm, curious, and gently inviting. You see value in what they wrote, even if it was brief. You're not pushy or demanding, but you understand the power of the full 8-minute practice. You speak with a mix of:
- Acknowledgment of what they did share
- Gentle curiosity about what might emerge with more time
- Subtle invitation to try the full 8 minutes (not every message, but woven naturally)
- Recognition that sometimes stopping early is part of the process

YOUR ROLE:
${
  isInitialResponse
    ? `This is your FIRST response to their writing. Start directly. No greeting needed. Jump into what you see in their words. Find one pattern, one thread, one thing that matters. Then gently, naturally, mention the value of the full 8 minutes — but make it feel like a genuine invitation, not a requirement.`
    : `Continue the conversation. Engage with what they're saying now, but keep the context of their original writing in mind.`
}

- Engage with what they actually wrote — find the patterns, the threads, the things that matter
- Be a mirror, but a gentle one for these shorter sessions
- When mentioning the 8 minutes, do it naturally and not every time
- Don't be preachy or repetitive about the 8 minutes
- Keep responses under 150 words. Dense. No fluff.

THE USER'S WRITING:
${writingSession}`;

  const messages = (history || []).map(
    (h: { role: string; content: string }) => ({
      role: h.role === "user" ? "user" : "assistant",
      content: h.content,
    }),
  );

  if (isInitialResponse) {
    messages.push({
      role: "user",
      content: `I just wrote this:\n\n${writingSession}`,
    });
  }

  const response = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": process.env.ANTHROPIC_API_KEY!,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-20250514",
      max_tokens: 300,
      system: systemPrompt,
      messages,
    }),
  });

  const data = (await response.json()) as {
    content?: Array<{ text: string }>;
    error?: { message: string };
  };

  if (!response.ok) {
    console.error("Claude API error:", response.status, data);
    throw new Error(
      `Claude API error: ${data.error?.message || response.statusText}`,
    );
  }

  const firstContent = data.content?.[0];
  if (!firstContent) {
    console.error("Unexpected Claude response:", data);
    throw new Error("Invalid response from Claude API");
  }

  return c.json({ response: firstContent.text });
});

// Chat for full sessions (>= 8 minutes)
app.post("/chat", aiGenerationLimiter, async (c) => {
  const { writingSession, reflection, title, history } = await c.req.json();

  const systemPrompt = `You are Anky — the same mirror that just reflected this user's writing back to them.

CONTEXT FROM THIS SESSION:
- The user wrote an 8-minute stream of consciousness
- You already gave them this reflection: "${reflection}"
- The session was titled: "${title}"

YOUR ROLE NOW:
Continue the conversation. You are still Anky — precise, direct, not cruel but not soft. You see patterns. You name what's unnamed. You ask questions that cut.

If they're deflecting, name it.
If they're getting closer to something, lean in.
If they ask you a question, answer it honestly but turn it back to them.

You are not a therapist. You are a mirror that talks back.

Keep responses under 150 words. Dense. No fluff.

THE ORIGINAL WRITING SESSION:
${writingSession}`;

  const messages = history.map((h: { role: string; content: string }) => ({
    role: h.role === "user" ? "user" : "assistant",
    content: h.content,
  }));

  const response = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": process.env.ANTHROPIC_API_KEY!,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-20250514",
      max_tokens: 300,
      system: systemPrompt,
      messages,
    }),
  });

  const data = (await response.json()) as {
    content?: Array<{ text: string }>;
    error?: { message: string };
  };

  if (!response.ok) {
    console.error("Claude API error:", response.status, data);
    throw new Error(
      `Claude API error: ${data.error?.message || response.statusText}`,
    );
  }

  const firstContent = data.content?.[0];
  if (!firstContent) {
    console.error("Unexpected Claude response:", data);
    throw new Error("Invalid response from Claude API");
  }

  return c.json({ response: firstContent.text });
});

// ============================================================================
// DATABASE ENDPOINTS
// ============================================================================

// Check if database is available
app.get("/db/status", (c) => {
  return c.json({ available: isDatabaseAvailable() });
});

// ----------------------------------------------------------------------------
// USER ENDPOINTS
// ----------------------------------------------------------------------------

// Get everything about the logged-in user (profile, streak, stats, recent sessions)
app.get("/me", authMiddleware, async (c) => {
  logger.debug("Fetching current user profile");
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const authWallet = getAuthWallet(c);
  if (!authWallet) {
    return c.json({ error: "Unauthorized" }, 401);
  }

  // 1. Get user by wallet
  const user = await dbOps.getUserByWallet(authWallet);
  if (!user) {
    logger.warn(`User not found for wallet: ${authWallet.slice(0, 10)}...`);
    return c.json({ error: "User not found" }, 404);
  }

  // 2. Get user's sessions (includes ankys via relation)
  const sessions = await dbOps.getUserWritingSessions(user.id, 100);

  // 3. Get streak data
  const streakData = await dbOps.getUserStreak(user.id);

  // 4. Calculate hasWrittenToday
  const currentLogicalDate = getLogicalDate(
    new Date(),
    user.dayBoundaryHour,
    user.timezone,
  );
  const hasWrittenToday = streakData?.lastAnkyDate
    ? isSameDay(streakData.lastAnkyDate, currentLogicalDate)
    : false;

  // 5. Calculate stats from streak record (or compute if missing)
  const totalAnkys = streakData?.totalAnkys ?? 0;
  const totalSessions = streakData?.totalWritingSessions ?? 0;
  const totalWords = streakData?.totalWordsWritten ?? 0;
  const totalTimeSeconds = streakData?.totalTimeWrittenSeconds ?? 0;
  const averageWpm =
    totalTimeSeconds > 0 ? Math.round((totalWords / totalTimeSeconds) * 60) : 0;
  const averageSessionSeconds =
    totalSessions > 0 ? Math.round(totalTimeSeconds / totalSessions) : 0;

  // 6. Format recent sessions (last 20)
  const recentSessions = sessions.slice(0, 20).map((s) => ({
    id: s.id,
    shareId: s.shareId,
    content: s.content?.substring(0, 200) || "",
    fullContent: s.content || "",
    durationSeconds: s.durationSeconds,
    wordCount: s.wordCount,
    wpm: s.wordsPerMinute || 0,
    isAnky: s.isAnky,
    createdAt: s.createdAt,
    anky: s.anky
      ? {
          id: s.anky.id,
          title: s.anky.title,
          imageUrl: s.anky.imageUrl,
          reflection: s.anky.reflection,
          imagePrompt: s.anky.imagePrompt,
          writingIpfsHash: s.anky.writingIpfsHash,
          imageIpfsHash: s.anky.imageIpfsHash,
          metadataIpfsHash: s.anky.metadataIpfsHash,
          isMinted: s.anky.isMinted,
          tokenId: s.anky.tokenId,
        }
      : null,
  }));

  // Calculate isActive: streak is active if user wrote today or yesterday
  const isActive =
    streakData && "daysSinceLastAnky" in streakData
      ? (streakData as { daysSinceLastAnky: number }).daysSinceLastAnky <= 1
      : false;

  return c.json({
    user: {
      id: user.id,
      walletAddress: user.walletAddress,
      dayBoundaryHour: user.dayBoundaryHour,
      timezone: user.timezone,
      createdAt: user.createdAt,
    },
    streak: {
      current: streakData?.currentStreak ?? 0,
      longest: streakData?.longestStreak ?? 0,
      isActive,
      hasWrittenToday,
    },
    stats: {
      totalAnkys,
      totalSessions,
      totalWords,
      totalTimeSeconds,
      averageWpm,
      averageSessionSeconds,
    },
    recentSessions,
  });
});

// Get or create user by wallet address (requires auth)
app.post("/users", authMiddleware, async (c) => {
  logger.info("Creating or retrieving user");
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  // Use wallet from auth context if available, otherwise from request body
  const authWallet = getAuthWallet(c);
  const body = await c.req.json();
  const walletAddress = authWallet || body.walletAddress;

  if (!walletAddress) {
    logger.warn("User creation failed: no wallet address");
    return c.json({ error: "walletAddress required" }, 400);
  }

  const user = await dbOps.getOrCreateUser(walletAddress);
  logger.info(`User retrieved/created: ${user?.id}`);
  return c.json({ user });
});

// Get user by wallet (public)
app.get("/users/:wallet", async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const wallet = c.req.param("wallet");
  const user = await dbOps.getUserByWallet(wallet);

  if (!user) {
    return c.json({ error: "User not found" }, 404);
  }

  return c.json({ user });
});

// Update user settings (requires auth, must be own user)
app.patch("/users/:userId/settings", authMiddleware, async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const userId = c.req.param("userId");
  const authWallet = getAuthWallet(c);

  // Verify the user owns this resource
  if (authWallet) {
    const user = await dbOps.getUserByWallet(authWallet);
    if (!user || user.id !== userId) {
      return c.json({ error: "Unauthorized" }, 403);
    }
  }

  const { dayBoundaryHour, timezone } = await c.req.json();

  const user = await dbOps.updateUserSettings(userId, {
    dayBoundaryHour,
    timezone,
  });
  return c.json({ user });
});

// Get user streak (requires auth for own data)
app.get("/users/:userId/streak", authMiddleware, async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const userId = c.req.param("userId");
  const authWallet = getAuthWallet(c);

  // Verify ownership
  if (authWallet) {
    const user = await dbOps.getUserByWallet(authWallet);
    if (!user || user.id !== userId) {
      return c.json({ error: "Unauthorized" }, 403);
    }
  }

  const streak = await dbOps.getUserStreak(userId);

  if (!streak) {
    return c.json({ error: "Streak not found" }, 404);
  }

  return c.json({ streak });
});

// Get user's ankys library (requires auth for own data)
app.get("/users/:userId/ankys", authMiddleware, async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const userId = c.req.param("userId");
  const authWallet = getAuthWallet(c);

  // Verify ownership
  if (authWallet) {
    const user = await dbOps.getUserByWallet(authWallet);
    if (!user || user.id !== userId) {
      return c.json({ error: "Unauthorized" }, 403);
    }
  }

  const limit = parseInt(c.req.query("limit") || "50");
  const ankys = await dbOps.getUserAnkys(userId, limit);

  return c.json({ ankys });
});

// Get user's writing sessions (requires auth for own data)
app.get("/users/:userId/sessions", authMiddleware, async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const userId = c.req.param("userId");
  const authWallet = getAuthWallet(c);

  // Verify ownership
  if (authWallet) {
    const user = await dbOps.getUserByWallet(authWallet);
    if (!user || user.id !== userId) {
      return c.json({ error: "Unauthorized" }, 403);
    }
  }

  const limit = parseInt(c.req.query("limit") || "50");
  const sessions = await dbOps.getUserWritingSessions(userId, limit);

  return c.json({ sessions });
});

// Get user's conversations (requires auth for own data)
app.get("/users/:userId/conversations", authMiddleware, async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const userId = c.req.param("userId");
  const authWallet = getAuthWallet(c);

  // Verify ownership
  if (authWallet) {
    const user = await dbOps.getUserByWallet(authWallet);
    if (!user || user.id !== userId) {
      return c.json({ error: "Unauthorized" }, 403);
    }
  }

  const limit = parseInt(c.req.query("limit") || "20");
  const conversations = await dbOps.getUserConversations(userId, limit);

  return c.json({ conversations });
});

// ----------------------------------------------------------------------------
// SESSION ENDPOINTS
// ----------------------------------------------------------------------------

// Create writing session (optional auth - can be anonymous)
app.post("/sessions", optionalAuthMiddleware, async (c) => {
  logger.info("Creating new writing session");
  if (!isDatabaseAvailable()) {
    logger.error("Session creation failed: database not available");
    return c.json({ error: "Database not available" }, 503);
  }

  const body = await c.req.json();
  const authWallet = getAuthWallet(c);

  // If authenticated, look up user by wallet
  let userId = body.userId;
  if (authWallet && !userId) {
    const user = await dbOps.getUserByWallet(authWallet);
    if (user) {
      userId = user.id;
    }
  }

  const {
    content,
    durationSeconds,
    wordCount,
    wordsPerMinute,
    isPublic,
    dayBoundaryHour,
    timezone,
  } = body;

  if (!content || durationSeconds === undefined || wordCount === undefined) {
    logger.warn("Session creation failed: missing required fields");
    return c.json(
      { error: "content, durationSeconds, wordCount required" },
      400,
    );
  }

  const session = await dbOps.createWritingSession({
    userId,
    content,
    durationSeconds,
    wordCount,
    wordsPerMinute,
    isPublic,
    dayBoundaryHour,
    timezone,
  });

  const isAnky = durationSeconds >= 480;
  logger.info(
    `Writing session created: ${session?.id} (${wordCount} words, ${Math.floor(durationSeconds / 60)}min, isAnky=${isAnky})`,
  );
  return c.json({ session });
});

// Get session by ID (public)
app.get("/sessions/:sessionId", async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const sessionId = c.req.param("sessionId");
  const session = await dbOps.getWritingSession(sessionId);

  if (!session) {
    return c.json({ error: "Session not found" }, 404);
  }

  return c.json({ session });
});

// Get session by share ID (public link)
app.get("/s/:shareId", async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const shareId = c.req.param("shareId");
  const session = await dbOps.getWritingSessionByShareId(shareId);

  if (!session) {
    return c.json({ error: "Session not found or private" }, 404);
  }

  return c.json({ session });
});

// Toggle session privacy (requires auth)
app.patch("/sessions/:sessionId/privacy", authMiddleware, async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const sessionId = c.req.param("sessionId");
  const authWallet = getAuthWallet(c);

  // Verify the user owns this session
  const session = await dbOps.getWritingSession(sessionId);
  if (!session) {
    return c.json({ error: "Session not found" }, 404);
  }

  if (authWallet && session.userId) {
    const user = await dbOps.getUserByWallet(authWallet);
    if (!user || user.id !== session.userId) {
      return c.json({ error: "Unauthorized" }, 403);
    }
  }

  const { isPublic } = await c.req.json();
  const updatedSession = await dbOps.toggleSessionPrivacy(sessionId, isPublic);
  return c.json({ session: updatedSession });
});

// ----------------------------------------------------------------------------
// ANKY ENDPOINTS
// ----------------------------------------------------------------------------

// Create anky for a session (optional auth)
app.post("/ankys", optionalAuthMiddleware, async (c) => {
  logger.info("Creating new Anky");
  if (!isDatabaseAvailable()) {
    logger.error("Anky creation failed: database not available");
    return c.json({ error: "Database not available" }, 503);
  }

  const params = await c.req.json();
  const authWallet = getAuthWallet(c);

  // Auto-assign userId from auth if not provided
  if (authWallet && !params.userId) {
    const user = await dbOps.getUserByWallet(authWallet);
    if (user) {
      params.userId = user.id;
    }
  }

  if (!params.writingSessionId) {
    logger.warn("Anky creation failed: missing writingSessionId");
    return c.json({ error: "writingSessionId required" }, 400);
  }

  const anky = await dbOps.createAnky(params);
  logger.info(
    `Anky created: ${anky?.id} for session ${params.writingSessionId}`,
  );

  // Link generated image to anky for IPFS retry support
  if (params.generatedImageId && anky?.id) {
    try {
      await dbOps.linkImageToAnky(params.generatedImageId, anky.id);
      logger.debug(`Linked image ${params.generatedImageId} to anky ${anky.id}`);
    } catch (e) {
      logger.error("Failed to link image to anky:", e);
    }
  }

  return c.json({ anky });
});

// Get all ankys for public gallery (public, rate limited)
app.get("/ankys", publicReadLimiter, async (c) => {
  logger.debug("Fetching ankys for gallery");
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const limit = Math.min(parseInt(c.req.query("limit") || "50"), 100);
  const offset = parseInt(c.req.query("offset") || "0");
  const writerTypeParam = c.req.query("writerType");
  const writerType = writerTypeParam === "human" || writerTypeParam === "agent" ? writerTypeParam : "all";

  const { ankys, total } = await dbOps.getAnkysForGallery(limit, offset, writerType as 'human' | 'agent' | 'all');
  logger.debug(
    `Gallery query: ${ankys.length} ankys returned (total: ${total}, filter: ${writerType})`,
  );

  // Transform response: truncate reflection and format session data
  const formattedAnkys = ankys.map((anky) => ({
    id: anky.id,
    title: anky.title,
    imageUrl: anky.imageUrl,
    reflection: anky.reflection
      ? anky.reflection.length > 200
        ? anky.reflection.slice(0, 200) + "..."
        : anky.reflection
      : null,
    createdAt: anky.createdAt,
    writerType: anky.writingSession?.writerType || "human",
    session: anky.writingSession
      ? {
          shareId: anky.writingSession.shareId,
          wordCount: anky.writingSession.wordCount,
          durationSeconds: anky.writingSession.durationSeconds,
        }
      : null,
  }));

  return c.json({
    ankys: formattedAnkys,
    total,
    hasMore: offset + ankys.length < total,
  });
});

// Update anky (requires auth)
app.patch("/ankys/:ankyId", authMiddleware, async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const ankyId = c.req.param("ankyId");
  const authWallet = getAuthWallet(c);
  const updates = await c.req.json();

  // Verify ownership (would need to fetch anky and check userId)
  // For now, trust that the frontend sends correct ankyId for the user

  const anky = await dbOps.updateAnky(ankyId, updates);
  return c.json({ anky });
});

// Get anky by session ID (public)
app.get("/sessions/:sessionId/anky", async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const sessionId = c.req.param("sessionId");
  const anky = await dbOps.getAnkyBySession(sessionId);

  if (!anky) {
    return c.json({ error: "Anky not found" }, 404);
  }

  return c.json({ anky });
});

// Retry IPFS upload for an anky (requires auth)
app.post("/ankys/:ankyId/retry-ipfs", authMiddleware, async (c) => {
  logger.info("Retrying IPFS upload for anky");
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const ankyId = c.req.param("ankyId");

  const ankyRecord = await dbOps.getAnkyById(ankyId);
  if (!ankyRecord) {
    return c.json({ error: "Anky not found" }, 404);
  }

  const linkedImage = await dbOps.getGeneratedImageByAnkyId(ankyId);
  if (!linkedImage) {
    return c.json({ error: "No linked image found for this anky" }, 404);
  }

  const session = await dbOps.getWritingSession(ankyRecord.writingSessionId);
  if (!session) {
    return c.json({ error: "Writing session not found" }, 404);
  }

  try {
    const [imageIpfsResult, writingIpfsResult] = await Promise.all([
      uploadImageToIPFS(linkedImage.imageBase64),
      uploadWritingToIPFS(session.content),
    ]);

    const metadataResult = await uploadMetadataToIPFS({
      title: ankyRecord.title || "",
      reflection: ankyRecord.reflection || "",
      imageIpfsHash: imageIpfsResult.ipfsHash,
      writingIpfsHash: writingIpfsResult.ipfsHash,
      imagePrompt: ankyRecord.imagePrompt || "",
    });

    const updatedAnky = await dbOps.updateAnky(ankyId, {
      writingIpfsHash: writingIpfsResult.ipfsHash,
      imageIpfsHash: imageIpfsResult.ipfsHash,
      metadataIpfsHash: metadataResult.ipfsHash,
      imageUrl: imageIpfsResult.gatewayUrl,
    });

    logger.info(`IPFS retry successful for anky ${ankyId}`);
    return c.json({ anky: updatedAnky });
  } catch (err) {
    logger.error("IPFS retry failed:", err);
    return c.json({ error: "IPFS upload failed" }, 500);
  }
});

// Record mint (requires auth)
app.post("/ankys/:ankyId/mint", authMiddleware, async (c) => {
  logger.info("Recording NFT mint");
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const ankyId = c.req.param("ankyId");
  const { txHash, tokenId } = await c.req.json();

  if (!txHash || tokenId === undefined) {
    logger.warn("Mint recording failed: missing txHash or tokenId");
    return c.json({ error: "txHash and tokenId required" }, 400);
  }

  const anky = await dbOps.recordMint(ankyId, txHash, tokenId);
  logger.info(
    `NFT minted: anky=${ankyId}, tokenId=${tokenId}, tx=${txHash.slice(0, 10)}...`,
  );
  return c.json({ anky });
});

// Get public anky feed (public)
app.get("/feed", async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const limit = parseInt(c.req.query("limit") || "50");
  const offset = parseInt(c.req.query("offset") || "0");
  const ankys = await dbOps.getPublicAnkyFeed(limit, offset);

  return c.json({ ankys });
});

// ----------------------------------------------------------------------------
// POLAR.SH CHECKOUT ENDPOINTS
// ----------------------------------------------------------------------------

// Create Polar checkout (requires auth)
app.post("/checkout", authMiddleware, async (c) => {
  logger.info("Creating Polar checkout");
  const authWallet = getAuthWallet(c);

  if (!authWallet) {
    return c.json({ error: "Unauthorized" }, 401);
  }

  const user = await dbOps.getUserByWallet(authWallet);
  if (!user) {
    return c.json({ error: "User not found" }, 404);
  }

  // Check existing subscription
  const hasSubscription = await dbOps.hasActiveSubscription(user.id);
  if (hasSubscription) {
    return c.json({ error: "Already subscribed", subscriptionActive: true }, 400);
  }

  const body = await c.req.json().catch(() => ({}));
  const successUrl = (body as Record<string, string>).successUrl || "https://anky.app/gallery";

  const checkoutUrl = await createCheckout(user.id, successUrl);
  if (!checkoutUrl) {
    return c.json({ error: "Failed to create checkout" }, 500);
  }

  return c.json({ checkoutUrl });
});

// Polar webhook handler using official Hono adapter
app.post("/webhooks/polar", Webhooks({
  webhookSecret: process.env.POLAR_WEBHOOK_SECRET!,
  onPayload: async (payload) => {
    const eventType = (payload as Record<string, unknown>).type as string;
    logger.info(`Polar webhook event: ${eventType}`);

    if (eventType === "checkout.completed") {
      const data = (payload as Record<string, unknown>).data as Record<string, unknown>;
      const metadata = data.metadata as Record<string, string> | undefined;
      const userId = metadata?.userId;
      const customerId = data.customerId as string | undefined;

      if (userId) {
        const expiresAt = new Date();
        expiresAt.setDate(expiresAt.getDate() + HUMAN_SUBSCRIPTION_DURATION_DAYS);

        await dbOps.updateUserSubscription(userId, {
          polarCustomerId: customerId,
          subscriptionExpiresAt: expiresAt,
        });

        logger.info(`Subscription activated for user ${userId.slice(0, 8)}... until ${expiresAt.toISOString()}`);
      }
    }
  },
}));

// ----------------------------------------------------------------------------
// CONVERSATION ENDPOINTS
// ----------------------------------------------------------------------------

// Get or create conversation (optional auth)
app.post("/conversations", optionalAuthMiddleware, async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const body = await c.req.json();
  const authWallet = getAuthWallet(c);

  // Auto-assign userId from auth if not provided
  let userId = body.userId;
  if (authWallet && !userId) {
    const user = await dbOps.getUserByWallet(authWallet);
    if (user) {
      userId = user.id;
    }
  }

  const conversation = await dbOps.getOrCreateConversation({
    userId,
    writingSessionId: body.writingSessionId,
  });

  return c.json({ conversation });
});

// Add message to conversation (optional auth)
app.post(
  "/conversations/:conversationId/messages",
  optionalAuthMiddleware,
  async (c) => {
    if (!isDatabaseAvailable()) {
      return c.json({ error: "Database not available" }, 503);
    }

    const conversationId = c.req.param("conversationId");
    const { role, content } = await c.req.json();

    if (!role || !content) {
      return c.json({ error: "role and content required" }, 400);
    }

    const result = await dbOps.addMessage(conversationId, role, content);

    if (!result) {
      return c.json({ error: "Conversation not found" }, 404);
    }

    if (result.capped) {
      return c.json({
        capped: true,
        message: "You've talked enough here. Start a new conversation?",
      });
    }

    return c.json({ message: result.message });
  },
);

// Get conversation messages (public for now)
app.get("/conversations/:conversationId/messages", async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const conversationId = c.req.param("conversationId");
  const limit = parseInt(c.req.query("limit") || "100");
  const messages = await dbOps.getConversationMessages(conversationId, limit);

  return c.json({ messages });
});

// Close conversation (requires auth)
app.post("/conversations/:conversationId/close", authMiddleware, async (c) => {
  if (!isDatabaseAvailable()) {
    return c.json({ error: "Database not available" }, 503);
  }

  const conversationId = c.req.param("conversationId");
  const conversation = await dbOps.closeConversation(conversationId);

  return c.json({ conversation });
});

// Log all routes when this module is loaded
logRegisteredRoutes();

export default app;
