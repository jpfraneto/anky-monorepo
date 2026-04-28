import { useEffect, useRef, useState } from "react";
import {
  Animated,
  Easing,
  Keyboard,
  KeyboardAvoidingView,
  NativeSyntheticEvent,
  Platform,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  TextInputKeyPressEventData,
  useWindowDimensions,
  View,
} from "react-native";
import Svg, { Circle } from "react-native-svg";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { RiteTimer } from "../components/anky/RiteTimer";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { WordFocus } from "../components/anky/WordFocus";
import {
  appendCharacter,
  appendFirstCharacter,
  closeSession,
  getLastAcceptedAt,
  hasTerminalLine,
  isAcceptedCharacter,
  parseAnky,
  reconstructText,
} from "../lib/ankyProtocol";
import {
  clearActiveDraft,
  readActiveDraft,
  saveClosedSession,
  writePendingReveal,
  writeActiveDraft,
} from "../lib/ankyStorage";
import { ankyColors } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Write">;

type FlyingWord = {
  id: string;
  progress: Animated.Value;
  threadIndex: number;
  word: string;
};

type StorageStatus = "closed" | "error" | "idle" | "persisted" | "persisting";

const SILENCE_LIMIT_MS = 8000;
const RITE_DURATION_MS = 8 * 60 * 1000;
const AURA_COLORS = [
  "#FFFFFF",
  "#B98CFF",
  "#7A5CFF",
  "#3CA7FF",
  "#D85CFF",
  "#FF5CC8",
  "#FF8A33",
  "#FF2D2D",
];

