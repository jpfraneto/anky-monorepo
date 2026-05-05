import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import * as FileSystem from "expo-file-system/legacy";

import { ensureAnkyDirectory, getAnkyDirectoryUri } from "../lib/ankyStorage";
import {
  ANKY_SEQUENCE_ORDER,
  DEFAULT_ANKY_SEQUENCE,
  getEmotionForSequence,
  getNextAnkySequence,
  type AnkyEmotion,
  type AnkyPresenceMode,
  type AnkySequenceName,
} from "./ankySequences";

/**
 * Anky Presence Rule:
 * Anky is a witness, not a mascot.
 * It should be available, never interruptive.
 * During writing, Anky becomes almost silent.
 */

export type AnkyPresenceIntensity = "minimal" | "normal";

export type AnkyPresenceScreenConfig = {
  avoidKeyboard?: boolean;
  emotion?: AnkyEmotion;
  intensity?: AnkyPresenceIntensity;
  maxMode?: AnkyPresenceMode;
  placement?: "left" | "right";
  preferredMode?: AnkyPresenceMode;
  sequence?: AnkySequenceName;
};

type AnkyPresenceContextValue = {
  clearScreenPresence: () => void;
  cycleSequence: () => void;
  effectiveMode: AnkyPresenceMode;
  emotion: AnkyEmotion;
  intensity: AnkyPresenceIntensity;
  mode: AnkyPresenceMode;
  screenConfig: AnkyPresenceScreenConfig | null;
  sequence: AnkySequenceName;
  setMode: (mode: AnkyPresenceMode) => void;
  setScreenPresence: (config: AnkyPresenceScreenConfig) => void;
  togglePresence: () => void;
};

type StoredAnkyPresence = {
  mode: AnkyPresenceMode;
  sequence: AnkySequenceName;
  version: 1;
};

const PRESENCE_PREFS_FILE = "ankyPresence.json";
const DEFAULT_MODE: AnkyPresenceMode = "hidden";
const MODE_RANK: Record<AnkyPresenceMode, number> = {
  companion: 2,
  hidden: 0,
  sigil: 1,
};

const AnkyPresenceContext = createContext<AnkyPresenceContextValue | null>(null);

export function AnkyPresenceProvider({ children }: { children: ReactNode }) {
  const [mode, setStoredMode] = useState<AnkyPresenceMode>(DEFAULT_MODE);
  const [selectedSequence, setSelectedSequence] =
    useState<AnkySequenceName>(DEFAULT_ANKY_SEQUENCE);
  const [screenConfig, setScreenConfig] = useState<AnkyPresenceScreenConfig | null>(null);
  const [hasSavedMode, setHasSavedMode] = useState(false);
  const [manualMode, setManualMode] = useState<AnkyPresenceMode | null>(null);
  const [manualSequence, setManualSequence] = useState<AnkySequenceName | null>(null);
  const selectedSequenceRef = useRef(selectedSequence);

  useEffect(() => {
    selectedSequenceRef.current = selectedSequence;
  }, [selectedSequence]);

  useEffect(() => {
    let mounted = true;

    async function loadPrefs() {
      const prefs = await readPresencePrefs();

      if (!mounted || prefs == null) {
        return;
      }

      setStoredMode(prefs.mode);
      setHasSavedMode(true);
      setSelectedSequence(prefs.sequence);
    }

    void loadPrefs();

    return () => {
      mounted = false;
    };
  }, []);

  const setMode = useCallback((nextMode: AnkyPresenceMode) => {
    setManualMode(nextMode);
    setStoredMode(nextMode);
    setHasSavedMode(true);
    void writePresencePrefs({
      mode: nextMode,
      sequence: selectedSequenceRef.current,
      version: 1,
    });
  }, []);

  const setScreenPresence = useCallback((config: AnkyPresenceScreenConfig) => {
    setManualMode(null);
    setManualSequence(null);
    setScreenConfig(config);
  }, []);

  const clearScreenPresence = useCallback(() => {
    setManualMode(null);
    setManualSequence(null);
    setScreenConfig(null);
  }, []);

  const sequence = manualSequence ?? screenConfig?.sequence ?? selectedSequence;
  const uncappedMode = chooseMode({
    hasSavedMode,
    manualMode,
    preferredMode: screenConfig?.preferredMode,
    storedMode: mode,
  });
  const effectiveMode = capMode(uncappedMode, screenConfig?.maxMode);
  const emotion = screenConfig?.emotion ?? getEmotionForSequence(sequence);
  const intensity = screenConfig?.intensity ?? "normal";

  const cycleSequence = useCallback(() => {
    const current = manualSequence ?? screenConfig?.sequence ?? selectedSequenceRef.current;
    const nextSequence = getNextAnkySequence(current);

    setManualSequence(nextSequence);
    setSelectedSequence(nextSequence);
    void writePresencePrefs({
      mode,
      sequence: nextSequence,
      version: 1,
    });
  }, [manualSequence, mode, screenConfig?.sequence]);

  const togglePresence = useCallback(() => {
    if (effectiveMode === "companion") {
      setMode("sigil");
      return;
    }

    setMode("companion");
  }, [effectiveMode, setMode]);

  const value = useMemo<AnkyPresenceContextValue>(
    () => ({
      clearScreenPresence,
      cycleSequence,
      effectiveMode,
      emotion,
      intensity,
      mode,
      screenConfig,
      sequence,
      setMode,
      setScreenPresence,
      togglePresence,
    }),
    [
      clearScreenPresence,
      cycleSequence,
      effectiveMode,
      emotion,
      intensity,
      mode,
      screenConfig,
      sequence,
      setMode,
      setScreenPresence,
      togglePresence,
    ],
  );

  return <AnkyPresenceContext.Provider value={value}>{children}</AnkyPresenceContext.Provider>;
}

