import { useEffect, useState } from "react";
import { ScrollView, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import * as Clipboard from "expo-clipboard";
import { Connection } from "@solana/web3.js";

import type { RootStackParamList } from "../../App";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { EntryActionRow } from "../components/entry/EntryActionRow";
import { EntryHeader } from "../components/entry/EntryHeader";
import { EntryReflectionCard } from "../components/entry/EntryReflectionCard";
import { EntryWritingCard } from "../components/entry/EntryWritingCard";
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
import { getAnkyApiClient } from "../lib/api/client";
import { CREDIT_COSTS } from "../lib/api/types";
import { parseAnky, reconstructText, verifyHash } from "../lib/ankyProtocol";
import {
  readLoomSealsForHash,
  readProcessingReceipt,
  readReflectionSidecar,
  readSavedAnkyFile,
  writeSealSidecar,
} from "../lib/ankyStorage";
import type { ProcessingReceiptSidecar } from "../lib/ankyStorage";
import type { AnkyLocalState } from "../lib/ankyState";
import { hasConfiguredBackend } from "../lib/auth/backendSession";
import {
  getReflectionCreditBalance,
  processReflectionWithMode,
} from "../lib/credits/processAnky";
import { useAnkyPrivyWallet } from "../lib/privy/useAnkyPrivyWallet";
import type { LoomSeal } from "../lib/solana/types";
import { hydrateMobileSealReceiptsForHashes } from "../lib/solana/mobileSealReceipts";
import { getSelectedLoom } from "../lib/solana/loomStorage";
import { loadMobileSolanaConfig } from "../lib/solana/mobileSolanaConfig";
import {
  getUtcDayFromUnixMs,
  isCurrentUtcDay,
  sealAnky as sealAnkyOnchain,
} from "../lib/solana/sealAnky";
import {
  getRiteDurationMs,
  getThreadModeForRawAnky,
  isCompleteParsedAnky,
} from "../lib/thread/threadLogic";
import { getThread } from "../lib/thread/threadStorage";
import type { AnkyThread } from "../lib/thread/types";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Entry">;

type EntryState = {
  hash: string;
  hashMatches: boolean;
  artifactKinds: string[];
  localState: AnkyLocalState;
  processingReceipt: ProcessingReceiptSidecar | null;
  raw: string;
  reflection: string | null;
  seal: LoomSeal | null;
  text: string;
  thread: AnkyThread | null;
  valid: boolean;
};

export function EntryScreen({ navigation, route }: Props) {
  const walletState = useAnkyPrivyWallet();
  const [actionState, setActionState] = useState<"idle" | "reflecting" | "sealing" | "error">("idle");
  const [creditBalance, setCreditBalance] = useState<number | null>(null);
  const [entry, setEntry] = useState<EntryState | null>(null);
  const [reflectionExpanded, setReflectionExpanded] = useState(false);
  const [reflectSheetVisible, setReflectSheetVisible] = useState(false);
  const [sealError, setSealError] = useState("");
  const [statusMessage, setStatusMessage] = useState("");

  useEffect(() => {
    let mounted = true;

    async function loadEntry() {
      const initialSaved = await readSavedAnkyFile(route.params.fileName);

      await hydrateMobileSealReceiptsForHashes([initialSaved.hash]);

      const saved = await readSavedAnkyFile(route.params.fileName);
      const raw = saved.raw;
      const hash = saved.hash;
      const parsed = parseAnky(raw);
      const [hashMatches, seals, reflection, processingReceipt, thread, credits] = await Promise.all([
        verifyHash(raw, hash),
        readLoomSealsForHash(hash),
        readReflectionSidecar(hash),
        readProcessingReceipt(hash),
        getThread(hash),
        getReflectionCreditBalance(),
      ]);

      if (mounted) {
        setCreditBalance(credits);
        setEntry({
          artifactKinds: saved.artifactKinds,
          hash,
          hashMatches,
          localState: saved.localState,
          processingReceipt,
          raw,
          reflection,
          seal: seals.at(-1) ?? null,
          text: reconstructText(raw),
          thread,
          valid: parsed.valid,
        });
      }
    }

    void loadEntry().catch((error) => {
      console.error(error);
    });
    const unsubscribe = navigation.addListener("focus", () => {
      void loadEntry().catch((error) => {
        console.error(error);
      });
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation, route.params.fileName]);

  async function handleCopyText(text: string) {
    await Clipboard.setStringAsync(text);
    setStatusMessage("text copied");
    setTimeout(() => setStatusMessage(""), 1600);
  }

  async function handleReflect(entryState: EntryState, mode: SessionReflectionMode) {
    if (!isCompleteParsedAnky(parseAnky(entryState.raw))) {
      setStatusMessage("fragments can be copied, but not sent to anky.");
      return;
    }

    const cost = mode === "full" ? CREDIT_COSTS.full_anky : REFLECTION_COST_CREDITS;

    const balance = hasConfiguredBackend() ? creditBalance : null;

    if (balance != null && balance < cost) {
      setStatusMessage("available credits: 0");
      return;
    }

    try {
      setActionState("reflecting");
      setStatusMessage("");
      setReflectSheetVisible(false);
      const result = await processReflectionWithMode(route.params.fileName, mode);
      const reflection = await readReflectionSidecar(entryState.hash);
      const processingReceipt = await readProcessingReceipt(entryState.hash);

      setCreditBalance(result.creditsRemaining);
      setEntry({
        ...entryState,
        artifactKinds: [...new Set([...entryState.artifactKinds, "reflection", "processing"])],
        processingReceipt,
        reflection,
      });
      setStatusMessage(mode === "full" ? "full reflection saved." : "reflection saved.");
      setActionState("idle");
    } catch (error) {
      console.error(error);
      setStatusMessage(
        error instanceof Error ? error.message : "Reflection failed. Your .anky is unchanged.",
      );
      setActionState("error");
    }
  }

  function handleBuyCredits() {
    setReflectSheetVisible(false);
    navigation.navigate("Credits", {
      fileName: route.params.fileName,
      processingType: "reflection",
    });
  }

  async function handleSeal(entryState: EntryState) {
    if (entryState.seal != null) {
      return;
    }

    try {
      setActionState("sealing");
      setSealError("");
      setStatusMessage("");

      const parsed = parseAnky(entryState.raw);
      const sessionUtcDay =
        parsed.startedAt == null ? null : getUtcDayFromUnixMs(parsed.startedAt);

      if (sessionUtcDay == null || !isCurrentUtcDay(sessionUtcDay)) {
        throw new Error("Only an Anky from the current UTC day can be sealed.");
      }

      const loom = await getSelectedLoom();

      if (loom == null) {
        throw new Error("loom is optional, but one is needed to seal a hash.");
      }

      const wallet = await walletState.getWallet();

      if (loom.owner != null && loom.owner !== wallet.publicKey) {
        throw new Error("The selected loom belongs to a different wallet.");
      }

      const config = await loadMobileSolanaConfig();
      const connection = new Connection(config.rpcUrl, "confirmed");
      const seal = await sealAnkyOnchain({
        connection,
        coreCollection: loom.collection,
        loomAsset: loom.asset,
        network: config.network,
        programId: config.sealProgramId,
        sessionHashHex: entryState.hash,
        sessionUtcDay,
        wallet,
      });

      await writeSealSidecar(seal);

      const api = getAnkyApiClient();
      if (api != null) {
        try {
          await api.recordMobileSeal({
            coreCollection: loom.collection,
            loomAsset: loom.asset,
            sessionHash: entryState.hash,
            signature: seal.signature,
            status: "confirmed",
            wallet: wallet.publicKey,
          });
        } catch (recordError) {
          console.warn("Sealed on chain, but backend seal record failed.", recordError);
        }
      }

      setEntry({ ...entryState, seal: toLoomSeal(seal) });
      setActionState("idle");
    } catch (error) {
      console.error(error);
      const nextMessage =
        error instanceof Error ? error.message : "Seal failed. Your .anky is still local.";
      setSealError(nextMessage);
      setStatusMessage("");
      setActionState("error");
    }
  }

  function openThread(entryState: EntryState) {
    if (!isCompleteParsedAnky(parseAnky(entryState.raw))) {
      setStatusMessage("fragments can be copied, but not sent to anky.");
      return;
    }

    navigation.navigate("Thread", {
      mode:
        entryState.thread?.mode ??
        getThreadModeForRawAnky(entryState.raw, entryState.reflection != null),
      sessionHash: entryState.hash,
      source: "entry",
    });
  }

  function goBack() {
    if (navigation.canGoBack()) {
      navigation.goBack();
      return;
    }

    navigation.navigate("Past");
  }

  if (entry == null) {
    return (
      <ScreenBackground variant="plain">
        <View style={styles.loadingWrap}>
          <GoldenThreadSpinner label="opening" />
        </View>
      </ScreenBackground>
    );
  }

  const parsed = parseAnky(entry.raw);
  const isFragmentEntry = !isCompleteParsedAnky(parsed);
  const sessionUtcDay =
    parsed.startedAt == null ? null : getUtcDayFromUnixMs(parsed.startedAt);
  const canSealCurrentUtcDay = sessionUtcDay != null && isCurrentUtcDay(sessionUtcDay);
  const dateParts = formatEntryDateParts(entry.raw);
  const durationLabel = formatDuration(getRiteDurationMs(parsed));
  const canReflect =
    !isFragmentEntry &&
    entry.valid &&
    entry.hashMatches &&
    entry.reflection == null &&
    actionState !== "reflecting";
  const canTalkToAnky = !isFragmentEntry && entry.reflection != null;
  const canSeal =
    !isFragmentEntry &&
    entry.valid &&
    entry.hashMatches &&
    entry.seal == null &&
    walletState.hasWallet &&
    canSealCurrentUtcDay &&
    actionState !== "sealing";
  const shouldShowSeal =
    !isFragmentEntry &&
    entry.valid &&
    entry.hashMatches &&
    (entry.seal != null || walletState.hasWallet);

  return (
    <ScreenBackground variant="plain">
      <AnkySessionSurface
        canContinue={canTalkToAnky}
        canCopy={entry.text.length > 0}
        canFullReflect={canReflect}
        canSimpleReflect={canReflect}
        dateLabel={dateParts.date}
        durationLabel={durationLabel}
        errorText={!entry.valid || !entry.hashMatches ? "this writing needs attention before reflection." : undefined}
        isComplete={!isFragmentEntry}
        isProcessing={actionState === "reflecting"}
        message={statusMessage}
        onBack={goBack}
        onContinue={() => openThread(entry)}
        onCopy={() => void handleCopyText(entry.text)}
        onFullReflect={() => void handleReflect(entry, "full")}
        onSimpleReflect={() => void handleReflect(entry, "simple")}
        reflection={entry.reflection}
        sealAction={
          shouldShowSeal ? (
            <SwipeToSealAction
              disabled={!canSeal}
              error={sealError}
              isSealing={actionState === "sealing"}
              onSeal={() => handleSeal(entry)}
              sealNetwork={entry.seal?.network}
              sealSignature={entry.seal?.txSignature}
              sealed={entry.seal != null}
              walletKind={walletState.walletKind}
            />
          ) : undefined
        }
        text={entry.text}
        timeLabel={dateParts.time}
      />
    </ScreenBackground>
  );
}

function toLoomSeal(seal: {
  created_at: string;
  loom_asset: string;
  network: LoomSeal["network"];
  session_hash: string;
  signature: string;
  writer: string;
}): LoomSeal {
  return {
    createdAt: seal.created_at,
    loomId: seal.loom_asset,
    network: seal.network,
    sessionHash: seal.session_hash,
    txSignature: seal.signature,
    writer: seal.writer,
  };
}

function formatEntryDateParts(raw: string): { date: string; time: string } {
  const parsed = parseAnky(raw);

  if (parsed.startedAt == null) {
    return {
      date: "date unknown",
      time: "time unknown",
    };
  }

  const date = new Date(parsed.startedAt);

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

function EntryBackgroundTexture() {
  return (
    <View pointerEvents="none" style={styles.backgroundTexture}>
      <View style={[styles.backgroundLine, { top: 86, width: "62%" }]} />
      <View style={[styles.backgroundLine, { top: 172, width: "78%" }]} />
      <View style={[styles.backgroundLine, { top: 258, width: "54%" }]} />
      <View style={[styles.backgroundLine, { bottom: 120, width: "68%" }]} />
    </View>
  );
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
  backgroundLine: {
    alignSelf: "center",
    backgroundColor: "rgba(215, 186, 115, 0.045)",
    height: StyleSheet.hairlineWidth,
    position: "absolute",
  },
  backgroundTexture: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "#090A12",
  },
  content: {
    padding: spacing.xl,
    paddingBottom: 54,
  },
  date: {
    color: ankyColors.text,
    fontSize: 31,
    fontWeight: "700",
    letterSpacing: 0,
    textAlign: "center",
  },
  dateBlock: {
    alignItems: "center",
    marginTop: spacing.lg,
  },
  duration: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    marginTop: 6,
    textAlign: "center",
    textTransform: "lowercase",
  },
  loading: {
    color: ankyColors.textMuted,
    fontSize: 16,
    textTransform: "lowercase",
  },
  loadingWrap: {
    alignItems: "center",
    flex: 1,
    justifyContent: "center",
  },
  message: {
    color: ankyColors.gold,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
    textAlign: "center",
    textTransform: "lowercase",
  },
  sealWrap: {
    marginTop: spacing.md,
  },
  time: {
    color: ankyColors.gold,
    fontSize: fontSize.md,
    letterSpacing: 0,
    marginTop: spacing.sm,
    textAlign: "center",
  },
});
