import { useEffect, useMemo, useState } from "react";
import {
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
import { GlassCard } from "../components/anky/GlassCard";
import { RitualButton } from "../components/anky/RitualButton";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { ChakanaLoom, getLoomCompletion } from "../components/sojourn/ChakanaLoom";
import { KingdomBadge } from "../components/sojourn/KingdomBadge";
import { AnkyApiError } from "../lib/api/ankyApi";
import { getAnkyApiClient } from "../lib/api/client";
import type { MobileLoomMint } from "../lib/api/types";
import { hasConfiguredBackend } from "../lib/auth/backendSession";
import { listAnkySessionSummaries } from "../lib/ankySessionIndex";
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
import {
  mintAndSaveLoom,
  restoreRecordedLoomSelection,
  retrySelectedLoomRecord,
  toSelectedLoom,
} from "../lib/solana/mobileLoomMint";
import type { MintAndSaveLoomStatus } from "../lib/solana/mobileLoomMint";
import { loadMobileSolanaConfig } from "../lib/solana/mobileSolanaConfig";
import type { MobileSolanaRuntimeConfig } from "../lib/solana/mobileSolanaConfig";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Loom">;
type MintState = "error" | "idle" | "loading" | "restoring" | "retrying" | MintAndSaveLoomStatus;

export function LoomScreen({ navigation }: Props) {
  const { width } = useWindowDimensions();
  const walletState = useAnkyPrivyWallet();
  const [inviteCode, setInviteCode] = useState("");
  const [manualAsset, setManualAsset] = useState("");
  const [message, setMessage] = useState("");
  const [mintState, setMintState] = useState<MintState>("idle");
  const [recordedLooms, setRecordedLooms] = useState<MobileLoomMint[]>([]);
  const [runtimeConfig, setRuntimeConfig] = useState<MobileSolanaRuntimeConfig | null>(null);
  const [selectedLoom, setSelectedLoom] = useState<SelectedLoom | null>(null);
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
  const mintBusy = isBusyMintState(mintState);
  const walletHasMintedLoom =
    recordedLooms.some((loom) => loom.status === "confirmed" || loom.status === "finalized") ||
    (selectedLoom?.signature != null &&
      (selectedLoom.owner == null || selectedLoom.owner === walletState.publicKey));

  useEffect(() => {
    let mounted = true;

    async function load() {
      try {
        setNow(new Date());
        const [nextSessions, nextConfig, localSelectedLoom] = await Promise.all([
          listAnkySessionSummaries(),
          loadMobileSolanaConfig(),
          getSelectedLoom(),
        ]);

        if (mounted) {
          setSessions(nextSessions);
          setRuntimeConfig(nextConfig);
          setSelectedLoom(localSelectedLoom);
        }

        if (walletState.publicKey != null && backendConfigured) {
          await restoreWalletLooms(walletState.publicKey, mounted);
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
        setMessage("Loom selection restored from the backend.");
      }
    } catch (error) {
      console.warn("Could not restore recorded Looms.", error);
    } finally {
      if (mounted) {
        setMintState((state) => (state === "restoring" ? "idle" : state));
      }
    }
  }

  async function handleWalletAction() {
    if (!walletState.authenticated) {
      navigation.navigate("Auth");
      return;
    }

    if (walletState.hasWallet) {
      return;
    }

    try {
      setMintState("loading");
      setMessage("");
      await walletState.createWallet();
      setMessage("Embedded Solana wallet ready.");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "Could not create a Solana wallet.");
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
      setMessage("Backend URL is not configured. Loom minting is unavailable.");
      setMintState("error");
      return;
    }

    if (!walletState.hasWallet) {
      await handleWalletAction();
      return;
    }

    if (walletHasMintedLoom) {
      setMessage("This wallet already has a Loom. Select the recorded Loom instead.");
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
          `Mint confirmed on ${config.network}. Backend recording failed, so the Loom was saved locally as pending.`,
        );
      } else {
        setMessage("Loom minted, confirmed, recorded, and selected.");
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
      setMessage("Backend URL is not configured. Recording cannot be retried.");
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
      setMessage("Loom record synced with the backend.");
      if (walletState.publicKey != null) {
        await restoreWalletLooms(walletState.publicKey);
      }
      setMintState("idle");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "Could not record this Loom.");
      setMintState("error");
    }
  }

  async function handleSelectRecordedLoom(loom: MobileLoomMint) {
    try {
      const selected = toSelectedLoom(loom);

      await saveSelectedLoom(selected);
      setSelectedLoom(selected);
      setMessage("Loom selected.");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "Could not select this Loom.");
      setMintState("error");
    }
  }

  async function handleSaveManualLoom() {
    try {
      const asset = manualAsset.trim();

      if (asset.length === 0) {
        setMessage("Paste a Core asset address first.");
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
      setMessage("Saved pasted Loom locally. This did not mint anything.");
      setManualAsset("");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "Could not save pasted Loom.");
      setMintState("error");
    }
  }

  async function handleClearSelectedLoom() {
    await clearSelectedLoom();
    setSelectedLoom(null);
    setMessage("Loom selection cleared.");
  }

  return (
    <ScreenBackground variant="plain">
      <ScrollView contentContainerStyle={styles.content}>
        <ChakanaLoom days={days} onPressDay={openDay} size={loomSize} />

        <View style={styles.summary}>
          <Text style={styles.title}>your loom</Text>
          <Text style={styles.sojourn}>Sojourn 9</Text>
          <KingdomBadge kingdom={today.kingdom} />

          <View style={styles.metrics}>
            <Metric label="sealed" value={`${sealedCount}`} />
            <Metric label="extra writing" value={`${extraThreadCount}`} />
            <Metric label="complete" value={`${completion}%`} />
          </View>

          <Text style={styles.meta}>Kingdom {today.kingdom.index}: {today.kingdom.name}</Text>
          <Text style={styles.meta}>day {currentDay} of {SOJOURN_LENGTH_DAYS}</Text>
        </View>

        <GlassCard style={styles.card}>
          <Text style={styles.label}>{runtimeConfig?.network ?? "Solana"} Loom</Text>
          <Text style={styles.cardTitle}>
            {selectedLoom == null ? "mint or select your Loom" : "selected Loom"}
          </Text>

          <Text style={styles.note}>
            {walletState.publicKey == null
              ? "No Solana wallet connected."
              : `${walletState.walletLabel ?? "wallet"} ${shortAddress(walletState.publicKey, 6)}`}
          </Text>
          <Text style={styles.note}>
            config {runtimeConfig?.source ?? "loading"} · collection{" "}
            {runtimeConfig == null ? "loading" : shortAddress(runtimeConfig.coreCollection, 6)}
          </Text>
          {!backendConfigured ? (
            <Text style={styles.errorText}>Backend URL is not configured. Minting is unavailable.</Text>
          ) : null}

          {selectedLoom == null ? (
            <Text style={styles.note}>No Loom selected on this device.</Text>
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
                  : "backend record confirmed"}
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

          <TextInput
            autoCapitalize="characters"
            autoCorrect={false}
            editable={!mintBusy}
            onChangeText={setInviteCode}
            placeholder="invite code optional"
            placeholderTextColor={ankyColors.textMuted}
            style={styles.input}
            value={inviteCode}
          />

          <View style={styles.buttonGroup}>
            {!walletState.authenticated || !walletState.hasWallet ? (
              <RitualButton
                disabled={mintBusy}
                label={walletState.authenticated ? "create embedded wallet" : "connect wallet"}
                onPress={() => void handleWalletAction()}
              />
            ) : (
              <RitualButton
                disabled={!backendConfigured || mintBusy || walletHasMintedLoom}
                label={
                  walletHasMintedLoom
                    ? "Loom already minted"
                    : mintBusy
                      ? mintStateLabel(mintState)
                      : "mint Loom"
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
                label="clear selected Loom"
                onPress={() => void handleClearSelectedLoom()}
                variant="ghost"
              />
            )}
          </View>

          <View style={styles.devFallback}>
            <Text style={styles.label}>developer fallback</Text>
            <TextInput
              autoCapitalize="none"
              autoCorrect={false}
              onChangeText={setManualAsset}
              placeholder="paste Core asset"
              placeholderTextColor={ankyColors.textMuted}
              style={styles.input}
              value={manualAsset}
            />
            <RitualButton
              label="save pasted Loom"
              onPress={() => void handleSaveManualLoom()}
              style={styles.smallButton}
              variant="secondary"
            />
          </View>

          {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}
        </GlassCard>

        <View style={styles.actions}>
          <RitualButton label="return to map" onPress={() => navigation.navigate("Track")} />
          <RitualButton
            label={today.status === "today_sealed" ? "write again" : "write 8 minutes"}
            onPress={writeToday}
            variant="secondary"
          />
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
      return "mint Loom";
  }
}

function formatMintError(error: unknown): string {
  if (
    error instanceof AnkyApiError &&
    error.status === 503 &&
    error.path.includes("/api/mobile/looms/prepare-mint")
  ) {
    return "Backend mint preparation is not configured. No Loom was minted.";
  }

  if (error instanceof Error) {
    if (/reject|cancel/i.test(error.message)) {
      return "Wallet signing was rejected. No Loom was saved.";
    }

    return error.message;
  }

  return "Loom mint failed. No Loom was saved.";
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
