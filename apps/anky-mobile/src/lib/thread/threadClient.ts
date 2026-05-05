import { getAnkyApiClient } from "../api/client";
import { createThreadMessage } from "./threadLogic";
import type { ThreadMessage, ThreadMode } from "./types";

export class ThreadBackendUnavailableError extends Error {
  constructor() {
    super("anky cannot keep writing right now. your writing is still saved.");
    this.name = "ThreadBackendUnavailableError";
  }
}

export async function sendThreadMessage({
  existingReflection,
  messages,
  mode,
  rawAnky,
  reconstructedText,
  reflectionKind,
  sessionHash,
  userMessage,
}: {
  existingReflection?: string;
  messages: ThreadMessage[];
  mode: ThreadMode;
  rawAnky: string;
  reconstructedText: string;
  reflectionKind?: "full" | "quick";
  sessionHash: string;
  userMessage: string;
}): Promise<ThreadMessage> {
  const api = getAnkyApiClient();

  if (api == null) {
    throw new ThreadBackendUnavailableError();
  }

  const response = await api.sendThreadMessage({
    existingReflection,
    messages: messages.map((message) => ({
      content: message.content,
      createdAt: message.createdAt,
      role: message.role,
    })),
    mode,
    rawAnky,
    reconstructedText,
    reflectionKind,
    sessionHash,
    userMessage,
  });

  if (response.message.role !== "anky" || response.message.content.trim().length === 0) {
    throw new Error("The thread response was empty.");
  }

  return createThreadMessage({
    content: response.message.content,
    createdAt: response.message.createdAt,
    id: response.message.id,
    role: "anky",
  });
}
