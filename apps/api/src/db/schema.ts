import { pgTable, text, timestamp, integer, boolean, uuid, jsonb, index, uniqueIndex } from 'drizzle-orm/pg-core';
import { relations } from 'drizzle-orm';

// Users - linked to wallet addresses (Farcaster ID optional for future)
export const users = pgTable('users', {
  id: uuid('id').primaryKey().defaultRandom(),
  walletAddress: text('wallet_address').notNull().unique(),
  fid: integer('fid'), // Farcaster ID, optional
  dayBoundaryHour: integer('day_boundary_hour').notNull().default(4), // Hour when "day" resets (0-23), default 4am
  timezone: text('timezone').notNull().default('UTC'),
  freeSessionUsed: boolean('free_session_used').notNull().default(false),
  polarCustomerId: text('polar_customer_id'),
  subscriptionExpiresAt: timestamp('subscription_expires_at'),
  createdAt: timestamp('created_at').notNull().defaultNow(),
  updatedAt: timestamp('updated_at').notNull().defaultNow(),
}, (table) => [
  index('users_wallet_idx').on(table.walletAddress),
  index('users_fid_idx').on(table.fid),
]);

// Agents - AI agents that can write through Anky
export const agents = pgTable('agents', {
  id: uuid('id').primaryKey().defaultRandom(),
  name: text('name').notNull().unique(),
  description: text('description'),
  model: text('model'),
  apiKeyHash: text('api_key_hash').notNull(),
  ownerId: uuid('owner_id').references(() => users.id),
  createdAt: timestamp('created_at').notNull().defaultNow(),
  lastActiveAt: timestamp('last_active_at'),
  sessionCount: integer('session_count').notNull().default(0),
  isActive: boolean('is_active').notNull().default(true),
  freeSessionsRemaining: integer('free_sessions_remaining').notNull().default(4),
  totalPaidSessions: integer('total_paid_sessions').notNull().default(0),
  totalUsdSpent: text('total_usd_spent').notNull().default('0'),
  totalAnkyTokensSpent: text('total_anky_tokens_spent').notNull().default('0'),
}, (table) => [
  index('agents_name_idx').on(table.name),
  index('agents_api_key_hash_idx').on(table.apiKeyHash),
]);

// Agent Payments - track on-chain payment proofs
export const agentPayments = pgTable('agent_payments', {
  id: uuid('id').primaryKey().defaultRandom(),
  agentId: uuid('agent_id').notNull().references(() => agents.id),
  sessionId: uuid('session_id').references(() => writingSessions.id),
  txHash: text('tx_hash').notNull().unique(),
  chain: text('chain').notNull().default('base'),
  paymentMethod: text('payment_method').notNull(), // 'usdc' | 'anky_token'
  amount: text('amount').notNull(),
  tokenAddress: text('token_address'),
  usdEquivalent: text('usd_equivalent'),
  verified: boolean('verified').notNull().default(false),
  createdAt: timestamp('created_at').notNull().defaultNow(),
}, (table) => [
  index('agent_payments_agent_idx').on(table.agentId),
  uniqueIndex('agent_payments_tx_hash_idx').on(table.txHash),
  index('agent_payments_session_idx').on(table.sessionId),
]);

// Writing Sessions - every time someone writes
export const writingSessions = pgTable('writing_sessions', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id').references(() => users.id),

  // The writing itself
  content: text('content').notNull(),
  durationSeconds: integer('duration_seconds').notNull(),
  wordCount: integer('word_count').notNull(),
  wordsPerMinute: integer('words_per_minute'),

  // Is this a full Anky session (8+ minutes)?
  isAnky: boolean('is_anky').notNull().default(false),

  // Logical day tracking for streaks
  logicalDate: timestamp('logical_date').notNull(),

  // Shareable - generate a short link ID
  shareId: text('share_id').unique(),
  isPublic: boolean('is_public').notNull().default(true),

  // Writer type: human or agent
  writerType: text('writer_type').notNull().default('human'),
  agentId: uuid('agent_id').references(() => agents.id),

  createdAt: timestamp('created_at').notNull().defaultNow(),
}, (table) => [
  index('sessions_user_idx').on(table.userId),
  index('sessions_logical_date_idx').on(table.logicalDate),
  index('sessions_share_idx').on(table.shareId),
  index('sessions_is_anky_idx').on(table.isAnky),
  index('sessions_writer_type_idx').on(table.writerType),
  index('sessions_agent_idx').on(table.agentId),
]);

// Ankys - generated from 8+ minute sessions
export const ankys = pgTable('ankys', {
  id: uuid('id').primaryKey().defaultRandom(),
  writingSessionId: uuid('writing_session_id').notNull().references(() => writingSessions.id),
  userId: uuid('user_id').references(() => users.id),

  // AI Generated content
  imagePrompt: text('image_prompt'),
  reflection: text('reflection'),
  title: text('title'),

  // Image storage (IPFS gateway URL)
  imageUrl: text('image_url'),

  // IPFS hashes
  writingIpfsHash: text('writing_ipfs_hash'),
  imageIpfsHash: text('image_ipfs_hash'),
  metadataIpfsHash: text('metadata_ipfs_hash'),

  // NFT minting info
  isMinted: boolean('is_minted').notNull().default(false),
  mintTxHash: text('mint_tx_hash'),
  tokenId: integer('token_id'),
  mintedAt: timestamp('minted_at'),

  // Full metadata for the NFT
  nftMetadata: jsonb('nft_metadata'),

  createdAt: timestamp('created_at').notNull().defaultNow(),
}, (table) => [
  index('ankys_session_idx').on(table.writingSessionId),
  index('ankys_user_idx').on(table.userId),
  index('ankys_minted_idx').on(table.isMinted),
  index('ankys_created_at_idx').on(table.createdAt),
]);

