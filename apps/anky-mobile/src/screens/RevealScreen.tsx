import { useEffect, useMemo, useRef, useState } from "react";
import {
  ActivityIndicator,
  KeyboardAvoidingView,
  Linking,
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
import { usePrivy } from "@privy-io/expo";
import * as Clipboard from "expo-clipboard";
import { Connection } from "@solana/web3.js";

import type { RootStackParamList } from "../../App";
import { useAuthModal } from "../auth/AuthModalContext";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { SubtleIconButton } from "../components/navigation/SubtleIconButton";
import { GoldenThreadSpinner } from "../components/session/AnkySessionSurface";
import { getAnkyApiClient } from "../lib/api/client";
import { CREDIT_COSTS } from "../lib/api/types";
import { addAnkySessionSummary, listAnkySessionSummaries } from "../lib/ankySessionIndex";
import { computeSessionHash, parseAnky, reconstructText, verifyHash } from "../lib/ankyProtocol";
import {
  clearActiveDraft,
  clearPendingReveal,
  readAnkyFile,
  readLoomSealsForHash,
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
import { getSelectedLoom, type SelectedLoom } from "../lib/solana/loomStorage";
import { hydrateMobileSealReceiptsForHashes } from "../lib/solana/mobileSealReceipts";
import { loadMobileSolanaConfig } from "../lib/solana/mobileSolanaConfig";
import { getUtcDayFromUnixMs, isCurrentUtcDay, sealAnky } from "../lib/solana/sealAnky";
import { getLoomSealProofState } from "../lib/solana/types";
import { sendThreadMessage } from "../lib/thread/threadClient";
import {
  FULL_ANKY_DURATION_MS,
  getRiteDurationMs,
  isCompleteParsedAnky,
} from "../lib/thread/threadLogic";
import type { ThreadMessage } from "../lib/thread/types";
import { useAnkyPresenceScreen } from "../presence/useAnkyPresenceScreen";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Entry" | "Reveal">;
type ActionState = "copying" | "error" | "idle" | "reflecting" | "saving" | "sealing";
type RevealKind = "complete" | "short";
type ScreenMode = "review" | "chat";
type ReflectionKind = "quick" | "full";
type RevealChatRole = "assistant" | "user";

type RevealChatMessage = {
  id: string;
  role: RevealChatRole;
  content: string;
  createdAt: string;
};

type PendingReplyRetry = {
  history: RevealChatMessage[];
  userMessage: RevealChatMessage;
};

type SealProof = {
  network?: "devnet" | "mainnet-beta";
  proofState?: "failed" | "none" | "proving" | "verified";
  proofTxSignature?: string;
  txSignature: string;
};

const GOLD = "#E8C879";
const GOLD_SOFT = "rgba(232, 200, 121, 0.72)";
const GOLD_DIM = "rgba(232, 200, 121, 0.38)";
const PAPER = "#FFF0C9";
const INK = "#080713";
const SERIF = Platform.select({ android: "serif", default: "Georgia", ios: "Georgia" });
const SPANISH_LOCALE = "es-CL";

export function RevealScreen({ navigation, route }: Props) {
  const { user } = usePrivy();
  const { openAuthModal } = useAuthModal();
  const wallet = useAnkyPrivyWallet();
  const autoIndexedHashRef = useRef<string | null>(null);
  const revealScrollRef = useRef<ScrollView>(null);
  const [actionState, setActionState] = useState<ActionState>("saving");
  const [creditBalance, setCreditBalance] = useState<number | null>(null);
  const [didSeal, setDidSeal] = useState(false);
  const [error, setError] = useState("");
  const [fileName, setFileName] = useState<string | null>(route.params?.fileName ?? null);
  const [hash, setHash] = useState("");
  const [hashMatches, setHashMatches] = useState(false);
  const [inputText, setInputText] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [messages, setMessages] = useState<RevealChatMessage[]>([]);
  const [pendingRetry, setPendingRetry] = useState<PendingReplyRetry | null>(null);
  const [pendingReflectionConfirm, setPendingReflectionConfirm] =
    useState<ReflectionKind | null>(null);
  const [raw, setRaw] = useState<string | null>(null);
  const [reflection, setReflection] = useState<string | null>(null);
  const [reflectionKind, setReflectionKind] = useState<ReflectionKind | null>(null);
  const [screenMode, setScreenMode] = useState<ScreenMode>("review");
  const [sealError, setSealError] = useState("");
  const [sealProof, setSealProof] = useState<SealProof | null>(null);
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);
  const [selectedLoom, setSelectedLoom] = useState<SelectedLoom | null>(null);
  const [presenceSequence, setPresenceSequence] = useState<"celebrate" | "idle_blink">(
    "celebrate",
  );

  const isEntryRoute = route.name === "Entry";
  const reconstructed = useMemo(() => (raw == null ? "" : reconstructText(raw)), [raw]);
  const parsed = useMemo(() => (raw == null ? null : parseAnky(raw)), [raw]);
  const riteDurationMs = useMemo(() => getRiteDurationMs(parsed), [parsed]);
  const sessionUtcDay = useMemo(
    () => (parsed?.startedAt == null ? null : getUtcDayFromUnixMs(parsed.startedAt)),
    [parsed],
  );
  const revealKind = getRevealKind(parsed);
  const isFullAnky = riteDurationMs != null && riteDurationMs >= FULL_ANKY_DURATION_MS;
  const currentHash = hash.length > 0 ? hash : fileName?.replace(/\.anky$/, "");
  const existingSummary = sessions.find((session) => session.sessionHash === currentHash);
  const sessionsBeforeCurrent = sessions.filter(
    (session) => session.sessionHash == null || session.sessionHash !== currentHash,
  );
  const completeSessionKind = getNextSessionKindForToday(sessionsBeforeCurrent);
  const summaryKind: AnkySessionSummary["kind"] =
    existingSummary?.kind ?? (revealKind === "complete" ? completeSessionKind : "fragment");
  const isSaving = actionState === "saving";
  const isLoggedIn = user != null;
  const canUseAnky = raw != null && parsed?.valid === true && hashMatches;
  const canCopy = reconstructed.length > 0 && actionState !== "copying";
  const canRequestReflection = canUseAnky && isFullAnky && !isSaving && !isLoading;
  const canShowSealWithLoom =
    canUseAnky &&
    isFullAnky &&
    selectedLoom != null &&
    wallet.hasWallet &&
    currentHash != null &&
    !isSaving;
  const canSealCurrentUtcDay = sessionUtcDay != null && isCurrentUtcDay(sessionUtcDay);
  const canSealWithLoom =
    canShowSealWithLoom &&
    canSealCurrentUtcDay &&
    !didSeal &&
    actionState !== "sealing";
  const sealUtcDayError =
    canShowSealWithLoom && !canSealCurrentUtcDay ? "only today's UTC anky can be sealed." : "";
  const quickReflectionCost = CREDIT_COSTS.reflection;
  const fullReflectionCost = CREDIT_COSTS.full_anky;
  const canSendChat =
    screenMode === "chat" &&
    inputText.trim().length > 0 &&
    raw != null &&
    currentHash != null &&
    reflectionKind != null &&
    !isLoading;
  const dateParts = useMemo(() => formatRevealDateParts(parsed?.startedAt ?? null), [parsed]);
  const durationLabel = formatWrittenDuration(riteDurationMs);
  const wordCountLabel = formatWordCount(countWords(reconstructed));

  useAnkyPresenceScreen({
    emotion: presenceSequence === "celebrate" ? "complete" : "idle",
    preferredMode: "companion",
    sequence: presenceSequence,
  });

  useEffect(() => {
    setPresenceSequence("celebrate");

    const timer = setTimeout(() => {
      setPresenceSequence("idle_blink");
    }, 4200);

    return () => clearTimeout(timer);
  }, [fileName]);

  useEffect(() => {
    let mounted = true;

    async function loadReveal() {
      setActionState("saving");
      setError("");
      setInputText("");
      setIsLoading(false);
      setMessage("");
      setMessages([]);
      setPendingRetry(null);
      setPendingReflectionConfirm(null);
      setReflectionKind(null);
      setSealError("");
      setSealProof(null);
      setScreenMode("review");

      const routeFileName = route.params?.fileName ?? null;
      const [nextRaw, nextCredits, nextSessions, nextSelectedLoom, nextSolanaConfig] =
        await Promise.all([
          routeFileName == null ? readPendingReveal() : readAnkyFile(routeFileName),
          getReflectionCreditBalance(),
          listAnkySessionSummaries(),
          getSelectedLoom(),
          loadMobileSolanaConfig(),
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
      let nextSealProof: SealProof | null = null;

      if (nextRaw == null) {
        nextMessage = "no hay escritura cerrada para revelar.";
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
          await hydrateMobileSealReceiptsForHashes([saved.hash]);
          const seals = await readLoomSealsForHash(saved.hash);
          const latestSeal = seals.at(-1) ?? null;
          nextDidSeal = latestSeal != null;
          nextSealProof =
            latestSeal == null
              ? null
              : {
                  network: latestSeal.network,
                  proofState: getLoomSealProofState(
                    latestSeal,
                    nextSolanaConfig.proofVerifierAuthority,
                  ),
                  proofTxSignature: latestSeal.proofTxSignature,
                  txSignature: latestSeal.txSignature,
                };
          nextReflection = await readReflectionSidecar(saved.hash);

          if (!isEntryRoute && autoIndexedHashRef.current !== saved.hash) {
            try {
              await addAnkySessionSummary(
                buildSessionSummary(saved, nextRaw, nextSummaryKind, saved.sealCount > 0),
              );
              await clearPendingReveal();
              await clearActiveDraft();
              autoIndexedHashRef.current = saved.hash;
            } catch (indexError) {
              console.error(indexError);
              nextMessage = "guardado localmente, pero el mapa necesita atención.";
            }
          }
        } else {
          nextHash =
            routeFileName == null ? await computeSessionHash(nextRaw) : routeFileName.replace(/\.anky$/, "");
          nextHashMatches = await verifyHash(nextRaw, nextHash);
          nextMessage = "esta escritura necesita atención antes de pedir una reflexión.";
        }
      }

      if (!mounted) {
        return;
      }

      setRaw(nextRaw);
      setReflection(nextReflection);
      setFileName(nextFileName);
      setCreditBalance(nextCredits);
      setSessions(nextSessions);
      setSelectedLoom(nextSelectedLoom);
      setHash(nextHash);
      setHashMatches(nextHashMatches);
      setSealProof(nextSealProof);
      setDidSeal(nextDidSeal);
      setMessage(nextMessage);
      setActionState(nextMessage.length > 0 && nextRaw != null ? "error" : "idle");
    }

    void loadReveal().catch((loadError) => {
      console.error(loadError);
      if (mounted) {
        setMessage(loadError instanceof Error ? loadError.message : "no se pudo abrir esta escritura.");
        setActionState("error");
      }
    });

    return () => {
      mounted = false;
    };
  }, [isEntryRoute, route.params?.fileName]);

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
      setMessage("copiado.");
      setActionState("idle");
      setTimeout(() => {
        setMessage((current) => (current === "copiado." ? "" : current));
      }, 1600);
    } catch (copyError) {
      console.error(copyError);
      setMessage("no se pudo copiar.");
      setActionState("error");
    }
  }

  async function refreshCredits() {
    const nextCredits = await getReflectionCreditBalance();

    setCreditBalance(nextCredits);
  }

  function openLoginForReflection() {
    openAuthModal({
      afterSuccess: refreshCredits,
      reason: "login to ask anky for a reflection. your writing stays here unless you choose processing.",
    });
  }

  function handleBuyCredits() {
    navigation.navigate("CreditsInfo");
  }

  function handleWriteAgain() {
    navigation.replace("Write");
  }

  async function handleSealWithLoom() {
    if (!canSealWithLoom || selectedLoom == null || currentHash == null) {
      return;
    }

    if (sessionUtcDay == null || !isCurrentUtcDay(sessionUtcDay)) {
      setSealError("only today's UTC anky can be sealed.");
      return;
    }

    try {
      setActionState("sealing");
      setMessage("");
      setError("");
      setSealError("");

      const config = await loadMobileSolanaConfig();
      const signingWallet = await wallet.getWallet();
      const connection = new Connection(config.rpcUrl, "confirmed");
      const receipt = await sealAnky({
        connection,
        coreCollection: selectedLoom.collection,
        loomAsset: selectedLoom.asset,
        network: config.network,
        programId: config.sealProgramId,
        sessionHashHex: currentHash,
        sessionUtcDay,
        wallet: signingWallet,
      });

      await writeSealSidecar(receipt);

      const api = getAnkyApiClient();

      if (api != null) {
        try {
          await api.recordMobileSeal({
            coreCollection: selectedLoom.collection,
            loomAsset: selectedLoom.asset,
            sessionHash: receipt.session_hash,
            signature: receipt.signature,
            status: "confirmed",
            utcDay: sessionUtcDay,
            wallet: receipt.writer,
          });
        } catch (recordError) {
          console.warn("Could not record mobile seal on backend.", recordError);
        }
      }

      if (raw != null) {
        const saved = await ensureSavedFile();
        await addAnkySessionSummary({
          ...buildSessionSummary(saved, raw, summaryKind, true),
          sealedOnchain: true,
        });
      }

      setDidSeal(true);
      setSealProof({
        network: receipt.network,
        proofState: "none",
        txSignature: receipt.signature,
      });
      setActionState("idle");
    } catch (sealError) {
      console.error(sealError);
      setSealError(
        sealError instanceof Error ? sealError.message : "seal failed. your writing is unchanged.",
      );
      setMessage("");
      setActionState("error");
    }
  }

  function requestReflectionWithConfirm(kind: ReflectionKind) {
    if (!canRequestReflection) {
      return;
    }

    if (!isLoggedIn) {
      openLoginForReflection();
      return;
    }

    const cost = kind === "full" ? fullReflectionCost : quickReflectionCost;

    if (creditBalance != null && creditBalance < cost) {
      setError("no hay créditos suficientes para esta reflexión.");
      setMessage("no hay créditos suficientes para esta reflexión.");
      return;
    }

    setError("");
    setMessage("");
    setPendingReflectionConfirm(kind);
  }

  async function handleStartReflection(kind: ReflectionKind) {
    if (!canRequestReflection || raw == null || hash.length === 0) {
      return;
    }

    if (!isLoggedIn) {
      openLoginForReflection();
      return;
    }

    const cost = kind === "full" ? CREDIT_COSTS.full_anky : CREDIT_COSTS.reflection;
    const balance = hasConfiguredBackend()
      ? creditBalance ?? (await getReflectionCreditBalance())
      : null;

    if (balance != null) {
      setCreditBalance(balance);
    }

    if (balance != null && balance < cost) {
      setError("no hay créditos suficientes para esta reflexión.");
      setMessage("no hay créditos suficientes para esta reflexión.");
      return;
    }

    setActionState("reflecting");
    setError("");
    setInputText("");
    setIsLoading(true);
    setMessage("");
    setMessages([]);
    setPendingRetry(null);
    setReflectionKind(kind);
    setScreenMode("chat");
    scrollRevealToEnd();

    try {
      const saved = await ensureSavedFile();
      const result = await processReflectionWithMode(
        saved.fileName,
        kind === "full" ? "full" : "simple",
      );
      const assistantMessage = createRevealChatMessage({
        content: result.markdown,
        role: "assistant",
      });

      await addAnkySessionSummary({
        ...buildSessionSummary(saved, raw, summaryKind, didSeal || saved.sealCount > 0),
        reflectionId: saved.hash,
      });
      if (!isEntryRoute) {
        await clearPendingReveal();
        await clearActiveDraft();
      }

      setCreditBalance(result.creditsRemaining);
      setReflection(result.markdown);
      setMessages([assistantMessage]);
      setActionState("idle");
    } catch (reflectionError) {
      console.error(reflectionError);
      setError(
        reflectionError instanceof Error
          ? reflectionError.message
          : "anky no pudo responder ahora. tu escritura sigue guardada.",
      );
      setActionState("error");
    } finally {
      setIsLoading(false);
      scrollRevealToEnd();
    }
  }

  async function handleSendChatMessage() {
    const trimmed = inputText.trim();

    if (!canSendChat || raw == null || currentHash == null || reflectionKind == null) {
      return;
    }

    const userMessage = createRevealChatMessage({
      content: trimmed,
      role: "user",
    });
    const history = messages;
    const committedMessages = [...history, userMessage];

    setError("");
    setInputText("");
    setIsLoading(true);
    setMessages(committedMessages);
    setPendingRetry(null);
    scrollRevealToEnd();

    try {
      const assistantMessage = await requestRevealChatReply({
        conversationHistory: history,
        existingReflection: reflection ?? undefined,
        rawAnky: raw,
        reconstructedText: reconstructed,
        reflectionKind,
        sessionHash: currentHash,
        userMessage: trimmed,
      });

      setMessages([...committedMessages, assistantMessage]);
    } catch (replyError) {
      console.error(replyError);
      setError(
        replyError instanceof Error
          ? replyError.message
          : "anky no pudo continuar ahora. tu escritura sigue guardada.",
      );
      setPendingRetry({ history, userMessage });
    } finally {
      setIsLoading(false);
      scrollRevealToEnd();
    }
  }

  async function handleRetryChat() {
    if (isLoading || raw == null || currentHash == null || reflectionKind == null) {
      return;
    }

    if (pendingRetry == null) {
      await handleStartReflection(reflectionKind);
      return;
    }

    setError("");
    setIsLoading(true);
    scrollRevealToEnd();

    try {
      const assistantMessage = await requestRevealChatReply({
        conversationHistory: pendingRetry.history,
        existingReflection: reflection ?? undefined,
        rawAnky: raw,
        reconstructedText: reconstructed,
        reflectionKind,
        sessionHash: currentHash,
        userMessage: pendingRetry.userMessage.content,
      });

      setMessages((current) => [...current, assistantMessage]);
      setPendingRetry(null);
    } catch (replyError) {
      console.error(replyError);
      setError(
        replyError instanceof Error
          ? replyError.message
          : "anky no pudo continuar ahora. tu escritura sigue guardada.",
      );
    } finally {
      setIsLoading(false);
      scrollRevealToEnd();
    }
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

  function handleScrollContentSizeChange() {
    if (screenMode === "chat") {
      scrollRevealToEnd();
    }
  }

  function scrollRevealToEnd() {
    setTimeout(() => {
      revealScrollRef.current?.scrollToEnd({ animated: true });
    }, 80);
  }

  if (raw == null) {
    return (
      <ScreenBackground variant="plain">
        <View style={styles.emptyState}>
          {actionState === "saving" ? <GoldenThreadSpinner label="opening" /> : null}
          {actionState === "saving" ? null : <Text style={styles.emptyTitle}>nada que revelar</Text>}
          {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}
        </View>
      </ScreenBackground>
    );
  }

  return (
    <ScreenBackground variant="plain">
      <KeyboardAvoidingView
        behavior={Platform.OS === "ios" ? "padding" : undefined}
        style={styles.keyboard}
      >
        <View style={styles.surface}>
          <RevealBackgroundTexture />
          <RevealHeader
            dateLine={`${dateParts.date} · ${dateParts.time}`}
            metaLine={`${durationLabel} · ${wordCountLabel}`}
            onBack={handleGoBack}
          />
          <ScrollView
            ref={revealScrollRef}
            contentContainerStyle={styles.content}
            keyboardShouldPersistTaps="handled"
            onContentSizeChange={handleScrollContentSizeChange}
            showsVerticalScrollIndicator={false}
          >
            <WritingBlock text={reconstructed} />
            <PrivacyDivider />

            {screenMode === "review" ? (
              <ReviewActions
                canCopy={canCopy}
                canSealWithLoom={canSealWithLoom}
                canRequestReflection={canRequestReflection}
                canShowSealWithLoom={canShowSealWithLoom}
                creditBalance={creditBalance}
                fullCost={fullReflectionCost}
                isLoggedIn={isLoggedIn}
                isFullAnky={isFullAnky}
                onBuyCredits={handleBuyCredits}
                onCopy={() => void handleCopy()}
                onFullReflection={() => requestReflectionWithConfirm("full")}
                onQuickReflection={() => requestReflectionWithConfirm("quick")}
                onSealWithLoom={() => void handleSealWithLoom()}
                onWriteAgain={handleWriteAgain}
                quickCost={quickReflectionCost}
                sealError={sealError || sealUtcDayError}
                sealProof={sealProof}
                sealed={didSeal}
                sealing={actionState === "sealing"}
              />
            ) : (
              <RevealChat
                canRetry={error.length > 0 && !isLoading && reflectionKind != null}
                canSend={canSendChat}
                error={error}
                inputText={inputText}
                isLoading={isLoading}
                messages={messages}
                onChangeInput={setInputText}
                onRetry={() => void handleRetryChat()}
                onSend={() => void handleSendChatMessage()}
              />
            )}

            {screenMode === "review" && message.length > 0 ? (
              <Text style={styles.message}>{message}</Text>
            ) : null}
            {screenMode === "review" && parsed != null && !parsed.valid ? (
              <Text style={styles.errorText}>esta escritura necesita atención antes de reflexión.</Text>
            ) : null}
          </ScrollView>
          <ReflectionSpendConfirmModal
            balance={creditBalance}
            fullCost={fullReflectionCost}
            kind={pendingReflectionConfirm}
            onCancel={() => setPendingReflectionConfirm(null)}
            onConfirm={() => {
              const kind = pendingReflectionConfirm;

              setPendingReflectionConfirm(null);

              if (kind != null) {
                void handleStartReflection(kind);
              }
            }}
            quickCost={quickReflectionCost}
          />
        </View>
      </KeyboardAvoidingView>
    </ScreenBackground>
  );
}

function RevealHeader({
  dateLine,
  metaLine,
  onBack,
}: {
  dateLine: string;
  metaLine: string;
  onBack: () => void;
}) {
  return (
    <View style={styles.fixedHeader}>
      <SubtleIconButton accessibilityLabel="go back" icon="←" onPress={onBack} />
      <View style={styles.headerMeta}>
        <Text numberOfLines={1} style={styles.headerDate}>
          {dateLine}
        </Text>
        <Text numberOfLines={1} style={styles.headerStats}>
          {metaLine}
        </Text>
      </View>
      <View style={styles.headerSpacer} />
    </View>
  );
}

function WritingBlock({ text }: { text: string }) {
  return (
    <View style={styles.writingBlock}>
      <Text selectable style={styles.writingText}>
        {text.length > 0 ? text : " "}
      </Text>
    </View>
  );
}

function PrivacyDivider() {
  return (
    <View style={styles.privacyWrap}>
      <View style={styles.dividerRow}>
        <View style={styles.dividerLine} />
        <Text accessibilityLabel="private writing boundary" style={styles.lock}>
          🔒
        </Text>
        <View style={styles.dividerLine} />
      </View>
      <Text style={styles.privacyText}>
        tu escritura es tuya. solo sale de tu dispositivo si pides una reflexión
      </Text>
    </View>
  );
}

function ReviewActions({
  canCopy,
  canSealWithLoom,
  canRequestReflection,
  canShowSealWithLoom,
  creditBalance,
  fullCost,
  isLoggedIn,
  isFullAnky,
  onBuyCredits,
  onCopy,
  onFullReflection,
  onQuickReflection,
  onSealWithLoom,
  onWriteAgain,
  quickCost,
  sealError,
  sealProof,
  sealed,
  sealing,
}: {
  canCopy: boolean;
  canSealWithLoom: boolean;
  canRequestReflection: boolean;
  canShowSealWithLoom: boolean;
  creditBalance: number | null;
  fullCost: number;
  isLoggedIn: boolean;
  isFullAnky: boolean;
  onBuyCredits: () => void;
  onCopy: () => void;
  onFullReflection: () => void;
  onQuickReflection: () => void;
  onSealWithLoom: () => void;
  onWriteAgain: () => void;
  quickCost: number;
  sealError: string;
  sealProof: SealProof | null;
  sealed: boolean;
  sealing: boolean;
}) {
  const notEnoughForQuick = isLoggedIn && creditBalance != null && creditBalance < quickCost;
  const notEnoughForFull = isLoggedIn && creditBalance != null && creditBalance < fullCost;
  const quickBadge = isFullAnky && isLoggedIn ? formatCreditBadge(quickCost) : undefined;
  const fullBadge = isFullAnky && isLoggedIn ? formatCreditBadge(fullCost) : undefined;
  const quickDisabled = !canRequestReflection || (isLoggedIn && notEnoughForQuick);
  const fullDisabled = !canRequestReflection || (isLoggedIn && notEnoughForFull);
  const statusLine = !isFullAnky
    ? "write 8 minutes to ask anky for reflection"
    : !isLoggedIn
      ? "login to ask anky for a reflection"
      : creditBalance == null
        ? "credits load when anky is reachable"
        : `you have ${creditBalance} ${creditBalance === 1 ? "credit" : "credits"}`;
  const shouldShowBuyCredits =
    isFullAnky && isLoggedIn && creditBalance != null && (notEnoughForQuick || notEnoughForFull);

  return (
    <View style={styles.reviewActions}>
      <RevealActionButton
        disabled={!canCopy}
        icon="⧉"
        label="copiar"
        onPress={onCopy}
        variant="secondary"
      />
      <LoomSealStatus
        canSeal={canSealWithLoom}
        canShow={canShowSealWithLoom}
        error={sealError}
        isSealing={sealing}
        onSeal={onSealWithLoom}
        proofState={sealProof?.proofState}
        proofSignature={sealProof?.proofTxSignature}
        sealNetwork={sealProof?.network}
        sealSignature={sealProof?.txSignature}
        sealed={sealed}
      />
      {!isFullAnky ? (
        <RevealActionButton
          icon="↻"
          label="write again"
          onPress={onWriteAgain}
          variant="primary"
        />
      ) : null}
      <View style={styles.actionSeparator} />
      <View style={styles.reflectionIntro}>
        <Text style={styles.reflectionStatus}>{statusLine}</Text>
      </View>
      <RevealActionButton
        badge={quickBadge}
        disabled={quickDisabled}
        icon="✦"
        label="ask for reflection"
        onPress={onQuickReflection}
        variant="primary"
      />
      <RevealActionButton
        badge={fullBadge}
        disabled={fullDisabled}
        icon="◎"
        label="full anky reflection"
        onPress={onFullReflection}
        variant="accent"
      />
      {shouldShowBuyCredits ? (
        <Pressable accessibilityRole="button" onPress={onBuyCredits} style={styles.buyCreditsLink}>
          <Text style={styles.buyCreditsText}>buy credits</Text>
        </Pressable>
      ) : null}
    </View>
  );
}

function formatCreditBadge(cost: number): string {
  return `${cost} credit${cost === 1 ? "" : "s"}`;
}

function ReflectionSpendConfirmModal({
  balance,
  fullCost,
  kind,
  onCancel,
  onConfirm,
  quickCost,
}: {
  balance: number | null;
  fullCost: number;
  kind: ReflectionKind | null;
  onCancel: () => void;
  onConfirm: () => void;
  quickCost: number;
}) {
  if (kind == null) {
    return null;
  }

  const cost = kind === "full" ? fullCost : quickCost;
  const title = `spend ${cost} ${cost === 1 ? "credit" : "credits"}?`;
  const modeLabel = kind === "full" ? "a full anky reflection" : "a reflection";
  const balanceLine =
    balance == null
      ? "anky will check your credits before processing."
      : `you have ${balance} ${balance === 1 ? "credit" : "credits"} left.`;

  return (
    <Modal animationType="fade" onRequestClose={onCancel} transparent visible>
      <View style={styles.confirmBackdrop}>
        <Pressable
          accessibilityLabel="close reflection confirmation"
          accessibilityRole="button"
          onPress={onCancel}
          style={StyleSheet.absoluteFill}
        />
        <View style={styles.confirmCard}>
          <View pointerEvents="none" style={styles.confirmThreadOverlay}>
            <View style={styles.confirmThreadWash} />
            <View style={styles.confirmThreadLineTop} />
            <View style={styles.confirmThreadLineBottom} />
          </View>
          <Text style={styles.confirmEyebrow}>before anky reads</Text>
          <Text style={styles.confirmTitle}>{title}</Text>
          <Text style={styles.confirmBody}>
            {balanceLine} this will ask anky for {modeLabel}.
          </Text>

          <View style={styles.confirmActions}>
            <Pressable
              accessibilityRole="button"
              onPress={onCancel}
              style={({ pressed }) => [
                styles.confirmSecondaryButton,
                pressed && styles.pressed,
              ]}
            >
              <Text style={styles.confirmSecondaryText}>not now</Text>
            </Pressable>

            <Pressable
              accessibilityRole="button"
              onPress={onConfirm}
              style={({ pressed }) => [
                styles.confirmPrimaryButton,
                pressed && styles.pressed,
              ]}
            >
              <Text style={styles.confirmPrimaryText}>spend {cost}</Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}

type LoomSealStatusProps = {
  canSeal: boolean;
  canShow: boolean;
  error: string;
  isSealing: boolean;
  onSeal: () => void;
  proofSignature?: string;
  proofState?: "failed" | "none" | "proving" | "verified";
  sealNetwork?: "devnet" | "mainnet-beta";
  sealSignature?: string;
  sealed: boolean;
};

function LoomSealStatus({
  canSeal,
  canShow,
  error,
  isSealing,
  onSeal,
  proofSignature,
  proofState = "none",
  sealNetwork,
  sealSignature,
  sealed,
}: LoomSealStatusProps) {
  const hasError = error.trim().length > 0;
  const hasSignature = sealSignature != null && sealSignature.length > 0;
  const visibleSignature = proofSignature ?? sealSignature;
  const sealedLabel =
    proofState === "verified"
      ? "proof verified"
      : proofState === "proving"
        ? "proving"
        : proofState === "failed"
          ? "proof failed"
          : "sealed";
  const proofHasReceipt = proofState === "verified" || proofState === "failed";

  if (!canShow && !sealed) {
    return null;
  }

  if (sealed) {
    return (
      <View style={styles.loomSealWrap}>
        <RevealActionButton
          centered
          disabled
          label={sealedLabel}
          variant={proofState === "failed" ? "sealFailed" : "seal"}
        />
        {visibleSignature != null && visibleSignature.length > 0 ? (
          <Pressable
            accessibilityLabel="view seal transaction on orb"
            accessibilityRole="link"
            onPress={() => {
              void openOrbTx(visibleSignature, sealNetwork);
            }}
            style={({ pressed }) => [styles.txHashPressable, pressed && styles.pressed]}
          >
            <Text style={styles.txHashLink}>
              {proofHasReceipt && proofSignature != null ? "proof " : "seal "}
              {shortenTx(visibleSignature)}
            </Text>
          </Pressable>
        ) : null}
        {proofState === "proving" && !hasSignature ? (
          <Text style={styles.txHashMuted}>sp1 receipt pending</Text>
        ) : null}
        {proofState === "failed" ? (
          <Text style={styles.txHashMuted}>sp1 receipt failed</Text>
        ) : null}
      </View>
    );
  }

  if (isSealing) {
    return (
      <View style={styles.loomSealWrap}>
        <RevealActionButton
          disabled
          helper="weaving your hash seal into the ankyverse"
          label="sealing..."
          loading
          variant="seal"
        />
      </View>
    );
  }

  if (hasError && canSeal) {
    return (
      <View style={styles.loomSealWrap}>
        <RevealActionButton
          helper={error}
          icon="!"
          label="try sealing again"
          onPress={onSeal}
          variant="sealFailed"
        />
      </View>
    );
  }

  if (canShow && canSeal) {
    return (
      <View style={styles.loomSealWrap}>
        <RevealActionButton icon="◇" label="seal with loom" onPress={onSeal} variant="seal" />
      </View>
    );
  }

  return (
    <View style={styles.loomSealWrap}>
      <RevealActionButton
        disabled
        helper={hasError ? error : "your writing is still whole without a seal"}
        icon="◇"
        label="loom unavailable"
        variant="sealMuted"
      />
    </View>
  );
}

type RevealActionVariant =
  | "accent"
  | "primary"
  | "seal"
  | "sealFailed"
  | "sealMuted"
  | "secondary";

function RevealActionButton({
  badge,
  centered = false,
  disabled = false,
  helper,
  icon,
  label,
  loading = false,
  onPress,
  variant,
}: {
  badge?: string;
  centered?: boolean;
  disabled?: boolean;
  helper?: string;
  icon?: string;
  label: string;
  loading?: boolean;
  onPress?: () => void;
  variant: RevealActionVariant;
}) {
  const pressableDisabled = disabled || loading || onPress == null;
  const hasBadge = badge != null;

  return (
    <Pressable
      accessibilityRole="button"
      disabled={pressableDisabled}
      onPress={onPress}
      style={({ pressed }) => [
        styles.actionButton,
        variant === "primary" && styles.actionButtonPrimary,
        variant === "accent" && styles.actionButtonAccent,
        variant === "secondary" && styles.actionButtonSecondary,
        variant === "seal" && styles.actionButtonSeal,
        variant === "sealFailed" && styles.actionButtonSealFailed,
        variant === "sealMuted" && styles.actionButtonSealMuted,
        centered && styles.actionButtonCentered,
        disabled &&
          (variant === "primary" || variant === "accent" || variant === "secondary") &&
          styles.disabled,
        pressed && !pressableDisabled && styles.pressed,
      ]}
    >
      <View pointerEvents="none" style={styles.actionThreadOverlay}>
        <View style={styles.actionThreadWash} />
        <View style={styles.actionThreadLineTop} />
        <View style={styles.actionThreadLineBottom} />
      </View>
      {centered ? (
        <View style={styles.actionCenteredContent}>
          <Text style={styles.actionInlineOrnament}>✦</Text>
          <Text
            adjustsFontSizeToFit
            minimumFontScale={0.86}
            numberOfLines={1}
            style={[
              styles.actionText,
              styles.actionTextCentered,
              variant === "primary" && styles.actionTextPrimary,
              variant === "sealFailed" && styles.actionTextFailed,
              disabled && variant !== "seal" && styles.actionTextDisabled,
            ]}
          >
            {label}
          </Text>
          <Text style={styles.actionInlineOrnament}>✦</Text>
        </View>
      ) : (
        <>
          <View style={styles.actionIconArea}>
            {loading ? (
              <ActivityIndicator color={GOLD} size="small" />
            ) : (
              <Text
                style={[
                  styles.actionIcon,
                  variant === "sealFailed" && styles.actionIconFailed,
                  disabled && variant !== "seal" && styles.actionTextDisabled,
                ]}
              >
                {icon}
              </Text>
            )}
          </View>
          <View style={[styles.actionContent, hasBadge && styles.actionContentWithBadge]}>
            <Text
              adjustsFontSizeToFit
              minimumFontScale={0.82}
              numberOfLines={2}
              style={[
                styles.actionText,
                variant === "primary" && styles.actionTextPrimary,
                variant === "sealFailed" && styles.actionTextFailed,
                disabled && variant !== "seal" && styles.actionTextDisabled,
              ]}
            >
              {label}
            </Text>
            {helper == null ? null : (
              <Text
                numberOfLines={2}
                style={[
                  styles.actionHelper,
                  variant === "sealFailed" && styles.actionHelperFailed,
                  disabled && variant !== "seal" && styles.actionHelperDisabled,
                ]}
              >
                {helper}
              </Text>
            )}
          </View>
        </>
      )}
      {badge == null ? null : (
        <View style={styles.creditBadge}>
          <Text style={styles.creditBadgeText}>{badge}</Text>
        </View>
      )}
    </Pressable>
  );
}

function RevealChat({
  canRetry,
  canSend,
  error,
  inputText,
  isLoading,
  messages,
  onChangeInput,
  onRetry,
  onSend,
}: {
  canRetry: boolean;
  canSend: boolean;
  error: string;
  inputText: string;
  isLoading: boolean;
  messages: RevealChatMessage[];
  onChangeInput: (value: string) => void;
  onRetry: () => void;
  onSend: () => void;
}) {
  return (
    <View style={styles.chatArea}>
      <View style={styles.chatMessages}>
        {messages.map((message) => (
          <RevealMessageBubble key={message.id} message={message} />
        ))}
        {isLoading ? (
          <View style={styles.loadingRow}>
            <GoldenThreadSpinner />
            <Text style={styles.loadingText}>anky está leyendo</Text>
          </View>
        ) : null}
      </View>

      {error.length === 0 ? null : (
        <View style={styles.chatError}>
          <Text style={styles.errorText}>{error}</Text>
          {canRetry ? (
            <Pressable accessibilityRole="button" onPress={onRetry} style={styles.retryButton}>
              <Text style={styles.retryText}>intentar otra vez</Text>
            </Pressable>
          ) : null}
        </View>
      )}

      <View style={styles.chatInputRow}>
        <TextInput
          autoCapitalize="sentences"
          multiline
          onChangeText={onChangeInput}
          placeholder="escribe de vuelta..."
          placeholderTextColor="rgba(255, 240, 201, 0.42)"
          style={styles.chatInput}
          value={inputText}
        />
        <Pressable
          accessibilityRole="button"
          disabled={!canSend}
          onPress={onSend}
          style={[styles.sendButton, !canSend && styles.disabled]}
        >
          <Text style={styles.sendText}>enviar</Text>
        </Pressable>
      </View>
    </View>
  );
}

function RevealMessageBubble({ message }: { message: RevealChatMessage }) {
  const isAssistant = message.role === "assistant";

  return (
    <View style={[styles.messageRow, isAssistant ? styles.assistantRow : styles.userRow]}>
      <View style={[styles.bubble, isAssistant ? styles.assistantBubble : styles.userBubble]}>
        <Text style={styles.bubbleLabel}>{isAssistant ? "anky" : "tú"}</Text>
        <Text selectable style={styles.bubbleText}>
          {message.content}
        </Text>
      </View>
    </View>
  );
}

function RevealBackgroundTexture() {
  return (
    <View pointerEvents="none" style={styles.backgroundTexture}>
      <View style={[styles.backgroundLine, { top: 96, width: "56%" }]} />
      <View style={[styles.backgroundLine, { top: 218, width: "76%" }]} />
      <View style={[styles.backgroundLine, { bottom: 180, width: "62%" }]} />
    </View>
  );
}

async function requestRevealChatReply({
  conversationHistory,
  existingReflection,
  rawAnky,
  reconstructedText,
  reflectionKind,
  sessionHash,
  userMessage,
}: {
  conversationHistory: RevealChatMessage[];
  existingReflection?: string;
  rawAnky: string;
  reconstructedText: string;
  reflectionKind: ReflectionKind;
  sessionHash: string;
  userMessage: string;
}): Promise<RevealChatMessage> {
  const response = await sendThreadMessage({
    existingReflection,
    messages: toThreadMessages(conversationHistory),
    mode: "reflection",
    rawAnky,
    reconstructedText,
    reflectionKind,
    sessionHash,
    userMessage,
  });

  return createRevealChatMessage({
    content: response.content,
    createdAt: response.createdAt,
    id: response.id,
    role: "assistant",
  });
}

function toThreadMessages(messages: RevealChatMessage[]): ThreadMessage[] {
  return messages.map((message) => ({
    content: message.content,
    createdAt: message.createdAt,
    id: message.id,
    role: message.role === "assistant" ? "anky" : "user",
  }));
}

function createRevealChatMessage({
  content,
  createdAt = new Date().toISOString(),
  id = createRevealMessageId(),
  role,
}: {
  content: string;
  createdAt?: string;
  id?: string;
  role: RevealChatRole;
}): RevealChatMessage {
  return {
    content,
    createdAt,
    id,
    role,
  };
}

function createRevealMessageId(): string {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
}

function shortenTx(signature?: string | null): string {
  if (signature == null || signature.length === 0) {
    return "";
  }

  if (signature.length <= 18) {
    return signature;
  }

  return `${signature.slice(0, 8)}...${signature.slice(-8)}`;
}

function getOrbTxUrl(
  signature: string,
  network?: "devnet" | "mainnet-beta",
): string {
  const cluster = network === "devnet" ? "?cluster=devnet" : "";
  return `https://orbmarkets.io/tx/${signature}${cluster}`;
}

async function openOrbTx(
  signature: string,
  network?: "devnet" | "mainnet-beta",
): Promise<void> {
  const url = getOrbTxUrl(signature, network);
  const canOpen = await Linking.canOpenURL(url);

  if (canOpen) {
    await Linking.openURL(url);
  }
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
    wordCount: countWords(text),
  };
}

function getRevealKind(parsed: ReturnType<typeof parseAnky> | null): RevealKind {
  return isCompleteParsedAnky(parsed) ? "complete" : "short";
}

function formatRevealDateParts(startedAt: number | null): { date: string; time: string } {
  if (startedAt == null) {
    return {
      date: "fecha desconocida",
      time: "hora desconocida",
    };
  }

  const date = new Date(startedAt);

  return {
    date: date.toLocaleDateString(SPANISH_LOCALE, {
      day: "numeric",
      month: "long",
      year: "numeric",
    }),
    time: date.toLocaleTimeString(SPANISH_LOCALE, {
      hour: "numeric",
      minute: "2-digit",
    }).toLowerCase(),
  };
}

function formatWrittenDuration(ms: number | null): string {
  if (ms == null) {
    return "tiempo desconocido";
  }

  const totalSeconds = Math.max(0, Math.round(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = String(totalSeconds % 60).padStart(2, "0");

  return `${minutes}:${seconds} escritos`;
}

function countWords(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

function formatWordCount(count: number): string {
  return `${count} ${count === 1 ? "palabra" : "palabras"}`;
}

const styles = StyleSheet.create({
  actionButton: {
    alignItems: "center",
    backgroundColor: "rgba(10, 8, 22, 0.94)",
    borderColor: "rgba(232, 200, 121, 0.24)",
    borderRadius: 18,
    borderWidth: 1,
    elevation: 2,
    flexDirection: "row",
    minHeight: 70,
    overflow: "hidden",
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    position: "relative",
    shadowColor: "#000",
    shadowOffset: { height: 8, width: 0 },
    shadowOpacity: 0.1,
    shadowRadius: 14,
    width: "100%",
  },
  actionButtonAccent: {
    backgroundColor: "rgba(55, 42, 93, 0.42)",
    borderColor: "rgba(232, 200, 121, 0.34)",
    shadowColor: ankyColors.violet,
    shadowOpacity: 0.1,
  },
  actionButtonCentered: {
    justifyContent: "center",
  },
  actionButtonPrimary: {
    backgroundColor: "rgba(68, 48, 23, 0.62)",
    borderColor: "rgba(232, 200, 121, 0.5)",
    shadowColor: GOLD,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.12,
    shadowRadius: 16,
  },
  actionButtonSeal: {
    backgroundColor: "rgba(15, 12, 30, 0.9)",
    borderColor: "rgba(232, 200, 121, 0.26)",
    shadowOpacity: 0.07,
  },
  actionButtonSealFailed: {
    backgroundColor: "rgba(31, 14, 24, 0.72)",
    borderColor: "rgba(241, 169, 130, 0.28)",
    shadowColor: "#F1A982",
    shadowOpacity: 0.08,
  },
  actionButtonSealMuted: {
    backgroundColor: "rgba(12, 10, 24, 0.68)",
    borderColor: "rgba(232, 200, 121, 0.16)",
    shadowOpacity: 0.04,
  },
  actionButtonSecondary: {
    backgroundColor: "rgba(255, 255, 255, 0.024)",
    borderColor: "rgba(232, 200, 121, 0.17)",
    shadowOpacity: 0.05,
  },
  actionCenteredContent: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.sm,
    justifyContent: "center",
    paddingHorizontal: spacing.lg,
    width: "100%",
  },
  actionContent: {
    flex: 1,
    justifyContent: "center",
    minWidth: 0,
  },
  actionContentWithBadge: {
    paddingRight: 82,
  },
  actionHelper: {
    color: "rgba(255, 240, 201, 0.54)",
    fontSize: fontSize.xs,
    lineHeight: 16,
    marginTop: 2,
    textTransform: "lowercase",
  },
  actionHelperDisabled: {
    color: "rgba(255, 240, 201, 0.42)",
  },
  actionHelperFailed: {
    color: "rgba(241, 169, 130, 0.74)",
  },
  actionIconArea: {
    alignItems: "center",
    justifyContent: "center",
    marginRight: spacing.md,
    width: 30,
  },
  actionIcon: {
    color: GOLD,
    fontSize: 22,
    lineHeight: 26,
    textAlign: "center",
  },
  actionIconFailed: {
    color: "#F1A982",
  },
  actionInlineOrnament: {
    color: "rgba(232, 200, 121, 0.58)",
    fontSize: 12,
    lineHeight: 16,
  },
  actionText: {
    color: PAPER,
    fontSize: 17,
    fontWeight: "800",
    lineHeight: 22,
    textAlign: "left",
    textTransform: "lowercase",
  },
  actionTextCentered: {
    color: GOLD_SOFT,
    flexShrink: 1,
    textAlign: "center",
  },
  actionTextDisabled: {
    color: "rgba(255, 240, 201, 0.48)",
  },
  actionTextFailed: {
    color: "#F1C29C",
  },
  actionTextPrimary: {
    color: GOLD,
  },
  actionSeparator: {
    backgroundColor: "rgba(232, 200, 121, 0.14)",
    height: StyleSheet.hairlineWidth,
    marginVertical: spacing.sm,
    width: "100%",
  },
  actionThreadLineBottom: {
    backgroundColor: "rgba(232, 200, 121, 0.1)",
    bottom: 8,
    height: StyleSheet.hairlineWidth,
    left: spacing.lg,
    position: "absolute",
    right: spacing.lg,
  },
  actionThreadLineTop: {
    backgroundColor: "rgba(232, 200, 121, 0.14)",
    height: StyleSheet.hairlineWidth,
    left: spacing.lg,
    position: "absolute",
    right: spacing.lg,
    top: 8,
  },
  actionThreadOverlay: {
    ...StyleSheet.absoluteFillObject,
    opacity: 0.66,
  },
  actionThreadWash: {
    backgroundColor: "rgba(139, 124, 246, 0.055)",
    bottom: 0,
    left: "18%",
    position: "absolute",
    right: "18%",
    top: 0,
  },
  assistantBubble: {
    backgroundColor: "rgba(17, 13, 31, 0.82)",
    borderColor: "rgba(232, 200, 121, 0.18)",
  },
  assistantRow: {
    justifyContent: "flex-start",
  },
  backgroundLine: {
    alignSelf: "center",
    backgroundColor: "rgba(232, 200, 121, 0.046)",
    height: StyleSheet.hairlineWidth,
    position: "absolute",
  },
  backgroundTexture: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: INK,
  },
  bubble: {
    borderRadius: 8,
    borderWidth: 1,
    maxWidth: "88%",
    paddingHorizontal: spacing.md,
    paddingVertical: 12,
  },
  bubbleLabel: {
    color: GOLD_DIM,
    fontSize: fontSize.xs,
    fontWeight: "800",
    marginBottom: 6,
    textTransform: "lowercase",
  },
  bubbleText: {
    color: PAPER,
    fontSize: fontSize.md,
    lineHeight: 24,
  },
  chatArea: {
    gap: spacing.md,
    marginTop: spacing.lg,
  },
  buyCreditsLink: {
    alignItems: "center",
    paddingVertical: spacing.sm,
  },
  buyCreditsText: {
    color: GOLD,
    fontSize: fontSize.sm,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  chatError: {
    alignItems: "center",
    gap: spacing.sm,
  },
  chatInput: {
    backgroundColor: "rgba(255, 255, 255, 0.045)",
    borderColor: "rgba(232, 200, 121, 0.2)",
    borderRadius: 8,
    borderWidth: 1,
    color: PAPER,
    flex: 1,
    fontSize: fontSize.md,
    lineHeight: 22,
    maxHeight: 118,
    minHeight: 48,
    paddingHorizontal: spacing.md,
    paddingVertical: 12,
    textAlignVertical: "top",
  },
  chatInputRow: {
    alignItems: "flex-end",
    flexDirection: "row",
    gap: spacing.sm,
    marginTop: spacing.xs,
  },
  chatMessages: {
    gap: spacing.md,
  },
  content: {
    paddingBottom: 44,
    paddingHorizontal: spacing.xl,
    paddingTop: spacing.lg,
  },
  confirmActions: {
    flexDirection: "row",
    gap: spacing.sm,
    marginTop: spacing.lg,
  },
  confirmBackdrop: {
    ...StyleSheet.absoluteFillObject,
    alignItems: "center",
    backgroundColor: "rgba(2, 2, 8, 0.78)",
    justifyContent: "center",
    paddingHorizontal: spacing.xl,
  },
  confirmBody: {
    color: "rgba(255, 240, 201, 0.68)",
    fontSize: fontSize.sm,
    lineHeight: 21,
    textAlign: "center",
    textTransform: "lowercase",
  },
  confirmCard: {
    backgroundColor: "rgba(10, 8, 22, 0.98)",
    borderColor: "rgba(232, 200, 121, 0.28)",
    borderRadius: 18,
    borderWidth: 1,
    elevation: 5,
    overflow: "hidden",
    padding: spacing.lg,
    position: "relative",
    shadowColor: GOLD,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.12,
    shadowRadius: 18,
    width: "100%",
  },
  confirmEyebrow: {
    color: GOLD_DIM,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0.6,
    marginBottom: spacing.xs,
    textAlign: "center",
    textTransform: "lowercase",
  },
  confirmPrimaryButton: {
    alignItems: "center",
    backgroundColor: "rgba(68, 48, 23, 0.62)",
    borderColor: "rgba(232, 200, 121, 0.5)",
    borderRadius: 16,
    borderWidth: 1,
    flex: 1,
    justifyContent: "center",
    minHeight: 52,
  },
  confirmPrimaryText: {
    color: GOLD,
    fontSize: fontSize.sm,
    fontWeight: "900",
    textTransform: "lowercase",
  },
  confirmSecondaryButton: {
    alignItems: "center",
    backgroundColor: "rgba(255, 255, 255, 0.024)",
    borderColor: "rgba(232, 200, 121, 0.17)",
    borderRadius: 16,
    borderWidth: 1,
    flex: 1,
    justifyContent: "center",
    minHeight: 52,
  },
  confirmSecondaryText: {
    color: "rgba(255, 240, 201, 0.72)",
    fontSize: fontSize.sm,
    fontWeight: "800",
    textTransform: "lowercase",
  },
  confirmTitle: {
    color: GOLD,
    fontFamily: SERIF,
    fontSize: 24,
    lineHeight: 30,
    marginBottom: spacing.sm,
    textAlign: "center",
    textTransform: "lowercase",
  },
  confirmThreadLineBottom: {
    backgroundColor: "rgba(232, 200, 121, 0.08)",
    bottom: 10,
    height: StyleSheet.hairlineWidth,
    left: spacing.lg,
    position: "absolute",
    right: spacing.lg,
  },
  confirmThreadLineTop: {
    backgroundColor: "rgba(232, 200, 121, 0.12)",
    height: StyleSheet.hairlineWidth,
    left: spacing.lg,
    position: "absolute",
    right: spacing.lg,
    top: 10,
  },
  confirmThreadOverlay: {
    ...StyleSheet.absoluteFillObject,
    opacity: 0.7,
  },
  confirmThreadWash: {
    backgroundColor: "rgba(139, 124, 246, 0.045)",
    bottom: 0,
    left: "18%",
    position: "absolute",
    right: "18%",
    top: 0,
  },
  creditBadge: {
    backgroundColor: "rgba(8, 7, 19, 0.78)",
    borderColor: "rgba(232, 200, 121, 0.42)",
    borderRadius: 999,
    borderWidth: 1,
    paddingHorizontal: 9,
    paddingVertical: 4,
    position: "absolute",
    right: 10,
    top: 10,
  },
  creditBadgeText: {
    color: GOLD_SOFT,
    fontSize: 10,
    fontWeight: "900",
    textTransform: "lowercase",
  },
  disabled: {
    opacity: 0.44,
  },
  dividerLine: {
    backgroundColor: "rgba(232, 200, 121, 0.22)",
    flex: 1,
    height: StyleSheet.hairlineWidth,
  },
  dividerRow: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.md,
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
    marginTop: spacing.sm,
    textAlign: "center",
    textTransform: "lowercase",
  },
  fixedHeader: {
    alignItems: "center",
    backgroundColor: "rgba(8, 7, 19, 0.96)",
    borderBottomColor: "rgba(232, 200, 121, 0.13)",
    borderBottomWidth: StyleSheet.hairlineWidth,
    flexDirection: "row",
    gap: spacing.md,
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
  },
  headerDate: {
    color: PAPER,
    fontSize: fontSize.sm,
    lineHeight: 18,
    textAlign: "center",
    textTransform: "lowercase",
  },
  headerMeta: {
    alignItems: "center",
    flex: 1,
  },
  headerSpacer: {
    width: 36,
  },
  headerStats: {
    color: GOLD_SOFT,
    fontSize: fontSize.xs,
    fontVariant: ["tabular-nums"],
    lineHeight: 16,
    marginTop: 2,
    textAlign: "center",
    textTransform: "lowercase",
  },
  keyboard: {
    flex: 1,
  },
  loadingRow: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.sm,
    justifyContent: "center",
    paddingVertical: spacing.sm,
  },
  loadingText: {
    color: GOLD_SOFT,
    fontSize: fontSize.sm,
    textTransform: "lowercase",
  },
  lock: {
    color: GOLD,
    fontSize: 18,
    lineHeight: 22,
  },
  loomSealWrap: {
    alignItems: "center",
    width: "100%",
  },
  message: {
    color: GOLD_SOFT,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
    textAlign: "center",
    textTransform: "lowercase",
  },
  messageRow: {
    flexDirection: "row",
  },
  pressed: {
    opacity: 0.72,
    transform: [{ scale: 0.99 }],
  },
  privacyText: {
    color: "rgba(255, 240, 201, 0.62)",
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.sm,
    textAlign: "center",
    textTransform: "lowercase",
  },
  privacyWrap: {
    marginTop: spacing.xl,
  },
  retryButton: {
    borderColor: "rgba(232, 200, 121, 0.3)",
    borderRadius: 8,
    borderWidth: 1,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
  },
  retryText: {
    color: GOLD,
    fontSize: fontSize.sm,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  reviewActions: {
    gap: spacing.sm,
    marginTop: spacing.lg,
  },
  reflectionIntro: {
    alignItems: "center",
    paddingBottom: 2,
    paddingHorizontal: spacing.sm,
  },
  reflectionStatus: {
    color: GOLD_SOFT,
    fontSize: fontSize.sm,
    lineHeight: 19,
    textAlign: "center",
    textTransform: "lowercase",
  },
  sendButton: {
    alignItems: "center",
    backgroundColor: "rgba(232, 200, 121, 0.2)",
    borderColor: "rgba(232, 200, 121, 0.45)",
    borderRadius: 8,
    borderWidth: 1,
    height: 48,
    justifyContent: "center",
    paddingHorizontal: spacing.md,
  },
  sendText: {
    color: GOLD,
    fontSize: fontSize.sm,
    fontWeight: "800",
    textTransform: "lowercase",
  },
  surface: {
    flex: 1,
  },
  userBubble: {
    backgroundColor: "rgba(232, 200, 121, 0.14)",
    borderColor: "rgba(232, 200, 121, 0.28)",
  },
  userRow: {
    justifyContent: "flex-end",
  },
  txHashLink: {
    color: "rgba(255, 240, 201, 0.62)",
    fontSize: fontSize.xs,
    fontVariant: ["tabular-nums"],
    textAlign: "center",
    textDecorationColor: "rgba(232, 200, 121, 0.38)",
    textDecorationLine: "underline",
  },
  txHashPressable: {
    marginTop: spacing.sm,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
  },
  txHashMuted: {
    color: "rgba(255, 240, 201, 0.46)",
    fontSize: fontSize.xs,
    marginTop: spacing.xs,
    textAlign: "center",
    textTransform: "lowercase",
  },
  writingBlock: {
    paddingVertical: spacing.sm,
  },
  writingText: {
    color: PAPER,
    fontFamily: SERIF,
    fontSize: 19,
    letterSpacing: 0,
    lineHeight: 31,
  },
});
