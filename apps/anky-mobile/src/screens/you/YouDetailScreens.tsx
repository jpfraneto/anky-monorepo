import { ReactNode, useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  Animated,
  Easing,
  Image,
  ImageBackground,
  ImageSourcePropType,
  type LayoutRectangle,
  Linking,
  Modal,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  View,
} from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import { usePrivy } from "@privy-io/expo";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { WebView } from "react-native-webview";

import type { RootStackParamList } from "../../../App";
import { useAuthModal } from "../../auth/AuthModalContext";
import { ScreenBackground } from "../../components/anky/ScreenBackground";
import { SubtleIconButton } from "../../components/navigation/SubtleIconButton";
import {
  exportAnkyBackupArchive,
  pickAndRestoreAnkyBackup,
  type AnkyBackupRestoreResult,
} from "../../lib/ankyBackup";
import { listAnkySessionSummaries } from "../../lib/ankySessionIndex";
import {
  deleteAllLocalAnkyData,
  listLocalLoomSeals,
  listSavedAnkyFiles,
  type SavedAnkyFile,
} from "../../lib/ankyStorage";
import { getAnkyApiClient } from "../../lib/api/client";
import type { CreditLedgerEntry } from "../../lib/api/types";
import {
  clearBackendAuthSession,
  getStoredBackendAuthSession,
  type BackendAuthSession,
} from "../../lib/auth/backendSession";
import { getMobileApiIdentityId } from "../../lib/auth/mobileIdentity";
import { useExternalSolanaWallet } from "../../lib/privy/ExternalSolanaWalletProvider";
import { getReflectionCreditBalance } from "../../lib/credits/processAnky";
import { CREDIT_PRODUCTS, type CreditProduct } from "../../lib/credits/products";
import {
  configureRevenueCat,
  getCreditsOfferingPackages,
  getRevenueCatCreditBalance,
  getRevenueCatCreditStatus,
  purchaseCreditsPackage,
  type AnkyCreditStorePackage,
  type AnkyRevenueCatPackageId,
  type RevenueCatCreditStatus,
} from "../../lib/credits/revenueCatCredits";
import { getPublicEnv } from "../../lib/config/env";
import { useAnkyPrivyWallet } from "../../lib/privy/useAnkyPrivyWallet";
import * as Haptics from "expo-haptics";
import { NotificationSettingsModal } from "../../notifications/NotificationSettingsModal";
import {
  DEFAULT_NOTIFICATION_SETTINGS,
  formatReminderTime,
  loadNotificationSettings,
  type AnkyNotificationSettings,
} from "../../notifications/notificationSettings";
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
type CreditProductVisualState = "dimmed" | "idle" | "processing" | "success";
type CreditTransferRun = {
  from: LayoutRectangle;
  productId: string;
  to: LayoutRectangle;
};

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
  const { openAuthModal } = useAuthModal();
  const externalWallet = useExternalSolanaWallet();
  const wallet = useAnkyPrivyWallet();
  const [backendSession, setBackendSession] = useState<BackendAuthSession | null>(null);
  const [message, setMessage] = useState("");
  const [notificationModalVisible, setNotificationModalVisible] = useState(false);
  const [notificationSettings, setNotificationSettings] = useState<AnkyNotificationSettings>(
    DEFAULT_NOTIFICATION_SETTINGS,
  );
  const [walletExportVisible, setWalletExportVisible] = useState(false);
  const walletExportUrl = getPublicEnv("EXPO_PUBLIC_PRIVY_WALLET_EXPORT_URL") ?? "";

  useEffect(() => {
    let mounted = true;

    async function load() {
      const [session, reminders] = await Promise.all([
        getStoredBackendAuthSession(),
        loadNotificationSettings(),
      ]);

      if (mounted) {
        setBackendSession(session);
        setNotificationSettings(reminders);
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
  const notificationSubtitle = notificationSettings.enabled
    ? `daily reminder at ${formatReminderTime(notificationSettings.hour, notificationSettings.minute)}`
    : "choose when anky should quietly remind you.";

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
          icon={assets.icons.privacy}
          onPress={() => setNotificationModalVisible(true)}
          rightText={notificationSettings.enabled ? "on" : "off"}
          subtitle={notificationSubtitle}
          title="notifications"
        />
        {wallet.hasEmbeddedWallet ? (
          <YouInfoRow
            badge={walletExportUrl.length === 0 ? "needs url" : undefined}
            disabled={walletExportUrl.length === 0}
            icon={assets.icons.loom}
            onPress={() => setWalletExportVisible(true)}
            subtitle={
              walletExportUrl.length === 0
                ? "wallet export needs EXPO_PUBLIC_PRIVY_WALLET_EXPORT_URL."
                : "opens the secure hosted privy export page."
            }
            title="export wallet"
          />
        ) : null}
    
      </View>

      <View style={styles.actions}>
        <YouActionButton
          label={connected ? "manage login" : "log in / connect"}
          onPress={() =>
            openAuthModal({
              reason: "account features are optional. writing stays local.",
            })
          }
        />
        {connected ? (
          <YouActionButton
            label="logout / disconnect"
            onPress={() => void disconnectAccount()}
            variant="secondary"
          />
        ) : null}
      </View>

      <SectionTitle label="danger zone" />
      <YouInfoRow
        badge="disabled"
        icon={assets.icons.privacy}
        onPress={requestDestroyAccount}
        subtitle="requires a backend and privy deletion path before it can run."
        title="destroy account forever"
        variant="danger"
      />

      <InlineMessage text={message} />
      <OwnershipCard text="your writing stays on your device. account features are optional." />
      <NotificationSettingsModal
        onClose={() => setNotificationModalVisible(false)}
        onSaved={setNotificationSettings}
        visible={notificationModalVisible}
      />
      <WalletExportModal
        onClose={() => setWalletExportVisible(false)}
        url={walletExportUrl}
        visible={walletExportVisible}
      />
    </YouDetailShell>
  );

  function requestDestroyAccount() {
    Alert.alert(
      "destroy account forever?",
      "this cannot run from the app until a verified backend and privy deletion endpoint exists. your local writing is not deleted here.",
      [
        { style: "cancel", text: "cancel" },
        {
          onPress: () => {
            setMessage("account destruction is not wired in this build.");
          },
          style: "destructive",
          text: "i understand",
        },
      ],
    );
  }
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
          subtitle="backup, restore, or delete local data deliberately."
          title="export and delete controls"
        />
      </View>

      <InlineMessage text={message} />
      <OwnershipCard text="your words are yours. anky is here to protect that." />
    </YouDetailShell>
  );
}

export function ExportDataScreen({ navigation }: ExportDataProps) {
  const [busyAction, setBusyAction] = useState<"delete" | "export" | "restore" | null>(null);
  const [message, setMessage] = useState("");
  const [summary, setSummary] = useState<ArchiveSummary>({
    ankyFiles: 0,
    completeAnkys: 0,
    keepWritingThreads: 0,
    reflections: 0,
    sealReceipts: 0,
    sessionIndexEntries: 0,
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

  function requestBackupExport() {
    Alert.alert(
      "export backup?",
      "this zip may include plaintext writing, reflections, keep-writing conversations, images, and local metadata. save it somewhere you trust.",
      [
        { style: "cancel", text: "cancel" },
        {
          onPress: () => {
            void confirmBackupExport();
          },
          text: "export backup",
        },
      ],
    );
  }

  async function confirmBackupExport() {
    try {
      setBusyAction("export");
      const backup = await exportAnkyBackupArchive();

      setMessage(`backup opened: ${backup.fileName} with ${backup.fileCount} local files.`);
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "export failed.");
    } finally {
      setBusyAction(null);
    }
  }

  function requestBackupRestore() {
    Alert.alert(
      "restore backup?",
      "choose an anky backup zip. restore merges files into this device, skips newer local files, and does not delete local data.",
      [
        { style: "cancel", text: "cancel" },
        {
          onPress: () => {
            void confirmBackupRestore();
          },
          text: "restore backup",
        },
      ],
    );
  }

  async function confirmBackupRestore() {
    try {
      setBusyAction("restore");
      const result = await pickAndRestoreAnkyBackup();

      if (result == null) {
        setMessage("restore canceled.");
        return;
      }

      await refreshSummary();
      setMessage(formatRestoreMessage(result));
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "restore failed.");
    } finally {
      setBusyAction(null);
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
      setBusyAction("delete");
      await deleteAllLocalAnkyData();
      await refreshSummary();
      setMessage("local anky data deleted from this device.");
    } catch (error) {
      console.error(error);
      setMessage(error instanceof Error ? error.message : "delete failed.");
    } finally {
      setBusyAction(null);
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
        status={`${summary.ankyFiles} .anky`}
        subtitle="Ankys live on this device. Deleting the app deletes local ankys unless you export them."
        title="local archive"
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
          count={summary.sessionIndexEntries}
          icon={assets.icons.account}
          subtitle="local map and entry lookup state."
          title="session index entries"
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
          disabled={busyAction != null}
          label={busyAction === "export" ? "preparing backup" : "export backup"}
          onPress={requestBackupExport}
        />
        <YouActionButton
          disabled={busyAction != null}
          label={busyAction === "restore" ? "restoring backup" : "restore backup"}
          onPress={requestBackupRestore}
          variant="secondary"
        />
        <YouActionButton
          disabled={busyAction != null}
          label={busyAction === "delete" ? "deleting local data" : "delete local data"}
          onPress={requestDeleteAll}
          variant="danger"
        />
      </View>

      <InlineMessage text={message} />
      <OwnershipCard text="your backup is a zip you manage. no plaintext is sent to anky for backup." />
    </YouDetailShell>
  );
}

export function CreditsInfoScreen({ navigation }: CreditsInfoProps) {
  const [balance, setBalance] = useState(0);
  const [displayBalance, setDisplayBalance] = useState(0);
  const balanceValue = useRef(new Animated.Value(0)).current;
  const displayBalanceRef = useRef(0);
  const transferProgress = useRef(new Animated.Value(0)).current;
  const [message, setMessage] = useState("");
  const [purchaseBusyId, setPurchaseBusyId] = useState<string | null>(null);
  const [purchaseStatus, setPurchaseStatus] =
    useState<RevenueCatCreditStatus>(getRevenueCatCreditStatus());
  const [ledgerEntries, setLedgerEntries] = useState<CreditLedgerEntry[]>([]);
  const [successProductId, setSuccessProductId] = useState<string | null>(null);
  const [transferRun, setTransferRun] = useState<CreditTransferRun | null>(null);
  const [heroLayout, setHeroLayout] = useState<LayoutRectangle | null>(null);
  const [packListLayout, setPackListLayout] = useState<LayoutRectangle | null>(null);
  const [cardLayouts, setCardLayouts] = useState<Record<string, LayoutRectangle>>({});
  const [storePackages, setStorePackages] = useState<
    Partial<Record<AnkyRevenueCatPackageId, AnkyCreditStorePackage>>
  >({});

  useEffect(() => {
    const listenerId = balanceValue.addListener(({ value }) => {
      const rounded = Math.max(0, Math.round(value));
      displayBalanceRef.current = rounded;
      setDisplayBalance(rounded);
    });

    return () => {
      balanceValue.removeListener(listenerId);
    };
  }, [balanceValue]);

  useEffect(() => {
    let mounted = true;

    async function load() {
      const [localBalance, session, identityId] = await Promise.all([
        getReflectionCreditBalance(),
        getStoredBackendAuthSession(),
        getMobileApiIdentityId(),
      ]);
      let nextBalance = localBalance;
      let nextLedgerEntries: CreditLedgerEntry[] = [];
      let nextStorePackages: Partial<Record<AnkyRevenueCatPackageId, AnkyCreditStorePackage>> = {};
      let nextMessage = "";
      const api = getAnkyApiClient();

      await configureRevenueCat().catch((error: unknown) => {
        console.warn("RevenueCat credits unavailable.", error);
      });

      if (api != null && session != null) {
        await api.claimWelcomeCreditGift(session.sessionToken).catch((error: unknown) => {
          console.warn("Welcome credits grant failed.", error);
        });
      }

      if (getRevenueCatCreditStatus() === "available") {
        try {
          const [packages, revenueCatBalance] = await Promise.all([
            getCreditsOfferingPackages(),
            getRevenueCatCreditBalance({ forceRefresh: session != null }),
          ]);

          nextBalance = revenueCatBalance;
          nextStorePackages = packages.reduce<
            Partial<Record<AnkyRevenueCatPackageId, AnkyCreditStorePackage>>
          >((packagesById, storePackage) => {
            packagesById[storePackage.packageId] = storePackage;
            return packagesById;
          }, {});
        } catch (error) {
          console.warn("RevenueCat credits load failed.", error);
          nextMessage =
            error instanceof Error ? error.message : "credits offering is unavailable.";
        }
      }

      if (api != null) {
        nextLedgerEntries = await api
          .getCreditLedgerHistory({
            identityId,
            sessionToken: session?.sessionToken,
          })
          .then((response) => response.entries)
          .catch((error: unknown) => {
            console.warn("Credit history load failed.", error);
            return [];
          });
      }

      if (mounted) {
        setBalance(nextBalance);
        setBalanceImmediately(nextBalance);
        setMessage(nextMessage);
        setPurchaseStatus(getRevenueCatCreditStatus());
        setLedgerEntries(nextLedgerEntries);
        setStorePackages(nextStorePackages);
      }
    }

    setPurchaseStatus("pending");
    void load().catch((error: unknown) => {
      console.error(error);
      if (mounted) {
        setPurchaseStatus(getRevenueCatCreditStatus());
      }
    });
    const unsubscribe = navigation.addListener("focus", () => {
      setPurchaseStatus("pending");
      void load().catch((error: unknown) => {
        console.error(error);
        if (mounted) {
          setPurchaseStatus(getRevenueCatCreditStatus());
        }
      });
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  function startPurchase(product: CreditProduct) {
    if (purchaseBusyId != null) {
      return;
    }

    setMessage("");

    if (purchaseStatus === "pending") {
      setMessage("credits are still loading.");
      return;
    }

    if (purchaseStatus !== "available") {
      setMessage("purchases unavailable in this build.");
      return;
    }

    if (storePackages[product.revenueCatPackageId] == null) {
      setMessage("credits offering is still loading.");
      return;
    }

    void triggerSelectionHaptic();
    void performPurchase(product);
  }

  async function performPurchase(product: CreditProduct) {
    setPurchaseBusyId(product.id);
    setSuccessProductId(null);
    setMessage("");

    try {
      const result = await purchaseCreditsPackage(product.revenueCatPackageId);

      if (result.status === "completed") {
        await syncSuccessfulPurchase(product, result).catch((error: unknown) => {
          console.warn("Credit purchase history sync failed.", error);
        });
        const nextBalance = await getRevenueCatCreditBalance({ forceRefresh: true }).catch(
          () => balance + product.totalCredits,
        );
        setMessage("credits added.");
        setBalance(nextBalance);
        runPurchaseSuccess(product.id, nextBalance);
        await triggerNotificationHaptic(Haptics.NotificationFeedbackType.Success);
        return;
      }

      setMessage(result.message);
      await triggerSelectionHaptic();
    } catch (error) {
      console.error(error);
      setMessage(
        error instanceof Error
          ? error.message
          : "purchase failed.",
      );
      await triggerNotificationHaptic(Haptics.NotificationFeedbackType.Error);
    } finally {
      setPurchaseBusyId(null);
    }
  }

  async function syncSuccessfulPurchase(
    product: CreditProduct,
    result: Extract<Awaited<ReturnType<typeof purchaseCreditsPackage>>, { status: "completed" }>,
  ) {
    const [identityId, session] = await Promise.all([
      getMobileApiIdentityId(),
      getStoredBackendAuthSession(),
    ]);
    const api = getAnkyApiClient();

    if (api == null) {
      return;
    }

    const transactionId =
      result.transactionId.trim().length > 0
        ? result.transactionId
        : `${result.productId}:${result.purchasedAt}`;
    const response = await api.syncCreditPurchaseHistory(
      {
        identityId,
        packageId: product.id,
        productId: result.productId,
        purchaseToken: result.purchaseToken,
        purchasedAt: result.purchasedAt,
        transactionId,
      },
      session?.sessionToken,
    );

    setLedgerEntries(response.entries);
  }

  function setBalanceImmediately(nextBalance: number) {
    displayBalanceRef.current = nextBalance;
    setDisplayBalance(nextBalance);
    balanceValue.setValue(nextBalance);
  }

  function animateBalanceTo(nextBalance: number) {
    balanceValue.stopAnimation();
    balanceValue.setValue(displayBalanceRef.current);
    Animated.timing(balanceValue, {
      duration: 900,
      easing: Easing.out(Easing.cubic),
      toValue: nextBalance,
      useNativeDriver: false,
    }).start(() => {
      displayBalanceRef.current = nextBalance;
      setDisplayBalance(nextBalance);
    });
  }

  function runPurchaseSuccess(productId: string, nextBalance: number) {
    setSuccessProductId(productId);
    animateBalanceTo(nextBalance);

    const cardLayout = cardLayouts[productId];
    const from =
      cardLayout != null
        ? {
            ...cardLayout,
            x: cardLayout.x + (packListLayout?.x ?? 0),
            y: cardLayout.y + (packListLayout?.y ?? 0),
          }
        : null;
    if (from != null && heroLayout != null) {
      setTransferRun({ from, productId, to: heroLayout });
      transferProgress.setValue(0);
      Animated.timing(transferProgress, {
        duration: 940,
        easing: Easing.inOut(Easing.cubic),
        toValue: 1,
        useNativeDriver: true,
      }).start(() => {
        setTransferRun(null);
      });
    }

    setTimeout(() => {
      setSuccessProductId((current) => (current === productId ? null : current));
    }, 1800);
  }

  return (
    <YouDetailShell
      onBack={() => navigation.goBack()}
      subtitle="fuel reflections and deeper mirrors."
      title="credits"
    >
      <View style={styles.creditsScene}>
        <View onLayout={(event) => setHeroLayout(event.nativeEvent.layout)}>
          <YouHeroCard
            icon={assets.icons.credits}
            subtitle="write freely. spend credits only when you ask anky to reflect."
            title={`${displayBalance} available`}
          />
        </View>

        <CreditTransferTrail progress={transferProgress} run={transferRun} />

        <CreditCostCard />

        <View
          onLayout={(event) => setPackListLayout(event.nativeEvent.layout)}
          style={styles.stack}
        >
          {CREDIT_PRODUCTS.map((product) => (
            <CreditProductRow
              key={product.id}
              onLayout={(layout) => {
                setCardLayouts((current) => ({ ...current, [product.id]: layout }));
              }}
              onPress={() => startPurchase(product)}
              product={product}
              state={getCreditProductVisualState({
                busyId: purchaseBusyId,
                productId: product.id,
                successId: successProductId,
              })}
              storePackage={storePackages[product.revenueCatPackageId]}
            />
          ))}
        </View>
      </View>

      <SectionTitle label="history" />
      {ledgerEntries.length === 0 ? (
        <Text style={styles.creditHistoryEmpty}>no credit history yet.</Text>
      ) : (
        <View style={styles.stack}>
          {ledgerEntries.map((entry) => (
            <CreditHistoryRow entry={entry} key={entry.id} />
          ))}
        </View>
      )}

      <InlineMessage text={message} />
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
          label={selectedLoom == null ? "mint loom" : "view loom"}
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

function WalletExportModal({
  onClose,
  url,
  visible,
}: {
  onClose: () => void;
  url: string;
  visible: boolean;
}) {
  return (
    <Modal animationType="slide" onRequestClose={onClose} visible={visible}>
      <ScreenBackground variant="plain">
        <View style={styles.walletExportHeader}>
          <SubtleIconButton accessibilityLabel="go back" icon="←" onPress={onClose} />
          <View style={styles.walletExportHeaderCopy}>
            <Text style={styles.walletExportTitle}>export wallet</Text>
            <Text style={styles.walletExportSubtitle}>
              secure export opens in a hosted privy page.
            </Text>
          </View>
          <View style={styles.headerSide} />
        </View>
        {url.length === 0 ? (
          <View style={styles.walletExportEmpty}>
            <Text style={styles.rowTitle}>wallet export needs configuration</Text>
            <Text style={styles.rowSubtitle}>
              set EXPO_PUBLIC_PRIVY_WALLET_EXPORT_URL to a hosted React Privy export page.
            </Text>
          </View>
        ) : (
          // Privy embedded wallet key export needs a secure hosted browser context.
          // Do not attempt to export private keys directly from native code.
          <WebView
            source={{ uri: url }}
            style={styles.walletExportWebView}
          />
        )}
      </ScreenBackground>
    </Modal>
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

function CreditProductRow({
  onLayout,
  onPress,
  product,
  state,
  storePackage,
}: {
  onLayout?: (layout: LayoutRectangle) => void;
  onPress: () => void;
  product: CreditProduct;
  state: CreditProductVisualState;
  storePackage?: AnkyCreditStorePackage;
}) {
  const active = state === "processing";
  const dimmed = state === "dimmed";
  const success = state === "success";
  const pulse = useRef(new Animated.Value(0)).current;
  const priceLabel = storePackage?.priceLabel ?? product.fallbackPriceLabel;

  useEffect(() => {
    if (!active) {
      pulse.stopAnimation();
      pulse.setValue(success ? 1 : 0);
      return;
    }

    const animation = Animated.loop(
      Animated.sequence([
        Animated.timing(pulse, {
          duration: 820,
          easing: Easing.inOut(Easing.quad),
          toValue: 1,
          useNativeDriver: false,
        }),
        Animated.timing(pulse, {
          duration: 820,
          easing: Easing.inOut(Easing.quad),
          toValue: 0,
          useNativeDriver: false,
        }),
      ]),
    );

    animation.start();

    return () => {
      animation.stop();
    };
  }, [active, pulse, success]);

  return (
    <Animated.View
      onLayout={(event) => onLayout?.(event.nativeEvent.layout)}
      style={[
        styles.creditProductShell,
        dimmed && styles.creditProductDimmed,
        {
          shadowOpacity: pulse.interpolate({
            inputRange: [0, 1],
            outputRange: success ? [0.18, 0.3] : [0.1, 0.28],
          }),
          transform: [
            {
              scale: pulse.interpolate({
                inputRange: [0, 1],
                outputRange: [1, active ? 1.01 : 1],
              }),
            },
          ],
        },
      ]}
    >
      <Pressable
        accessibilityRole="button"
        disabled={active || dimmed}
        onPress={onPress}
        style={({ pressed }) => [
          styles.creditProductPanel,
          product.recommended && styles.creditProductRecommended,
          active && styles.creditProductProcessing,
          success && styles.creditProductSuccess,
          pressed && !active && !dimmed && styles.pressed,
        ]}
      >
        <View pointerEvents="none" style={styles.creditProductAura} />
        {active ? <Animated.View pointerEvents="none" style={[styles.creditProductShimmer, {
          opacity: pulse.interpolate({
            inputRange: [0, 1],
            outputRange: [0.18, 0.5],
          }),
        }]} /> : null}
        <View style={styles.creditProductIconFrame}>
          <Image accessibilityIgnoresInvertColors source={assets.icons.credits} style={styles.rowIcon} />
        </View>
        <View style={styles.creditProductCopy}>
          <View style={styles.creditProductTitleRow}>
            <Text style={styles.creditProductTitle}>{product.title}</Text>
            {product.recommended ? <Pill label="recommended" variant="highlight" /> : null}
          </View>
          <Text style={styles.creditProductSubtitle}>{product.description}</Text>
        </View>
        <Text style={styles.creditProductPrice}>{priceLabel}</Text>
      </Pressable>
    </Animated.View>
  );
}

function CreditCostCard() {
  return (
    <View style={styles.creditCostCard}>
      {[
        "1 credit = simple mirror",
        "5 credits = full mirror",
        "writing is always free",
      ].map((line) => (
        <View key={line} style={styles.creditCostLine}>
          <View style={styles.creditCostDot} />
          <Text style={styles.creditCostText}>{line}</Text>
        </View>
      ))}
    </View>
  );
}

function CreditHistoryRow({ entry }: { entry: CreditLedgerEntry }) {
  const positive = entry.amount > 0;
  const amount = `${positive ? "+" : ""}${entry.amount}`;

  return (
    <YouInfoRow
      icon={assets.icons.credits}
      rightText={amount}
      subtitle={formatShortDate(entry.createdAt)}
      title={entry.label}
      variant={positive ? "highlight" : "normal"}
    />
  );
}

function CreditTransferTrail({
  progress,
  run,
}: {
  progress: Animated.Value;
  run: CreditTransferRun | null;
}) {
  if (run == null) {
    return null;
  }

  const startX = run.from.x + run.from.width * 0.5 - 14;
  const startY = run.from.y + run.from.height * 0.42 - 14;
  const endX = run.to.x + run.to.width * 0.5 - 14;
  const endY = run.to.y + run.to.height * 0.52 - 14;
  const translateX = progress.interpolate({
    inputRange: [0, 1],
    outputRange: [startX, endX],
  });
  const translateY = progress.interpolate({
    inputRange: [0, 1],
    outputRange: [startY, endY],
  });
  const opacity = progress.interpolate({
    inputRange: [0, 0.15, 0.82, 1],
    outputRange: [0, 1, 0.9, 0],
  });
  const scale = progress.interpolate({
    inputRange: [0, 0.55, 1],
    outputRange: [0.7, 1.12, 0.76],
  });

  return (
    <Animated.View
      pointerEvents="none"
      style={[
        styles.creditTransferOrb,
        {
          opacity,
          transform: [{ translateX }, { translateY }, { scale }],
        },
      ]}
    >
      <View style={styles.creditTransferCore} />
    </Animated.View>
  );
}

function getCreditProductVisualState({
  busyId,
  productId,
  successId,
}: {
  busyId: string | null;
  productId: string;
  successId: string | null;
}): CreditProductVisualState {
  if (successId === productId) {
    return "success";
  }

  if (busyId === productId) {
    return "processing";
  }

  if (busyId != null) {
    return "dimmed";
  }

  return "idle";
}

async function triggerSelectionHaptic() {
  try {
    await Haptics.selectionAsync();
  } catch {
    // Haptics are unavailable on some simulators and web builds.
  }
}

async function triggerNotificationHaptic(type: Haptics.NotificationFeedbackType) {
  try {
    await Haptics.notificationAsync(type);
  } catch {
    // Haptics are unavailable on some simulators and web builds.
  }
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
  reflections: number;
  sealReceipts: number;
  sessionIndexEntries: number;
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
    reflections: Math.max(sidecarReflectionCount, sessionReflectionCount),
    sealReceipts: seals.length,
    sessionIndexEntries: sessions.length,
  };
}

function countCompleteAnkys(files: SavedAnkyFile[]): number {
  return files.filter((file) => isCompleteRawAnky(file.raw)).length;
}

function formatRestoreMessage(result: AnkyBackupRestoreResult): string {
  const changed = result.added + result.overwritten + result.mergedIndexEntries;
  const skipped = result.duplicates + result.skippedNewer + result.conflicts + result.invalid;

  return [
    `restore complete: ${changed} item${changed === 1 ? "" : "s"} merged.`,
    skipped === 0 ? null : `${skipped} skipped or already present.`,
  ]
    .filter((line): line is string => line != null)
    .join(" ");
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
  creditCostCard: {
    backgroundColor: "rgba(9, 8, 20, 0.66)",
    borderColor: "rgba(233, 190, 114, 0.24)",
    borderRadius: 16,
    borderWidth: 1,
    gap: 8,
    marginTop: 14,
    paddingHorizontal: 14,
    paddingVertical: 12,
  },
  creditCostDot: {
    backgroundColor: "rgba(242, 211, 146, 0.78)",
    borderRadius: 3,
    height: 6,
    width: 6,
  },
  creditCostLine: {
    alignItems: "center",
    flexDirection: "row",
    gap: 9,
  },
  creditCostText: {
    color: "rgba(216, 201, 212, 0.78)",
    fontFamily: SERIF,
    fontSize: 12.5,
    lineHeight: 17,
    textTransform: "lowercase",
  },
  creditHistoryEmpty: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 13,
    lineHeight: 18,
    marginTop: 10,
    textAlign: "center",
    textTransform: "lowercase",
  },
  creditProductAura: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(244, 211, 146, 0.035)",
  },
  creditProductCopy: {
    flex: 1,
    minWidth: 0,
  },
  creditProductDimmed: {
    opacity: 0.48,
  },
  creditProductIconFrame: {
    alignItems: "center",
    backgroundColor: "rgba(9, 8, 20, 0.72)",
    borderColor: "rgba(217, 143, 63, 0.46)",
    borderRadius: 16,
    borderWidth: 1,
    height: 46,
    justifyContent: "center",
    marginRight: 12,
    width: 46,
  },
  creditProductPanel: {
    alignItems: "center",
    backgroundColor: "rgba(13, 12, 27, 0.78)",
    borderColor: "rgba(217, 143, 63, 0.48)",
    borderRadius: 18,
    borderWidth: 1,
    flexDirection: "row",
    minHeight: 76,
    overflow: "hidden",
    paddingHorizontal: 12,
    paddingVertical: 12,
  },
  creditProductPrice: {
    color: "rgba(242, 211, 146, 0.92)",
    fontFamily: SERIF,
    fontSize: 13,
    lineHeight: 17,
    marginLeft: 10,
    minWidth: 54,
    textAlign: "right",
  },
  creditProductProcessing: {
    borderColor: "rgba(242, 211, 146, 0.82)",
    backgroundColor: "rgba(31, 21, 54, 0.84)",
  },
  creditProductRecommended: {
    borderColor: "rgba(232, 113, 207, 0.58)",
  },
  creditProductShell: {
    borderRadius: 18,
    shadowColor: GOLD_BRIGHT,
    shadowOffset: { height: 0, width: 0 },
    shadowRadius: 18,
  },
  creditProductShimmer: {
    bottom: -20,
    position: "absolute",
    right: -36,
    top: -20,
    transform: [{ rotate: "12deg" }],
    width: 78,
    backgroundColor: "rgba(242, 211, 146, 0.18)",
  },
  creditProductSubtitle: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 12.5,
    lineHeight: 17,
    marginTop: 2,
    textTransform: "lowercase",
  },
  creditProductSuccess: {
    backgroundColor: "rgba(20, 43, 31, 0.72)",
    borderColor: "rgba(139, 234, 166, 0.72)",
  },
  creditProductTitle: {
    color: GOLD_BRIGHT,
    flexShrink: 1,
    fontFamily: SERIF,
    fontSize: 16,
    lineHeight: 20,
    textTransform: "lowercase",
  },
  creditProductTitleRow: {
    alignItems: "center",
    flexDirection: "row",
    flexWrap: "wrap",
    gap: 8,
  },
  creditsScene: {
    position: "relative",
  },
  creditTransferCore: {
    backgroundColor: "rgba(139, 234, 166, 0.88)",
    borderRadius: 8,
    height: 16,
    shadowColor: "#8BEAA6",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.9,
    shadowRadius: 16,
    width: 16,
  },
  creditTransferOrb: {
    alignItems: "center",
    borderColor: "rgba(242, 211, 146, 0.4)",
    borderRadius: 14,
    borderWidth: 1,
    height: 28,
    justifyContent: "center",
    left: 0,
    position: "absolute",
    top: 0,
    width: 28,
    zIndex: 8,
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
  walletExportEmpty: {
    backgroundColor: PANEL,
    borderColor: "rgba(233, 190, 114, 0.28)",
    borderRadius: 12,
    borderWidth: 1,
    margin: 20,
    padding: 16,
  },
  walletExportHeader: {
    alignItems: "center",
    flexDirection: "row",
    minHeight: 70,
    paddingHorizontal: 18,
    paddingTop: 10,
  },
  walletExportHeaderCopy: {
    alignItems: "center",
    flex: 1,
    paddingHorizontal: 8,
  },
  walletExportSubtitle: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 12,
    lineHeight: 16,
    marginTop: 2,
    textAlign: "center",
    textTransform: "lowercase",
  },
  walletExportTitle: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 24,
    lineHeight: 30,
    textTransform: "lowercase",
  },
  walletExportWebView: {
    backgroundColor: "#080713",
    flex: 1,
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
