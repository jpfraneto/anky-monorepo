import { useEffect, useRef, useState } from "react";
import {
  Animated,
  AppState,
  Easing,
  InteractionManager,
  Keyboard,
  NativeSyntheticEvent,
  Platform,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  TextInputKeyPressEventData,
  useWindowDimensions,
  View,
} from "react-native";
import Svg, {
  Circle as SvgCircle,
  G,
  Line as SvgLine,
  Path as SvgPath,
} from "react-native-svg";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import * as Haptics from "expo-haptics";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import type { RootStackParamList } from "../../App";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { SubtleIconButton } from "../components/navigation/SubtleIconButton";
import { WritingOpeningPrompt } from "../components/ritual/WritingOpeningPrompt";
import {
  appendCharacter,
  appendFirstCharacter,
  closeSession,
  getLastAcceptedAt,
  hasTerminalLine,
  parseAnky,
  reconstructText,
} from "../lib/ankyProtocol";
import {
  clearActiveDraft,
  readActiveDraft,
  saveClosedSession,
  type SavedAnkyFile,
  writeActiveDraft,
  writePendingReveal,
} from "../lib/ankyStorage";
import { getAcceptedInputCharacter } from "../lib/inputPolicy";
import { useAnkyPresenceScreen } from "../presence/useAnkyPresenceScreen";
import { ankyColors } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "ActiveWriting">;
type RevealPhase = "active" | "revealed" | "revealing";

const SILENCE_LIMIT_MS = 8000;
const SILENCE_WARNING_MS = 3000;
const SILENCE_WARNING_START_FILL = 0.675;
const RITE_DURATION_MS = 8 * 60 * 1000;
const RITE_SEGMENT_COUNT = 8;
const RITE_SEGMENT_MS = RITE_DURATION_MS / RITE_SEGMENT_COUNT;
const TEXT_REVEAL_DURATION_MS = 1300;
const RITE_SEGMENT_COLORS = [
  "#FF3B30",
  "#FF8A00",
  "#FFD60A",
  "#34C759",
  "#0A84FF",
  "#5E5CE6",
  "#BF5AF2",
  "#F2F2F7",
] as const;
const ANKY_PROMPT_POOL = [
  "what is trying to get your attention?",
  "start with the thing you keep avoiding.",
  "what feels true before you explain it?",
  "where is your body asking you to listen?",
  "write the sentence you do not want to polish.",
  "what changed in you today?",
  "what are you still carrying from yesterday?",
  "name the quiet thing under the noise.",
  "what do you want without defending it?",
  "what would you say if nobody needed you to be coherent?",
  "begin with the smallest honest detail.",
  "what is alive right now?",
  "what are you pretending not to know?",
  "follow the first image that appears.",
  "what feels unfinished in your chest?",
  "write toward the part of you that stayed quiet.",
  "what are you afraid will be true?",
  "what deserves tenderness here?",
  "what did the day leave inside you?",
  "start where the pressure is.",
] as const;

