import { ReactNode, useEffect, useMemo, useState } from "react";
import {
  Alert,
  Image,
  ImageBackground,
  ImageSourcePropType,
  Linking,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  View,
} from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import * as Clipboard from "expo-clipboard";
import { usePrivy } from "@privy-io/expo";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import type { RootStackParamList } from "../../../App";
import { ScreenBackground } from "../../components/anky/ScreenBackground";
import { SubtleIconButton } from "../../components/navigation/SubtleIconButton";
import { listAnkySessionSummaries } from "../../lib/ankySessionIndex";
import {
  deleteAllLocalAnkyData,
  listLocalLoomSeals,
  listSavedAnkyFiles,
  readProcessingReceipt,
  readReflectionSidecar,
  type ProcessingReceiptSidecar,
  type SavedAnkyFile,
} from "../../lib/ankyStorage";
import { reconstructText } from "../../lib/ankyProtocol";
import {
  clearBackendAuthSession,
  getStoredBackendAuthSession,
  hasConfiguredBackend,
  type BackendAuthSession,
} from "../../lib/auth/backendSession";
import { useExternalSolanaWallet } from "../../lib/privy/ExternalSolanaWalletProvider";
import { getReflectionCreditBalance } from "../../lib/credits/processAnky";
import { useAnkyPrivyWallet } from "../../lib/privy/useAnkyPrivyWallet";
import {
  buildSojournDays,
  getCurrentSojournDay,
  getNextSessionKindForToday,
  SOJOURN_LENGTH_DAYS,
  type AnkySessionSummary,
} from "../../lib/sojourn";
import { getSelectedLoom, shortAddress, type SelectedLoom } from "../../lib/solana/loomStorage";
import {
  hasThreadProcessingConsent,
  markThreadProcessingConsent,
  resetThreadProcessingConsent,
} from "../../lib/thread/threadConsent";
import { listThreads } from "../../lib/thread/threadStorage";
import {
  isCompleteRawAnky,
} from "../../lib/thread/threadLogic";

type AccountProps = NativeStackScreenProps<RootStackParamList, "Account">;
type PrivacyProps = NativeStackScreenProps<RootStackParamList, "Privacy">;
type ExportDataProps = NativeStackScreenProps<RootStackParamList, "ExportData">;
type CreditsInfoProps = NativeStackScreenProps<RootStackParamList, "CreditsInfo">;
type LoomInfoProps = NativeStackScreenProps<RootStackParamList, "LoomInfo">;
type IconName = "account" | "credits" | "exportData" | "loom" | "privacy";
type RowVariant = "danger" | "highlight" | "normal";
type ActionVariant = "danger" | "primary" | "secondary";

const assets = {
  avatar: require("../../../assets/anky-you/avatar-anky.png"),
  background: require("../../../assets/anky-you/bg-cosmos.png"),
  icons: {
    account: require("../../../assets/anky-you/icons/account.png"),
    credits: require("../../../assets/anky-you/icons/credits.png"),
    exportData: require("../../../assets/anky-you/icons/export.png"),
    loom: require("../../../assets/anky-you/icons/loom.png"),
    privacy: require("../../../assets/anky-you/icons/privacy.png"),
  } satisfies Record<IconName, ImageSourcePropType>,
};

const GOLD = "#E9BE72";
const GOLD_BRIGHT = "#F2D392";
const COPY = "#D8C9D4";
const COPY_DIM = "rgba(216, 201, 212, 0.72)";
const DANGER = "#F19A72";
const PANEL = "rgba(13, 12, 27, 0.76)";
const PANEL_DEEP = "rgba(9, 8, 20, 0.88)";
const SERIF = Platform.select({ android: "serif", default: "Georgia", ios: "Georgia" });
const PRIVACY_POLICY_URL = "https://www.anky.app/privacy-policy.md";

