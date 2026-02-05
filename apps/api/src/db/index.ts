import { drizzle } from 'drizzle-orm/postgres-js';
import postgres from 'postgres';
import * as schema from './schema';
import { Logger } from '../lib/logger';

const logger = Logger("Database");

const connectionString = process.env.DATABASE_URL;

if (!connectionString) {
  logger.warn('DATABASE_URL not set - database features will be disabled');
} else {
  logger.info('Connecting to database...');
}

// Create postgres connection (only if DATABASE_URL is set)
const client = connectionString ? postgres(connectionString) : null;

// Create drizzle instance
export const db = client ? drizzle(client, { schema }) : null;

if (db) {
  logger.info('Database connection established');
}

// Export schema for use elsewhere
export * from './schema';

// Helper to check if database is available
export const isDatabaseAvailable = () => db !== null;
