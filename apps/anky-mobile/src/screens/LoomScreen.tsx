import { useEffect, useMemo, useState } from "react";
import {
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  useWindowDimensions,
  View,
} from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import { Connection } from "@solana/web3.js";

import type { RootStackParamList } from "../../App";
import { useAuthModal } from "../auth/AuthModalContext";
import { GlassCard } from "../components/anky/GlassCard";
import { RitualButton } from "../components/anky/RitualButton";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { ChakanaLoom, getLoomCompletion } from "../components/sojourn/ChakanaLoom";
import { KingdomBadge } from "../components/sojourn/KingdomBadge";
import { AnkyApiError } from "../lib/api/ankyApi";
import { getAnkyApiClient } from "../lib/api/client";
import type {
  MobileLoomMint,
  MobileSealPointsHistory,
  MobileSealScoreResponse,
} from "../lib/api/types";
import { hasConfiguredBackend } from "../lib/auth/backendSession";
import { listAnkySessionSummaries } from "../lib/ankySessionIndex";
import { listLocalLoomSeals, listSavedAnkyFiles, type SavedAnkyFile } from "../lib/ankyStorage";
import { useAnkyPrivyWallet } from "../lib/privy/useAnkyPrivyWallet";
import {
  buildSojournDays,
  getCurrentSojournDay,
  getNextSessionKindForToday,
  SOJOURN_LENGTH_DAYS,
} from "../lib/sojourn";
import type { AnkySessionSummary, DayState } from "../lib/sojourn";
import {
  clearSelectedLoom,
  createDevnetLoomRecord,
  getSelectedLoom,
  saveSelectedLoom,
  shortAddress,
} from "../lib/solana/loomStorage";
import type { SelectedLoom } from "../lib/solana/loomStorage";
import type { LoomSeal } from "../lib/solana/types";
import {
  mintAndSaveLoom,
  restoreRecordedLoomSelection,
  retrySelectedLoomRecord,
  toSelectedLoom,
} from "../lib/solana/mobileLoomMint";
import type { MintAndSaveLoomStatus } from "../lib/solana/mobileLoomMint";
import { loadMobileSolanaConfig } from "../lib/solana/mobileSolanaConfig";
import type { MobileSolanaRuntimeConfig } from "../lib/solana/mobileSolanaConfig";
import { useAnkyPresenceScreen } from "../presence/useAnkyPresenceScreen";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Loom">;
type MintState = "error" | "idle" | "loading" | "restoring" | "retrying" | MintAndSaveLoomStatus;