export function WriteScreen({ navigation, route }: Props) {
  const { height, width } = useWindowDimensions();
  const inputRef = useRef<TextInput>(null);
  const ankyStringRef = useRef("");
  const startedAtRef = useRef<number | null>(null);
  const lastAcceptedAtRef = useRef<number | null>(null);
  const currentWordRef = useRef("");
  const persistChainRef = useRef<Promise<void>>(Promise.resolve());
  const riteStartedAtRef = useRef<number | null>(null);
  const closedRef = useRef(false);
  const wovenCountRef = useRef(0);

  const [, setAnkyString] = useState("");
  const [currentWord, setCurrentWord] = useState("");
  const [silenceMs, setSilenceMs] = useState(0);
  const [flyingWords, setFlyingWords] = useState<FlyingWord[]>([]);
  const [keyboardHeight, setKeyboardHeight] = useState(0);
  const [riteRemainingMs, setRiteRemainingMs] = useState(RITE_DURATION_MS);
  const [storageStatus, setStorageStatus] = useState<StorageStatus>("idle");
  const [wovenCount, setWovenCount] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function loadRecoverableDraft() {
      const activeDraft = await readActiveDraft();

      if (!mounted || activeDraft == null || hasTerminalLine(activeDraft)) {
        return;
      }

      const parsed = parseAnky(activeDraft);
      const visibleText = reconstructText(activeDraft);
      const recoveredWord = getCurrentWord(visibleText);

      ankyStringRef.current = activeDraft;
      startedAtRef.current = parsed.startedAt;
      riteStartedAtRef.current = parsed.startedAt;
      lastAcceptedAtRef.current = getLastAcceptedAt(activeDraft);
      currentWordRef.current = recoveredWord;
      setAnkyString(activeDraft);
      setCurrentWord(recoveredWord);
    }

    if (route.params?.recoverDraft) {
      void loadRecoverableDraft();
    }

    const focusTimer = setTimeout(() => inputRef.current?.focus(), 120);
    const unsubscribe = navigation.addListener("focus", () => {
      setTimeout(() => inputRef.current?.focus(), 60);
    });

    return () => {
      mounted = false;
      clearTimeout(focusTimer);
      unsubscribe();
    };
  }, [navigation, route.params?.recoverDraft]);

  useEffect(() => {
    const showEvent = Platform.OS === "ios" ? "keyboardWillShow" : "keyboardDidShow";
    const hideEvent = Platform.OS === "ios" ? "keyboardWillHide" : "keyboardDidHide";
    const showSubscription = Keyboard.addListener(showEvent, (event) => {
      setKeyboardHeight(event.endCoordinates.height);
    });
    const hideSubscription = Keyboard.addListener(hideEvent, () => {
      setKeyboardHeight(0);
    });

    return () => {
      showSubscription.remove();
      hideSubscription.remove();
    };
  }, []);

  useEffect(() => {
    const interval = setInterval(() => {
      if (closedRef.current || lastAcceptedAtRef.current == null) {
        return;
      }

      const nextSilenceMs = Date.now() - lastAcceptedAtRef.current;
      setSilenceMs(nextSilenceMs);

      if (nextSilenceMs >= SILENCE_LIMIT_MS) {
        void closeIntoReveal();
      }
    }, 80);

    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    const interval = setInterval(() => {
      if (closedRef.current || riteStartedAtRef.current == null) {
        return;
      }

      const remaining = Math.max(0, RITE_DURATION_MS - (Date.now() - riteStartedAtRef.current));
      setRiteRemainingMs(remaining);

      if (remaining === 0 && ankyStringRef.current.length > 0) {
        void closeIntoReveal();
      }
    }, 250);

    return () => clearInterval(interval);
  }, []);

  const auraColor = auraFromSilence(silenceMs);
  const visibleHeight = keyboardHeight > 0 ? height - keyboardHeight : height;
  const portalSize = Math.max(190, Math.min(280, width * 0.62, visibleHeight * 0.72));
  const wordOpacity =
    currentWord.length === 0 ? 1 : Math.max(0, 1 - silenceMs / SILENCE_LIMIT_MS);

  function handleChangeText(input: string) {
    const characters = Array.from(input);

    if (characters.length !== 1 || characters[0] !== input) {
      return;
    }

    handlePotentialCharacter(characters[0]);
  }

  function handleKeyPress(event: NativeSyntheticEvent<TextInputKeyPressEventData>) {
    const { key } = event.nativeEvent;

    if (key === "Backspace" || key === "Delete" || key === "Enter" || key === "\n") {
      refocusInput();
      return;
    }

    if (key.startsWith("Arrow")) {
      refocusInput();
    }
  }

  function handlePotentialCharacter(char: string) {
    if (closedRef.current) {
      return;
    }

    if (char === "\n" || !isAcceptedCharacter(char)) {
      refocusInput();
      return;
    }

    handleAcceptedCharacter(char);
  }

  function handleAcceptedCharacter(char: string) {
    appendAcceptedCharacter(char);

    if (char === " ") {
      const word = currentWordRef.current;

      if (word.trim().length > 0) {
        spawnSwallowedWord(word);
      }

      currentWordRef.current = "";
      setCurrentWord("");
      return;
    }

    const nextWord = `${currentWordRef.current}${char}`;
    currentWordRef.current = nextWord;
    setCurrentWord(nextWord);
  }

  function appendAcceptedCharacter(char: string) {
    const now = Date.now();
    const lastAcceptedAt = lastAcceptedAtRef.current;
    let nextRaw: string;

    if (lastAcceptedAt == null) {
      startedAtRef.current = now;
      riteStartedAtRef.current = now;
      nextRaw = appendFirstCharacter(char, now);
    } else {
      nextRaw = appendCharacter(ankyStringRef.current, char, now, lastAcceptedAt).raw;
    }

    ankyStringRef.current = nextRaw;
    lastAcceptedAtRef.current = now;
    setAnkyString(nextRaw);
    setSilenceMs(0);
    enqueueDraftPersist(nextRaw);
  }

  function spawnSwallowedWord(word: string) {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    const progress = new Animated.Value(0);
    const threadIndex = wovenCountRef.current + 1;
    const flyingWord = { id, progress, threadIndex, word };

    wovenCountRef.current = threadIndex;
    setWovenCount(threadIndex);
    setFlyingWords((previous) => [...previous, flyingWord]);

    Animated.timing(progress, {
      duration: 920,
      easing: Easing.inOut(Easing.cubic),
      toValue: 1,
      useNativeDriver: true,
    }).start(() => {
      setFlyingWords((previous) => previous.filter((item) => item.id !== id));
    });
  }

  function enqueueDraftPersist(nextRaw: string) {
    setStorageStatus("persisting");

    persistChainRef.current = persistChainRef.current
      .catch(() => undefined)
      .then(() => writeActiveDraft(nextRaw))
      .then(() => setStorageStatus("persisted"))
      .catch((error) => {
        console.error(error);
        setStorageStatus("error");
      });
  }

  async function closeIntoReveal() {
    if (closedRef.current || ankyStringRef.current.length === 0) {
      return;
    }

    closedRef.current = true;
    setStorageStatus("persisting");

    try {
      const closedRaw = closeSession(ankyStringRef.current);
      ankyStringRef.current = closedRaw;
      setAnkyString(closedRaw);
      setCurrentWord("");
      currentWordRef.current = "";

      await persistChainRef.current.catch(() => undefined);
      await writeActiveDraft(closedRaw);
      const saved = await saveClosedSession(closedRaw);
      await writePendingReveal(closedRaw);
      await clearActiveDraft();

      setStorageStatus("closed");
      navigation.replace("Reveal", { fileName: saved.fileName });
    } catch (error) {
      console.error(error);
      closedRef.current = false;
      setStorageStatus("error");
      refocusInput();
    }
  }

  function refocusInput() {
    if (!closedRef.current) {
      setTimeout(() => inputRef.current?.focus(), 20);
    }
  }

  function handleBlur() {
    refocusInput();
  }

  return (
    <ScreenBackground safe={false} variant="centerGlow">
      <KeyboardAvoidingView
        behavior={Platform.OS === "ios" ? "padding" : "height"}
        keyboardVerticalOffset={0}
        style={styles.keyboardFrame}
      >
        <Pressable onPress={() => inputRef.current?.focus()} style={styles.stage}>
          <Portal auraColor={auraColor} size={portalSize} wovenCount={wovenCount} />

          <View style={styles.chamberMeta}>
            <Text style={styles.chamberMetaText}>
              {route.params?.dayNumber == null
                ? "anky is listening"
                : `day ${route.params.dayNumber} · anky is listening`}
            </Text>
          </View>

          <View style={styles.timerWrap}>
            <RiteTimer
              label={formatRiteTimer(riteRemainingMs)}
              remainingMs={riteRemainingMs}
              size={Math.max(238, Math.min(310, width * 0.74, visibleHeight * 0.72))}
              totalMs={RITE_DURATION_MS}
            >
              <Animated.View style={{ opacity: wordOpacity }}>
                <WordFocus word={currentWord} />
              </Animated.View>
            </RiteTimer>
          </View>

          <ThreadStream
            auraColor={auraColor}
            flyingWords={flyingWords}
            height={visibleHeight}
            portalSize={portalSize}
            width={width}
          />

          {storageStatus === "error" ? (
            <Text style={styles.storageError}>storage error</Text>
          ) : null}
        </Pressable>
      </KeyboardAvoidingView>

      <TextInput
        ref={inputRef}
        autoCapitalize="none"
        autoComplete="off"
        autoCorrect={false}
        blurOnSubmit={false}
        caretHidden
        contextMenuHidden
        disableFullscreenUI
        editable={!closedRef.current}
        importantForAutofill="no"
        keyboardAppearance="dark"
        multiline={false}
        onBlur={handleBlur}
        onChangeText={handleChangeText}
        onKeyPress={handleKeyPress}
        onSubmitEditing={refocusInput}
        returnKeyType="default"
        selectTextOnFocus={false}
        selection={{ start: 0, end: 0 }}
        showSoftInputOnFocus
        spellCheck={false}
        style={styles.hiddenInput}
        textContentType="none"
        value=""
      />
    </ScreenBackground>
  );
}

