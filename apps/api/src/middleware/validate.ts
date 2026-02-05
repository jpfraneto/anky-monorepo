import { createMiddleware } from "hono/factory";
import type { Context, Next } from "hono";
import type { ZodSchema, ZodError } from "zod";

// Extend Hono context with validated body
declare module "hono" {
  interface ContextVariableMap {
    validatedBody: unknown;
  }
}

export function validate(schema: ZodSchema) {
  return createMiddleware(async (c: Context, next: Next) => {
    let body: unknown;
    try {
      body = await c.req.json();
    } catch {
      return c.json({ error: "Invalid JSON body" }, 400);
    }

    const result = schema.safeParse(body);

    if (!result.success) {
      const error = result.error as ZodError;
      const details: Record<string, string[]> = {};
      for (const issue of error.issues) {
        const path = issue.path.join(".") || "_root";
        if (!details[path]) details[path] = [];
        details[path].push(issue.message);
      }
      return c.json({ error: "Validation failed", details }, 400);
    }

    c.set("validatedBody", result.data);
    await next();
  });
}
