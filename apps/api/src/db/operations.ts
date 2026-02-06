import { eq, desc, and, sql, gt } from 'drizzle-orm';
import { db, users, writingSessions, ankys, conversations, conversationMessages, userStreaks, generatedImages, agents, agentPayments } from './index';
import { getLogicalDate, areConsecutiveDays, isSameDay, daysSince, generateShareId } from './streak-utils';

// ============================================================================
// USER OPERATIONS
// ============================================================================

export async function getOrCreateUser(walletAddress: string) {
  if (!db) return null;

  const normalizedAddress = walletAddress.toLowerCase();

  // Try to find existing user
  const existing = await db.query.users.findFirst({
    where: eq(users.walletAddress, normalizedAddress),
  });

  if (existing) return existing;

  // Create new user
  const result = await db.insert(users).values({
    walletAddress: normalizedAddress,
  }).returning();

  const newUser = result[0];
  if (!newUser) return null;

  // Initialize streak record
  await db.insert(userStreaks).values({
    userId: newUser.id,
  });

  return newUser;
}

export async function getUserByWallet(walletAddress: string) {
  if (!db) return null;

  return db.query.users.findFirst({
    where: eq(users.walletAddress, walletAddress.toLowerCase()),
  });
}

export async function updateUserSettings(userId: string, settings: {
  dayBoundaryHour?: number;
  timezone?: string;
}) {
  if (!db) return null;

  const [updated] = await db.update(users)
    .set({
      ...settings,
      updatedAt: new Date(),
    })
    .where(eq(users.id, userId))
    .returning();

  return updated;
}

// ============================================================================
// WRITING SESSION OPERATIONS
// ============================================================================

export async function createWritingSession(params: {
  userId?: string;
  content: string;
  durationSeconds: number;
  wordCount: number;
  wordsPerMinute?: number;
  isPublic?: boolean;
  dayBoundaryHour?: number;
  timezone?: string;
}) {
  if (!db) return null;

  const isAnky = params.durationSeconds >= 480; // 8 minutes
  const logicalDate = getLogicalDate(
    new Date(),
    params.dayBoundaryHour || 4,
    params.timezone || 'UTC'
  );
  const shareId = generateShareId();

  const [session] = await db.insert(writingSessions).values({
    userId: params.userId,
    content: params.content,
    durationSeconds: params.durationSeconds,
    wordCount: params.wordCount,
    wordsPerMinute: params.wordsPerMinute,
    isAnky,
    logicalDate,
    shareId,
    isPublic: params.isPublic ?? true,
  }).returning();

  // Update user streak if this is an Anky and user is logged in
  if (isAnky && params.userId) {
    await updateStreak(params.userId, logicalDate, params.wordCount, params.durationSeconds);
  } else if (params.userId) {
    // Update total stats for non-Anky sessions too
    await updateSessionStats(params.userId, params.wordCount, params.durationSeconds);
  }

  return session;
}

export async function getWritingSession(sessionId: string) {
  if (!db) return null;

  return db.query.writingSessions.findFirst({
    where: eq(writingSessions.id, sessionId),
    with: {
      anky: true,
      user: true,
    },
  });
}

export async function getWritingSessionByShareId(shareId: string) {
  if (!db) return null;

  return db.query.writingSessions.findFirst({
    where: and(
      eq(writingSessions.shareId, shareId),
      eq(writingSessions.isPublic, true)
    ),
    with: {
      anky: true,
    },
  });
}

export async function getUserWritingSessions(userId: string, limit = 50) {
  if (!db) return [];

  return db.query.writingSessions.findMany({
    where: eq(writingSessions.userId, userId),
    orderBy: [desc(writingSessions.createdAt)],
    limit,
    with: {
      anky: true,
    },
  });
}

export async function toggleSessionPrivacy(sessionId: string, isPublic: boolean) {
  if (!db) return null;

  const [updated] = await db.update(writingSessions)
    .set({ isPublic })
    .where(eq(writingSessions.id, sessionId))
    .returning();

  return updated;
}

// ============================================================================
// ANKY OPERATIONS
// ============================================================================

export async function createAnky(params: {
  writingSessionId: string;
  userId?: string;
  imagePrompt?: string;
  reflection?: string;
  title?: string;
  imageUrl?: string;
  writingIpfsHash?: string;
  imageIpfsHash?: string;
  metadataIpfsHash?: string;
  nftMetadata?: Record<string, unknown>;
}) {
  if (!db) return null;

  const [anky] = await db.insert(ankys).values(params).returning();
  return anky;
}

