import { parseAnky } from "../ankyProtocol";
import type { AnkyThread, ThreadMessage, ThreadMode, ThreadRole } from "./types";

export const TERMINAL_SILENCE_MS = 8000;
export const FULL_ANKY_DURATION_MS = 8 * 60 * 1000;

export const MAX_FREE_THREAD_USER_MESSAGES = 3;
export const FULL_ANKY_THREAD_THRESHOLD_MS = FULL_ANKY_DURATION_MS;
export const THREAD_RESTING_MESSAGE =
  "this feels like enough for now. let the thread rest. you can write again when the moment comes.";

export function getAcceptedWritingDurationMs(
  parsed: ReturnType<typeof parseAnky> | null,
): number | null {
  if (parsed == null || !parsed.valid || parsed.startedAt == null || parsed.events.length === 0) {
    return null;
  }

  return Math.max(0, parsed.events[parsed.events.length - 1].acceptedAt - parsed.startedAt);
}

export function getRiteDurationMs(parsed: ReturnType<typeof parseAnky> | null): number | null {
  const acceptedWritingDurationMs = getAcceptedWritingDurationMs(parsed);

  if (parsed == null || acceptedWritingDurationMs == null) {
    return null;
  }

  return parsed.closed ? acceptedWritingDurationMs + TERMINAL_SILENCE_MS : acceptedWritingDurationMs;
}

export function isCompleteParsedAnky(parsed: ReturnType<typeof parseAnky> | null): boolean {
  const riteDurationMs = getRiteDurationMs(parsed);

  return riteDurationMs != null && riteDurationMs >= FULL_ANKY_DURATION_MS;
}

export function isCompleteRawAnky(raw: string): boolean {
  return isCompleteParsedAnky(parseAnky(raw));
}

export function getThreadModeForRawAnky(
  raw: string,
  hasReflection = false,
): ThreadMode {
  const parsed = parseAnky(raw);

  if (!isCompleteParsedAnky(parsed)) {
    return "fragment";
  }

  return hasReflection ? "reflection" : "complete";
}

export function getInitialAnkyMessage(mode: ThreadMode): string {
  switch (mode) {
    case "fragment":
      return "i feel the thread still moving. what wants to continue?";
    case "reflection":
      return "we already saw one mirror. what do you want to stay with?";
    case "complete":
      return "i'm here with what you wrote. what still feels alive?";
  }
}

export function createThreadMessage({
  content,
  createdAt = new Date().toISOString(),
  id = createThreadMessageId(),
  role,
}: {
  content: string;
  createdAt?: string;
  id?: string;
  role: ThreadRole;
}): ThreadMessage {
  return {
    content,
    createdAt,
    id,
    role,
  };
}

export function createInitialThread({
  createdAt = new Date().toISOString(),
  mode,
  sessionHash,
}: {
  createdAt?: string;
  mode: ThreadMode;
  sessionHash: string;
}): AnkyThread {
  const firstMessage = createThreadMessage({
    content: getInitialAnkyMessage(mode),
    createdAt,
    role: "anky",
  });

  return {
    version: 1,
    createdAt,
    messages: [firstMessage],
    mode,
    sessionHash,
    updatedAt: createdAt,
    userMessageCount: 0,
  };
}

export function appendThreadMessagesToThread(
  thread: AnkyThread,
  messages: ThreadMessage[],
  updatedAt = new Date().toISOString(),
): AnkyThread {
  const nextMessages = [...thread.messages, ...messages];

  return {
    ...thread,
    messages: nextMessages,
    updatedAt,
    userMessageCount: countUserMessages(nextMessages),
  };
}

export function hasReachedFreeThreadLimit(thread: AnkyThread): boolean {
  return thread.userMessageCount >= MAX_FREE_THREAD_USER_MESSAGES;
}

export function hasRestingMessage(thread: AnkyThread): boolean {
  return thread.messages.some(
    (message) => message.role === "anky" && message.content === THREAD_RESTING_MESSAGE,
  );
}

export function countUserMessages(messages: ThreadMessage[]): number {
  return messages.filter((message) => message.role === "user").length;
}

function createThreadMessageId(): string {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
}
