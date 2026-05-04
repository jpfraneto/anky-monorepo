import { useEffect, useMemo, useRef, useState } from "react";
import { Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import * as Clipboard from "expo-clipboard";
import { Connection } from "@solana/web3.js";

import type { RootStackParamList } from "../../App";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { SubtleIconButton } from "../components/navigation/SubtleIconButton";
import {
  ReflectCreditSheet,
  REFLECTION_COST_CREDITS,
} from "../components/reflection/ReflectCreditSheet";
import { SwipeToSealAction } from "../components/seal/SwipeToSealAction";
import {
  AnkySessionSurface,
  GoldenThreadSpinner,
  type SessionReflectionMode,
} from "../components/session/AnkySessionSurface";
import { CREDIT_COSTS } from "../lib/api/types";
import { getAnkyApiClient } from "../lib/api/client";
import { addAnkySessionSummary, listAnkySessionSummaries } from "../lib/ankySessionIndex";
import { computeSessionHash, parseAnky, reconstructText, verifyHash } from "../lib/ankyProtocol";
import {
  clearActiveDraft,
  clearPendingReveal,
  readAnkyFile,
  readPendingReveal,
  readReflectionSidecar,
  saveClosedSession,
  writeSealSidecar,
} from "../lib/ankyStorage";
import type { SavedAnkyFile } from "../lib/ankyStorage";
import {
  getReflectionCreditBalance,
  processReflectionWithMode,
} from "../lib/credits/processAnky";
import { hasConfiguredBackend } from "../lib/auth/backendSession";
import { useAnkyPrivyWallet } from "../lib/privy/useAnkyPrivyWallet";
import { getCurrentSojournDay, getNextSessionKindForToday } from "../lib/sojourn";
import type { AnkySessionSummary } from "../lib/sojourn";
import { getSelectedLoom } from "../lib/solana/loomStorage";
import type { SelectedLoom } from "../lib/solana/loomStorage";
import { loadMobileSolanaConfig } from "../lib/solana/mobileSolanaConfig";
import type { MobileSolanaRuntimeConfig } from "../lib/solana/mobileSolanaConfig";
import { sealAnky as sealAnkyOnchain } from "../lib/solana/sealAnky";
import {
  getRiteDurationMs,
  getThreadModeForRawAnky,
  isCompleteParsedAnky,
} from "../lib/thread/threadLogic";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Reveal">;
type ActionState = "copying" | "error" | "idle" | "reflecting" | "sealing" | "saving";
type RevealKind = "complete" | "short";

const GOLD = "#E8C879";
const GOLD_SOFT = "rgba(232, 200, 121, 0.72)";
const GOLD_DIM = "rgba(232, 200, 121, 0.38)";
const PAPER = "#FFF0C9";
const INK = "#080713";
const CARD = "rgba(17, 13, 31, 0.92)";
const BORDER = "rgba(232, 200, 121, 0.34)";

export function RevealScreen({ navigation, route }: Props) {
  const walletState = useAnkyPrivyWallet();
  const autoIndexedHashRef = useRef<string | null>(null);
  const revealScrollRef = useRef<ScrollView>(null);
  const [actionState, setActionState] = useState<ActionState>("saving");
  const [creditBalance, setCreditBalance] = useState<number | null>(null);
  const [didSeal, setDidSeal] = useState(false);
  const [fileName, setFileName] = useState<string | null>(route.params?.fileName ?? null);
  const [hash, setHash] = useState("");
  const [hashMatches, setHashMatches] = useState(false);
  const [message, setMessage] = useState("");
  const [raw, setRaw] = useState<string | null>(null);
  const [reflection, setReflection] = useState<string | null>(null);
  const [reflectionExpanded, setReflectionExpanded] = useState(true);
  const [reflectSheetVisible, setReflectSheetVisible] = useState(false);
  const [runtimeConfig, setRuntimeConfig] = useState<MobileSolanaRuntimeConfig | null>(null);
  const [sealError, setSealError] = useState("");
  const [selectedLoom, setSelectedLoom] = useState<SelectedLoom | null>(null);
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);

  const reconstructed = useMemo(() => (raw == null ? "" : reconstructText(raw)), [raw]);
  const parsed = useMemo(() => (raw == null ? null : parseAnky(raw)), [raw]);
  const riteDurationMs = useMemo(() => getRiteDurationMs(parsed), [parsed]);
  const revealKind = getRevealKind(parsed);
  const currentHash = hash.length > 0 ? hash : fileName?.replace(/\.anky$/, "");
  const sessionsBeforeCurrent = sessions.filter(
    (session) => session.sessionHash == null || session.sessionHash !== currentHash,
  );
  const completeSessionKind = getNextSessionKindForToday(sessionsBeforeCurrent);
  const summaryKind: AnkySessionSummary["kind"] =
    revealKind === "complete" ? completeSessionKind : "fragment";
  const isBusy = actionState === "reflecting" || actionState === "sealing" || actionState === "saving";
  const canUseAnky = raw != null && parsed?.valid === true && hashMatches;
  const canReflect = canUseAnky && revealKind === "complete" && !isBusy && reflection == null;
  const canCopy = reconstructed.length > 0 && actionState !== "copying";
  const canSeal =
    canUseAnky &&
    revealKind === "complete" &&
    !didSeal &&
    !isBusy &&
    selectedLoom != null &&
    walletState.hasWallet;
  const shouldShowSeal =
    revealKind === "complete" &&
    (didSeal || actionState === "sealing" || (selectedLoom != null && walletState.hasWallet));
  const dateParts = useMemo(() => formatRevealDateParts(parsed?.startedAt ?? null), [parsed]);
  const durationLabel = formatDuration(riteDurationMs);

  useEffect(() => {
    let mounted = true;

    async function loadReveal() {
      setActionState("saving");
      setMessage("");

      const routeFileName = route.params?.fileName ?? null;
      const [nextRaw, nextCredits, nextSessions, nextConfig, nextSelectedLoom] =
        await Promise.all([
          routeFileName == null ? readPendingReveal() : readAnkyFile(routeFileName),
          getReflectionCreditBalance(),
          listAnkySessionSummaries(),
          loadMobileSolanaConfig(),
          getSelectedLoom(),
        ]);

      if (!mounted) {
        return;
      }

      let nextFileName = routeFileName;
      let nextHash = "";
      let nextHashMatches = false;
      let nextDidSeal = false;
      let nextMessage = "";
      let nextReflection: string | null = null;

      if (nextRaw == null) {
        nextMessage = "no closed writing is waiting to reveal.";
      } else {
        const nextParsed = parseAnky(nextRaw);

        if (nextParsed.valid) {
          const saved = await saveClosedSession(nextRaw);
          const nextKind = getRevealKind(nextParsed);
          const nextSummaryKind =
            nextKind === "complete"
              ? getNextSessionKindForToday(
                  nextSessions.filter((session) => session.sessionHash !== saved.hash),
                )
              : "fragment";

          nextFileName = saved.fileName;
          nextHash = saved.hash;
          nextHashMatches = saved.hashMatches;
          nextDidSeal = saved.sealCount > 0;
          nextReflection = await readReflectionSidecar(saved.hash);

          if (autoIndexedHashRef.current !== saved.hash) {
            try {
              await addAnkySessionSummary(
                buildSessionSummary(saved, nextRaw, nextSummaryKind, saved.sealCount > 0),
              );
              await clearPendingReveal();
              await clearActiveDraft();
              autoIndexedHashRef.current = saved.hash;
            } catch (indexError) {
              console.error(indexError);
              nextMessage = "saved locally, but the map index needs attention.";
            }
          }
        } else {
          nextHash =
            routeFileName == null ? await computeSessionHash(nextRaw) : routeFileName.replace(/\.anky$/, "");
          nextHashMatches = await verifyHash(nextRaw, nextHash);
          nextMessage = "this .anky needs attention before reflection or sealing.";
        }
      }

      if (!mounted) {
        return;
      }

      setRaw(nextRaw);
      setReflection(nextReflection);
      setReflectionExpanded(nextReflection != null);
      setFileName(nextFileName);
      setCreditBalance(nextCredits);
      setRuntimeConfig(nextConfig);
      setSelectedLoom(nextSelectedLoom);
      setSessions(nextSessions);
      setHash(nextHash);
      setHashMatches(nextHashMatches);
      setDidSeal(nextDidSeal);
      setMessage(nextMessage);
      setActionState(nextMessage.length > 0 && nextRaw != null ? "error" : "idle");
    }

    void loadReveal().catch((error) => {
      console.error(error);
      if (mounted) {
        setMessage(error instanceof Error ? error.message : "Could not load this .anky.");
        setActionState("error");
      }
    });

    return () => {
      mounted = false;
    };
  }, [route.params?.fileName]);

  async function ensureSavedFile(): Promise<SavedAnkyFile> {
    if (raw == null) {
      throw new Error("No closed .anky is waiting to be revealed.");
    }

    const saved = await saveClosedSession(raw);
    setFileName(saved.fileName);
    setHash(saved.hash);
    setHashMatches(saved.hashMatches);

    return saved;
  }

  async function handleCopy() {
    if (!canCopy) {
      return;
    }

    try {
      setActionState("copying");
      await Clipboard.setStringAsync(reconstructed);
      setMessage("writing copied.");
      setActionState("idle");
      setTimeout(() => {
        setMessage((current) => (current === "writing copied." ? "" : current));
      }, 1600);
    } catch (error) {
      console.error(error);
      setMessage("copy failed.");
      setActionState("error");
    }
  }

  async function handleReflect(mode: SessionReflectionMode) {
    if (!canReflect || revealKind !== "complete" || raw == null || hash.length === 0) {
      return;
    }

    setReflectSheetVisible(false);
    const cost = mode === "full" ? CREDIT_COSTS.full_anky : REFLECTION_COST_CREDITS;
    const balance = hasConfiguredBackend()
      ? creditBalance ?? (await getReflectionCreditBalance())
      : null;

    if (balance != null && balance < cost) {
      setMessage("available credits: 0");
      return;
    }

    try {
      setActionState("reflecting");
      setMessage("");
      const saved = await ensureSavedFile();
      const result = await processReflectionWithMode(saved.fileName, mode);

      await addAnkySessionSummary({
        ...buildSessionSummary(saved, raw, summaryKind, didSeal || saved.sealCount > 0),
        reflectionId: saved.hash,
      });
      await clearPendingReveal();
      await clearActiveDraft();

      setCreditBalance(result.creditsRemaining);
      setReflection(result.markdown);
      setReflectionExpanded(true);
      setMessage(mode === "full" ? "full reflection ready." : "reflection ready.");
      setActionState("idle");
      setTimeout(() => {
        revealScrollRef.current?.scrollToEnd({ animated: true });
      }, 120);
    } catch (error) {
      console.error(error);
      setMessage(
        error instanceof Error
          ? error.message
          : "Reflection failed. Your .anky is unchanged.",
      );
      setActionState("error");
    }
  }

  function handleBuyCredits() {
    setReflectSheetVisible(false);
    navigation.navigate("Credits", {
      fileName: fileName ?? undefined,
      processingType: "reflection",
    });
  }

  function handleContinue() {
    if (raw == null || currentHash == null || revealKind !== "complete" || reflection == null) {
      return;
    }

    navigation.navigate("Thread", {
      mode: getThreadModeForRawAnky(raw, true),
      sessionHash: currentHash,
      source: "entry",
    });
  }

  function handleGoBack() {
    const state = navigation.getState();
    const previousRoute = state.index > 0 ? state.routes[state.index - 1] : null;

    if (navigation.canGoBack() && previousRoute?.name !== "Write") {
      navigation.goBack();
      return;
    }

    navigation.replace("Track");
  }

  function handleTryAgain() {
    if (isBusy) {
      return;
    }

    const now = new Date();

    navigation.replace("ActiveWriting", {
      dayNumber: getCurrentSojournDay(now),
      isoDate: now.toISOString().slice(0, 10),
      recoverDraft: false,
      sessionKind: getNextSessionKindForToday(sessions, now),
      sojourn: 9,
    });
  }

  async function handleSeal() {
    if (!canSeal || raw == null) {
      return;
    }

    try {
      setMessage("");
      setSealError("");
      setActionState("sealing");
      const saved = await ensureSavedFile();
      const loom = selectedLoom ?? (await getSelectedLoom());

      if (loom == null) {
        throw new Error("Select or mint a Loom before sealing. Your .anky is still safe locally.");
      }

      const wallet = await walletState.getWallet();

      if (loom.owner != null && loom.owner !== wallet.publicKey) {
        throw new Error("The selected Loom belongs to a different wallet.");
      }

      const config = runtimeConfig ?? (await loadMobileSolanaConfig());
      const connection = new Connection(config.rpcUrl, "confirmed");
      const seal = await sealAnkyOnchain({
        connection,
        coreCollection: loom.collection,
        loomAsset: loom.asset,
        network: config.network,
        programId: config.sealProgramId,
        sessionHashHex: saved.hash,
        wallet,
      });

      await writeSealSidecar(seal);

      const api = getAnkyApiClient();
      if (api != null) {
        try {
          await api.recordMobileSeal({
            coreCollection: loom.collection,
            loomAsset: loom.asset,
            sessionHash: saved.hash,
            signature: seal.signature,
            status: "confirmed",
            wallet: wallet.publicKey,
          });
        } catch (recordError) {
          console.warn("Sealed on chain, but backend seal record failed.", recordError);
        }
      }

      await addAnkySessionSummary(buildSessionSummary(saved, raw, summaryKind, true));
      await clearActiveDraft();
      await clearPendingReveal();
      setDidSeal(true);
      setMessage("sealed. hash only was written; local writing stayed private.");
      setActionState("idle");
    } catch (error) {
      console.error(error);
      const nextMessage =
        error instanceof Error ? error.message : "Seal failed. Your .anky is still safe locally.";
      setSealError(nextMessage);
      setMessage(nextMessage);
      setActionState("error");
    }
  }

  if (raw == null) {
    return (
      <ScreenBackground variant="plain">
        <View style={styles.emptyState}>
          {actionState === "saving" ? <GoldenThreadSpinner label="opening" /> : null}
          {actionState === "saving" ? null : <Text style={styles.emptyTitle}>nothing to reveal</Text>}
          {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}
        </View>
      </ScreenBackground>
    );
  }

  return (
    <ScreenBackground variant="plain">
      <AnkySessionSurface
        canContinue={reflection != null && revealKind === "complete" && canUseAnky}
        canCopy={canCopy}
        canFullReflect={canReflect}
        canSimpleReflect={canReflect}
        dateLabel={dateParts.date}
        durationLabel={durationLabel}
        errorText={parsed != null && !parsed.valid ? "this writing needs attention before reflection." : undefined}
        isComplete={revealKind === "complete"}
        isProcessing={actionState === "reflecting"}
        message={message}
        onBack={handleGoBack}
        onContinue={handleContinue}
        onCopy={() => void handleCopy()}
        onFullReflect={() => void handleReflect("full")}
        onSimpleReflect={() => void handleReflect("simple")}
        onTryAgain={handleTryAgain}
        reflection={reflection}
        text={reconstructed}
        timeLabel={dateParts.time}
      />
    </ScreenBackground>
  );
}

