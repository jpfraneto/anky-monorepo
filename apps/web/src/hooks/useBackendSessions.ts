import { useState, useCallback, useEffect } from "react";
import { fetchMe, updateAnky, type RecentSession } from "../api";
import type { WritingSession, AnkyData, ChatMessage } from "../types";

// Convert backend RecentSession to frontend WritingSession format
function toWritingSession(session: RecentSession): WritingSession {
  return {
    id: session.id,
    shareId: session.shareId,
    content: session.fullContent || session.content,
    duration: session.durationSeconds,
    stats: {
      wordCount: session.wordCount,
      wpm: session.wpm,
    },
    timestamp: new Date(session.createdAt).getTime(),
    ankyData: session.anky
      ? {
          reflection: session.anky.reflection || undefined,
          url: session.anky.imageUrl || undefined,
          title: session.anky.title || undefined,
          imagePrompt: session.anky.imagePrompt || undefined,
          writingSessionIpfs: session.anky.writingIpfsHash || undefined,
          imageIpfs: session.anky.imageIpfsHash || undefined,
          tokenUri: session.anky.metadataIpfsHash || undefined,
        }
      : undefined,
    ankyTitle: session.anky?.title || undefined,
    minted: session.anky?.isMinted || false,
  };
}

export function useBackendSessions(userId?: string) {
  const [sessions, setSessions] = useState<WritingSession[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load sessions from backend
  const loadSessions = useCallback(async () => {
    if (!userId) {
      setSessions([]);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const meData = await fetchMe();
      const writingSessions = meData.recentSessions.map(toWritingSession);
      setSessions(writingSessions);
    } catch (e) {
      console.error("Failed to load sessions from backend:", e);
      setError("Failed to load sessions");
    } finally {
      setIsLoading(false);
    }
  }, [userId]);

  // Load on mount and when userId changes
  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  // Add a new session to the local state (after backend creates it)
  const addSession = useCallback(
    (session: {
      id: string;
      shareId: string;
      content: string;
      durationSeconds: number;
      wordCount: number;
      wpm: number;
    }): number => {
      const newSession: WritingSession = {
        id: session.id,
        shareId: session.shareId,
        content: session.content,
        duration: session.durationSeconds,
        stats: {
          wordCount: session.wordCount,
          wpm: session.wpm,
        },
        timestamp: Date.now(),
      };

      setSessions((prev) => [...prev, newSession]);
      return sessions.length; // Return the index of the new session
    },
    [sessions.length]
  );

  // Update anky data for a session (also updates backend)
  const updateSessionAnkyData = useCallback(
    async (
      index: number,
      ankyData: AnkyData,
      ankyId?: string,
      title?: string
    ) => {
      // Update local state
      setSessions((prev) => {
        if (index < 0 || index >= prev.length) return prev;
        const updated = [...prev];
        updated[index] = {
          ...updated[index],
          ankyData,
          ...(title && { ankyTitle: title }),
        };
        return updated;
      });

      // Update backend if we have ankyId and IPFS data
      if (ankyId && (ankyData.writingSessionIpfs || ankyData.imageIpfs || ankyData.tokenUri)) {
        try {
          await updateAnky(ankyId, {
            writingIpfsHash: ankyData.writingSessionIpfs,
            imageIpfsHash: ankyData.imageIpfs,
            metadataIpfsHash: ankyData.tokenUri,
          });
        } catch (e) {
          console.error("Failed to update anky in backend:", e);
        }
      }
    },
    []
  );

  // Update chat history for a session (stored locally only for now)
  const updateSessionChatHistory = useCallback(
    (index: number, chatHistory: ChatMessage[]) => {
      setSessions((prev) => {
        if (index < 0 || index >= prev.length) return prev;
        const updated = [...prev];
        updated[index] = { ...updated[index], chatHistory };
        return updated;
      });
    },
    []
  );

  // Mark session as minted
  const markSessionMinted = useCallback((index: number) => {
    setSessions((prev) => {
      if (index < 0 || index >= prev.length) return prev;
      const updated = [...prev];
      updated[index] = { ...updated[index], minted: true };
      return updated;
    });
    // Backend is updated via recordAnkyMint in HomePage
  }, []);

  // Get a specific session
  const getSession = useCallback(
    (index: number): WritingSession | null => {
      if (index < 0 || index >= sessions.length) return null;
      return sessions[index];
    },
    [sessions]
  );

  // Find session index by backend ID
  const findSessionIndexById = useCallback(
    (sessionId: string): number => {
      return sessions.findIndex((s) => s.id === sessionId);
    },
    [sessions]
  );

  return {
    sessions,
    isLoading,
    error,
    loadSessions,
    addSession,
    updateSessionAnkyData,
    updateSessionChatHistory,
    markSessionMinted,
    getSession,
    findSessionIndexById,
  };
}