export function useAnkyPresence() {
  const value = useContext(AnkyPresenceContext);

  if (value == null) {
    throw new Error("useAnkyPresence must be used inside AnkyPresenceProvider.");
  }

  return value;
}

function chooseMode({
  hasSavedMode,
  manualMode,
  preferredMode,
  storedMode,
}: {
  hasSavedMode: boolean;
  manualMode: AnkyPresenceMode | null;
  preferredMode?: AnkyPresenceMode;
  storedMode: AnkyPresenceMode;
}): AnkyPresenceMode {
  if (manualMode != null) {
    return manualMode;
  }

  if (preferredMode == null || (storedMode === "hidden" && hasSavedMode)) {
    return storedMode;
  }

  return MODE_RANK[preferredMode] > MODE_RANK[storedMode] ? preferredMode : storedMode;
}

function capMode(mode: AnkyPresenceMode, maxMode?: AnkyPresenceMode): AnkyPresenceMode {
  if (maxMode == null || MODE_RANK[mode] <= MODE_RANK[maxMode]) {
    return mode;
  }

  return maxMode;
}

async function readPresencePrefs(): Promise<StoredAnkyPresence | null> {
  await ensureAnkyDirectory();

  const uri = getPresencePrefsUri();
  const info = await FileSystem.getInfoAsync(uri);

  if (!info.exists) {
    return null;
  }

  try {
    const raw = await FileSystem.readAsStringAsync(uri, {
      encoding: FileSystem.EncodingType.UTF8,
    });
    const parsed = JSON.parse(raw) as unknown;

    return isStoredAnkyPresence(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

async function writePresencePrefs(prefs: StoredAnkyPresence): Promise<void> {
  await ensureAnkyDirectory();
  await FileSystem.writeAsStringAsync(getPresencePrefsUri(), JSON.stringify(prefs, null, 2), {
    encoding: FileSystem.EncodingType.UTF8,
  });
}

function getPresencePrefsUri(): string {
  return `${getAnkyDirectoryUri()}${PRESENCE_PREFS_FILE}`;
}

function isStoredAnkyPresence(value: unknown): value is StoredAnkyPresence {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const prefs = value as Partial<StoredAnkyPresence>;

  return (
    prefs.version === 1 &&
    isAnkyPresenceMode(prefs.mode) &&
    isAnkySequenceName(prefs.sequence)
  );
}

function isAnkyPresenceMode(value: unknown): value is AnkyPresenceMode {
  return value === "hidden" || value === "sigil" || value === "companion";
}

function isAnkySequenceName(value: unknown): value is AnkySequenceName {
  return typeof value === "string" && ANKY_SEQUENCE_ORDER.includes(value as AnkySequenceName);
}

export type { AnkyEmotion, AnkyPresenceMode, AnkySequenceName };