export function AccountScreen({ navigation }: AccountProps) {
  const { logout, user } = usePrivy();
  const externalWallet = useExternalSolanaWallet();
  const wallet = useAnkyPrivyWallet();
  const [backendSession, setBackendSession] = useState<BackendAuthSession | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    let mounted = true;

    async function load() {
      const session = await getStoredBackendAuthSession();

      if (mounted) {
        setBackendSession(session);
      }
    }

    void load().catch(console.error);
    const unsubscribe = navigation.addListener("focus", () => {
      void load().catch(console.error);
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  const email = backendSession?.email ?? getPrivyEmail(user) ?? null;
  const connected = user != null || backendSession != null || wallet.authenticated || wallet.hasWallet;
  const walletLabel =
    wallet.publicKey == null
      ? "optional"
      : `${wallet.walletLabel ?? "wallet"} ${shortAddress(wallet.publicKey, 6)}`;
  const identityTitle = wallet.hasWallet ? "wallet connected" : connected ? "signed in" : "local account";

  async function disconnectAccount() {
    try {
      if (externalWallet.activeProvider != null) {
        await externalWallet.disconnectWallet(externalWallet.activeProvider);
      }

      if (user != null) {
        await logout();
      }

      await clearBackendAuthSession();
      setBackendSession(null);
      setMessage("disconnected.");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "disconnect failed.");
    }
  }

  return (
    <YouDetailShell
      onBack={() => navigation.goBack()}
      subtitle="your identity in anky."
      title="account"
    >
      <YouHeroCard
        icon={assets.avatar}
        status={connected ? "connected" : "local-first"}
        subtitle="no login required to write."
        title={identityTitle}
      >
        <Text style={styles.heroBody}>
          {connected
            ? "account features can help with recovery, credits, and loom actions."
            : "writing works fully on this device without creating an account."}
        </Text>
      </YouHeroCard>

      <View style={styles.stack}>
        {email == null ? null : (
          <YouInfoRow
            icon={assets.icons.account}
            rightText={email}
            subtitle="email identity connected through privy."
            title="email"
          />
        )}
        <YouInfoRow
          icon={assets.icons.loom}
          rightText={walletLabel}
          subtitle="used only for optional loom minting and hash sealing."
          title="wallet"
        />
        <YouInfoRow
          badge="not configured"
          icon={assets.icons.privacy}
          subtitle="this build does not include native reminder permissions."
          title="notifications"
        />
    
      </View>

      <View style={styles.actions}>
        <YouActionButton
          label={connected ? "manage login" : "log in / connect"}
          onPress={() => navigation.navigate("Auth")}
        />
        {connected ? (
          <YouActionButton
            label="logout / disconnect"
            onPress={() => void disconnectAccount()}
            variant="secondary"
          />
        ) : null}
      </View>

      <InlineMessage text={message} />
      <OwnershipCard text="your writing stays on your device. account features are optional." />
    </YouDetailShell>
  );
}

export function PrivacyScreen({ navigation }: PrivacyProps) {
  const [message, setMessage] = useState("");
  const [threadConsent, setThreadConsent] = useState(false);

  useEffect(() => {
    let mounted = true;

    async function load() {
      const consent = await hasThreadProcessingConsent();

      if (mounted) {
        setThreadConsent(consent);
      }
    }

    void load().catch(console.error);
    const unsubscribe = navigation.addListener("focus", () => {
      void load().catch(console.error);
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  async function updateThreadConsent(nextValue: boolean) {
    if (nextValue) {
      await markThreadProcessingConsent();
      setMessage("keep writing consent is remembered on this device.");
    } else {
      await resetThreadProcessingConsent();
      setMessage("keep writing will ask before processing again.");
    }

    setThreadConsent(nextValue);
  }

  async function resetProcessingConsent() {
    await resetThreadProcessingConsent();
    setThreadConsent(false);
    setMessage("processing consent was reset for keep writing.");
  }

  return (
    <YouDetailShell
      onBack={() => navigation.goBack()}
      subtitle="your writing belongs to you."
      title="privacy"
    >
      <YouHeroCard
        icon={assets.icons.privacy}
        status="local-first"
        subtitle="your writing is stored on your device. processing only happens when you choose it."
        title="local-first. private. sovereign."
      />

      <View style={styles.stack}>
        <YouToggleRow
          disabled
          icon={assets.icons.privacy}
          rightText="asks each time"
          subtitle="reflection asks before sending writing for processing."
          title="reflections may process writing"
          value={false}
        />
        <YouToggleRow
          icon={assets.icons.account}
          onValueChange={(nextValue) => void updateThreadConsent(nextValue)}
          rightText={threadConsent ? "remembered" : "asks first"}
          subtitle="when enabled, keep writing will not ask again on this device."
          title="keep writing processing consent"
          value={threadConsent}
        />
        <YouInfoRow
          icon={assets.icons.privacy}
          onPress={() => {
            void Linking.openURL(PRIVACY_POLICY_URL);
          }}
          subtitle="open the current anky privacy policy."
          title="privacy policy"
        />
        <YouInfoRow
          icon={assets.icons.privacy}
          onPress={() => void resetProcessingConsent()}
          subtitle="clear remembered processing consent for keep writing."
          title="reset processing consent"
        />
        <YouInfoRow
          icon={assets.icons.loom}
          subtitle="your writing stays private. only a hash proves existence."
          title="onchain sealing publishes only the hash"
        />
        <YouInfoRow
          icon={assets.icons.exportData}
          onPress={() => navigation.navigate("ExportData")}
          subtitle="copy a complete local archive or delete local data deliberately."
          title="export and delete controls"
        />
      </View>

      <InlineMessage text={message} />
      <OwnershipCard text="your words are yours. anky is here to protect that." />
    </YouDetailShell>
  );
}

export function ExportDataScreen({ navigation }: ExportDataProps) {
  const [message, setMessage] = useState("");
  const [summary, setSummary] = useState<ArchiveSummary>({
    ankyFiles: 0,
    completeAnkys: 0,
    keepWritingThreads: 0,
    reconstructedText: 0,
    reflections: 0,
    sealReceipts: 0,
  });

  useEffect(() => {
    let mounted = true;

    async function load() {
      const nextSummary = await loadArchiveSummary();

      if (mounted) {
        setSummary(nextSummary);
      }
    }

    void load().catch(console.error);
    const unsubscribe = navigation.addListener("focus", () => {
      void load().catch(console.error);
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  async function refreshSummary() {
    const nextSummary = await loadArchiveSummary();

    setSummary(nextSummary);
  }

  async function requestCompleteExport() {
    try {
      const archive = await buildCompleteArchiveText();

      if (archive.trim().length === 0) {
        setMessage("no local writing to export yet.");
        return;
      }

      await Clipboard.setStringAsync(archive);
      setMessage("complete local archive copied.");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "export failed.");
    }
  }

  function requestDeleteAll() {
    Alert.alert(
      "delete local anky data?",
      "this removes local writings, drafts, reflections, threads, and seal receipts from this device. export first if you need a backup.",
      [
        { style: "cancel", text: "cancel" },
        {
          onPress: () => {
            void confirmDeleteAll();
          },
          style: "destructive",
          text: "delete local data",
        },
      ],
    );
  }

  async function confirmDeleteAll() {
    try {
      await deleteAllLocalAnkyData();
      await refreshSummary();
      setMessage("local anky data deleted from this device.");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "delete failed.");
    }
  }

  return (
    <YouDetailShell
      onBack={() => navigation.goBack()}
      subtitle="your writing belongs to you."
      title="export data"
    >
      <YouHeroCard
        icon={assets.icons.exportData}
        status={`${summary.ankyFiles} files`}
        subtitle="your local archive can be prepared for backup or transfer."
        title="your archive summary"
      >
        <MetricGrid
          metrics={[
            { label: "ankys", value: summary.completeAnkys },
            { label: "reflections", value: summary.reflections },
            { label: "threads", value: summary.keepWritingThreads },
            { label: "seals", value: summary.sealReceipts },
          ]}
        />
      </YouHeroCard>

      <View style={styles.stack}>
        <ExportItemRow
          count={summary.ankyFiles}
          icon={assets.icons.exportData}
          subtitle="original writing traces."
          title=".anky files"
        />
        <ExportItemRow
          count={summary.reconstructedText}
          icon={assets.icons.account}
          subtitle="readable text of your entries."
          title="reconstructed text"
        />
        <ExportItemRow
          count={summary.reflections}
          icon={assets.icons.credits}
          subtitle="mirror letters saved beside entries."
          title="reflections"
        />
        <ExportItemRow
          count={summary.keepWritingThreads}
          icon={assets.icons.account}
          subtitle="local keep-writing conversations."
          title="keep writing threads"
        />
        <ExportItemRow
          count={summary.sealReceipts}
          icon={assets.icons.loom}
          subtitle="hash-only loom seal receipts."
          title="seal receipts"
        />
      </View>

      <View style={styles.actions}>
        <YouActionButton
          label="copy complete archive"
          onPress={() => void requestCompleteExport()}
        />
        <YouActionButton
          label="delete all local data"
          onPress={requestDeleteAll}
          variant="danger"
        />
      </View>

      <InlineMessage text={message} />
      <OwnershipCard text="export copies one complete local archive. deletion affects this device only." />
    </YouDetailShell>
  );
}

export function CreditsInfoScreen({ navigation }: CreditsInfoProps) {
  const [balance, setBalance] = useState(0);
  const [message, setMessage] = useState("");
  const [recentUsage, setRecentUsage] = useState<ProcessingReceiptSidecar[]>([]);

  useEffect(() => {
    let mounted = true;

    async function load() {
      const [nextBalance, files] = await Promise.all([
        getReflectionCreditBalance(),
        listSavedAnkyFiles(),
      ]);
      const receipts = await Promise.all(files.map((file) => readProcessingReceipt(file.hash)));
      const sortedReceipts = receipts
        .filter((receipt): receipt is ProcessingReceiptSidecar => receipt != null)
        .sort((left, right) => Date.parse(right.created_at) - Date.parse(left.created_at));

      if (mounted) {
        setBalance(nextBalance);
        setRecentUsage(sortedReceipts.slice(0, 3));
      }
    }

    void load().catch(console.error);
    const unsubscribe = navigation.addListener("focus", () => {
      void load().catch(console.error);
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  function restorePurchases() {
    // TODO: connect restore purchases when mobile IAP exists.
    setMessage("restore purchases is not wired in this build.");
  }

  return (
    <YouDetailShell
      onBack={() => navigation.goBack()}
      subtitle="fuel reflections and deeper mirrors."
      title="credits"
    >
      <YouHeroCard
        icon={assets.icons.credits}
        status={hasConfiguredBackend() ? "server-backed" : "backend off"}
        subtitle="1 simple reflection = 1 credit. writing is always free."
        title={`${balance} available`}
      />

      <View style={styles.stack}>
        <CreditTierRow amount={10} />
        <CreditTierRow amount={25} />
        <CreditTierRow amount={50} />
      </View>

      <SectionTitle label="recent usage" />
      {recentUsage.length === 0 ? (
        <YouInfoRow
          icon={assets.icons.privacy}
          subtitle="no credits spent yet."
          title="quiet so far"
        />
      ) : (
        <View style={styles.stack}>
          {recentUsage.map((receipt) => (
            <YouInfoRow
              icon={assets.icons.credits}
              key={receipt.created_at}
              rightText={`${receipt.credits_spent}`}
              subtitle={`${formatShortDate(receipt.created_at)} · ${receipt.credits_remaining} remaining`}
              title="reflection"
            />
          ))}
        </View>
      )}

      <View style={styles.actions}>
        <YouActionButton label="open credit tools" onPress={() => navigation.navigate("Credits")} />
        <YouActionButton label="restore purchases" onPress={restorePurchases} variant="secondary" />
      </View>

      <InlineMessage text={message} />
      <OwnershipCard text="credits are only for optional processing. writing is always free." />
    </YouDetailShell>
  );
}

export function LoomInfoScreen({ navigation }: LoomInfoProps) {
  const wallet = useAnkyPrivyWallet();
  const [selectedLoom, setSelectedLoom] = useState<SelectedLoom | null>(null);
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);

  useEffect(() => {
    let mounted = true;

    async function load() {
      const [nextSelectedLoom, nextSessions] = await Promise.all([
        getSelectedLoom(),
        listAnkySessionSummaries(),
      ]);

      if (mounted) {
        setSelectedLoom(nextSelectedLoom);
        setSessions(nextSessions);
      }
    }

    void load().catch(console.error);
    const unsubscribe = navigation.addListener("focus", () => {
      void load().catch(console.error);
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  const loomState = useMemo(() => buildLoomState(selectedLoom, sessions, wallet.hasWallet), [
    selectedLoom,
    sessions,
    wallet.hasWallet,
  ]);
  const selectedLabel =
    selectedLoom == null ? "none" : `${selectedLoom.name} · ${shortAddress(selectedLoom.asset, 5)}`;

  return (
    <YouDetailShell
      onBack={() => navigation.goBack()}
      subtitle="seal a hash when available."
      title="loom"
      titleAccessory={<Pill label="optional" />}
    >
      <YouHeroCard
        icon={assets.icons.loom}
        status={loomState.status}
        subtitle="writing never requires a loom."
        title={selectedLoom == null ? "no loom selected" : selectedLoom.name}
      >
        <Text style={styles.heroBody}>
          {selectedLoom == null
            ? "you can write every day without choosing a loom."
            : `selected on ${selectedLoom.network}. only hashes are sealed.`}
        </Text>
      </YouHeroCard>

      <View style={styles.stack}>
        <YouInfoRow
          icon={assets.icons.loom}
          subtitle="creates a daily seal from your writing hash when you choose."
          title="what a loom does"
        />
        <YouInfoRow
          icon={assets.icons.account}
          rightText={selectedLabel}
          subtitle="the loom currently remembered on this device."
          title="selected loom"
        />
        <YouInfoRow
          badge={loomState.badge}
          icon={assets.icons.loom}
          subtitle={loomState.detail}
          title="daily seal"
          variant={loomState.variant}
        />
        <YouInfoRow
          icon={assets.icons.privacy}
          subtitle="only a hash is sealed. your writing stays private."
          title="hash only — writing stays private"
        />
        <YouInfoRow
          icon={assets.icons.exportData}
          subtitle="a loom is optional — never required."
          title="no loom? you can still write every day."
        />
      </View>

      <View style={styles.actions}>
        <YouActionButton
          label="open loom"
          onPress={() => navigation.navigate("Loom")}
        />
      </View>

      <OwnershipCard text="loom and sealing are optional. writing never depends on them." />
    </YouDetailShell>
  );
}

function YouDetailShell({
  children,
  onBack,
  subtitle,
  title,
  titleAccessory,
}: {
  children: ReactNode;
  onBack: () => void;
  subtitle: string;
  title: string;
  titleAccessory?: ReactNode;
}) {
  const insets = useSafeAreaInsets();

  return (
    <ScreenBackground safe={false} variant="plain">
      <ImageBackground resizeMode="cover" source={assets.background} style={styles.screen}>
        <View pointerEvents="none" style={styles.cosmosWash} />
        <View style={[styles.shell, { paddingTop: insets.top + 10 }]}>
          <View style={styles.header}>
            <SubtleIconButton accessibilityLabel="go back" icon="←" onPress={onBack} />
            <View style={styles.headerCenter}>
              <View style={styles.headerTitleRow}>
                <Text numberOfLines={1} style={styles.headerTitle}>
                  {title}
                </Text>
                {titleAccessory}
              </View>
              <Text numberOfLines={2} style={styles.headerSubtitle}>
                {subtitle}
              </Text>
            </View>
            <View style={styles.headerSide} />
          </View>

          <ScrollView
            contentContainerStyle={[
              styles.content,
              { paddingBottom: Math.max(34, insets.bottom + 24) },
            ]}
            showsVerticalScrollIndicator={false}
          >
            {children}
            <BottomOrnament />
          </ScrollView>
        </View>
      </ImageBackground>
    </ScreenBackground>
  );
}

function YouHeroCard({
  children,
  icon,
  status,
  subtitle,
  title,
}: {
  children?: ReactNode;
  icon: ImageSourcePropType;
  status?: string;
  subtitle: string;
  title: string;
}) {
  return (
    <View style={styles.heroCard}>
      <View style={styles.heroTop}>
        <View style={styles.heroIconFrame}>
          <Image accessibilityIgnoresInvertColors source={icon} style={styles.heroIcon} />
        </View>
        <View style={styles.heroCopy}>
          <View style={styles.heroTitleRow}>
            <Text style={styles.heroTitle}>{title}</Text>
            {status == null ? null : <Pill label={status} />}
          </View>
          <Text style={styles.heroSubtitle}>{subtitle}</Text>
        </View>
      </View>
      {children == null ? null : <View style={styles.heroChildren}>{children}</View>}
    </View>
  );
}

function YouInfoRow({
  badge,
  children,
  disabled = false,
  icon,
  onPress,
  rightText,
  subtitle,
  title,
  variant = "normal",
}: {
  badge?: string;
  children?: ReactNode;
  disabled?: boolean;
  icon: ImageSourcePropType;
  onPress?: () => void;
  rightText?: string;
  subtitle: string;
  title: string;
  variant?: RowVariant;
}) {
  const pressable = onPress != null && !disabled;

  return (
    <Pressable
      accessibilityRole={pressable ? "button" : undefined}
      disabled={!pressable}
      onPress={onPress}
      style={({ pressed }) => [
        styles.rowPanel,
        variant === "highlight" && styles.rowPanelHighlight,
        variant === "danger" && styles.rowPanelDanger,
        disabled && styles.disabled,
        pressed && styles.pressed,
      ]}
    >
      <View
        style={[
          styles.rowIconFrame,
          variant === "highlight" && styles.rowIconHighlight,
          variant === "danger" && styles.rowIconDanger,
        ]}
      >
        <Image accessibilityIgnoresInvertColors source={icon} style={styles.rowIcon} />
      </View>
      <View style={styles.rowCopy}>
        <Text style={[styles.rowTitle, variant === "danger" && styles.rowTitleDanger]}>
          {title}
        </Text>
        <Text style={styles.rowSubtitle}>{subtitle}</Text>
        {children}
      </View>
      {rightText == null ? null : (
        <Text numberOfLines={2} style={styles.rowRightText}>
          {rightText}
        </Text>
      )}
      {badge == null ? null : <Pill label={badge} variant={variant} />}
      {pressable ? <Text style={styles.chevron}>›</Text> : null}
    </Pressable>
  );
}

function YouToggleRow({
  disabled = false,
  icon,
  onValueChange,
  rightText,
  subtitle,
  title,
  value,
}: {
  disabled?: boolean;
  icon: ImageSourcePropType;
  onValueChange?: (value: boolean) => void;
  rightText?: string;
  subtitle: string;
  title: string;
  value: boolean;
}) {
  const canChange = onValueChange != null && !disabled;

  return (
    <YouInfoRow
      badge={rightText}
      disabled={!canChange}
      icon={icon}
      onPress={canChange ? () => onValueChange(!value) : undefined}
      subtitle={subtitle}
      title={title}
    >
      <View style={styles.toggleWrap}>
        <Switch
          disabled={!canChange}
          ios_backgroundColor="rgba(244, 241, 234, 0.14)"
          onValueChange={onValueChange}
          thumbColor={value ? GOLD_BRIGHT : "rgba(244, 241, 234, 0.72)"}
          trackColor={{ false: "rgba(244, 241, 234, 0.14)", true: "rgba(214, 147, 68, 0.58)" }}
          value={value}
        />
      </View>
    </YouInfoRow>
  );
}

function YouActionButton({
  disabled = false,
  label,
  onPress,
  variant = "primary",
}: {
  disabled?: boolean;
  label: string;
  onPress?: () => void;
  variant?: ActionVariant;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      disabled={disabled || onPress == null}
      onPress={onPress}
      style={({ pressed }) => [
        styles.actionButton,
        variant === "secondary" && styles.actionButtonSecondary,
        variant === "danger" && styles.actionButtonDanger,
        disabled && styles.disabled,
        pressed && styles.pressed,
      ]}
    >
      <Text
        style={[
          styles.actionText,
          variant === "secondary" && styles.actionTextSecondary,
          variant === "danger" && styles.actionTextDanger,
        ]}
      >
        {label}
      </Text>
    </Pressable>
  );
}

function ExportItemRow({
  count,
  icon,
  subtitle,
  title,
}: {
  count: number;
  icon: ImageSourcePropType;
  subtitle: string;
  title: string;
}) {
  return (
    <YouInfoRow icon={icon} rightText={`${count}`} subtitle={subtitle} title={title}>
      <View style={styles.exportState}>
        <View style={styles.checkDot} />
        <Text style={styles.exportStateText}>included</Text>
      </View>
    </YouInfoRow>
  );
}

function CreditTierRow({ amount }: { amount: number }) {
  return (
    <YouInfoRow
      badge="coming soon"
      disabled
      icon={assets.icons.credits}
      subtitle="native purchases are not configured in this build."
      title={`${amount} credits`}
    />
  );
}

function MetricGrid({ metrics }: { metrics: Array<{ label: string; value: number }> }) {
  return (
    <View style={styles.metricGrid}>
      {metrics.map((metric) => (
        <View key={metric.label} style={styles.metricCell}>
          <Text style={styles.metricValue}>{metric.value}</Text>
          <Text style={styles.metricLabel}>{metric.label}</Text>
        </View>
      ))}
    </View>
  );
}

function SectionTitle({ label }: { label: string }) {
  return <Text style={styles.sectionTitle}>{label}</Text>;
}

function Pill({ label, variant = "normal" }: { label: string; variant?: RowVariant }) {
  return (
    <View style={[styles.pill, variant === "danger" && styles.pillDanger]}>
      <Text style={[styles.pillText, variant === "danger" && styles.pillTextDanger]}>
        {label}
      </Text>
    </View>
  );
}

function OwnershipCard({ text }: { text: string }) {
  return (
    <View style={styles.ownershipCard}>
      <Image accessibilityIgnoresInvertColors source={assets.icons.privacy} style={styles.ownershipIcon} />
      <Text style={styles.ownershipText}>{text}</Text>
    </View>
  );
}

function InlineMessage({ text }: { text: string }) {
  if (text.length === 0) {
    return null;
  }

  return <Text style={styles.inlineMessage}>{text}</Text>;
}

function BottomOrnament() {
  return (
    <View style={styles.bottomOrnament}>
      <View style={styles.bottomLine} />
      <View style={styles.bottomDiamond} />
      <View style={styles.bottomLine} />
    </View>
  );
}

type ArchiveSummary = {
  ankyFiles: number;
  completeAnkys: number;
  keepWritingThreads: number;
  reconstructedText: number;
  reflections: number;
  sealReceipts: number;
};

async function loadArchiveSummary(): Promise<ArchiveSummary> {
  const [files, sessions, threads, seals] = await Promise.all([
    listSavedAnkyFiles(),
    listAnkySessionSummaries(),
    listThreads(),
    listLocalLoomSeals(),
  ]);
  const sidecarThreadCount = files.filter((file) =>
    file.artifactKinds.includes("conversation"),
  ).length;
  const sessionThreadCount = sessions.filter((session) => session.hasThread === true).length;
  const sidecarReflectionCount = files.filter((file) =>
    file.artifactKinds.includes("reflection"),
  ).length;
  const sessionReflectionCount = sessions.filter((session) => session.reflectionId != null).length;

  return {
    ankyFiles: files.length,
    completeAnkys: countCompleteAnkys(files),
    keepWritingThreads: Math.max(threads.length, sidecarThreadCount, sessionThreadCount),
    reconstructedText: files.length,
    reflections: Math.max(sidecarReflectionCount, sessionReflectionCount),
    sealReceipts: seals.length,
  };
}

async function buildCompleteArchiveText(): Promise<string> {
  const [files, threads] = await Promise.all([listSavedAnkyFiles(), listThreads()]);

  if (files.length === 0 && threads.length === 0) {
    return "";
  }

  const sections = await Promise.all(
    files.map(async (file) => {
      const reflection = await readReflectionSidecar(file.hash);
      const receipt = await readProcessingReceipt(file.hash);
      const kind = isCompleteRawAnky(file.raw) ? "complete anky" : "fragment";

      return [
        `file: ${file.fileName}`,
        `kind: ${kind}`,
        receipt == null ? null : `reflection credits spent: ${receipt.credits_spent}`,
        "",
        "writing:",
        reconstructText(file.raw),
        "",
        "raw .anky:",
        file.raw,
        reflection == null ? null : "",
        reflection == null ? null : "anky reflection:",
        reflection,
      ]
        .filter((line): line is string => line != null)
        .join("\n");
    }),
  );

  const threadSections = threads.map((thread) =>
    [
      `thread: ${thread.sessionHash}`,
      `mode: ${thread.mode}`,
      ...thread.messages.map((message) => `${message.role}: ${message.content}`),
    ].join("\n"),
  );

  return [
    "anky local archive",
    `exported: ${new Date().toISOString()}`,
    "",
    ...sections,
    ...threadSections,
  ].join("\n\n---\n\n");
}

function countCompleteAnkys(files: SavedAnkyFile[]): number {
  return files.filter((file) => isCompleteRawAnky(file.raw)).length;
}

function buildLoomState(
  selectedLoom: SelectedLoom | null,
  sessions: AnkySessionSummary[],
  hasWallet: boolean,
): {
  badge: string;
  detail: string;
  status: string;
  variant: RowVariant;
} {
  if (selectedLoom == null) {
    return {
      badge: "not selected",
      detail: "choose a loom only if you want optional hash sealing.",
      status: "no loom selected",
      variant: "normal",
    };
  }

  if (!hasWallet) {
    return {
      badge: "connect",
      detail: "connect a wallet only if you want to seal a hash.",
      status: "wallet not connected",
      variant: "normal",
    };
  }

  const nextKind = getNextSessionKindForToday(sessions);
  const today = buildSojournDays(sessions)[getCurrentSojournDay() - 1];

  if (nextKind === "daily_seal") {
    return {
      badge: `day ${today.day}`,
      detail: `day ${today.day} of ${SOJOURN_LENGTH_DAYS} can be sealed after a complete anky.`,
      status: "daily seal available",
      variant: "highlight",
    };
  }

  return {
    badge: "woven",
    detail: "today already has its daily seal. extra writing can still be saved locally.",
    status: "today sealed",
    variant: "normal",
  };
}

function getPrivyEmail(user: unknown): string | null {
  const directEmail = readStringField(user, "email") ?? readStringField(user, "emailAddress");

  if (isEmail(directEmail)) {
    return directEmail;
  }

  const linkedAccounts = readArrayField(user, "linkedAccounts");

  for (const account of linkedAccounts) {
    const email = readStringField(account, "email") ?? readStringField(account, "address");

    if (isEmail(email)) {
      return email;
    }
  }

  return null;
}

function readStringField(value: unknown, key: string): string | null {
  if (typeof value !== "object" || value == null || !(key in value)) {
    return null;
  }

  const field = (value as Record<string, unknown>)[key];

  return typeof field === "string" && field.length > 0 ? field : null;
}

function readArrayField(value: unknown, key: string): unknown[] {
  if (typeof value !== "object" || value == null || !(key in value)) {
    return [];
  }

  const field = (value as Record<string, unknown>)[key];

  return Array.isArray(field) ? field : [];
}

function isEmail(value: string | null): value is string {
  return value != null && value.includes("@");
}

function formatShortDate(value: string): string {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return "recent";
  }

  return date.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
  }).toLowerCase();
}

const styles = StyleSheet.create({
  actionButton: {
    alignItems: "center",
    backgroundColor: "rgba(233, 190, 114, 0.18)",
    borderColor: "rgba(242, 211, 146, 0.72)",
    borderRadius: 16,
    borderWidth: 1,
    minHeight: 48,
    justifyContent: "center",
    paddingHorizontal: 18,
  },
  actionButtonDanger: {
    backgroundColor: "rgba(241, 154, 114, 0.13)",
    borderColor: "rgba(241, 154, 114, 0.54)",
  },
  actionButtonSecondary: {
    backgroundColor: "rgba(244, 241, 234, 0.08)",
    borderColor: "rgba(244, 241, 234, 0.16)",
  },
  actions: {
    gap: 10,
    marginTop: 16,
  },
  actionText: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 16,
    lineHeight: 20,
    textAlign: "center",
    textTransform: "lowercase",
  },
  actionTextDanger: {
    color: DANGER,
  },
  actionTextSecondary: {
    color: COPY,
  },
  bottomDiamond: {
    borderColor: GOLD,
    borderWidth: 1,
    height: 8,
    marginHorizontal: 10,
    transform: [{ rotate: "45deg" }],
    width: 8,
  },
  bottomLine: {
    backgroundColor: "rgba(221, 142, 67, 0.35)",
    height: 1,
    width: 96,
  },
  bottomOrnament: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "center",
    marginTop: 22,
  },
  checkDot: {
    backgroundColor: GOLD_BRIGHT,
    borderRadius: 4,
    height: 8,
    width: 8,
  },
  chevron: {
    color: "rgba(242, 211, 146, 0.76)",
    fontSize: 26,
    lineHeight: 30,
    marginLeft: 2,
  },
  content: {
    paddingHorizontal: 20,
    paddingTop: 12,
  },
  cosmosWash: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(5, 5, 14, 0.22)",
  },
  disabled: {
    opacity: 0.62,
  },
  exportState: {
    alignItems: "center",
    flexDirection: "row",
    gap: 7,
    marginTop: 8,
  },
  exportStateText: {
    color: "rgba(242, 211, 146, 0.84)",
    fontFamily: SERIF,
    fontSize: 12,
    lineHeight: 15,
    textTransform: "lowercase",
  },
  header: {
    alignItems: "center",
    flexDirection: "row",
    minHeight: 56,
    paddingHorizontal: 18,
  },
  headerCenter: {
    alignItems: "center",
    flex: 1,
    paddingHorizontal: 8,
  },
  headerSide: {
    width: 36,
  },
  headerSubtitle: {
    color: "rgba(223, 209, 213, 0.78)",
    fontFamily: SERIF,
    fontSize: 12.5,
    lineHeight: 16,
    marginTop: -1,
    textAlign: "center",
    textTransform: "lowercase",
  },
  headerTitle: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 26,
    lineHeight: 32,
    textAlign: "center",
    textShadowColor: "rgba(237, 179, 94, 0.24)",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 14,
    textTransform: "lowercase",
  },
  headerTitleRow: {
    alignItems: "center",
    flexDirection: "row",
    gap: 8,
    justifyContent: "center",
    minWidth: 0,
  },
  heroBody: {
    color: COPY,
    fontFamily: SERIF,
    fontSize: 13,
    lineHeight: 18,
    textTransform: "lowercase",
  },
  heroCard: {
    backgroundColor: PANEL,
    borderColor: "rgba(217, 143, 63, 0.62)",
    borderRadius: 20,
    borderWidth: 1,
    padding: 16,
    shadowColor: "#E5A550",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.12,
    shadowRadius: 14,
  },
  heroChildren: {
    marginTop: 14,
  },
  heroCopy: {
    flex: 1,
    minWidth: 0,
  },
  heroIcon: {
    height: 58,
    width: 58,
  },
  heroIconFrame: {
    alignItems: "center",
    backgroundColor: PANEL_DEEP,
    borderColor: "rgba(217, 143, 63, 0.62)",
    borderRadius: 22,
    borderWidth: 1,
    height: 82,
    justifyContent: "center",
    marginRight: 14,
    width: 82,
  },
  heroSubtitle: {
    color: COPY,
    fontFamily: SERIF,
    fontSize: 13,
    lineHeight: 18,
    marginTop: 4,
    textTransform: "lowercase",
  },
  heroTitle: {
    color: GOLD_BRIGHT,
    flexShrink: 1,
    fontFamily: SERIF,
    fontSize: 21,
    lineHeight: 26,
    textTransform: "lowercase",
  },
  heroTitleRow: {
    alignItems: "center",
    flexDirection: "row",
    flexWrap: "wrap",
    gap: 8,
  },
  heroTop: {
    alignItems: "center",
    flexDirection: "row",
  },
  inlineMessage: {
    color: "rgba(242, 211, 146, 0.86)",
    fontFamily: SERIF,
    fontSize: 13,
    lineHeight: 18,
    marginTop: 14,
    textAlign: "center",
    textTransform: "lowercase",
  },
  metricCell: {
    alignItems: "center",
    backgroundColor: "rgba(9, 8, 20, 0.5)",
    borderColor: "rgba(178, 80, 129, 0.36)",
    borderRadius: 12,
    borderWidth: 1,
    flex: 1,
    minHeight: 58,
    justifyContent: "center",
  },
  metricGrid: {
    flexDirection: "row",
    gap: 8,
  },
  metricLabel: {
    color: "#D8B8EA",
    fontFamily: SERIF,
    fontSize: 10.5,
    lineHeight: 13,
    textTransform: "lowercase",
  },
  metricValue: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 20,
    lineHeight: 24,
  },
  ownershipCard: {
    alignItems: "center",
    borderBottomColor: "rgba(219, 143, 63, 0.36)",
    borderBottomWidth: StyleSheet.hairlineWidth,
    borderRadius: 10,
    borderTopColor: "rgba(219, 143, 63, 0.28)",
    borderTopWidth: StyleSheet.hairlineWidth,
    flexDirection: "row",
    justifyContent: "center",
    marginTop: 16,
    minHeight: 44,
    paddingHorizontal: 10,
  },
  ownershipIcon: {
    height: 18,
    marginRight: 7,
    opacity: 0.92,
    width: 18,
  },
  ownershipText: {
    color: "rgba(242, 211, 146, 0.9)",
    flex: 1,
    fontFamily: SERIF,
    fontSize: 12,
    lineHeight: 16,
    textTransform: "lowercase",
  },
  pill: {
    backgroundColor: "rgba(11, 10, 22, 0.55)",
    borderColor: "rgba(233, 190, 114, 0.48)",
    borderRadius: 8,
    borderWidth: 1,
    paddingHorizontal: 8,
    paddingVertical: 2,
  },
  pillDanger: {
    borderColor: "rgba(241, 154, 114, 0.54)",
  },
  pillText: {
    color: "rgba(233, 213, 170, 0.88)",
    fontFamily: SERIF,
    fontSize: 10.5,
    lineHeight: 13,
    textTransform: "lowercase",
  },
  pillTextDanger: {
    color: DANGER,
  },
  pressed: {
    opacity: 0.72,
    transform: [{ scale: 0.995 }],
  },
  rowCopy: {
    flex: 1,
    minWidth: 0,
  },
  rowIcon: {
    height: 30,
    width: 30,
  },
  rowIconDanger: {
    borderColor: "rgba(241, 154, 114, 0.44)",
  },
  rowIconFrame: {
    alignItems: "center",
    backgroundColor: "rgba(9, 8, 20, 0.72)",
    borderColor: "rgba(217, 143, 63, 0.44)",
    borderRadius: 15,
    borderWidth: 1,
    height: 44,
    justifyContent: "center",
    marginRight: 12,
    width: 44,
  },
  rowIconHighlight: {
    borderColor: "rgba(232, 113, 207, 0.54)",
  },
  rowPanel: {
    alignItems: "center",
    backgroundColor: PANEL,
    borderColor: "rgba(217, 143, 63, 0.46)",
    borderRadius: 16,
    borderWidth: 1,
    flexDirection: "row",
    minHeight: 66,
    paddingHorizontal: 12,
    paddingVertical: 10,
  },
  rowPanelDanger: {
    borderColor: "rgba(241, 154, 114, 0.42)",
  },
  rowPanelHighlight: {
    borderColor: "rgba(232, 113, 207, 0.58)",
  },
  rowRightText: {
    color: "rgba(242, 211, 146, 0.82)",
    fontFamily: SERIF,
    fontSize: 12,
    lineHeight: 15,
    marginLeft: 10,
    maxWidth: 96,
    textAlign: "right",
    textTransform: "lowercase",
  },
  rowSubtitle: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 12.5,
    lineHeight: 17,
    marginTop: 2,
    textTransform: "lowercase",
  },
  rowTitle: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 16,
    lineHeight: 20,
    textTransform: "lowercase",
  },
  rowTitleDanger: {
    color: DANGER,
  },
  screen: {
    backgroundColor: "#070812",
    flex: 1,
  },
  sectionTitle: {
    color: "rgba(242, 211, 146, 0.88)",
    fontFamily: SERIF,
    fontSize: 14,
    lineHeight: 18,
    marginBottom: 8,
    marginTop: 18,
    textTransform: "lowercase",
  },
  shell: {
    flex: 1,
  },
  stack: {
    gap: 10,
    marginTop: 14,
  },
  toggleWrap: {
    alignItems: "flex-start",
    marginTop: 9,
  },
});
