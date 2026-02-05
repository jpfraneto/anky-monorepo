import { useState, useCallback, useEffect } from "react";
import type { WritingSession, AnkyData, ChatMessage } from "../types";

const STORAGE_KEY = "ankySessions";
const MAX_SESSIONS = 50; // Limit to prevent localStorage quota exceeded

export function useSessionStorage() {
  const [sessions, setSessions] = useState<WritingSession[]>([]);

  // Load sessions from localStorage on mount
  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      try {
        setSessions(JSON.parse(stored));
      } catch {
        setSessions([]);
      }
    }
  }, []);

  const saveSession = useCallback(
    (session: Omit<WritingSession, "timestamp">) => {
      const newSession: WritingSession = {
        ...session,
        timestamp: Date.now(),
      };

      setSessions((prev) => {
        // Keep only the most recent sessions to prevent quota exceeded
        const trimmed = prev.length >= MAX_SESSIONS ? prev.slice(-MAX_SESSIONS + 1) : prev;
        const updated = [...trimmed, newSession];

        try {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
        } catch (e) {
          // If quota exceeded, remove older sessions and retry
          if (e instanceof DOMException && e.name === 'QuotaExceededError') {
            const reduced = updated.slice(-10); // Keep only last 10 sessions
            try {
              localStorage.setItem(STORAGE_KEY, JSON.stringify(reduced));
              return reduced;
            } catch {
              // If still fails, clear all and save only current
              localStorage.setItem(STORAGE_KEY, JSON.stringify([newSession]));
              return [newSession];
            }
          }
        }
        return updated;
      });

      return sessions.length; // Return the index of the new session
    },
    [sessions.length]
  );

  const updateSession = useCallback(
    (
      index: number,
      updates: Partial<WritingSession>
    ) => {
      setSessions((prev) => {
        if (index < 0 || index >= prev.length) return prev;

        const updated = [...prev];
        updated[index] = { ...updated[index], ...updates };

        try {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
        } catch (e) {
          if (e instanceof DOMException && e.name === 'QuotaExceededError') {
            // Keep only recent sessions
            const reduced = updated.slice(-10);
            try {
              localStorage.setItem(STORAGE_KEY, JSON.stringify(reduced));
              return reduced;
            } catch {
              // Fail silently - session stays in memory but not persisted
              console.warn('Could not persist session to localStorage');
            }
          }
        }
        return updated;
      });
    },
    []
  );

  const updateSessionAnkyData = useCallback(
    (index: number, ankyData: AnkyData, title?: string) => {
      updateSession(index, {
        ankyData,
        ...(title && { ankyTitle: title }),
      });
    },
    [updateSession]
  );

  const updateSessionChatHistory = useCallback(
    (index: number, chatHistory: ChatMessage[]) => {
      updateSession(index, { chatHistory });
    },
    [updateSession]
  );

  const markSessionMinted = useCallback(
    (index: number) => {
      updateSession(index, { minted: true });
    },
    [updateSession]
  );

  const updateSessionBackendIds = useCallback(
    (index: number, backendId: string, shareId: string) => {
      updateSession(index, { id: backendId, shareId });
    },
    [updateSession]
  );

  const getSession = useCallback(
    (index: number): WritingSession | null => {
      if (index < 0 || index >= sessions.length) return null;
      return sessions[index];
    },
    [sessions]
  );

  const clearSessions = useCallback(() => {
    setSessions([]);
    localStorage.removeItem(STORAGE_KEY);
  }, []);

  return {
    sessions,
    saveSession,
    updateSession,
    updateSessionAnkyData,
    updateSessionChatHistory,
    markSessionMinted,
    updateSessionBackendIds,
    getSession,
    clearSessions,
  };
}
