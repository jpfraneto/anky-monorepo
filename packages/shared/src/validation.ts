import { z } from "zod";

export const agentRegisterSchema = z.object({
  name: z
    .string()
    .min(1, "Name is required")
    .max(64, "Name must be 64 characters or less")
    .regex(/^[a-zA-Z0-9_-]+$/, "Name can only contain alphanumeric characters, hyphens, and underscores"),
  description: z.string().max(500, "Description must be 500 characters or less").optional(),
  model: z.string().max(100, "Model must be 100 characters or less").optional(),
});

export const sessionSubmitSchema = z.object({
  content: z
    .string()
    .min(1, "Content is required")
    .max(100_000, "Content must be 100,000 characters or less"),
  durationSeconds: z
    .number()
    .int()
    .min(1, "Duration must be at least 1 second")
    .max(7200, "Duration must be 7200 seconds or less"),
  wordCount: z
    .number()
    .int()
    .min(0, "Word count must be non-negative")
    .max(50_000, "Word count must be 50,000 or less"),
  wordsPerMinute: z.number().int().min(0).max(1000).optional(),
  isPublic: z.boolean().optional(),
  dayBoundaryHour: z.number().int().min(0).max(23).optional(),
  timezone: z.string().max(50).optional(),
});

export const agentSessionSubmitSchema = z.object({
  content: z
    .string()
    .min(1, "Content is required")
    .max(100_000, "Content must be 100,000 characters or less"),
  durationSeconds: z
    .number()
    .int()
    .min(1, "Duration must be at least 1 second")
    .max(7200, "Duration must be 7200 seconds or less"),
  wordCount: z
    .number()
    .int()
    .min(0, "Word count must be non-negative")
    .max(50_000, "Word count must be 50,000 or less"),
  wordsPerMinute: z.number().int().min(0).max(1000).optional(),
  payment: z
    .object({
      txHash: z.string().regex(/^0x[a-fA-F0-9]{64}$/, "Invalid transaction hash"),
      chain: z.literal("base"),
      method: z.enum(["usdc", "anky_token"]),
    })
    .optional(),
});

export type AgentRegisterInput = z.infer<typeof agentRegisterSchema>;
export type SessionSubmitInput = z.infer<typeof sessionSubmitSchema>;
export type AgentSessionSubmitInput = z.infer<typeof agentSessionSubmitSchema>;