// Conversations - chat sessions with Anky
export const conversations = pgTable('conversations', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id').references(() => users.id),
  writingSessionId: uuid('writing_session_id').references(() => writingSessions.id),

  messageCount: integer('message_count').notNull().default(0),
  isActive: boolean('is_active').notNull().default(true),

  maxMessages: integer('max_messages').notNull().default(50),

  createdAt: timestamp('created_at').notNull().defaultNow(),
  updatedAt: timestamp('updated_at').notNull().defaultNow(),
}, (table) => [
  index('conversations_user_idx').on(table.userId),
  index('conversations_session_idx').on(table.writingSessionId),
  index('conversations_active_idx').on(table.isActive),
]);

// Conversation Messages
export const conversationMessages = pgTable('conversation_messages', {
  id: uuid('id').primaryKey().defaultRandom(),
  conversationId: uuid('conversation_id').notNull().references(() => conversations.id),

  role: text('role').notNull(), // 'user' | 'assistant'
  content: text('content').notNull(),

  createdAt: timestamp('created_at').notNull().defaultNow(),
}, (table) => [
  index('messages_conversation_idx').on(table.conversationId),
]);

// Generated Images
export const generatedImages = pgTable('generated_images', {
  id: uuid('id').primaryKey().defaultRandom(),
  prompt: text('prompt').notNull(),
  imageBase64: text('image_base64').notNull(),
  imageUrl: text('image_url'),
  ankyId: uuid('anky_id').references(() => ankys.id),
  model: text('model').default('gemini-2.5-flash-preview-05-20'),
  generationTimeMs: integer('generation_time_ms'),
  createdAt: timestamp('created_at').notNull().defaultNow(),
}, (table) => [
  index('generated_images_created_idx').on(table.createdAt),
  index('generated_images_anky_idx').on(table.ankyId),
]);

// User Streaks
export const userStreaks = pgTable('user_streaks', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id').notNull().references(() => users.id).unique(),

  currentStreak: integer('current_streak').notNull().default(0),
  longestStreak: integer('longest_streak').notNull().default(0),

  lastAnkyDate: timestamp('last_anky_date'),

  totalAnkys: integer('total_ankys').notNull().default(0),
  totalWritingSessions: integer('total_writing_sessions').notNull().default(0),
  totalWordsWritten: integer('total_words_written').notNull().default(0),
  totalTimeWrittenSeconds: integer('total_time_written_seconds').notNull().default(0),

  updatedAt: timestamp('updated_at').notNull().defaultNow(),
}, (table) => [
  index('streaks_user_idx').on(table.userId),
]);

// Relations
export const usersRelations = relations(users, ({ many, one }) => ({
  writingSessions: many(writingSessions),
  ankys: many(ankys),
  conversations: many(conversations),
  streak: one(userStreaks),
  ownedAgents: many(agents),
}));

export const agentsRelations = relations(agents, ({ one, many }) => ({
  owner: one(users, {
    fields: [agents.ownerId],
    references: [users.id],
  }),
  writingSessions: many(writingSessions),
  payments: many(agentPayments),
}));

export const agentPaymentsRelations = relations(agentPayments, ({ one }) => ({
  agent: one(agents, {
    fields: [agentPayments.agentId],
    references: [agents.id],
  }),
  session: one(writingSessions, {
    fields: [agentPayments.sessionId],
    references: [writingSessions.id],
  }),
}));

export const writingSessionsRelations = relations(writingSessions, ({ one, many }) => ({
  user: one(users, {
    fields: [writingSessions.userId],
    references: [users.id],
  }),
  agent: one(agents, {
    fields: [writingSessions.agentId],
    references: [agents.id],
  }),
  anky: one(ankys),
  conversations: many(conversations),
}));

export const ankysRelations = relations(ankys, ({ one }) => ({
  writingSession: one(writingSessions, {
    fields: [ankys.writingSessionId],
    references: [writingSessions.id],
  }),
  user: one(users, {
    fields: [ankys.userId],
    references: [users.id],
  }),
}));

export const conversationsRelations = relations(conversations, ({ one, many }) => ({
  user: one(users, {
    fields: [conversations.userId],
    references: [users.id],
  }),
  writingSession: one(writingSessions, {
    fields: [conversations.writingSessionId],
    references: [writingSessions.id],
  }),
  messages: many(conversationMessages),
}));

export const conversationMessagesRelations = relations(conversationMessages, ({ one }) => ({
  conversation: one(conversations, {
    fields: [conversationMessages.conversationId],
    references: [conversations.id],
  }),
}));

export const userStreaksRelations = relations(userStreaks, ({ one }) => ({
  user: one(users, {
    fields: [userStreaks.userId],
    references: [users.id],
  }),
}));

export const generatedImagesRelations = relations(generatedImages, ({ one }) => ({
  anky: one(ankys, {
    fields: [generatedImages.ankyId],
    references: [ankys.id],
  }),
}));
