import { useEffect, useRef, useState } from "react";
import {
  KeyboardAvoidingView,
  Modal,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { AnkyGlyph } from "../components/anky/AnkyGlyph";
import { GlassCard } from "../components/anky/GlassCard";
import { RitualButton } from "../components/anky/RitualButton";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { SubtleIconButton } from "../components/navigation/SubtleIconButton";
import { parseAnky, reconstructText } from "../lib/ankyProtocol";
import { readReflectionSidecar, readSavedAnkyFile } from "../lib/ankyStorage";
import { hasConfiguredBackend } from "../lib/auth/backendSession";
import { sendThreadMessage } from "../lib/thread/threadClient";
import {
  appendThreadMessagesToThread,
  createInitialThread,
  createThreadMessage,
  getThreadModeForRawAnky,
  hasReachedFreeThreadLimit,
  hasRestingMessage,
  isCompleteRawAnky,
  THREAD_RESTING_MESSAGE,
} from "../lib/thread/threadLogic";
import {
  hasThreadProcessingConsent,
  markThreadProcessingConsent,
} from "../lib/thread/threadConsent";
import { getThread, saveThread } from "../lib/thread/threadStorage";
import type { AnkyThread, ThreadMessage, ThreadMode } from "../lib/thread/types";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Thread">;

type ThreadState = {
  createdAtLabel: string;
  raw: string;
  reconstructed: string;
  reflection: string | null;
  thread: AnkyThread;
};

export function ThreadScreen({ navigation, route }: Props) {
  const scrollRef = useRef<ScrollView>(null);
  const sessionHash = route.params.sessionHash.toLowerCase();
  const [consentVisible, setConsentVisible] = useState(false);
  const [error, setError] = useState("");
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [pendingMessage, setPendingMessage] = useState("");
  const [sending, setSending] = useState(false);
  const [state, setState] = useState<ThreadState | null>(null);

  const thread = state?.thread ?? null;
  const isResting = thread == null ? false : hasReachedFreeThreadLimit(thread);
  const canSend = input.trim().length > 0 && !sending && !isResting && state != null;

  useEffect(() => {
    let mounted = true;

    async function load() {
      setLoading(true);
      setError("");

      try {
        const fileName = `${sessionHash}.anky`;
        const [saved, reflection, existingThread] = await Promise.all([
          readSavedAnkyFile(fileName),
          readReflectionSidecar(sessionHash),
          getThread(sessionHash),
        ]);

        if (!isCompleteRawAnky(saved.raw)) {
          if (mounted) {
            setError("fragments can be read and copied, but not sent to anky.");
            setState(null);
          }
          return;
        }

        const mode =
          existingThread?.mode ??
          route.params.mode ??
          getThreadModeForRawAnky(saved.raw, reflection != null);
        const nextThread = existingThread ?? createInitialThread({ mode, sessionHash });

        if (existingThread == null) {
          await saveThread(nextThread);
        }

        if (mounted) {
          setState({
            createdAtLabel: formatStartedAt(saved.raw),
            raw: saved.raw,
            reconstructed: reconstructText(saved.raw),
            reflection,
            thread: nextThread,
          });
        }
      } catch (loadError) {
        console.error(loadError);
        if (mounted) {
          setError("keep writing could not open right now.");
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    }

    void load();

    return () => {
      mounted = false;
    };
  }, [route.params.mode, sessionHash]);

  async function handleSend() {
    const message = input.trim();

    if (!canSend || state == null || thread == null) {
      return;
    }

    if (!hasConfiguredBackend()) {
      setError("anky cannot keep writing right now. your writing is still saved.");
      return;
    }

    if (!(await hasThreadProcessingConsent())) {
      setPendingMessage(message);
      setConsentVisible(true);
      return;
    }

    await continueWithMessage(message);
  }

  async function continueWithMessage(message: string) {
    if (state == null) {
      return;
    }

    setSending(true);
    setError("");

    try {
      const userMessage = createThreadMessage({
        content: message,
        role: "user",
      });
      const ankyMessage = await sendThreadMessage({
        existingReflection: state.reflection ?? undefined,
        messages: state.thread.messages,
        mode: state.thread.mode,
        rawAnky: state.raw,
        reconstructedText: state.reconstructed,
        sessionHash: state.thread.sessionHash,
        userMessage: message,
      });
      const messagesToAppend: ThreadMessage[] = [userMessage, ankyMessage];
      let nextThread = appendThreadMessagesToThread(state.thread, messagesToAppend);

      if (hasReachedFreeThreadLimit(nextThread) && !hasRestingMessage(nextThread)) {
        nextThread = appendThreadMessagesToThread(nextThread, [
          createThreadMessage({
            content: THREAD_RESTING_MESSAGE,
            role: "anky",
          }),
        ]);
      }

      await saveThread(nextThread);
      setState({ ...state, thread: nextThread });
      setInput("");
    } catch (sendError) {
      console.error(sendError);
      setError("keep writing could not continue right now.");
    } finally {
      setSending(false);
    }
  }

  async function handleConsentContinue() {
    const message = pendingMessage;

    setConsentVisible(false);
    setPendingMessage("");
    await markThreadProcessingConsent();

    if (message.length > 0) {
      await continueWithMessage(message);
    }
  }

  function handleConsentDecline() {
    setConsentVisible(false);
    setPendingMessage("");

    if (navigation.canGoBack()) {
      navigation.goBack();
      return;
    }

    navigation.navigate("Entry", { fileName: `${sessionHash}.anky` });
  }

  function goBack() {
    if (navigation.canGoBack()) {
      navigation.goBack();
      return;
    }

    navigation.navigate("Entry", { fileName: `${sessionHash}.anky` });
  }

  return (
    <ScreenBackground variant="plain">
      <KeyboardAvoidingView
        behavior={Platform.OS === "ios" ? "padding" : undefined}
        style={styles.keyboard}
      >
        <View style={styles.header}>
          <SubtleIconButton accessibilityLabel="back" icon="←" onPress={goBack} />
          <View style={styles.headerTitle}>
            <AnkyGlyph size={26} />
            <View>
              <Text style={styles.title}>keep writing</Text>
              <Text style={styles.subtitle}>
                with anky{state?.createdAtLabel ? ` · ${state.createdAtLabel}` : ""}
              </Text>
            </View>
          </View>
        </View>

        {loading ? (
          <View style={styles.center}>
            <Text style={styles.muted}>the chamber is opening</Text>
          </View>
        ) : state == null ? (
          <View style={styles.center}>
            <Text style={styles.errorText}>{error || "keep writing could not open right now."}</Text>
            <View style={styles.centerActions}>
              <RitualButton label="return" onPress={goBack} variant="secondary" />
              <RitualButton label="write again" onPress={() => navigation.navigate("ActiveWriting", { sojourn: 9 })} />
            </View>
          </View>
        ) : (
          <>
            <ScrollView
              ref={scrollRef}
              contentContainerStyle={styles.content}
              keyboardShouldPersistTaps="handled"
              onContentSizeChange={() => scrollRef.current?.scrollToEnd({ animated: true })}
            >
              <GlassCard style={styles.contextCard}>
                <Text style={styles.contextTitle}>{contextTitle(state.thread.mode)}</Text>
                <Text style={styles.preview} numberOfLines={4}>
                  {state.reconstructed.length > 0 ? state.reconstructed : "no visible text."}
                </Text>
                <Text style={styles.privacy}>
                  your writing stays local until you send a message.
                </Text>
              </GlassCard>

              <View style={styles.messages}>
                {state.thread.messages.length === 0 ? (
                  <GlassCard style={styles.emptyThread}>
                    <Text style={styles.contextTitle}>keep writing is waiting.</Text>
                    <Text style={styles.note}>anky can sit with what you wrote.</Text>
                  </GlassCard>
                ) : (
                  state.thread.messages.map((message) => (
                    <ThreadMessageCard key={message.id} message={message} />
                  ))
                )}
              </View>

              {sending ? <Text style={styles.listening}>anky is listening...</Text> : null}
              {error.length === 0 ? null : <Text style={styles.errorText}>{error}</Text>}
              {error.length === 0 ? null : (
                <View style={styles.errorActions}>
                  <RitualButton label="return" onPress={goBack} variant="secondary" />
                  <RitualButton
                    label="write again"
                    onPress={() => navigation.navigate("ActiveWriting", { sojourn: 9 })}
                    variant="ghost"
                  />
                </View>
              )}

              {isResting ? (
                <View style={styles.restActions}>
                  <RitualButton
                    label="write again"
                    onPress={() => navigation.navigate("ActiveWriting", { sojourn: 9 })}
                  />
                  <RitualButton
                    label="return to entry"
                    onPress={() => navigation.navigate("Entry", { fileName: `${sessionHash}.anky` })}
                    variant="secondary"
                  />
                  <RitualButton
                    label="return to map"
                    onPress={() => navigation.navigate("Track")}
                    variant="ghost"
                  />
                </View>
              ) : null}
            </ScrollView>

            {!isResting ? (
              <View style={styles.inputDock}>
                <TextInput
                  autoCapitalize="sentences"
                  multiline
                  onChangeText={setInput}
                  placeholder="write back..."
                  placeholderTextColor={ankyColors.textMuted}
                  style={styles.input}
                  value={input}
                />
                <Pressable
                  accessibilityRole="button"
                  disabled={!canSend}
                  onPress={() => void handleSend()}
                  style={[styles.sendButton, !canSend && styles.disabled]}
                >
                  <Text style={styles.sendText}>send</Text>
                </Pressable>
              </View>
            ) : null}
          </>
        )}
      </KeyboardAvoidingView>

      <Modal animationType="fade" transparent visible={consentVisible}>
        <View style={styles.modalBackdrop}>
          <GlassCard style={styles.consentCard}>
            <Text style={styles.contextTitle}>keep writing with anky</Text>
            <Text style={styles.consentText}>
              to keep writing with anky, this anky is sent for processing. your writing is never
              published. only continue if that feels right.
            </Text>
            <View style={styles.consentActions}>
              <RitualButton label="continue" onPress={() => void handleConsentContinue()} />
              <RitualButton label="not now" onPress={handleConsentDecline} variant="secondary" />
            </View>
          </GlassCard>
        </View>
      </Modal>
    </ScreenBackground>
  );
}

function ThreadMessageCard({ message }: { message: ThreadMessage }) {
  const isAnky = message.role === "anky";

  return (
    <View style={[styles.messageWrap, isAnky ? styles.ankyWrap : styles.userWrap]}>
      {isAnky ? (
        <View style={styles.messageGlyph}>
          <AnkyGlyph size={22} />
        </View>
      ) : null}
      <View style={[styles.messageCard, isAnky ? styles.ankyMessage : styles.userMessage]}>
        <Text style={styles.messageLabel}>{isAnky ? "anky" : "you"}</Text>
        <Text style={styles.messageText}>{message.content}</Text>
      </View>
    </View>
  );
}

function contextTitle(mode: ThreadMode): string {
  switch (mode) {
    case "fragment":
      return "this began as a fragment";
    case "reflection":
      return "this began after a mirror";
    case "complete":
      return "this began as an anky";
  }
}

function formatStartedAt(raw: string): string {
  const parsed = parseAnky(raw);

  if (parsed.startedAt == null) {
    return "";
  }

  return new Date(parsed.startedAt).toLocaleDateString([], {
    day: "numeric",
    month: "short",
  }).toLowerCase();
}

const styles = StyleSheet.create({
  ankyMessage: {
    backgroundColor: ankyColors.card,
    borderColor: ankyColors.border,
    borderWidth: 1,
  },
  ankyWrap: {
    alignSelf: "stretch",
    flexDirection: "row",
  },
  center: {
    alignItems: "center",
    flex: 1,
    justifyContent: "center",
    padding: spacing.xl,
  },
  centerActions: {
    gap: spacing.sm,
    marginTop: spacing.lg,
    width: "100%",
  },
  consentActions: {
    gap: spacing.sm,
    marginTop: spacing.lg,
  },
  consentCard: {
    width: "100%",
  },
  consentText: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    lineHeight: 24,
    marginTop: spacing.md,
    textTransform: "lowercase",
  },
  content: {
    padding: spacing.xl,
    paddingBottom: 24,
  },
  contextCard: {
    marginBottom: spacing.lg,
  },
  contextTitle: {
    color: ankyColors.gold,
    fontSize: fontSize.lg,
    fontWeight: "700",
    lineHeight: 26,
    textTransform: "lowercase",
  },
  disabled: {
    opacity: 0.42,
  },
  emptyThread: {
    marginTop: spacing.sm,
  },
  errorActions: {
    gap: spacing.sm,
    marginTop: spacing.md,
  },
  errorText: {
    color: ankyColors.danger,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
    textAlign: "center",
  },
  header: {
    alignItems: "center",
    borderBottomColor: ankyColors.border,
    borderBottomWidth: 1,
    flexDirection: "row",
    gap: spacing.md,
    paddingHorizontal: spacing.xl,
    paddingVertical: spacing.md,
  },
  headerTitle: {
    alignItems: "center",
    flexDirection: "row",
    flex: 1,
    gap: spacing.sm,
  },
  input: {
    backgroundColor: ankyColors.bg3,
    borderColor: ankyColors.border,
    borderRadius: 8,
    borderWidth: 1,
    color: ankyColors.text,
    flex: 1,
    fontSize: fontSize.md,
    maxHeight: 104,
    minHeight: 46,
    paddingHorizontal: spacing.md,
    paddingVertical: 12,
  },
  inputDock: {
    alignItems: "flex-end",
    borderTopColor: ankyColors.border,
    borderTopWidth: 1,
    flexDirection: "row",
    gap: spacing.sm,
    padding: spacing.md,
  },
  keyboard: {
    flex: 1,
  },
  listening: {
    color: ankyColors.violetBright,
    fontSize: fontSize.sm,
    marginTop: spacing.md,
    textAlign: "center",
  },
  messageCard: {
    borderRadius: 8,
    maxWidth: "88%",
    padding: spacing.md,
  },
  messageGlyph: {
    alignItems: "center",
    height: 34,
    justifyContent: "center",
    marginRight: spacing.sm,
    width: 34,
  },
  messageLabel: {
    color: ankyColors.textMuted,
    fontSize: fontSize.xs,
    fontWeight: "800",
    marginBottom: 6,
    textTransform: "uppercase",
  },
  messageText: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    lineHeight: 24,
  },
  messages: {
    gap: spacing.md,
  },
  messageWrap: {
    marginVertical: 2,
  },
  modalBackdrop: {
    alignItems: "center",
    backgroundColor: "rgba(0, 0, 0, 0.72)",
    flex: 1,
    justifyContent: "center",
    padding: spacing.xl,
  },
  muted: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
  },
  note: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.sm,
  },
  preview: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    lineHeight: 24,
    marginTop: spacing.md,
  },
  privacy: {
    color: ankyColors.textMuted,
    fontSize: fontSize.xs,
    lineHeight: 18,
    marginTop: spacing.md,
    textTransform: "lowercase",
  },
  restActions: {
    gap: spacing.sm,
    marginTop: spacing.lg,
  },
  sendButton: {
    alignItems: "center",
    backgroundColor: ankyColors.text,
    borderRadius: 8,
    height: 46,
    justifyContent: "center",
    paddingHorizontal: spacing.lg,
  },
  sendText: {
    color: ankyColors.bg,
    fontSize: fontSize.sm,
    fontWeight: "800",
    textTransform: "lowercase",
  },
  subtitle: {
    color: ankyColors.textMuted,
    fontSize: fontSize.xs,
    marginTop: 2,
  },
  title: {
    color: ankyColors.gold,
    fontSize: fontSize.lg,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  userMessage: {
    backgroundColor: "transparent",
    borderColor: ankyColors.borderStrong,
    borderWidth: 1,
  },
  userWrap: {
    alignItems: "flex-end",
  },
});