function Portal({
  auraColor,
  size,
  wovenCount,
}: {
  auraColor: string;
  size: number;
  wovenCount: number;
}) {
  const wovenOpacity = Math.min(0.52, 0.1 + wovenCount * 0.025);
  const dashPhase = 13 + (wovenCount % 7) * 2;

  return (
    <View style={[styles.portal, { height: size, width: size, left: -size * 0.32 }]}>
      <View
        style={[
          styles.portalGlow,
          {
            borderColor: auraColor,
            borderRadius: size / 2,
            shadowColor: auraColor,
          },
        ]}
      />
      <Svg height={size} width={size} viewBox="0 0 280 280">
        <Circle
          cx="140"
          cy="140"
          fill="transparent"
          opacity="0.95"
          r="110"
          stroke={auraColor}
          strokeWidth="4"
        />
        <Circle
          cx="140"
          cy="140"
          fill="transparent"
          opacity="0.35"
          r="88"
          stroke={auraColor}
          strokeWidth="1"
        />
        <Circle
          cx="140"
          cy="140"
          fill="transparent"
          opacity="0.18"
          r="58"
          stroke={auraColor}
          strokeWidth="1"
        />
        <Circle
          cx="140"
          cy="140"
          fill="transparent"
          opacity={wovenOpacity}
          r="72"
          stroke={auraColor}
          strokeDasharray={`${dashPhase} 10`}
          strokeWidth="1.4"
        />
        <Circle
          cx="140"
          cy="140"
          fill="transparent"
          opacity={wovenOpacity * 0.7}
          r="38"
          stroke={auraColor}
          strokeDasharray={`7 ${dashPhase}`}
          strokeWidth="1"
        />
      </Svg>
    </View>
  );
}