export function LoomScreen({ navigation }: Props) {
  const { width } = useWindowDimensions();
  const { openAuthModal } = useAuthModal();
  const walletState = useAnkyPrivyWallet();
  const [inviteCode, setInviteCode] = useState("");
  const [manualAsset, setManualAsset] = useState("");
  const [message, setMessage] = useState("");
  const [mintState, setMintState] = useState<MintState>("idle");
  const [recordedLooms, setRecordedLooms] = useState<MobileLoomMint[]>([]);
  const [runtimeConfig, setRuntimeConfig] = useState<MobileSolanaRuntimeConfig | null>(null);
  const [selectedLoom, setSelectedLoom] = useState<SelectedLoom | null>(null);
  const [sealPoints, setSealPoints] = useState<MobileSealPointsHistory | null>(null);
  const [sealScore, setSealScore] = useState<MobileSealScoreResponse | null>(null);
  const [sealScoreState, setSealScoreState] = useState<"idle" | "loading" | "unavailable">("idle");
  const [seals, setSeals] = useState<LoomSeal[]>([]);
  const [files, setFiles] = useState<SavedAnkyFile[]>([]);
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);
  const [now, setNow] = useState(() => new Date());
  const days = useMemo(() => buildSojournDays(sessions, now), [sessions, now]);
  const currentDay = getCurrentSojournDay(now);
  const today = days[currentDay - 1];
  const sealedCount = days.filter((day) => day.dailySeal != null).length;
  const extraThreadCount = sessions.filter((session) => session.kind === "extra_thread").length;
  const completion = Math.round(getLoomCompletion(days) * 100);
  const nextKind = getNextSessionKindForToday(sessions, now);
  const loomSize = Math.min(330, Math.max(272, width - spacing.xl * 2));
  const backendConfigured = hasConfiguredBackend();
  const fileByHash = useMemo(() => new Map(files.map((file) => [file.hash, file])), [files]);
  const sealedSessions = useMemo(
    () => {
      const loomSealHashes =
        selectedLoom == null
          ? new Set<string>()
          : new Set(
              seals
                .filter((seal) => seal.loomId === selectedLoom.asset)
                .map((seal) => seal.sessionHash),
            );

      return sessions
        .filter((session) =>
          session.sessionHash == null
            ? false
            : loomSealHashes.size > 0
              ? loomSealHashes.has(session.sessionHash)
              : session.sealedOnchain === true,
        )
        .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt));
    },
    [seals, selectedLoom, sessions],
  );
  const mintBusy = isBusyMintState(mintState);
  const walletHasMintedLoom =
    recordedLooms.some((loom) => loom.status === "confirmed" || loom.status === "finalized") ||
    (selectedLoom?.signature != null &&
      (selectedLoom.owner == null || selectedLoom.owner === walletState.publicKey));

  useAnkyPresenceScreen({
    emotion: "idle",
    preferredMode: "sigil",
    sequence: "seated",
  });

  useEffect(() => {
    let mounted = true;

    async function load() {
      try {
        setNow(new Date());
        const [nextSessions, nextFiles, nextSeals, nextConfig, localSelectedLoom] = await Promise.all([
          listAnkySessionSummaries(),
          listSavedAnkyFiles(),
          listLocalLoomSeals(),
          loadMobileSolanaConfig(),
          getSelectedLoom(),
        ]);

        if (mounted) {
          setSessions(nextSessions);
          setFiles(nextFiles);
          setSeals(nextSeals);
          setRuntimeConfig(nextConfig);
          setSelectedLoom(localSelectedLoom);
        }

        if (walletState.publicKey != null && backendConfigured) {
          await Promise.all([
            restoreWalletLooms(walletState.publicKey, mounted),
            restoreSealPoints(walletState.publicKey, mounted),
          ]);
        } else if (mounted) {
          setSealPoints(null);
          setSealScore(null);
          setSealScoreState(walletState.publicKey != null ? "unavailable" : "idle");
        }
      } catch (error) {
        console.error(error);
      }
    }

    void load();
    const unsubscribe = navigation.addListener("focus", () => {
      void load();
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [backendConfigured, navigation, walletState.publicKey]);

  async function restoreSealPoints(wallet: string, mounted = true) {
    const api = getAnkyApiClient();

    if (api == null) {
      if (mounted) {
        setSealPoints(null);
        setSealScore(null);
        setSealScoreState("unavailable");
      }
      return;
    }

    try {
      if (mounted) {
        setSealScoreState("loading");
      }
      const points = await api.lookupMobileSealPoints(wallet);

      if (!mounted) {
        return;
      }

      setSealPoints(points);
      setSealScore(null);
      setSealScoreState("idle");
    } catch (error) {
      console.warn("Could not restore mobile seal points.", error);
      if (mounted) {
        setSealPoints(null);
        setSealScore(null);
        setSealScoreState("unavailable");
      }
    }
  }

  function openDay(day: DayState) {
    navigation.navigate("DayChamber", { day: day.day });
  }

  function writeToday() {
    navigation.navigate("ActiveWriting", {
      dayNumber: today.day,
      isoDate: today.dateUtc.slice(0, 10),
      sessionKind: nextKind,
      sojourn: 9,
    });
  }

  async function restoreWalletLooms(wallet: string, mounted = true) {
    const api = getAnkyApiClient();

    if (api == null) {
      return;
    }

    try {
      setMintState((state) => (state === "idle" ? "restoring" : state));
      const restored = await restoreRecordedLoomSelection({ api, wallet });

      if (!mounted) {
        return;
      }

      setRecordedLooms(restored.looms);
      setSelectedLoom(restored.selectedLoom);
      if (restored.selectedLoom != null) {
        setMessage("loom restored from the backend.");
      }
    } catch (error) {
      console.warn("Could not restore recorded looms.", error);
    } finally {
      if (mounted) {
        setMintState((state) => (state === "restoring" ? "idle" : state));
      }
    }
  }

  async function handleWalletAction() {
    if (!walletState.authenticated) {
      openAuthModal({
        reason: "login or connect a wallet only if you want to mint a loom.",
      });
      return;
    }

    if (walletState.hasWallet) {
      return;
    }

    try {
      setMintState("loading");
      setMessage("");
      await walletState.createWallet();
      setMessage("embedded solana wallet ready.");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "could not create a solana wallet.");
      setMintState("error");
    } finally {
      setMintState((state) => (state === "loading" ? "idle" : state));
    }
  }

  async function handleMintLoom() {
    if (mintBusy) {
      return;
    }

    const api = getAnkyApiClient();

    if (api == null) {
      setMessage("backend url is not configured. loom minting is unavailable.");
      setMintState("error");
      return;
    }

    if (!walletState.hasWallet) {
      await handleWalletAction();
      return;
    }

    if (walletHasMintedLoom) {
      setMessage("this wallet already has a loom. select the recorded loom instead.");
      return;
    }

    try {
      setMessage("");
      setMintState("loading");
      const config = runtimeConfig ?? (await loadMobileSolanaConfig());
      const wallet = await walletState.getWallet();
      const connection = new Connection(config.rpcUrl, "confirmed");
      const result = await mintAndSaveLoom({
        api,
        config,
        connection,
        inviteCode,
        onStatus: setMintState,
        wallet,
      });

      setRuntimeConfig(config);
      setSelectedLoom(result.selectedLoom);
      await restoreWalletLooms(wallet.publicKey);

      if (result.recordStatus === "pending_record") {
        setMessage(
          `mint confirmed on ${config.network}. backend recording failed, so the loom was saved locally as pending.`,
        );
      } else {
        setMessage("loom minted, confirmed, recorded, and selected.");
      }
      setMintState("idle");
    } catch (error) {
      console.error(error);
      setMessage(formatMintError(error));
      setMintState("error");
    }
  }

  async function handleRetryRecord() {
    if (selectedLoom == null || mintBusy) {
      return;
    }

    const api = getAnkyApiClient();

    if (api == null) {
      setMessage("backend url is not configured. recording cannot be retried.");
      setMintState("error");
      return;
    }

    try {
      setMessage("");
      setMintState("retrying");
      const recorded = await retrySelectedLoomRecord({
        api,
        loom: selectedLoom,
        wallet: walletState.publicKey,
      });

      setSelectedLoom(recorded);
      setMessage("loom record synced with the backend.");
      if (walletState.publicKey != null) {
        await restoreWalletLooms(walletState.publicKey);
      }
      setMintState("idle");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "could not record this loom.");
      setMintState("error");
    }
  }

  async function handleSelectRecordedLoom(loom: MobileLoomMint) {
    try {
      const selected = toSelectedLoom(loom);

      await saveSelectedLoom(selected);
      setSelectedLoom(selected);
      setMessage("loom selected.");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "could not select this loom.");
      setMintState("error");
    }
  }

  async function handleSaveManualLoom() {
    try {
      const asset = manualAsset.trim();

      if (asset.length === 0) {
        setMessage("paste a core asset address first.");
        return;
      }

      const selected = createDevnetLoomRecord({
        asset,
        collection: runtimeConfig?.coreCollection,
        network: runtimeConfig?.network,
        owner: walletState.publicKey,
        recordStatus: "pending_record",
      });

      await saveSelectedLoom(selected);
      setSelectedLoom(selected);
      setMessage("saved pasted loom locally. this did not mint anything.");
      setManualAsset("");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "could not save pasted loom.");
      setMintState("error");
    }
  }

  async function handleClearSelectedLoom() {
    await clearSelectedLoom();
    setSelectedLoom(null);
    setMessage("loom selection cleared.");
  }

  return (
    <ScreenBackground variant="plain">
      <ScrollView contentContainerStyle={styles.content}>
        <Text style={styles.title}>loom</Text>
        <Text style={styles.sojourn}>
          a loom is the onchain place where your ankys can be sealed. your writing stays private; solana records hash seals, ownership, and verified receipts when they exist.
        </Text>

        <GlassCard style={styles.card}>
          <Text style={styles.label}>{runtimeConfig?.network ?? "solana"} loom</Text>
          <Text style={styles.cardTitle}>
            {selectedLoom == null ? "no loom yet" : selectedLoom.name}
          </Text>

          <Text style={styles.note}>
            {walletState.publicKey == null
              ? "no solana wallet connected."
              : `${walletState.walletLabel ?? "wallet"} ${shortAddress(walletState.publicKey, 6)}`}
          </Text>
          {!backendConfigured ? (
            <Text style={styles.errorText}>backend url is not configured. minting is unavailable.</Text>
          ) : null}

          {selectedLoom == null ? (
            <Text style={styles.note}>
              minting creates your loom on solana. writing does not require this.
            </Text>
          ) : (
            <View style={styles.loomDetails}>
              <Text selectable style={styles.addressLine}>
                asset {selectedLoom.asset}
              </Text>
              <Text style={styles.note}>collection {shortAddress(selectedLoom.collection, 6)}</Text>
              {selectedLoom.owner == null ? null : (
                <Text style={styles.note}>owner {shortAddress(selectedLoom.owner, 6)}</Text>
              )}
              {selectedLoom.signature == null ? null : (
                <Text selectable style={styles.note}>
                  tx {shortAddress(selectedLoom.signature, 6)}
                </Text>
              )}
              <Text
                style={[
                  styles.status,
                  selectedLoom.recordStatus === "pending_record" && styles.pendingStatus,
                ]}
              >
                {selectedLoom.recordStatus === "pending_record"
                  ? "backend record pending"
                  : "loom ready"}
              </Text>
            </View>
          )}

          {recordedLooms.length === 0 ? null : (
            <View style={styles.recordedList}>
              <Text style={styles.label}>recorded for wallet</Text>
              {recordedLooms.slice(0, 3).map((loom) => (
                <RitualButton
                  key={loom.id}
                  label={`select ${shortAddress(loom.loomAsset, 4)}`}
                  onPress={() => void handleSelectRecordedLoom(loom)}
                  style={styles.smallButton}
                  variant="secondary"
                />
              ))}
            </View>
          )}

          <View style={styles.buttonGroup}>
            {!walletState.authenticated || !walletState.hasWallet ? (
              <RitualButton
                disabled={mintBusy}
                label={
                  walletState.hasWallet
                    ? "continue with wallet"
                    : walletState.authenticated
                      ? "create embedded wallet"
                      : "login / connect wallet"
                }
                onPress={() => void handleWalletAction()}
              />
            ) : (
              <RitualButton
                disabled={!backendConfigured || mintBusy || walletHasMintedLoom}
                label={
                  walletHasMintedLoom
                    ? "loom already minted"
                    : mintBusy
                      ? mintStateLabel(mintState)
                      : "mint loom"
                }
                onPress={() => void handleMintLoom()}
              />
            )}
            {selectedLoom?.recordStatus === "pending_record" ? (
              <RitualButton
                disabled={!backendConfigured || mintBusy}
                label={mintState === "retrying" ? "recording" : "retry backend record"}
                onPress={() => void handleRetryRecord()}
                variant="secondary"
              />
            ) : null}
            {selectedLoom == null ? null : (
              <RitualButton
                disabled={mintBusy}
                label="clear selected loom"
                onPress={() => void handleClearSelectedLoom()}
                variant="ghost"
              />
            )}
          </View>

          {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}
        </GlassCard>

        {walletState.publicKey == null ? null : (
          <GlassCard style={styles.card}>
            <Text style={styles.label}>indexed score</Text>
            <View style={styles.metrics}>
              <Metric
                label="score"
                value={String(sealPoints?.score ?? sealScore?.score ?? 0)}
              />
              <Metric
                label="sealed"
                value={String(sealPoints?.uniqueSealDays ?? sealScore?.uniqueSealDays ?? 0)}
              />
              <Metric
                label="proof +2"
                value={String((sealPoints?.verifiedSealDays ?? sealScore?.verifiedSealDays ?? 0) * 2)}
              />
              <Metric
                label="bonus"
                value={String(sealPoints?.streakBonus ?? sealScore?.streakBonus ?? 0)}
              />
            </View>
            <Text style={styles.note}>
              {sealScoreState === "loading"
                ? "syncing finalized receipts"
                : !backendConfigured
                  ? "backend score unavailable"
                  : sealScoreState === "unavailable"
                    ? "score sync unavailable"
                    : sealPoints == null && sealScore == null
                      ? "no finalized score yet"
                      : `${sealPoints?.network ?? sealScore?.network} · finalized receipts only`}
            </Text>
            <Text style={styles.note}>seal hash = +1, prove rite = +2.</Text>
          </GlassCard>
        )}

        {walletState.publicKey == null ? null : (
          <GlassCard style={styles.card}>
            <Text style={styles.label}>points history</Text>
            {sealPoints == null || sealPoints.entries.length === 0 ? (
              <Text style={styles.note}>no finalized points indexed yet.</Text>
            ) : (
              <View style={styles.sealedList}>
                {sealPoints.entries.slice(0, 20).map((entry) => {
                  const file = fileByHash.get(entry.sessionHash);

                  return (
                    <Pressable
                      accessibilityRole="button"
                      disabled={file == null}
                      key={`${entry.sessionHash}:${entry.utcDay}`}
                      onPress={() => {
                        if (file != null) {
                          navigation.navigate("Entry", { fileName: file.fileName });
                        }
                      }}
                      style={({ pressed }) => [
                        styles.sealedRow,
                        file == null && styles.disabledRow,
                        pressed && file != null && styles.pressed,
                      ]}
                    >
                      <View style={styles.sealedDot} />
                      <View style={styles.sealedCopy}>
                        <Text style={styles.sealedTitle}>
                          {formatUtcDay(entry.utcDay)} · +{entry.totalPoints}
                        </Text>
                        <Text style={styles.sealedMeta}>
                          sealed +{entry.sealPoints}
                          {entry.proofPoints > 0 ? ` · proved +${entry.proofPoints}` : ""}
                          {` · ${formatProofStatus(entry.proofStatus)}`}
                        </Text>
                        {file == null ? (
                          <Text style={styles.sealedMeta}>not on this device</Text>
                        ) : null}
                      </View>
                      {file == null ? null : <Text style={styles.sealedChevron}>›</Text>}
                    </Pressable>
                  );
                })}
              </View>
            )}
          </GlassCard>
        )}

        {selectedLoom == null ? null : (
          <GlassCard style={styles.card}>
            <Text style={styles.label}>sealed ankys</Text>
            {sealedSessions.length === 0 ? (
              <Text style={styles.note}>no ankys sealed with this loom yet.</Text>
            ) : (
              <View style={styles.sealedList}>
                {sealedSessions.slice(0, 12).map((session) => {
                  const file = session.sessionHash == null ? undefined : fileByHash.get(session.sessionHash);

                  return (
                    <Pressable
                      accessibilityRole="button"
                      disabled={file == null}
                      key={session.id}
                      onPress={() => {
                        if (file != null) {
                          navigation.navigate("Entry", { fileName: file.fileName });
                        }
                      }}
                      style={({ pressed }) => [
                        styles.sealedRow,
                        file == null && styles.disabledRow,
                        pressed && file != null && styles.pressed,
                      ]}
                    >
                      <View style={styles.sealedDot} />
                      <View style={styles.sealedCopy}>
                        <Text style={styles.sealedTitle}>
                          {session.kind === "fragment" ? "fragment" : "anky"}
                        </Text>
                        <Text style={styles.sealedMeta}>{formatLoomDate(session.createdAt)}</Text>
                      </View>
                      <Text style={styles.sealedChevron}>›</Text>
                    </Pressable>
                  );
                })}
              </View>
            )}
          </GlassCard>
        )}

        <View style={styles.actions}>
          <RitualButton label="return to map" onPress={() => navigation.navigate("Track")} />
        </View>
      </ScrollView>
    </ScreenBackground>
  );
}