export function WriteScreen({ navigation, route }: Props) {
  const insets = useSafeAreaInsets();
  const { height, width } = useWindowDimensions();
  const backgroundScrollRef = useRef<ScrollView>(null);
  const inputRef = useRef<TextInput>(null);
  const inputValueRef = useRef("");
  const ankyStringRef = useRef("");
  const startedAtRef = useRef<number | null>(null);
  const lastAcceptedAtRef = useRef<number | null>(null);
  const persistChainRef = useRef<Promise<void>>(Promise.resolve());
  const riteStartedAtRef = useRef<number | null>(null);
  const closedRef = useRef(false);
  const focusTimersRef = useRef<ReturnType<typeof setTimeout>[]>([]);
  const lastRiteSegmentRef = useRef(0);
  const revealNavigationRequestedRef = useRef(false);
  const riteThresholdHapticRef = useRef(false);
  const silenceEndHapticRef = useRef(false);
  const silenceWarningHapticRef = useRef(false);

  const [inputValue, setInputValue] = useState("");
  const [isExiting, setIsExiting] = useState(false);
  const [keyboardHeight, setKeyboardHeight] = useState(0);
  const [lastCharacter, setLastCharacter] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [promptSuggestion, setPromptSuggestion] = useState<string | null>(null);
  const [openingPromptVisible, setOpeningPromptVisible] = useState(
    route.params?.recoverDraft !== true,
  );
  const [revealPhase, setRevealPhase] = useState<RevealPhase>("active");
  const [revealProgress, setRevealProgress] = useState(0);
  const [savedSession, setSavedSession] = useState<SavedAnkyFile | null>(null);
  const [riteElapsedMs, setRiteElapsedMs] = useState(0);
  const [silenceMs, setSilenceMs] = useState(0);
  const [typedText, setTypedText] = useState("");
  const hasWritten = typedText.length > 0;

  useAnkyPresenceScreen(
    hasWritten
      ? {
          avoidKeyboard: true,
          emotion: "listening",
          intensity: "minimal",
          maxMode: "hidden",
          placement: "left",
          preferredMode: "hidden",
          sequence: "shy_listening",
        }
      : {
          avoidKeyboard: true,
          emotion: "welcome",
          intensity: "normal",
          maxMode: "companion",
          placement: "left",
          preferredMode: "companion",
          sequence: "finding_thread",
        },
  );

  useEffect(() => {
    let mounted = true;

    async function loadRecoverableDraft() {
      const activeDraft = await readActiveDraft();

      if (!mounted || activeDraft == null || hasTerminalLine(activeDraft)) {
        return;
      }

      const parsed = parseAnky(activeDraft);

      if (parsed.events.length === 0 || parsed.startedAt == null) {
        return;
      }

      const lastAcceptedAt = getLastAcceptedAt(activeDraft);
      const recoveredText = reconstructText(activeDraft);
      const recoveredCharacters = Array.from(recoveredText);

      inputValueRef.current = recoveredText;
      ankyStringRef.current = activeDraft;
      startedAtRef.current = parsed.startedAt;
      riteStartedAtRef.current = parsed.startedAt;
      lastAcceptedAtRef.current = lastAcceptedAt;
      lastRiteSegmentRef.current = getRiteSegment(Date.now() - parsed.startedAt);
      riteThresholdHapticRef.current = Date.now() - parsed.startedAt >= RITE_DURATION_MS;
      setInputValue(recoveredText);
      setLastCharacter(recoveredCharacters.at(-1) ?? null);
      setRiteElapsedMs(Math.max(0, Date.now() - parsed.startedAt));
      setSilenceMs(lastAcceptedAt == null ? 0 : Math.max(0, Date.now() - lastAcceptedAt));
      setTypedText(recoveredText);
      setOpeningPromptVisible(false);
    }

    if (route.params?.recoverDraft) {
      void loadRecoverableDraft();
    }

    scheduleInputFocus([0, 80, 180, 360]);
    const interaction = InteractionManager.runAfterInteractions(() => {
      scheduleInputFocus([0, 120, 280]);
    });
    const unsubscribe = navigation.addListener("focus", () => {
      scheduleInputFocus([0, 80, 180, 360]);
    });

    return () => {
      mounted = false;
      interaction.cancel();
      clearInputFocusTimers();
      unsubscribe();
    };
  }, [navigation, route.params?.recoverDraft]);

  useEffect(() => {
    const subscription = AppState.addEventListener("change", (state) => {
      if (state === "active") {
        scheduleInputFocus([40, 180, 360]);
      }
    });

    return () => subscription.remove();
  }, []);

  useEffect(() => {
    const showEvent = Platform.OS === "ios" ? "keyboardWillShow" : "keyboardDidShow";
    const hideEvent = Platform.OS === "ios" ? "keyboardWillHide" : "keyboardDidHide";
    const showSubscription = Keyboard.addListener(showEvent, (event) => {
      setKeyboardHeight(event.endCoordinates.height);
    });
    const hideSubscription = Keyboard.addListener(hideEvent, () => {
      setKeyboardHeight(0);
      if (!closedRef.current) {
        scheduleInputFocus([60, 180]);
      }
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

      if (nextSilenceMs >= SILENCE_WARNING_MS && !silenceWarningHapticRef.current) {
        silenceWarningHapticRef.current = true;
        triggerSilenceWarningHaptic();
      }

      if (nextSilenceMs >= SILENCE_LIMIT_MS) {
        if (!silenceEndHapticRef.current) {
          silenceEndHapticRef.current = true;
          triggerSilenceEndHaptic();
        }
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

      const elapsed = Math.max(0, Date.now() - riteStartedAtRef.current);
      setRiteElapsedMs(elapsed);
      notifyRiteProgress(elapsed);
    }, 250);

    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (revealPhase !== "revealing") {
      return;
    }

    const started = Date.now();
    const interval = setInterval(() => {
      const progress = Math.min(1, (Date.now() - started) / TEXT_REVEAL_DURATION_MS);

      setRevealProgress(progress);

      if (progress >= 1) {
        clearInterval(interval);
        setRevealPhase("revealed");
      }
    }, 50);

    return () => clearInterval(interval);
  }, [revealPhase]);

  useEffect(() => {
    if (
      revealPhase !== "revealed" ||
      savedSession == null ||
      revealNavigationRequestedRef.current
    ) {
      return;
    }

    revealNavigationRequestedRef.current = true;
    navigation.replace("Reveal", { fileName: savedSession.fileName });
  }, [navigation, revealPhase, savedSession]);

  function handleChangeText(input: string) {
    const previousInput = inputValueRef.current;
    const decision = getAcceptedInputCharacter(previousInput, input);

    if (!decision.accepted) {
      inputValueRef.current = previousInput;
      setInputValue(previousInput);
      refocusInput();
      return;
    }

    const nextInput = `${previousInput}${decision.char}`;

    if (handlePotentialCharacter(decision.char)) {
      inputValueRef.current = nextInput;
      setInputValue(nextInput);
      return;
    }

    inputValueRef.current = previousInput;
    setInputValue(previousInput);
    refocusInput();
  }

  function handleKeyPress(event: NativeSyntheticEvent<TextInputKeyPressEventData>) {
    const { key } = event.nativeEvent;

    if (key === "Backspace" || key === "Delete") {
      handleDeletionVisual();
      refocusInput();
      return;
    }

    if (key.startsWith("Arrow")) {
      refocusInput();
    }
  }

  function handlePotentialCharacter(char: string): boolean {
    if (closedRef.current) {
      return false;
    }

    return handleAcceptedCharacter(char);
  }

  function handleAcceptedCharacter(char: string): boolean {
    if (!appendAcceptedCharacter(char)) {
      return false;
    }

    setOpeningPromptVisible(false);
    triggerKeystrokeHaptic();
    setLastCharacter(char);
    setTypedText((current) => `${current}${char}`);

    return true;
  }

  function handleDeletionVisual() {
    refocusInput();
  }

  function appendAcceptedCharacter(char: string): boolean {
    const now = Date.now();
    const lastAcceptedAt = lastAcceptedAtRef.current;
    const startedAt = startedAtRef.current ?? now;
    let nextRaw: string;

    try {
      if (lastAcceptedAt == null) {
        startedAtRef.current = now;
        riteStartedAtRef.current = now;
        lastRiteSegmentRef.current = 0;
        riteThresholdHapticRef.current = false;
        nextRaw = appendFirstCharacter(char, now);
      } else {
        nextRaw = appendCharacter(ankyStringRef.current, char, now, lastAcceptedAt).raw;
      }
    } catch (error) {
      console.error(error);
      refocusInput();
      return false;
    }

    ankyStringRef.current = nextRaw;
    lastAcceptedAtRef.current = now;
    startedAtRef.current = startedAt;
    const elapsed = Math.max(0, now - startedAt);
    setRiteElapsedMs(elapsed);
    setSilenceMs(0);
    silenceEndHapticRef.current = false;
    silenceWarningHapticRef.current = false;
    notifyRiteProgress(elapsed);
    enqueueDraftPersist(nextRaw);

    return true;
  }

  function enqueueDraftPersist(nextRaw: string) {
    persistChainRef.current = persistChainRef.current
      .catch(() => undefined)
      .then(() => writeActiveDraft(nextRaw))
      .catch((error) => {
        console.error(error);
      });
  }

  async function closeIntoReveal() {
    if (closedRef.current || ankyStringRef.current.length === 0) {
      return;
    }

    closedRef.current = true;
    clearInputFocusTimers();
    Keyboard.dismiss();
    setMessage("");
    setRevealPhase("revealing");
    setRevealProgress(0);

    const closedRaw = closeSession(ankyStringRef.current);

    ankyStringRef.current = closedRaw;
    setLastCharacter(null);

    try {
      await persistChainRef.current.catch(() => undefined);
      // The terminal draft stays in active.anky.draft until both durable reveal artifacts exist.
      await writeActiveDraft(closedRaw);
      const saved = await saveClosedSession(closedRaw);
      await writePendingReveal(closedRaw);
      await clearActiveDraft();
      setSavedSession(saved);
    } catch (error) {
      console.error(error);
      setMessage("Your writing is closed, but saving needs attention. Keep the app open.");
      try {
        await writePendingReveal(closedRaw);
      } catch (pendingError) {
        console.error(pendingError);
      }
    }
  }

  async function handleExitToHome() {
    if (closedRef.current) {
      return;
    }

    closedRef.current = true;
    setIsExiting(true);
    clearInputFocusTimers();
    await persistChainRef.current.catch(() => undefined);
    navigation.replace("Track");
    Keyboard.dismiss();
  }

  function refocusInput() {
    if (!closedRef.current) {
      scheduleInputFocus([20, 120]);
    }
  }

  function handleBlur() {
    refocusInput();
  }

  function triggerKeystrokeHaptic() {
    void Haptics.selectionAsync().catch(() => undefined);
  }

  function triggerRiteSegmentHaptic(segment: number) {
    const style =
      segment >= RITE_SEGMENT_COUNT
        ? Haptics.ImpactFeedbackStyle.Heavy
        : Haptics.ImpactFeedbackStyle.Medium;

    void Haptics.impactAsync(style).catch(() => undefined);
  }

  function triggerRiteThresholdHaptic() {
    void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success).catch(() => undefined);
  }

  function triggerSilenceWarningHaptic() {
    void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Warning).catch(() => undefined);
  }

  function triggerSilenceEndHaptic() {
    void Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Heavy).catch(() => undefined);
  }

  function notifyRiteProgress(elapsedMs: number) {
    const nextSegment = getRiteSegment(elapsedMs);

    if (nextSegment > lastRiteSegmentRef.current) {
      lastRiteSegmentRef.current = nextSegment;
      triggerRiteSegmentHaptic(nextSegment);
    }

    if (elapsedMs >= RITE_DURATION_MS && !riteThresholdHapticRef.current) {
      riteThresholdHapticRef.current = true;
      triggerRiteThresholdHaptic();
    }
  }

  function scheduleInputFocus(delays: number[]) {
    if (closedRef.current) {
      return;
    }

    clearInputFocusTimers();
    focusTimersRef.current = delays.map((delay) =>
      setTimeout(() => {
        if (!closedRef.current) {
          inputRef.current?.focus();
        }
      }, delay),
    );
  }

  function clearInputFocusTimers() {
    focusTimersRef.current.forEach((timer) => clearTimeout(timer));
    focusTimersRef.current = [];
  }

  function handleBackgroundContentSizeChange() {
    backgroundScrollRef.current?.scrollToEnd({ animated: false });
  }

  function handlePromptPress() {
    setOpeningPromptVisible(true);
    setPromptSuggestion((current) => getRandomPromptSuggestion(current));
    scheduleInputFocus([40, 140]);
  }

  const visibleLastCharacter = lastCharacter == null ? "" : visibleCharacter(lastCharacter);
  const isRevealed = revealPhase === "revealed";
  const isShowingClosedWriting = revealPhase !== "active";
  const estimatedKeyboardHeight = getEstimatedKeyboardHeight(height);
  const effectiveKeyboardHeight =
    keyboardHeight > 0 ? keyboardHeight : isShowingClosedWriting ? 0 : estimatedKeyboardHeight;
  const workspaceBottom = isShowingClosedWriting || isExiting ? 0 : effectiveKeyboardHeight;
  const visibleHeight = Math.max(320, height - workspaceBottom);
  const backgroundPaddingTop = insets.top + 42;
  const backgroundPaddingBottom = isShowingClosedWriting ? insets.bottom + 72 : 0;
  const backgroundPlaneMinHeight = Math.max(
    240,
    visibleHeight - backgroundPaddingTop - backgroundPaddingBottom,
  );
  const riteProgress = Math.max(0, Math.min(1, riteElapsedMs / RITE_DURATION_MS));
  const riteThresholdPassed = riteElapsedMs >= RITE_DURATION_MS;
  const silenceProgress = Math.max(0, Math.min(1, silenceMs / SILENCE_LIMIT_MS));
  const letterOpacity = lastCharacter == null ? 0 : Math.max(0, Math.min(1, 1 - silenceProgress * 0.72));
  const backgroundTextOpacity = isShowingClosedWriting ? 0.24 + revealProgress * 0.72 : 0.17;
  const backgroundPlaneScale = isShowingClosedWriting ? 1 - revealProgress * 0.035 : 1;
  const ringSize = Math.max(176, Math.min(268, width * 0.62, visibleHeight * 0.5));
  const showWritingCircle = revealPhase === "active" && !isExiting;

  return (
    <ScreenBackground safe={false} variant="plain">
      <View style={styles.root}>
        <View style={[styles.workspace, { bottom: workspaceBottom, opacity: isExiting ? 0 : 1 }]}>
          <ScrollView
            contentContainerStyle={[
              styles.backgroundWritingContent,
              {
                minHeight: visibleHeight,
                paddingBottom: backgroundPaddingBottom,
                paddingTop: backgroundPaddingTop,
              },
            ]}
            onContentSizeChange={handleBackgroundContentSizeChange}
            pointerEvents={isShowingClosedWriting ? "auto" : "none"}
            ref={backgroundScrollRef}
            scrollEnabled={isShowingClosedWriting}
            showsVerticalScrollIndicator={isShowingClosedWriting}
            style={styles.backgroundWritingScroll}
          >
            <View
              style={[
                styles.backgroundWritingPlane,
                {
                  minHeight: backgroundPlaneMinHeight,
                  transform: [{ scale: backgroundPlaneScale }],
                },
              ]}
            >
              <Text style={[styles.backgroundWritingShadow, { opacity: backgroundTextOpacity * 0.72 }]}>
                {typedText}
              </Text>
              <Text style={[styles.backgroundWriting, { opacity: backgroundTextOpacity }]}>
                {typedText}
              </Text>
            </View>
          </ScrollView>

          {showWritingCircle ? (
            <View pointerEvents="none" style={styles.lastCharacterWrap}>
              <View
                style={[
                  styles.writingCircle,
                  {
                    height: ringSize,
                    width: ringSize,
                  },
                ]}
              >
                <View style={styles.circleCastShadow} />
                <WritingProgressCircle
                  complete={riteThresholdPassed}
                  progress={riteProgress}
                  size={ringSize}
                />
                <InactivityCountdownCircle
                  elapsedMs={silenceMs}
                  size={ringSize}
                  visible={silenceMs >= SILENCE_WARNING_MS}
                />
                <View style={styles.writingCircleCore} />
                {hasWritten ? (
                  <LastCharacterGlyph
                    opacity={letterOpacity}
                    silenceProgress={silenceProgress}
                    size={ringSize}
                    value={visibleLastCharacter}
                  />
                ) : (
                  <WritingCursor size={ringSize} />
                )}
                {riteThresholdPassed ? <RiteThresholdCue size={ringSize} /> : null}
              </View>
            </View>
          ) : null}

          {!hasWritten && !isExiting ? (
            <>
              <SubtleIconButton
                accessibilityLabel="show a writing prompt"
                icon="✦"
                onPress={handlePromptPress}
                style={[styles.topLeftButton, { top: insets.top + 10 }]}
              />

              <SubtleIconButton
                accessibilityLabel="close writing"
                icon="×"
                onPress={() => void handleExitToHome()}
                style={[styles.topRightButton, { top: insets.top + 10 }]}
              />
            </>
          ) : null}

          {isRevealed && savedSession == null && message.length > 0 ? (
            <View style={[styles.postSessionActions, { paddingBottom: insets.bottom + 14 }]}>
              <Text style={styles.postSessionMessage}>{message}</Text>
            </View>
          ) : null}

          <WritingOpeningPrompt
            keyboardHeight={effectiveKeyboardHeight}
            prompt={promptSuggestion}
            safeBottom={insets.bottom}
            visible={openingPromptVisible && revealPhase === "active" && !hasWritten && !isExiting}
          />
        </View>

        <TextInput
          ref={inputRef}
          autoFocus
          autoCapitalize="none"
          autoComplete="off"
          autoCorrect={false}
          blurOnSubmit={false}
          caretHidden
          contextMenuHidden
          disableFullscreenUI
          editable={!closedRef.current && !isExiting}
          importantForAutofill="no"
          keyboardAppearance="dark"
          multiline
          onBlur={handleBlur}
          onChangeText={handleChangeText}
          onKeyPress={handleKeyPress}
          returnKeyType="default"
          selectTextOnFocus={false}
          selection={{ end: inputValue.length, start: inputValue.length }}
          showSoftInputOnFocus
          spellCheck={false}
          style={styles.hiddenInput}
          textContentType="none"
          value={inputValue}
        />
      </View>
    </ScreenBackground>
  );
}

