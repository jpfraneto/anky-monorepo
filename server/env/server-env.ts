import { z } from "zod";
import dotenv from "dotenv";

// Load environment variables from .env file
dotenv.config();

// Define the environment schema
const envSchema = z.object({
  OPENROUTER_API_KEY: z.string({
    required_error: "OPENROUTER_API_KEY is required",
  }),
  SUPABASE_PASSWORD: z.string({
    required_error: "SUPABASE_PASSWORD is required",
  }),
  SUPABASE_URL: z.string({
    required_error: "SUPABASE_URL is required",
  }),
  SUPABASE_API_KEY: z.string({
    required_error: "SUPABASE_API_KEY is required",
  }),
});

// Function to get validated environment variables
export function getEnv(env: Record<string, string>) {
  // Merge process.env with provided env object to ensure .env values are used
  const mergedEnv = { ...process.env, ...env };
  return envSchema.parse(mergedEnv);
}

// Type for the environment
export type Env = z.infer<typeof envSchema>;