function isBusyMintState(state: MintState): boolean {
  return [
    "authorizing",
    "confirming",
    "loading",
    "preparing",
    "recording",
    "restoring",
    "retrying",
    "signing",
  ].includes(state);
}

function mintStateLabel(state: MintState): string {
  switch (state) {
    case "authorizing":
      return "authorizing";
    case "preparing":
      return "preparing mint";
    case "signing":
      return "sign in wallet";
    case "confirming":
      return "confirming";
    case "recording":
      return "recording";
    case "restoring":
      return "restoring";
    case "retrying":
      return "recording";
    case "loading":
      return "loading";
    case "error":
    case "idle":
      return "mint loom";
  }
}

function formatMintError(error: unknown): string {
  if (
    error instanceof AnkyApiError &&
    error.status === 503 &&
    error.path.includes("/api/mobile/looms/prepare-mint")
  ) {
    return "backend mint preparation is not configured. no loom was minted.";
  }

  if (error instanceof Error) {
    if (/reject|cancel/i.test(error.message)) {
      return "wallet signing was rejected. no loom was saved.";
    }

    return error.message;
  }

  return "loom mint failed. no loom was saved.";
}

function formatLoomDate(value: string): string {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return "recent";
  }

  return date.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
  }).toLowerCase();
}

function formatUtcDay(utcDay: number): string {
  const date = new Date(utcDay * 86_400_000);

  if (Number.isNaN(date.getTime())) {
    return `day ${utcDay}`;
  }

  return date.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
  }).toLowerCase();
}