function WritingProgressCircle({
  complete,
  progress,
  size,
}: {
  complete: boolean;
  progress: number;
  size: number;
}) {
  const strokeWidth = 6;
  const center = size / 2;
  const radius = center - strokeWidth - 2;
  const safeProgress = Math.max(0, Math.min(1, progress));

  return (
    <Svg height={size} style={StyleSheet.absoluteFill} viewBox={`0 0 ${size} ${size}`} width={size}>
      <SvgCircle
        cx={center}
        cy={center}
        fill="transparent"
        r={radius}
        stroke="rgba(244, 241, 234, 0.08)"
        strokeWidth={1}
      />
      <G>
        {RITE_SEGMENT_COLORS.map((color, index) => {
          const segmentStart = index / RITE_SEGMENT_COUNT;
          const segmentEnd = (index + 1) / RITE_SEGMENT_COUNT;
          const segmentFill = Math.max(
            0,
            Math.min(segmentEnd, safeProgress) - segmentStart,
          );
          const fillEnd = segmentStart + segmentFill;
          const hasFill = segmentFill > 0.001;

          return (
            <G key={color}>
              <SvgPath
                d={makeArcPath(center, center, radius, segmentStart, segmentEnd - 0.006)}
                fill="transparent"
                stroke={color}
                strokeLinecap="round"
                strokeOpacity={0.2}
                strokeWidth={2.2}
              />
              {hasFill ? (
                <SvgPath
                  d={makeArcPath(center, center, radius, segmentStart, fillEnd)}
                  fill="transparent"
                  stroke={color}
                  strokeLinecap="round"
                  strokeOpacity={complete ? 0.95 : 0.86}
                  strokeWidth={strokeWidth}
                />
              ) : null}
            </G>
          );
        })}
      </G>
      {Array.from({ length: RITE_SEGMENT_COUNT }).map((_, index) => {
        const pointA = polarPoint(center, center, radius - 3, index / RITE_SEGMENT_COUNT);
        const pointB = polarPoint(center, center, radius + 3, index / RITE_SEGMENT_COUNT);

        return (
          <SvgLine
            key={index}
            stroke={RITE_SEGMENT_COLORS[index]}
            strokeLinecap="round"
            strokeOpacity={0.26}
            strokeWidth={index === 0 ? 1.2 : 0.8}
            x1={pointA.x}
            x2={pointB.x}
            y1={pointA.y}
            y2={pointB.y}
          />
        );
      })}
      {complete ? (
        <SvgCircle
          cx={center}
          cy={center}
          fill="transparent"
          r={radius - 11}
          stroke="#F2F2F7"
          strokeOpacity={0.34}
          strokeWidth={1.3}
        />
      ) : null}
    </Svg>
  );
}