export async function updateAnky(ankyId: string, updates: {
  imagePrompt?: string;
  reflection?: string;
  title?: string;
  imageUrl?: string;
  writingIpfsHash?: string;
  imageIpfsHash?: string;
  metadataIpfsHash?: string;
  isMinted?: boolean;
  mintTxHash?: string;
  tokenId?: number;
  mintedAt?: Date;
  nftMetadata?: Record<string, unknown>;
}) {
  if (!db) return null;

  const [updated] = await db.update(ankys)
    .set(updates)
    .where(eq(ankys.id, ankyId))
    .returning();

  return updated;
}

export async function getAnkyBySession(writingSessionId: string) {
  if (!db) return null;

  return db.query.ankys.findFirst({
    where: eq(ankys.writingSessionId, writingSessionId),
    with: {
      writingSession: true,
    },
  });
}

export async function getUserAnkys(userId: string, limit = 50) {
  if (!db) return [];

  return db.query.ankys.findMany({
    where: eq(ankys.userId, userId),
    orderBy: [desc(ankys.createdAt)],
    limit,
    with: {
      writingSession: true,
    },
  });
}

export async function getPublicAnkyFeed(limit = 50, offset = 0) {
  if (!db) return [];

  return db.query.ankys.findMany({
    columns: {
      id: true,
      title: true,
      imageUrl: true,
      reflection: true,
      imagePrompt: true,
      createdAt: true,
      writingSessionId: true,
      userId: true,
    },
    orderBy: [desc(ankys.createdAt)],
    limit,
    offset,
    with: {
      writingSession: {
        columns: {
          content: true,
          durationSeconds: true,
          wordCount: true,
          isPublic: true,
          createdAt: true,
        },
      },
      user: {
        columns: {
          walletAddress: true,
        },
      },
    },
  });
}

// Gallery-specific column selection
const galleryAnkyColumns = {
  id: true,
  title: true,
  imageUrl: true,
  reflection: true,
  createdAt: true,
  writingSessionId: true,
  userId: true,
} as const;

export async function getAnkysForGallery(limit = 50, offset = 0, writerType?: 'human' | 'agent' | 'all') {
  if (!db) return { ankys: [], total: 0 };

  // For 'all' or no filter, use simple query
  const baseCondition = sql`${ankys.imageUrl} IS NOT NULL`;

  if (!writerType || writerType === 'all') {
    // Run data and count queries in parallel for speed
    const [results, countResult] = await Promise.all([
      db.query.ankys.findMany({
        columns: galleryAnkyColumns,
        where: baseCondition,
        orderBy: [desc(ankys.createdAt)],
        limit,
        offset,
        with: {
          writingSession: {
            columns: {
              shareId: true,
              wordCount: true,
              durationSeconds: true,
              writerType: true,
              agentId: true,
            },
          },
        },
      }),
      db.select({ count: sql<number>`count(*)::int` })
        .from(ankys)
        .where(baseCondition),
    ]);

    return { ankys: results, total: countResult[0]?.count ?? 0 };
  }

  // For filtered queries, use efficient JOIN instead of subquery
  // Fetch more than needed and filter in app (faster than subquery for small datasets)
  const results = await db.query.ankys.findMany({
    columns: galleryAnkyColumns,
    where: baseCondition,
    orderBy: [desc(ankys.createdAt)],
    limit: limit * 3, // Fetch more to account for filtering
    with: {
      writingSession: {
        columns: {
          shareId: true,
          wordCount: true,
          durationSeconds: true,
          writerType: true,
          agentId: true,
        },
      },
    },
  });

  // Filter in application layer (fast since data is already loaded)
  const filtered = results.filter(anky => anky.writingSession?.writerType === writerType);
  const paginatedResults = filtered.slice(offset, offset + limit);

  return { ankys: paginatedResults, total: filtered.length };
}

export async function recordMint(ankyId: string, txHash: string, tokenId: number) {
  if (!db) return null;

  const [updated] = await db.update(ankys)
    .set({
      isMinted: true,
      mintTxHash: txHash,
      tokenId,
      mintedAt: new Date(),
    })
    .where(eq(ankys.id, ankyId))
    .returning();

  return updated;
}

// ============================================================================
// CONVERSATION OPERATIONS
// ============================================================================