function ThreadStream({
  auraColor,
  flyingWords,
  height,
  portalSize,
  width,
}: {
  auraColor: string;
  flyingWords: FlyingWord[];
  height: number;
  portalSize: number;
  width: number;
}) {
  const startX = width * 0.58;
  const endX = Math.max(18, portalSize * 0.18);
  const startY = height * 0.34;
  const endY = height * 0.42;
  const strandWidth = Math.max(120, startX - endX + 120);

  return (
    <View pointerEvents="none" style={StyleSheet.absoluteFill}>
      {flyingWords.map((flyingWord) => {
        const phase = flyingWord.threadIndex % 5;
        const strandTop = startY + (phase - 2) * 4;
        const strandOpacity = flyingWord.progress.interpolate({
          inputRange: [0, 0.12, 0.74, 1],
          outputRange: [0, 0.9, 0.62, 0],
        });
        const strandScaleX = flyingWord.progress.interpolate({
          inputRange: [0, 0.22, 0.82, 1],
          outputRange: [0.1, 1, 0.86, 0.18],
        });
        const strandTranslateX = flyingWord.progress.interpolate({
          inputRange: [0, 1],
          outputRange: [0, -strandWidth * 0.22],
        });

        return (
          <View key={flyingWord.id} style={StyleSheet.absoluteFill}>
            <Animated.View
              style={[
                styles.threadStrand,
                {
                  backgroundColor: auraColor,
                  left: endX,
                  opacity: strandOpacity,
                  shadowColor: auraColor,
                  top: strandTop,
                  width: strandWidth,
                  transform: [{ translateX: strandTranslateX }, { scaleX: strandScaleX }],
                },
              ]}
            />
            <Animated.View
              style={[
                styles.threadStrandThin,
                {
                  backgroundColor: auraColor,
                  left: endX + 8,
                  opacity: strandOpacity,
                  shadowColor: auraColor,
                  top: strandTop + 7,
                  width: strandWidth * 0.92,
                  transform: [{ translateX: strandTranslateX }, { scaleX: strandScaleX }],
                },
              ]}
            />

            {Array.from(flyingWord.word).map((char, index) => {
              const yDrift = Math.sin((index + phase) * 1.4) * 15;
              const translateX = flyingWord.progress.interpolate({
                inputRange: [0, 1],
                outputRange: [startX + index * 17, endX + 20 + index * 0.7],
              });
              const translateY = flyingWord.progress.interpolate({
                inputRange: [0, 0.55, 1],
                outputRange: [startY + yDrift, startY - 30 - yDrift * 0.35, endY + yDrift * 0.14],
              });
              const opacity = flyingWord.progress.interpolate({
                inputRange: [0, 0.76, 1],
                outputRange: [1, 0.88, 0],
              });
              const scale = flyingWord.progress.interpolate({
                inputRange: [0, 1],
                outputRange: [1, 0.2],
              });

              return (
                <Animated.Text
                  key={`${flyingWord.id}-${index}`}
                  style={[
                    styles.flyingGlyph,
                    {
                      color: auraColor,
                      opacity,
                      textShadowColor: auraColor,
                      transform: [{ translateX }, { translateY }, { scale }],
                    },
                  ]}
                >
                  {char}
                </Animated.Text>
              );
            })}

            <FlyingKnot
              auraColor={auraColor}
              endX={endX}
              endY={endY}
              flyingWord={flyingWord}
              startX={startX + Array.from(flyingWord.word).length * 17}
              startY={startY + 12}
            />
          </View>
        );
      })}
    </View>
  );
}