function InactivityCountdownCircle({
  elapsedMs,
  size,
  visible,
}: {
  elapsedMs: number;
  size: number;
  visible: boolean;
}) {
  if (!visible) {
    return null;
  }

  const center = size / 2;
  const radius = size * 0.29;
  const warningProgress = getSilenceWarningProgress(elapsedMs);
  const remainingFill = Math.max(0, SILENCE_WARNING_START_FILL * (1 - warningProgress));
  const color = warningProgress > 0.72 ? "#FFB35C" : ankyColors.gold;

  return (
    <Svg height={size} style={StyleSheet.absoluteFill} viewBox={`0 0 ${size} ${size}`} width={size}>
      <SvgCircle
        cx={center}
        cy={center}
        fill="transparent"
        r={radius}
        stroke="rgba(244, 241, 234, 0.055)"
        strokeWidth={1}
      />
      {remainingFill > 0.002 ? (
        <SvgPath
          d={makeCounterClockwiseArcPath(center, center, radius, remainingFill)}
          fill="transparent"
          stroke={color}
          strokeLinecap="round"
          strokeOpacity={0.38 + warningProgress * 0.2}
          strokeWidth={2}
        />
      ) : null}
    </Svg>
  );
}

function LastCharacterGlyph({
  opacity,
  silenceProgress,
  size,
  value,
}: {
  opacity: number;
  silenceProgress: number;
  size: number;
  value: string;
}) {
  const glyphSize = size * 0.62;
  const fontSize = size * 0.44;
  const lineHeight = size * 0.54;

  return (
    <View
      style={[
        styles.lastCharacterClip,
        {
          height: glyphSize,
          width: glyphSize,
        },
      ]}
    >
      <Text
        adjustsFontSizeToFit
        numberOfLines={1}
        style={[
          styles.lastCharacter,
          {
            fontSize,
            lineHeight,
            opacity,
          },
        ]}
      >
        {value}
      </Text>
      <View
        style={[
          styles.lastCharacterEraser,
          {
            height: glyphSize * silenceProgress,
          },
        ]}
      />
    </View>
  );
}