export async function getOrCreateConversation(params: {
  userId?: string;
  writingSessionId?: string;
}) {
  if (!db) return null;

  // Try to find existing active conversation for this session
  if (params.writingSessionId) {
    const existing = await db.query.conversations.findFirst({
      where: and(
        eq(conversations.writingSessionId, params.writingSessionId),
        eq(conversations.isActive, true)
      ),
      with: {
        messages: {
          orderBy: [desc(conversationMessages.createdAt)],
        },
      },
    });

    if (existing) return existing;
  }

  // Create new conversation
  const [conversation] = await db.insert(conversations).values({
    userId: params.userId,
    writingSessionId: params.writingSessionId,
  }).returning();

  return { ...conversation, messages: [] };
}

export async function addMessage(conversationId: string, role: 'user' | 'assistant', content: string) {
  if (!db) return null;

  // Get conversation to check message count
  const conversation = await db.query.conversations.findFirst({
    where: eq(conversations.id, conversationId),
  });

  if (!conversation) return null;

  // Check if conversation has hit the cap
  if (conversation.messageCount >= conversation.maxMessages) {
    return { capped: true, message: null };
  }

  // Add message
  const [message] = await db.insert(conversationMessages).values({
    conversationId,
    role,
    content,
  }).returning();

  // Update message count
  await db.update(conversations)
    .set({
      messageCount: conversation.messageCount + 1,
      updatedAt: new Date(),
    })
    .where(eq(conversations.id, conversationId));

  return { capped: false, message };
}

export async function getConversationMessages(conversationId: string, limit = 100) {
  if (!db) return [];

  return db.query.conversationMessages.findMany({
    where: eq(conversationMessages.conversationId, conversationId),
    orderBy: [conversationMessages.createdAt],
    limit,
  });
}

export async function closeConversation(conversationId: string) {
  if (!db) return null;

  const [updated] = await db.update(conversations)
    .set({
      isActive: false,
      updatedAt: new Date(),
    })
    .where(eq(conversations.id, conversationId))
    .returning();

  return updated;
}

export async function getUserConversations(userId: string, limit = 20) {
  if (!db) return [];

  return db.query.conversations.findMany({
    where: eq(conversations.userId, userId),
    orderBy: [desc(conversations.updatedAt)],
    limit,
    with: {
      writingSession: {
        columns: {
          id: true,
          isAnky: true,
          createdAt: true,
        },
      },
    },
  });
}

// ============================================================================
// STREAK OPERATIONS
// ============================================================================

async function updateStreak(userId: string, logicalDate: Date, wordCount: number, durationSeconds: number) {
  if (!db) return;

  const streak = await db.query.userStreaks.findFirst({
    where: eq(userStreaks.userId, userId),
  });

  if (!streak) {
    // Create streak record
    await db.insert(userStreaks).values({
      userId,
      currentStreak: 1,
      longestStreak: 1,
      lastAnkyDate: logicalDate,
      totalAnkys: 1,
      totalWritingSessions: 1,
      totalWordsWritten: wordCount,
      totalTimeWrittenSeconds: durationSeconds,
    });
    return;
  }

  let newCurrentStreak = streak.currentStreak;
  let newLongestStreak = streak.longestStreak;

  if (streak.lastAnkyDate) {
    const days = daysSince(streak.lastAnkyDate, logicalDate);

    if (days === 0) {
      // Same day, streak stays the same
      // But still update totals
    } else if (days === 1) {
      // Consecutive day, increment streak
      newCurrentStreak = streak.currentStreak + 1;
      newLongestStreak = Math.max(newLongestStreak, newCurrentStreak);
    } else {
      // Streak broken, start fresh
      newCurrentStreak = 1;
    }
  } else {
    // First Anky ever
    newCurrentStreak = 1;
    newLongestStreak = 1;
  }

  await db.update(userStreaks)
    .set({
      currentStreak: newCurrentStreak,
      longestStreak: newLongestStreak,
      lastAnkyDate: logicalDate,
      totalAnkys: streak.totalAnkys + 1,
      totalWritingSessions: streak.totalWritingSessions + 1,
      totalWordsWritten: streak.totalWordsWritten + wordCount,
      totalTimeWrittenSeconds: streak.totalTimeWrittenSeconds + durationSeconds,
      updatedAt: new Date(),
    })
    .where(eq(userStreaks.userId, userId));
}

async function updateSessionStats(userId: string, wordCount: number, durationSeconds: number) {
  if (!db) return;

  await db.update(userStreaks)
    .set({
      totalWritingSessions: sql`${userStreaks.totalWritingSessions} + 1`,
      totalWordsWritten: sql`${userStreaks.totalWordsWritten} + ${wordCount}`,
      totalTimeWrittenSeconds: sql`${userStreaks.totalTimeWrittenSeconds} + ${durationSeconds}`,
      updatedAt: new Date(),
    })
    .where(eq(userStreaks.userId, userId));
}