function formatProofStatus(status: string): string {
  switch (status) {
    case "finalized":
      return "verified";
    case "backfill_required":
    case "confirmed":
    case "pending":
    case "processed":
    case "syncing":
      return "verified on-chain · syncing";
    case "failed":
      return "proof failed";
    case "queued":
    case "proving":
      return "proving";
    case "unavailable":
      return "proof unavailable";
    default:
      return "seal only";
  }
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <View style={styles.metric}>
      <Text style={styles.metricValue}>{value}</Text>
      <Text style={styles.metricLabel}>{label}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  actions: {
    gap: spacing.sm,
    marginTop: spacing.xl,
  },
  addressLine: {
    color: ankyColors.text,
    fontSize: 12,
    lineHeight: 18,
    marginTop: spacing.sm,
  },
  buttonGroup: {
    gap: spacing.sm,
    marginTop: spacing.md,
  },
  card: {
    marginTop: spacing.xl,
  },
  cardTitle: {
    color: ankyColors.text,
    fontSize: fontSize.lg,
    fontWeight: "700",
    marginTop: spacing.sm,
  },
  content: {
    padding: spacing.xl,
    paddingBottom: 44,
  },
  devFallback: {
    borderColor: ankyColors.border,
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.lg,
    padding: spacing.md,
  },
  disabledRow: {
    opacity: 0.48,
  },
  errorText: {
    color: ankyColors.danger,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.sm,
  },
  input: {
    backgroundColor: ankyColors.bg3,
    borderColor: ankyColors.border,
    borderRadius: 8,
    borderWidth: 1,
    color: ankyColors.text,
    fontSize: fontSize.md,
    marginTop: spacing.md,
    paddingHorizontal: spacing.md,
    paddingVertical: 12,
  },
  label: {
    color: ankyColors.gold,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
  loomDetails: {
    marginTop: spacing.md,
  },
  message: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
  },
  meta: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
    lineHeight: 24,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  metric: {
    alignItems: "center",
    borderColor: ankyColors.border,
    borderRadius: 8,
    borderWidth: 1,
    flex: 1,
    paddingHorizontal: spacing.sm,
    paddingVertical: spacing.md,
  },
  metricLabel: {
    color: ankyColors.textMuted,
    fontSize: 11,
    marginTop: 4,
    textAlign: "center",
    textTransform: "lowercase",
  },
  metricValue: {
    color: ankyColors.text,
    fontSize: fontSize.lg,
    fontWeight: "700",
  },
  metrics: {
    flexDirection: "row",
    gap: spacing.sm,
    marginBottom: spacing.lg,
    marginTop: spacing.lg,
  },
  note: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.sm,
  },
  pendingStatus: {
    color: ankyColors.gold,
  },
  recordedList: {
    gap: spacing.sm,
    marginTop: spacing.lg,
  },
  pressed: {
    opacity: 0.72,
  },
  sealedChevron: {
    color: ankyColors.gold,
    fontSize: 24,
    marginLeft: spacing.sm,
  },
  sealedCopy: {
    flex: 1,
    minWidth: 0,
  },
  sealedDot: {
    backgroundColor: ankyColors.gold,
    borderRadius: 4,
    height: 8,
    marginRight: spacing.sm,
    opacity: 0.78,
    width: 8,
  },
  sealedList: {
    gap: spacing.sm,
    marginTop: spacing.md,
  },
  sealedMeta: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    marginTop: 2,
    textTransform: "lowercase",
  },
  sealedRow: {
    alignItems: "center",
    backgroundColor: "rgba(255,255,255,0.035)",
    borderColor: ankyColors.border,
    borderRadius: 8,
    borderWidth: 1,
    flexDirection: "row",
    minHeight: 54,
    paddingHorizontal: spacing.md,
  },
  sealedTitle: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  smallButton: {
    marginTop: spacing.sm,
  },
  sojourn: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    marginBottom: spacing.lg,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  summary: {
    marginTop: spacing.lg,
  },
  status: {
    color: ankyColors.success,
    fontSize: fontSize.sm,
    fontWeight: "700",
    marginTop: spacing.sm,
  },
  title: {
    color: ankyColors.gold,
    fontSize: fontSize.xl,
    fontWeight: "700",
    letterSpacing: 0,
    textAlign: "center",
    textTransform: "lowercase",
  },
});