function WritingCursor({ size }: { size: number }) {
  const opacity = useRef(new Animated.Value(1)).current;

  useEffect(() => {
    const animation = Animated.loop(
      Animated.sequence([
        Animated.timing(opacity, {
          duration: 620,
          easing: Easing.inOut(Easing.quad),
          toValue: 0.16,
          useNativeDriver: true,
        }),
        Animated.timing(opacity, {
          duration: 620,
          easing: Easing.inOut(Easing.quad),
          toValue: 1,
          useNativeDriver: true,
        }),
      ]),
    );

    animation.start();

    return () => animation.stop();
  }, [opacity]);

  return (
    <Animated.View
      pointerEvents="none"
      style={[
        styles.writingCursor,
        {
          height: size * 0.22,
          opacity,
        },
      ]}
    />
  );
}

function RiteThresholdCue({ size }: { size: number }) {
  return (
    <View pointerEvents="none" style={[styles.riteThresholdCue, { bottom: size * 0.16 }]}>
      <Text style={styles.riteThresholdText}>8:00+</Text>
    </View>
  );
}

function getRiteSegment(elapsedMs: number): number {
  return Math.max(0, Math.min(RITE_SEGMENT_COUNT, Math.floor(elapsedMs / RITE_SEGMENT_MS)));
}