function RevealBackgroundTexture() {
  return (
    <View pointerEvents="none" style={styles.backgroundTexture}>
      <View style={[styles.backgroundLine, { top: 88, width: "58%" }]} />
      <View style={[styles.backgroundLine, { top: 192, width: "78%" }]} />
      <View style={[styles.backgroundLine, { top: 316, width: "48%" }]} />
      <View style={[styles.backgroundLine, { bottom: 146, width: "68%" }]} />
    </View>
  );
}

function WritingCard({ text }: { text: string }) {
  return (
    <View style={styles.writingCard}>
      <View style={styles.cardOrnamentTop} />
      <Text selectable style={styles.writingText}>
        {text}
      </Text>
      <View style={styles.cardOrnamentBottom} />
    </View>
  );
}

function RevealReflectionCard({
  expanded,
  onPress,
  reflection,
}: {
  expanded: boolean;
  onPress: () => void;
  reflection: string;
}) {
  return (
    <Pressable accessibilityRole="button" onPress={onPress} style={styles.reflectionCard}>
      <View style={styles.reflectionHeader}>
        <Text style={styles.reflectionMark}>✦</Text>
        <Text style={styles.reflectionLabel}>reflection</Text>
        <Text style={styles.reflectionHint}>{expanded ? "less" : "more"}</Text>
      </View>
      <Text selectable numberOfLines={expanded ? undefined : 8} style={styles.reflectionText}>
        {reflection}
      </Text>
    </Pressable>
  );
}