export async function getUserStreak(userId: string) {
  if (!db) return null;

  const streak = await db.query.userStreaks.findFirst({
    where: eq(userStreaks.userId, userId),
  });

  if (!streak) return null;

  // Check if streak is still active (not broken)
  const user = await db.query.users.findFirst({
    where: eq(users.id, userId),
  });

  if (!user || !streak.lastAnkyDate) return streak;

  const currentLogicalDate = getLogicalDate(new Date(), user.dayBoundaryHour, user.timezone);
  const days = daysSince(streak.lastAnkyDate, currentLogicalDate);

  // If more than 1 day has passed, streak is broken (but we don't update it until next Anky)
  const streakIsActive = days <= 1;

  return {
    ...streak,
    streakIsActive,
    daysSinceLastAnky: days,
  };
}

// ============================================================================
// GENERATED IMAGES OPERATIONS
// ============================================================================

export async function saveGeneratedImage(params: {
  prompt: string;
  imageBase64: string;
  imageUrl?: string;
  model?: string;
  generationTimeMs?: number;
}) {
  if (!db) return null;

  const [image] = await db.insert(generatedImages).values({
    prompt: params.prompt,
    imageBase64: params.imageBase64,
    imageUrl: params.imageUrl,
    model: params.model || 'gemini-2.5-flash-preview-05-20',
    generationTimeMs: params.generationTimeMs,
  }).returning();

  return image;
}

export async function getGeneratedImages(limit = 50, offset = 0) {
  if (!db) return [];

  return db.query.generatedImages.findMany({
    orderBy: [desc(generatedImages.createdAt)],
    limit,
    offset,
  });
}

export async function getGeneratedImageById(id: string) {
  if (!db) return null;

  return db.query.generatedImages.findFirst({
    where: eq(generatedImages.id, id),
  });
}

export async function getAnkyById(ankyId: string) {
  if (!db) return null;

  return db.query.ankys.findFirst({
    where: eq(ankys.id, ankyId),
  });
}

export async function getGeneratedImageByAnkyId(ankyId: string) {
  if (!db) return null;

  return db.query.generatedImages.findFirst({
    where: eq(generatedImages.ankyId, ankyId),
  });
}

export async function linkImageToAnky(imageId: string, ankyId: string) {
  if (!db) return null;

  const [updated] = await db.update(generatedImages)
    .set({ ankyId })
    .where(eq(generatedImages.id, imageId))
    .returning();

  return updated;
}

// ============================================================================
// AGENT OPERATIONS
// ============================================================================

export async function createAgent(params: {
  name: string;
  description?: string;
  model?: string;
  apiKeyHash: string;
  ownerId?: string;
}) {
  if (!db) return null;

  const [agent] = await db.insert(agents).values(params).returning();
  return agent;
}

export async function getAgentByApiKeyHash(hash: string) {
  if (!db) return null;

  return db.query.agents.findFirst({
    where: eq(agents.apiKeyHash, hash),
  });
}

export async function getAgentById(agentId: string) {
  if (!db) return null;

  return db.query.agents.findFirst({
    where: eq(agents.id, agentId),
  });
}

export async function getAgentByName(name: string) {
  if (!db) return null;

  return db.query.agents.findFirst({
    where: eq(agents.name, name),
  });
}

export async function getAgentSessions(agentId: string, limit = 50) {
  if (!db) return [];

  return db.query.writingSessions.findMany({
    where: eq(writingSessions.agentId, agentId),
    orderBy: [desc(writingSessions.createdAt)],
    limit,
    with: {
      anky: {
        columns: {
          id: true,
          title: true,
          imageUrl: true,
          reflection: true,
          createdAt: true,
        },
      },
    },
  });
}

export async function incrementAgentSessionCount(agentId: string) {
  if (!db) return null;

  const [updated] = await db.update(agents)
    .set({
      sessionCount: sql`${agents.sessionCount} + 1`,
      lastActiveAt: new Date(),
    })
    .where(eq(agents.id, agentId))
    .returning();

  return updated;
}

export async function updateAgent(agentId: string, updates: {
  name?: string;
  description?: string;
  model?: string;
  isActive?: boolean;
}) {
  if (!db) return null;

  const [updated] = await db.update(agents)
    .set(updates)
    .where(eq(agents.id, agentId))
    .returning();

  return updated;
}