function getSilenceWarningProgress(elapsedMs: number): number {
  return Math.max(
    0,
    Math.min(1, (elapsedMs - SILENCE_WARNING_MS) / (SILENCE_LIMIT_MS - SILENCE_WARNING_MS)),
  );
}

function getEstimatedKeyboardHeight(screenHeight: number): number {
  const ratio = Platform.OS === "ios" ? 0.36 : 0.34;
  const minimum = Platform.OS === "ios" ? 280 : 240;
  const maximum = Platform.OS === "ios" ? 336 : 320;

  return Math.min(maximum, Math.max(minimum, screenHeight * ratio));
}

function makeArcPath(
  cx: number,
  cy: number,
  radius: number,
  startProgress: number,
  endProgress: number,
): string {
  const start = polarPoint(cx, cy, radius, startProgress);
  const end = polarPoint(cx, cy, radius, endProgress);
  const delta = Math.max(0, endProgress - startProgress);
  const largeArcFlag = delta > 0.5 ? 1 : 0;

  return `M ${start.x} ${start.y} A ${radius} ${radius} 0 ${largeArcFlag} 1 ${end.x} ${end.y}`;
}

function makeCounterClockwiseArcPath(
  cx: number,
  cy: number,
  radius: number,
  fill: number,
): string {
  const safeFill = Math.max(0, Math.min(1, fill));
  const start = polarPoint(cx, cy, radius, 0);
  const end = polarPoint(cx, cy, radius, -safeFill);
  const largeArcFlag = safeFill > 0.5 ? 1 : 0;

  return `M ${start.x} ${start.y} A ${radius} ${radius} 0 ${largeArcFlag} 0 ${end.x} ${end.y}`;
}