function RevealButton({
  disabled,
  emphasized,
  label,
  onPress,
  symbol,
}: {
  disabled?: boolean;
  emphasized?: boolean;
  label: string;
  onPress: () => void;
  symbol: string;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => [
        styles.revealButton,
        emphasized && styles.revealButtonEmphasized,
        disabled && styles.disabled,
        pressed && !disabled && styles.pressed,
      ]}
    >
      <Text style={styles.revealButtonSymbol}>{symbol}</Text>
      <Text style={styles.revealButtonText}>{label}</Text>
    </Pressable>
  );
}

function ShortSessionNote() {
  return (
    <View style={styles.shortNote}>
      <Text style={styles.shortNoteTitle}>saved as a fragment</Text>
      <Text style={styles.shortNoteText}>
        this session ended before the full rite. it remains local, and a full anky begins at 8 minutes.
      </Text>
    </View>
  );
}

function buildSessionSummary(
  saved: SavedAnkyFile,
  raw: string,
  kind: AnkySessionSummary["kind"],
  sealedOnchain = saved.sealCount > 0,
): AnkySessionSummary {
  const parsed = parseAnky(raw);
  const createdAt =
    parsed.startedAt == null ? new Date().toISOString() : new Date(parsed.startedAt).toISOString();
  const text = reconstructText(raw);

  return {
    id: saved.hash,
    characterCount: text.length,
    createdAt,
    kind,
    localFileUri: saved.uri,
    reflectionId: saved.artifactKinds.includes("reflection") ? saved.hash : undefined,
    sealedOnchain,
    sessionHash: saved.hash,
    sojournDay: getCurrentSojournDay(new Date(createdAt)),
    wordCount: text.trim().split(/\s+/).filter(Boolean).length,
  };
}