// ============================================================================
// MODIFIED: CREATE WRITING SESSION WITH AGENT SUPPORT
// ============================================================================

export async function createWritingSessionForAgent(params: {
  agentId: string;
  content: string;
  durationSeconds: number;
  wordCount: number;
  wordsPerMinute?: number;
  isPublic?: boolean;
}) {
  if (!db) return null;

  const isAnky = params.durationSeconds >= 480; // 8 minutes
  const logicalDate = getLogicalDate(new Date(), 4, 'UTC');
  const shareId = generateShareId();

  const [session] = await db.insert(writingSessions).values({
    content: params.content,
    durationSeconds: params.durationSeconds,
    wordCount: params.wordCount,
    wordsPerMinute: params.wordsPerMinute,
    isAnky,
    logicalDate,
    shareId,
    isPublic: params.isPublic ?? true,
    writerType: 'agent',
    agentId: params.agentId,
  }).returning();

  // Increment agent session count
  await incrementAgentSessionCount(params.agentId);

  return session;
}

// ============================================================================
// AGENT PAYMENT OPERATIONS
// ============================================================================

export async function getPaymentByTxHash(txHash: string) {
  if (!db) return null;

  return db.query.agentPayments.findFirst({
    where: eq(agentPayments.txHash, txHash),
  });
}

export async function createAgentPayment(params: {
  agentId: string;
  sessionId?: string;
  txHash: string;
  chain?: string;
  paymentMethod: string;
  amount: string;
  tokenAddress?: string;
  usdEquivalent?: string;
  verified?: boolean;
}) {
  if (!db) return null;

  const [payment] = await db.insert(agentPayments).values({
    agentId: params.agentId,
    sessionId: params.sessionId,
    txHash: params.txHash,
    chain: params.chain || 'base',
    paymentMethod: params.paymentMethod,
    amount: params.amount,
    tokenAddress: params.tokenAddress,
    usdEquivalent: params.usdEquivalent,
    verified: params.verified ?? false,
  }).returning();

  return payment;
}

export async function decrementAgentFreeSessions(agentId: string) {
  if (!db) return null;

  const [updated] = await db.update(agents)
    .set({
      freeSessionsRemaining: sql`GREATEST(${agents.freeSessionsRemaining} - 1, 0)`,
    })
    .where(eq(agents.id, agentId))
    .returning();

  return updated;
}

export async function updateAgentPaymentStats(agentId: string, method: 'usdc' | 'anky_token', amount: string) {
  if (!db) return null;

  const updates: Record<string, unknown> = {
    totalPaidSessions: sql`${agents.totalPaidSessions} + 1`,
  };

  if (method === 'usdc') {
    updates.totalUsdSpent = sql`(CAST(${agents.totalUsdSpent} AS DECIMAL) + ${parseFloat(amount)})::TEXT`;
  } else {
    updates.totalAnkyTokensSpent = sql`(CAST(${agents.totalAnkyTokensSpent} AS DECIMAL) + ${parseFloat(amount)})::TEXT`;
  }

  const [updated] = await db.update(agents)
    .set(updates)
    .where(eq(agents.id, agentId))
    .returning();

  return updated;
}

// ============================================================================
// SUBSCRIPTION OPERATIONS
// ============================================================================

export async function updateUserSubscription(userId: string, params: {
  polarCustomerId?: string;
  subscriptionExpiresAt?: Date;
}) {
  if (!db) return null;

  const [updated] = await db.update(users)
    .set({
      ...params,
      updatedAt: new Date(),
    })
    .where(eq(users.id, userId))
    .returning();

  return updated;
}

export async function markFreeSessionUsed(userId: string) {
  if (!db) return null;

  const [updated] = await db.update(users)
    .set({
      freeSessionUsed: true,
      updatedAt: new Date(),
    })
    .where(eq(users.id, userId))
    .returning();

  return updated;
}

export async function hasActiveSubscription(userId: string): Promise<boolean> {
  if (!db) return false;

  const user = await db.query.users.findFirst({
    where: and(
      eq(users.id, userId),
      gt(users.subscriptionExpiresAt, new Date())
    ),
  });

  return !!user;
}

export async function getUserById(userId: string) {
  if (!db) return null;

  return db.query.users.findFirst({
    where: eq(users.id, userId),
  });
}

// ============================================================================
// MIGRATION HELPER
// ============================================================================

export async function runMigrations() {
  console.log('Database migrations are handled by drizzle-kit');
  console.log('Run: bun run db:push to sync schema');
}