function polarPoint(
  cx: number,
  cy: number,
  radius: number,
  progress: number,
): { x: number; y: number } {
  const angle = -Math.PI / 2 + progress * Math.PI * 2;

  return {
    x: cx + Math.cos(angle) * radius,
    y: cy + Math.sin(angle) * radius,
  };
}

function visibleCharacter(value: string): string {
  if (value === " ") {
    return ".";
  }

  return value;
}

function getRandomPromptSuggestion(current: string | null): string {
  const promptCount: number = ANKY_PROMPT_POOL.length;

  if (promptCount <= 1) {
    return ANKY_PROMPT_POOL[0];
  }

  let next = ANKY_PROMPT_POOL[Math.floor(Math.random() * promptCount)];

  while (next === current) {
    const index = Math.floor(Math.random() * promptCount);
    next = ANKY_PROMPT_POOL[index];
  }

  return next;
}

const styles = StyleSheet.create({
  backgroundWriting: {
    color: "rgba(255, 232, 180, 0.66)",
    fontSize: 20,
    fontWeight: "500",
    left: 0,
    letterSpacing: 0,
    lineHeight: 32,
    opacity: 0.16,
    position: "absolute",
    right: 0,
    textShadowColor: "rgba(0, 0, 0, 0.88)",
    textShadowOffset: { height: 1, width: 0 },
    textShadowRadius: 2,
  },
  backgroundWritingContent: {
    flexGrow: 1,
    justifyContent: "flex-start",
    minHeight: "100%",
    paddingHorizontal: 22,
  },
  backgroundWritingPlane: {
    overflow: "hidden",
    paddingHorizontal: 2,
  },
  backgroundWritingScroll: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "#000000",
    zIndex: 0,
  },
  backgroundWritingShadow: {
    color: "rgba(0, 0, 0, 0.72)",
    fontSize: 20,
    fontWeight: "500",
    letterSpacing: 0,
    lineHeight: 32,
    paddingTop: 1,
  },
  hiddenInput: {
    bottom: 0,
    height: 1,
    opacity: 0,
    position: "absolute",
    width: 1,
    zIndex: 2,
  },
  postSessionActions: {
    backgroundColor: "rgba(8, 9, 11, 0.88)",
    borderTopColor: "rgba(244, 241, 234, 0.12)",
    borderTopWidth: 1,
    bottom: 0,
    gap: 8,
    left: 0,
    paddingHorizontal: 18,
    paddingTop: 14,
    position: "absolute",
    right: 0,
    zIndex: 5,
  },
  postSessionMessage: {
    color: ankyColors.textMuted,
    fontSize: 12,
    lineHeight: 18,
    textAlign: "center",
  },
  topLeftButton: {
    left: 16,
    position: "absolute",
    zIndex: 4,
  },
  topRightButton: {
    position: "absolute",
    right: 16,
    zIndex: 4,
  },
  lastCharacter: {
    color: ankyColors.text,
    fontSize: 112,
    fontWeight: "400",
    letterSpacing: 0,
    lineHeight: 132,
    textAlign: "center",
    textShadowColor: "rgba(255, 240, 201, 0.24)",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 18,
  },
  lastCharacterClip: {
    alignItems: "center",
    justifyContent: "center",
    overflow: "hidden",
    zIndex: 3,
  },
  lastCharacterEraser: {
    backgroundColor: "rgba(0, 0, 0, 0.96)",
    left: 0,
    position: "absolute",
    right: 0,
    top: 0,
  },
  lastCharacterWrap: {
    ...StyleSheet.absoluteFillObject,
    alignItems: "center",
    justifyContent: "center",
    paddingHorizontal: 34,
    zIndex: 3,
  },
  circleCastShadow: {
    backgroundColor: "rgba(0, 0, 0, 0.42)",
    borderRadius: 999,
    bottom: 8,
    left: 8,
    position: "absolute",
    right: 8,
    shadowColor: "#000000",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.7,
    shadowRadius: 28,
    top: 8,
  },
  riteThresholdCue: {
    alignItems: "center",
    alignSelf: "center",
    backgroundColor: "rgba(242, 242, 247, 0.12)",
    borderColor: "rgba(242, 242, 247, 0.4)",
    borderRadius: 8,
    borderWidth: 1,
    paddingHorizontal: 10,
    paddingVertical: 4,
    position: "absolute",
    zIndex: 4,
  },
  riteThresholdText: {
    color: "#F2F2F7",
    fontSize: 12,
    fontWeight: "800",
    letterSpacing: 0,
  },
  root: {
    backgroundColor: "#000000",
    flex: 1,
  },
  workspace: {
    backgroundColor: "#000000",
    left: 0,
    position: "absolute",
    right: 0,
    top: 0,
  },
  writingCircle: {
    alignItems: "center",
    justifyContent: "center",
  },
  writingCircleCore: {
    backgroundColor: "rgba(0, 0, 0, 0.96)",
    borderColor: "rgba(255, 240, 201, 0.08)",
    borderRadius: 999,
    borderWidth: 1,
    bottom: "18%",
    left: "18%",
    position: "absolute",
    right: "18%",
    top: "18%",
    zIndex: 1,
  },
  writingCursor: {
    backgroundColor: "rgba(244, 241, 234, 0.86)",
    borderRadius: 999,
    shadowColor: "#F4F1EA",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.22,
    shadowRadius: 10,
    width: 3,
    zIndex: 3,
  },
});