function getRevealKind(parsed: ReturnType<typeof parseAnky> | null): RevealKind {
  return isCompleteParsedAnky(parsed) ? "complete" : "short";
}

function formatRevealDateParts(startedAt: number | null): { date: string; time: string } {
  const date = startedAt == null ? new Date() : new Date(startedAt);

  return {
    date: date.toLocaleDateString([], {
      day: "numeric",
      month: "long",
      year: "numeric",
    }),
    time: date.toLocaleTimeString([], {
      hour: "numeric",
      minute: "2-digit",
    }),
  };
}

function formatDuration(ms: number | null): string {
  if (ms == null) {
    return "duration unknown";
  }

  const totalSeconds = Math.max(0, Math.round(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = String(totalSeconds % 60).padStart(2, "0");

  return `${minutes}:${seconds}`;
}

const styles = StyleSheet.create({
  actions: {
    flexDirection: "row",
    gap: 12,
    marginTop: spacing.lg,
  },
  backgroundLine: {
    alignSelf: "center",
    backgroundColor: "rgba(232, 200, 121, 0.052)",
    height: StyleSheet.hairlineWidth,
    position: "absolute",
  },
  backgroundTexture: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: INK,
  },
  cardOrnamentBottom: {
    alignSelf: "center",
    backgroundColor: INK,
    borderColor: GOLD,
    borderWidth: 1,
    bottom: -5,
    height: 9,
    position: "absolute",
    transform: [{ rotate: "45deg" }],
    width: 9,
  },
  cardOrnamentTop: {
    alignSelf: "center",
    backgroundColor: INK,
    borderColor: GOLD,
    borderWidth: 1,
    height: 9,
    position: "absolute",
    top: -5,
    transform: [{ rotate: "45deg" }],
    width: 9,
  },
  content: {
    padding: 22,
    paddingBottom: 46,
    paddingTop: 24,
  },
  date: {
    color: GOLD_SOFT,
    fontSize: 15,
    marginTop: 14,
    textAlign: "center",
  },
  disabled: {
    opacity: 0.44,
  },
  dot: {
    color: GOLD_DIM,
  },
  durationText: {
    color: GOLD_SOFT,
    fontSize: 14,
    marginTop: 14,
    textAlign: "center",
    textTransform: "lowercase",
  },
  emptyState: {
    alignItems: "center",
    flex: 1,
    justifyContent: "center",
    padding: spacing.xl,
  },
  emptyTitle: {
    color: GOLD,
    fontSize: fontSize.lg,
    fontWeight: "700",
    textAlign: "center",
    textTransform: "lowercase",
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
    marginBottom: spacing.lg,
  },
  kicker: {
    color: GOLD_DIM,
    fontSize: 13,
    letterSpacing: 5,
    marginBottom: spacing.sm,
    textTransform: "lowercase",
  },
  message: {
    color: GOLD_SOFT,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
    textAlign: "center",
    textTransform: "lowercase",
  },
  pressed: {
    opacity: 0.72,
    transform: [{ scale: 0.985 }],
  },
  revealButton: {
    alignItems: "center",
    backgroundColor: "rgba(16, 14, 28, 0.88)",
    borderColor: BORDER,
    borderRadius: 8,
    borderWidth: 1,
    flex: 1,
    flexDirection: "row",
    justifyContent: "center",
    minHeight: 54,
    paddingHorizontal: 12,
  },
  revealButtonEmphasized: {
    backgroundColor: "rgba(123, 77, 255, 0.18)",
    borderColor: "rgba(232, 200, 121, 0.48)",
  },
  revealButtonSymbol: {
    color: GOLD,
    fontSize: 18,
    marginRight: 8,
  },
  revealButtonText: {
    color: PAPER,
    fontSize: 17,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  reflectionCard: {
    backgroundColor: "rgba(21, 17, 10, 0.62)",
    borderColor: "rgba(232, 200, 121, 0.34)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.lg,
    padding: spacing.lg,
  },
  reflectionHeader: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.sm,
  },
  reflectionHint: {
    color: GOLD_SOFT,
    fontSize: 12,
    fontWeight: "700",
    marginLeft: "auto",
    textTransform: "lowercase",
  },
  reflectionLabel: {
    color: GOLD,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
  reflectionMark: {
    color: GOLD,
    fontSize: 22,
    lineHeight: 24,
  },
  reflectionText: {
    color: "#F1C776",
    fontSize: fontSize.md,
    lineHeight: 25,
    marginTop: spacing.md,
  },
  shortNote: {
    backgroundColor: "rgba(232, 200, 121, 0.055)",
    borderColor: "rgba(232, 200, 121, 0.2)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.lg,
    padding: 18,
  },
  shortNoteText: {
    color: "rgba(255, 240, 201, 0.66)",
    fontSize: 14,
    lineHeight: 21,
  },
  shortNoteTitle: {
    color: GOLD,
    fontSize: 18,
    fontWeight: "700",
    marginBottom: 6,
    textTransform: "lowercase",
  },
  subtitle: {
    color: "rgba(255, 240, 201, 0.68)",
    fontSize: 14,
    lineHeight: 21,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  title: {
    color: GOLD,
    fontSize: 34,
    fontWeight: "700",
    lineHeight: 40,
    textAlign: "center",
    textShadowColor: "rgba(232, 200, 121, 0.22)",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 16,
    textTransform: "lowercase",
  },
  topBar: {
    alignItems: "center",
    flexDirection: "row",
    marginBottom: spacing.lg,
    minHeight: 44,
  },
  topBarLabel: {
    color: GOLD_DIM,
    flex: 1,
    fontSize: 12,
    fontWeight: "800",
    letterSpacing: 0,
    textAlign: "center",
    textTransform: "uppercase",
  },
  topBarSide: {
    flex: 1,
  },
  writingCard: {
    backgroundColor: CARD,
    borderColor: BORDER,
    borderRadius: 8,
    borderWidth: 1,
    paddingHorizontal: 24,
    paddingVertical: 28,
    shadowColor: GOLD,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.12,
    shadowRadius: 22,
  },
  writingText: {
    color: PAPER,
    fontSize: 20,
    letterSpacing: 0,
    lineHeight: 33,
  },
});