function FlyingKnot({
  auraColor,
  endX,
  endY,
  flyingWord,
  startX,
  startY,
}: {
  auraColor: string;
  endX: number;
  endY: number;
  flyingWord: FlyingWord;
  startX: number;
  startY: number;
}) {
  const translateX = flyingWord.progress.interpolate({
    inputRange: [0, 1],
    outputRange: [startX, endX + 18],
  });
  const translateY = flyingWord.progress.interpolate({
    inputRange: [0, 0.55, 1],
    outputRange: [startY, startY - 20, endY],
  });
  const opacity = flyingWord.progress.interpolate({
    inputRange: [0, 0.8, 1],
    outputRange: [0.9, 0.86, 0],
  });
  const scale = flyingWord.progress.interpolate({
    inputRange: [0, 1],
    outputRange: [1, 0.16],
  });

  return (
    <Animated.View
      style={[
        styles.threadKnot,
        {
          backgroundColor: auraColor,
          opacity,
          shadowColor: auraColor,
          transform: [{ translateX }, { translateY }, { scale }],
        },
      ]}
    />
  );
}

function auraFromSilence(ms: number): string {
  const index = Math.min(AURA_COLORS.length - 1, Math.floor(ms / 1000));

  return AURA_COLORS[index];
}

function getCurrentWord(value: string): string {
  return value.trimEnd().split(/\s+/).filter(Boolean).at(-1) ?? "";
}

function formatRiteTimer(ms: number): string {
  const totalSeconds = Math.ceil(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;

  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

const styles = StyleSheet.create({
  chamberMeta: {
    left: 24,
    position: "absolute",
    right: 24,
    top: 58,
  },
  chamberMetaText: {
    color: ankyColors.textMuted,
    fontSize: 12,
    fontWeight: "700",
    letterSpacing: 1.2,
    textAlign: "center",
    textTransform: "uppercase",
  },
  flyingGlyph: {
    fontSize: 20,
    fontWeight: "700",
    left: 0,
    position: "absolute",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 14,
    top: 0,
  },
  hiddenInput: {
    bottom: 0,
    height: 1,
    opacity: 0.01,
    position: "absolute",
    width: 1,
  },
  keyboardFrame: {
    flex: 1,
  },
  portal: {
    alignItems: "center",
    justifyContent: "center",
    position: "absolute",
    top: "22%",
  },
  portalGlow: {
    ...StyleSheet.absoluteFillObject,
    borderWidth: 1,
    opacity: 0.7,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.95,
    shadowRadius: 28,
  },
  stage: {
    flex: 1,
    overflow: "hidden",
  },
  storageError: {
    bottom: 18,
    color: ankyColors.danger,
    fontSize: 12,
    left: 0,
    letterSpacing: 0.8,
    position: "absolute",
    right: 0,
    textAlign: "center",
  },
  threadKnot: {
    borderRadius: 5,
    height: 10,
    left: 0,
    position: "absolute",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.95,
    shadowRadius: 12,
    top: 0,
    width: 10,
  },
  threadStrand: {
    borderRadius: 999,
    height: 3,
    position: "absolute",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.88,
    shadowRadius: 16,
  },
  threadStrandThin: {
    borderRadius: 999,
    height: 1,
    position: "absolute",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.72,
    shadowRadius: 12,
  },
  timerWrap: {
    alignItems: "center",
    justifyContent: "center",
    left: 0,
    position: "absolute",
    right: 0,
    top: "18%",
  },
});
