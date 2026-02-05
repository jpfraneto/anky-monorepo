import { serve } from "bun";
import { Hono } from "hono";
import { serveStatic } from "hono/bun";
import { cors } from "hono/cors";
import apiRoutes from "./api/index.js";
import v1Routes from "./api/v1.js";
import { getConfig, printStartupBanner } from "./config";
import { Logger } from "./lib/logger";

const logger = Logger("Server");
const config = getConfig();

const app = new Hono();

// Top-level request logging (before any other middleware)
app.use("*", async (c, next) => {
  const method = c.req.method;
  const path = c.req.path;
  const url = c.req.url;
  const origin = c.req.header("origin") || "no-origin";

  logger.info(`[SERVER] Incoming: ${method} ${path}`);
  logger.info(`[SERVER] Full URL: ${url}`);
  logger.info(`[SERVER] Origin: ${origin}`);

  // Log preflight requests specifically
  if (method === "OPTIONS") {
    logger.info(`[SERVER] PREFLIGHT REQUEST for ${path}`);
    logger.info(
      `[SERVER] Access-Control-Request-Method: ${c.req.header("access-control-request-method")}`,
    );
    logger.info(
      `[SERVER] Access-Control-Request-Headers: ${c.req.header("access-control-request-headers")}`,
    );
  }

  await next();

  const status = c.res.status;
  const contentType = c.res.headers.get("content-type") || "unknown";
  const corsOrigin =
    c.res.headers.get("access-control-allow-origin") || "not-set";
  const corsMethods =
    c.res.headers.get("access-control-allow-methods") || "not-set";

  logger.info(
    `[SERVER] Response: ${method} ${path} → ${status} (${contentType})`,
  );
  logger.info(
    `[SERVER] CORS Headers: Allow-Origin=${corsOrigin}, Allow-Methods=${corsMethods}`,
  );
});

// CORS configuration for API routes
app.use(
  "/api/*",
  cors({
    origin: [...config.cors.origins, process.env.FRONTEND_URL || ""].filter(
      Boolean,
    ),
    credentials: true,
    allowMethods: ["GET", "POST", "PATCH", "DELETE", "OPTIONS"],
    allowHeaders: ["Content-Type", "Authorization", "X-API-Key"],
  }),
);

// Mount API routes FIRST (before static files)
app.route("/api", apiRoutes);
app.route("/api/v1", v1Routes);

logger.info("API routes mounted at /api");
logger.info("v1 API routes mounted at /api/v1");

// Serve skill.md at both /skill.md and /api/v1/skill.md
app.get("/skill.md", serveStatic({ path: "./public/skill.md" }));
app.get("/api/v1/skill.md", serveStatic({ path: "./public/skill.md" }));
logger.info("skill.md served at /skill.md and /api/v1/skill.md");

// Serve static files from public directory (for reference images etc.)
app.use("/public/*", serveStatic({ root: "./public" }));

// Serve frontend from web dist (built frontend)
app.use("/*", serveStatic({ root: "../web/dist" }));

// Fallback to index.html for SPA routing (skip API paths)
app.get("*", async (c, next) => {
  // Don't serve SPA fallback for API routes
  if (c.req.path.startsWith("/api/") || c.req.path.startsWith("/api")) {
    return next();
  }
  return serveStatic({ path: "../web/dist/index.html" })(c, next);
});

const port = config.runtime.port;

// Print startup banner
printStartupBanner();

serve({
  fetch: app.fetch,
  port,
});

logger.info(`Server started successfully on port ${port}`);
