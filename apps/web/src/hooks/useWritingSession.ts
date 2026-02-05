import { useState, useRef, useCallback, useEffect } from "react";
import type { KeystrokeStats } from "../types";

const INACTIVITY_TIMEOUT = 8000;
const ANKY_THRESHOLD = 480; // 8 minutes in seconds
const TIMER_SHOW_DELAY = 3000; // 3 seconds before showing timer bar

interface UseWritingSessionReturn {
  isWriting: boolean;
  content: string;
  duration: number;
  wordCount: number;
  timerProgress: number;
  timerVisible: boolean;
  isAnky: boolean;
  keystrokeStats: KeystrokeStats;
  handleInput: (value: string) => void;
  handleKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  endSession: () => {
    content: string;
    elapsed: number;
    wordCount: number;
    wpm: number;
    isFullSession: boolean;
    keystrokeStats: KeystrokeStats;
  } | null;
  resetSession: () => void;
}

export function useWritingSession(
  onSessionEnd?: (data: {
    content: string;
    elapsed: number;
    wordCount: number;
    wpm: number;
    isFullSession: boolean;
    keystrokeStats: KeystrokeStats;
  }) => void
): UseWritingSessionReturn {
  const [isWriting, setIsWriting] = useState(false);
  const [content, setContent] = useState("");
  const [duration, setDuration] = useState(0);
  const [timerProgress, setTimerProgress] = useState(100);
  const [timerVisible, setTimerVisible] = useState(false);
  const [keystrokeStats, setKeystrokeStats] = useState<KeystrokeStats>({
    backspace: 0,
    enter: 0,
    arrows: 0,
  });

  const sessionStartTimeRef = useRef<number | null>(null);
  const lastKeyTimeRef = useRef<number | null>(null);
  const inactivityTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const timerIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const durationIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const timerShowTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const wordCount = content.trim().split(/\s+/).filter(Boolean).length;
  const isAnky = duration >= ANKY_THRESHOLD;

  const clearAllTimers = useCallback(() => {
    if (inactivityTimeoutRef.current) {
      clearTimeout(inactivityTimeoutRef.current);
      inactivityTimeoutRef.current = null;
    }
    if (timerIntervalRef.current) {
      clearInterval(timerIntervalRef.current);
      timerIntervalRef.current = null;
    }
    if (durationIntervalRef.current) {
      clearInterval(durationIntervalRef.current);
      durationIntervalRef.current = null;
    }
    if (timerShowTimeoutRef.current) {
      clearTimeout(timerShowTimeoutRef.current);
      timerShowTimeoutRef.current = null;
    }
  }, []);

  const endSession = useCallback(() => {
    if (!isWriting || !sessionStartTimeRef.current) return null;

    clearAllTimers();
    setIsWriting(false);
    setTimerVisible(false);

    const elapsed = Math.floor(
      (Date.now() - sessionStartTimeRef.current) / 1000
    );
    const words = content.trim().split(/\s+/).filter(Boolean).length;
    const wpm = elapsed > 0 ? Math.round((words / elapsed) * 60) : 0;
    const isFullSession = content.trim().length >= 100 && elapsed >= ANKY_THRESHOLD;

    const sessionData = {
      content,
      elapsed,
      wordCount: words,
      wpm,
      isFullSession,
      keystrokeStats,
    };

    onSessionEnd?.(sessionData);
    return sessionData;
  }, [isWriting, content, keystrokeStats, clearAllTimers, onSessionEnd]);

  const resetInactivityTimer = useCallback(() => {
    lastKeyTimeRef.current = Date.now();
    setTimerVisible(false);
    setTimerProgress(100);

    if (inactivityTimeoutRef.current) {
      clearTimeout(inactivityTimeoutRef.current);
    }
    if (timerIntervalRef.current) {
      clearInterval(timerIntervalRef.current);
    }
    if (timerShowTimeoutRef.current) {
      clearTimeout(timerShowTimeoutRef.current);
    }

    // After 3 seconds of inactivity, show the timer bar
    timerShowTimeoutRef.current = setTimeout(() => {
      setTimerVisible(true);
    }, TIMER_SHOW_DELAY);

    // Update timer progress only after 3 seconds
    timerIntervalRef.current = setInterval(() => {
      if (!lastKeyTimeRef.current) return;
      const elapsed = Date.now() - lastKeyTimeRef.current;

      // Only show progress after 3 seconds
      if (elapsed >= TIMER_SHOW_DELAY) {
        const remainingAfterDelay = Math.max(0, INACTIVITY_TIMEOUT - elapsed);
        const totalAfterDelay = INACTIVITY_TIMEOUT - TIMER_SHOW_DELAY;
        const progress = (remainingAfterDelay / totalAfterDelay) * 100;
        setTimerProgress(progress);
      }
    }, 100);

    inactivityTimeoutRef.current = setTimeout(() => {
      endSession();
    }, INACTIVITY_TIMEOUT);
  }, [endSession]);

  const startSession = useCallback(() => {
    setIsWriting(true);
    sessionStartTimeRef.current = Date.now();
    lastKeyTimeRef.current = Date.now();

    resetInactivityTimer();

    durationIntervalRef.current = setInterval(() => {
      if (!sessionStartTimeRef.current) return;
      const elapsed = Math.floor(
        (Date.now() - sessionStartTimeRef.current) / 1000
      );
      setDuration(elapsed);
    }, 1000);
  }, [resetInactivityTimer]);

  const handleInput = useCallback(
    (value: string) => {
      // Only allow adding characters, not removing
      if (value.length >= content.length) {
        setContent(value);
      }

      if (!isWriting && value.length > 0) {
        startSession();
      } else if (isWriting) {
        resetInactivityTimer();
      }
    },
    [isWriting, content.length, startSession, resetInactivityTimer]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // Prevent keyboard shortcuts
      if (e.ctrlKey || e.metaKey) {
        if (["a", "x", "c", "v", "z", "y"].includes(e.key.toLowerCase())) {
          e.preventDefault();
        }
      }

      // Track and prevent backspace, delete, and enter
      if (e.key === "Backspace" || e.key === "Delete") {
        e.preventDefault();
        setKeystrokeStats((prev) => ({ ...prev, backspace: prev.backspace + 1 }));
      }

      if (e.key === "Enter") {
        e.preventDefault();
        setKeystrokeStats((prev) => ({ ...prev, enter: prev.enter + 1 }));
      }

      // Track and prevent arrow keys and home/end
      if (
        [
          "ArrowLeft",
          "ArrowRight",
          "ArrowUp",
          "ArrowDown",
          "Home",
          "End",
        ].includes(e.key)
      ) {
        e.preventDefault();
        setKeystrokeStats((prev) => ({ ...prev, arrows: prev.arrows + 1 }));
      }

      // End session on Escape
      if (e.key === "Escape" && isWriting) {
        e.preventDefault();
        endSession();
      }
    },
    [isWriting, endSession]
  );

  const resetSession = useCallback(() => {
    clearAllTimers();
    setIsWriting(false);
    setContent("");
    setDuration(0);
    setTimerProgress(100);
    setTimerVisible(false);
    setKeystrokeStats({ backspace: 0, enter: 0, arrows: 0 });
    sessionStartTimeRef.current = null;
    lastKeyTimeRef.current = null;
  }, [clearAllTimers]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      clearAllTimers();
    };
  }, [clearAllTimers]);

  return {
    isWriting,
    content,
    duration,
    wordCount,
    timerProgress,
    timerVisible,
    isAnky,
    keystrokeStats,
    handleInput,
    handleKeyDown,
    endSession,
    resetSession,
  };
}
