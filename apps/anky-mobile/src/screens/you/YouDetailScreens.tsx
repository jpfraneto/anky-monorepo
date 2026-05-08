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
import {
  clearSelectedLoom,
  getSelectedLoomForWallet,
  shortAddress,
  type SelectedLoom,
} from "../../lib/solana/loomStorage";
import type { MobileSealPointsHistory } from "../../lib/api/types";
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
type CreditHistoryStatus = "loading" | "ready" | "requires_account" | "unavailable";
type CreditHistoryEntry = CreditLedgerEntry & {
  optimistic?: boolean;
  syncing?: boolean;
};
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
  const loginMethod = getAccountLoginMethod({
    email,
    externalWalletLabel: externalWallet.activeWallet?.label,
    user,
    walletKind: wallet.walletKind,
  });
  const walletLabel =
    wallet.publicKey == null
      ? "wallet setup pending"
      : shortAddress(wallet.publicKey, 6);
  const identityTitle = connected ? `logged in via ${loginMethod}` : "local account";
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
      await clearSelectedLoom();
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
          subtitle="your solana wallet for loom minting and hash sealing."
          title="wallet"
        >
          {wallet.publicKey == null ? null : (
            <Text selectable style={styles.walletAddressText}>
              {wallet.publicKey}
            </Text>
          )}
        </YouInfoRow>
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

        {connected ? (
          <YouActionButton
            label="logout"
            onPress={() => void disconnectAccount()}
            variant="secondary"
          />
        ) : <YouActionButton
        label={"log in / connect"}
        onPress={() =>
          openAuthModal({
            reason: "account features are optional. writing stays local.",
          })
        }
      />}
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
      subtitle="backup, restore, or clear this device."
      title="local data"
    >
      <Text style={styles.backupSummary}>
        {formatBackupSummary(summary)}
      </Text>

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
      <Text style={styles.backupWarning}>
        backups may include plaintext writing, reflections, conversations, images, and local seal
        receipts. keep the zip somewhere you trust.
      </Text>
    </YouDetailShell>
  );
}

export function CreditsInfoScreen({ navigation }: CreditsInfoProps) {
  const { openAuthModal } = useAuthModal();
  const [balance, setBalance] = useState(0);
  const [balanceLoading, setBalanceLoading] = useState(true);
  const [displayBalance, setDisplayBalance] = useState(0);
  const balanceValue = useRef(new Animated.Value(0)).current;
  const displayBalanceRef = useRef(0);
  const transferProgress = useRef(new Animated.Value(0)).current;
  const [message, setMessage] = useState("");
  const [purchaseBusyId, setPurchaseBusyId] = useState<string | null>(null);
  const [purchaseStatus, setPurchaseStatus] =
    useState<RevenueCatCreditStatus>(getRevenueCatCreditStatus());
  const [historyStatus, setHistoryStatus] = useState<CreditHistoryStatus>("loading");
  const [ledgerEntries, setLedgerEntries] = useState<CreditHistoryEntry[]>([]);
  const [successProductId, setSuccessProductId] = useState<string | null>(null);
  const [transferRun, setTransferRun] = useState<CreditTransferRun | null>(null);
  const [heroLayout, setHeroLayout] = useState<LayoutRectangle | null>(null);
  const [historyReloadNonce, setHistoryReloadNonce] = useState(0);
  const [packListLayout, setPackListLayout] = useState<LayoutRectangle | null>(null);
  const [cardLayouts, setCardLayouts] = useState<Record<string, LayoutRectangle>>({});
  const pendingPurchaseSyncsRef = useRef<
    Array<{
      product: CreditProduct;
      result: Extract<Awaited<ReturnType<typeof purchaseCreditsPackage>>, { status: "completed" }>;
      transactionId: string;
    }>
  >([]);
  const welcomeGiftAttemptedForUserRef = useRef<string | null>(null);
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
      if (mounted) {
        setHistoryStatus("loading");
        setBalanceLoading(true);
      }

      const [localBalance, session, identityId] = await Promise.all([
        getReflectionCreditBalance(),
        getStoredBackendAuthSession(),
        getMobileApiIdentityId(),
      ]);
      let nextBalance = localBalance;
      let nextLedgerEntries: CreditLedgerEntry[] | null = null;
      let nextHistoryStatus: CreditHistoryStatus = "loading";
      let nextStorePackages: Partial<Record<AnkyRevenueCatPackageId, AnkyCreditStorePackage>> = {};
      let nextMessage = "";
      const api = getAnkyApiClient();

      await configureRevenueCat().catch((error: unknown) => {
        console.warn("RevenueCat credits unavailable.", error);
      });

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

      if (api == null) {
        nextHistoryStatus = "unavailable";
        devCreditLog("history unavailable: api client is not configured");
      } else if (session == null) {
        nextHistoryStatus = "requires_account";
        devCreditLog("history requires backend session", { identityId });
      } else {
        let historyLoaded = false;

        try {
          devCreditLog("fetching credit history", { identityId, userId: session.userId });
          const historyResponse = await api.getCreditLedgerHistory({
            identityId,
            sessionToken: session.sessionToken,
          });
          devCreditLog("credit history response", historyResponse);
          nextLedgerEntries = historyResponse.entries;
          nextHistoryStatus = "ready";
          historyLoaded = true;
        } catch (error) {
          devCreditLog("credit history error", error);
          console.warn("Credit history load failed.", error);
          nextHistoryStatus = "unavailable";
        }

        if (welcomeGiftAttemptedForUserRef.current !== session.userId) {
          devCreditLog("calling welcome gift endpoint", { userId: session.userId });

          try {
            const giftResponse = await api.claimWelcomeCreditGift(session.sessionToken);
            devCreditLog("welcome gift response", giftResponse);
            welcomeGiftAttemptedForUserRef.current = session.userId;
            nextLedgerEntries = giftResponse.entries;
          } catch (error) {
            devCreditLog("welcome gift error", error);
            console.warn("Welcome credit gift failed.", error);
          }

          try {
            devCreditLog("refetching credit history after welcome gift", {
              identityId,
              userId: session.userId,
            });
            const historyResponse = await api.getCreditLedgerHistory({
              identityId,
              sessionToken: session.sessionToken,
            });
            devCreditLog("credit history response after welcome gift", historyResponse);
            nextLedgerEntries = historyResponse.entries;
            nextHistoryStatus = "ready";
            historyLoaded = true;
          } catch (error) {
            devCreditLog("credit history refetch error", error);
            console.warn("Credit history reload failed after welcome gift.", error);
            if (!historyLoaded && nextLedgerEntries == null) {
              nextHistoryStatus = "unavailable";
            }
          }
        }

        if (historyLoaded || nextHistoryStatus === "ready") {
          void retryPendingPurchaseSyncs();
        }
      }

      if (mounted) {
        setBalance(nextBalance);
        setBalanceImmediately(nextBalance);
        setMessage(nextMessage);
        setPurchaseStatus(getRevenueCatCreditStatus());
        setHistoryStatus(nextHistoryStatus);
        setBalanceLoading(false);
        if (nextLedgerEntries != null) {
          setLedgerEntries((current) => mergeServerLedgerEntries(nextLedgerEntries, current));
        } else if (nextHistoryStatus === "requires_account") {
          setLedgerEntries((current) => current.filter((entry) => entry.optimistic === true));
        }
        setStorePackages(nextStorePackages);
      }
    }

    setPurchaseStatus("pending");
    void load().catch((error: unknown) => {
      console.error(error);
      if (mounted) {
        setPurchaseStatus(getRevenueCatCreditStatus());
        setBalanceLoading(false);
      }
    });
    const unsubscribe = navigation.addListener("focus", () => {
      setPurchaseStatus("pending");
      setBalanceLoading(true);
      void load().catch((error: unknown) => {
        console.error(error);
        if (mounted) {
          setPurchaseStatus(getRevenueCatCreditStatus());
          setBalanceLoading(false);
        }
      });
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [historyReloadNonce, navigation]);

  function requestHistorySync() {
    welcomeGiftAttemptedForUserRef.current = null;

    if (historyStatus === "requires_account") {
      openAuthModal({
        afterSuccess: () => setHistoryReloadNonce((current) => current + 1),
        reason: "sync your account to show credit history.",
      });
      return;
    }

    setHistoryReloadNonce((current) => current + 1);
  }

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
        const transactionId = getPurchaseHistoryTransactionId(result);
        addOptimisticPurchaseEntry(product, result, transactionId);
        const nextBalance = await getRevenueCatCreditBalance({ forceRefresh: true }).catch(
          () => balance + product.totalCredits,
        );
        setMessage("credits added.");
        setBalance(nextBalance);
        runPurchaseSuccess(product.id, nextBalance);
        await triggerNotificationHaptic(Haptics.NotificationFeedbackType.Success);
        void syncSuccessfulPurchase(product, result).catch((error: unknown) => {
          devCreditLog("purchase sync error", error);
          console.warn("Credit purchase history sync failed.", error);
          markOptimisticPurchaseSyncing(transactionId);
        });
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

  function addOptimisticPurchaseEntry(
    product: CreditProduct,
    result: Extract<Awaited<ReturnType<typeof purchaseCreditsPackage>>, { status: "completed" }>,
    transactionId: string,
  ) {
    const optimisticEntry = buildOptimisticPurchaseEntry(product, transactionId);

    pendingPurchaseSyncsRef.current = [
      { product, result, transactionId },
      ...pendingPurchaseSyncsRef.current.filter((pending) => pending.transactionId !== transactionId),
    ];
    setHistoryStatus("ready");
    setLedgerEntries((current) => [
      optimisticEntry,
      ...current.filter((entry) => entry.referenceId !== transactionId),
    ]);
  }

  function markOptimisticPurchaseSyncing(transactionId: string) {
    setLedgerEntries((current) =>
      current.map((entry) =>
        entry.referenceId === transactionId && entry.optimistic === true
          ? { ...entry, syncing: true }
          : entry,
      ),
    );
  }

  async function retryPendingPurchaseSyncs() {
    const pending = [...pendingPurchaseSyncsRef.current];

    for (const item of pending) {
      await syncSuccessfulPurchase(item.product, item.result).catch((error: unknown) => {
        devCreditLog("pending purchase sync retry failed", error);
        markOptimisticPurchaseSyncing(item.transactionId);
      });
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
      devCreditLog("purchase sync skipped: api client unavailable");
      markOptimisticPurchaseSyncing(getPurchaseHistoryTransactionId(result));
      return;
    }

    if (session == null) {
      devCreditLog("purchase sync skipped: backend session unavailable", { identityId });
      markOptimisticPurchaseSyncing(getPurchaseHistoryTransactionId(result));
      return;
    }

    const transactionId = getPurchaseHistoryTransactionId(result);
    const payload = {
      identityId,
      packageId: product.id,
      productId: result.productId,
      purchaseToken: result.purchaseToken,
      purchasedAt: result.purchasedAt,
      transactionId,
    };

    devCreditLog("purchase sync payload", payload);
    const response = await api.syncCreditPurchaseHistory(
      payload,
      session.sessionToken,
    );

    devCreditLog("purchase sync response", response);
    pendingPurchaseSyncsRef.current = pendingPurchaseSyncsRef.current.filter(
      (pending) => pending.transactionId !== transactionId,
    );
    setHistoryStatus("ready");
    setLedgerEntries((current) => mergeServerLedgerEntries(response.entries, current));
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
      showBottomOrnament={false}
      subtitle="fuel reflections and deeper mirrors"
      title="credits"
      variant="credits"
    >
      <View style={styles.creditsScene}>
        <CreditsHero
          balance={displayBalance}
          loading={balanceLoading}
          onLayout={(layout) => setHeroLayout(layout)}
        />

        <CreditTransferTrail progress={transferProgress} run={transferRun} />

        <CreditRulesList />

        <SectionTitle label="packages" />

        <View
          onLayout={(event) => setPackListLayout(event.nativeEvent.layout)}
          style={styles.creditPackageList}
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

        {/* <SectionTitle label="history" />
        {ledgerEntries.length === 0 ? (
          <CreditHistoryEmptyState
            balance={balance}
            onSync={requestHistorySync}
            status={historyStatus}
          />
        ) : (
          <View style={styles.creditHistoryList}>
            {ledgerEntries.map((entry) => (
              <CreditHistoryRow entry={entry} key={entry.id} />
            ))}
          </View>
        )} */}

        <InlineMessage text={message} />
      </View>
    </YouDetailShell>
  );
}

export function LoomInfoScreen({ navigation }: LoomInfoProps) {
  const wallet = useAnkyPrivyWallet();
  const [files, setFiles] = useState<SavedAnkyFile[]>([]);
  const [pointsHistory, setPointsHistory] = useState<MobileSealPointsHistory | null>(null);
  const [pointsState, setPointsState] = useState<"idle" | "loading" | "unavailable">("idle");
  const [selectedLoom, setSelectedLoom] = useState<SelectedLoom | null>(null);
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);

  useEffect(() => {
    let mounted = true;

    async function load() {
      const [nextSelectedLoom, nextSessions, nextFiles] = await Promise.all([
        getSelectedLoomForWallet(wallet.publicKey),
        listAnkySessionSummaries(),
        listSavedAnkyFiles(),
      ]);

      if (mounted) {
        setSelectedLoom(nextSelectedLoom);
        setSessions(nextSessions);
        setFiles(nextFiles);
      }

      const api = getAnkyApiClient();

      if (wallet.publicKey == null || api == null) {
        if (mounted) {
          setPointsHistory(null);
          setPointsState(wallet.publicKey == null ? "idle" : "unavailable");
        }
        return;
      }

      if (mounted) {
        setPointsState("loading");
      }

      try {
        const nextPoints = await api.lookupMobileSealPoints(wallet.publicKey);

        if (mounted) {
          setPointsHistory(nextPoints);
          setPointsState("idle");
        }
      } catch (error) {
        console.warn("Could not restore loom points history.", error);
        if (mounted) {
          setPointsHistory(null);
          setPointsState("unavailable");
        }
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
  }, [navigation, wallet.publicKey]);

  const loomState = useMemo(() => buildLoomState(selectedLoom, sessions, wallet.hasWallet), [
    selectedLoom,
    sessions,
    wallet.hasWallet,
  ]);
  const fileByHash = useMemo(() => new Map(files.map((file) => [file.hash, file])), [files]);
  const proofPoints = (pointsHistory?.verifiedSealDays ?? 0) * 2;
  const selectedLabel =
    selectedLoom == null ? "none" : shortAddress(selectedLoom.asset, 5);
  const walletLabel = wallet.publicKey == null ? "not ready" : shortAddress(wallet.publicKey, 6);

  return (
    <YouDetailShell
      onBack={() => navigation.goBack()}
      subtitle="owned by your wallet."
      title="loom"
    >
      <YouHeroCard
        icon={assets.icons.loom}
        status={loomState.status}
        subtitle={
          wallet.publicKey == null
            ? "log in to see the loom for this wallet."
            : selectedLoom == null
              ? "this wallet does not have a loom yet."
              : "this loom belongs to the connected wallet."
        }
        title={selectedLoom == null ? "no loom for this wallet" : selectedLoom.name}
      >
        <Text style={styles.heroBody}>
          {selectedLoom == null
            ? "writing remains fully available."
            : `asset ${selectedLoom.asset}`}
        </Text>
      </YouHeroCard>

      <View style={styles.stack}>
        <YouInfoRow
          icon={assets.icons.account}
          rightText={walletLabel}
          subtitle="the wallet that owns or mints the loom."
          title="wallet"
        />
        {selectedLoom == null ? (
          <YouInfoRow
            badge={loomState.badge}
            icon={assets.icons.loom}
            subtitle={loomState.detail}
            title="loom status"
            variant={loomState.variant}
          />
        ) : (
          <>
            <YouInfoRow
              icon={assets.icons.loom}
              rightText={selectedLabel}
              subtitle={`owned on ${selectedLoom.network}.`}
              title="loom asset"
            />
            <YouInfoRow
              badge={loomState.badge}
              icon={assets.icons.loom}
              subtitle={loomState.detail}
              title="daily seal"
              variant={loomState.variant}
            />
          </>
        )}
        <YouInfoRow
          icon={assets.icons.privacy}
          subtitle="only a hash is sealed. your writing stays private."
          title="hash only — writing stays private"
        />
        {selectedLoom == null ? null : (
          <>
            <YouInfoRow
              icon={assets.icons.loom}
              rightText={pointsHistory == null ? "0" : String(pointsHistory.score)}
              subtitle="seal hash = +1, prove rite = +2."
              title="current score"
            />
            <YouInfoRow
              icon={assets.icons.loom}
              rightText={String(pointsHistory?.uniqueSealDays ?? 0)}
              subtitle="finalized hash seals indexed by the backend."
              title="sealed days"
            />
            <YouInfoRow
              icon={assets.icons.loom}
              rightText={String(proofPoints)}
              subtitle="finalized SP1 receipt points indexed by the backend."
              title="proof points"
            />
            <YouInfoRow
              icon={assets.icons.loom}
              rightText={String(pointsHistory?.streakBonus ?? 0)}
              subtitle="the current backend streak rule."
              title="streak bonus"
            />
          </>
        )}
      </View>

      {selectedLoom == null ? null : (
        <View style={styles.pointHistoryList}>
        <SectionTitle label="points history" />
        {pointsState === "loading" ? (
          <Text style={styles.helperText}>syncing finalized proof-of-practice receipts.</Text>
        ) : pointsHistory == null || pointsHistory.entries.length === 0 ? (
          <Text style={styles.helperText}>
            {pointsState === "unavailable" ? "points history unavailable." : "no indexed points yet."}
          </Text>
        ) : (
          pointsHistory.entries.slice(0, 8).map((entry) => {
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
                  styles.pointHistoryRow,
                  file == null && styles.pointHistoryRowDisabled,
                  pressed && file != null && styles.pressed,
                ]}
              >
                <View style={styles.pointHistoryCopy}>
                  <Text style={styles.pointHistoryTitle}>
                    {formatProfileUtcDay(entry.utcDay)} · +{entry.totalPoints}
                  </Text>
                  <Text style={styles.pointHistoryMeta}>
                    sealed +{entry.sealPoints}
                    {entry.proofPoints > 0 ? ` · proved +${entry.proofPoints}` : ""}
                    {` · ${formatProfileProofStatus(entry.proofStatus)}`}
                    {file == null ? " · not on this device" : ""}
                  </Text>
                </View>
                {file == null ? null : <Text style={styles.pointHistoryChevron}>›</Text>}
              </Pressable>
            );
          })
        )}
        </View>
      )}

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
  showBottomOrnament = true,
  subtitle,
  title,
  titleAccessory,
  variant = "default",
}: {
  children: ReactNode;
  onBack: () => void;
  showBottomOrnament?: boolean;
  subtitle: string;
  title: string;
  titleAccessory?: ReactNode;
  variant?: "credits" | "default";
}) {
  const insets = useSafeAreaInsets();
  const creditsVariant = variant === "credits";

  return (
    <ScreenBackground safe={false} variant="plain">
      <ImageBackground resizeMode="cover" source={assets.background} style={styles.screen}>
        <View pointerEvents="none" style={styles.cosmosWash} />
        {creditsVariant ? <View pointerEvents="none" style={styles.creditsCosmosWash} /> : null}
        <View style={[styles.shell, { paddingTop: insets.top + 10 }]}>
          <View style={[styles.header, creditsVariant && styles.headerCredits]}>
            <SubtleIconButton accessibilityLabel="go back" icon="←" onPress={onBack} />
            <View style={styles.headerCenter}>
              <View style={styles.headerTitleRow}>
                <Text
                  numberOfLines={1}
                  style={[styles.headerTitle, creditsVariant && styles.headerTitleCredits]}
                >
                  {title}
                </Text>
                {titleAccessory}
              </View>
              <Text
                numberOfLines={2}
                style={[styles.headerSubtitle, creditsVariant && styles.headerSubtitleCredits]}
              >
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
            {showBottomOrnament ? <BottomOrnament /> : null}
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

function CreditsHero({
  balance,
  loading,
  onLayout,
}: {
  balance: number;
  loading: boolean;
  onLayout?: (layout: LayoutRectangle) => void;
}) {
  return (
    <View
      onLayout={(event) => onLayout?.(event.nativeEvent.layout)}
      style={styles.creditsHero}
    >
      <View style={styles.creditsHeroMain}>
        <View style={styles.creditsHeroEmblem}>
          <View style={styles.creditsHeroEmblemRing}>
            <Text style={styles.creditsHeroEmblemGlyph}>✧</Text>
          </View>
        </View>
        <View style={styles.creditsHeroBalanceCopy}>
          <Text
            adjustsFontSizeToFit
            numberOfLines={1}
            style={[styles.creditsHeroBalance, loading && styles.creditsHeroBalanceLoading]}
          >
            {loading ? "•••" : balance}
          </Text>
          <Text style={styles.creditsHeroAvailable}>{loading ? "syncing" : "available"}</Text>
        </View>
      </View>
      <Text style={styles.creditsHeroSubtitle}>
        write freely. spend credits only when you ask anky to reflect.
      </Text>
    </View>
  );
}

function CreditRulesList() {
  return (
    <View style={styles.creditRules}>
      <CreditRuleRow icon="✧" label="1 credit = reflection" />
      <CreditRuleRow icon="✦" label="full reflection is coming soon" />
      <CreditRuleRow icon="⌁" label="writing is always free" last />
    </View>
  );
}

function CreditRuleRow({
  icon,
  label,
  last = false,
}: {
  icon: string;
  label: string;
  last?: boolean;
}) {
  return (
    <View style={[styles.creditRuleRow, !last && styles.creditRuleRowBorder]}>
      <View style={styles.creditRuleIconFrame}>
        <Text style={styles.creditRuleIcon}>{icon}</Text>
      </View>
      <Text style={styles.creditRuleText}>{label}</Text>
    </View>
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
  const accessibilityLabel = getCreditProductAccessibilityLabel(product);

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
        accessibilityLabel={accessibilityLabel}
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
        {active || success ? (
          <Animated.View
            pointerEvents="none"
            style={[
              styles.creditProductAura,
              {
                opacity: pulse.interpolate({
                  inputRange: [0, 1],
                  outputRange: success ? [0.26, 0.38] : [0.14, 0.3],
                }),
              },
            ]}
          />
        ) : null}
        <View style={styles.creditProductIconFrame}>
          <Text style={styles.creditProductIconGlyph}>✧</Text>
        </View>
        <View style={styles.creditProductCopy}>
          <View style={styles.creditProductTitleRow}>
            <Text style={styles.creditProductTitle}>{product.title}</Text>
            {product.recommended ? <Pill label="recommended" variant="highlight" /> : null}
          </View>
          <Text style={styles.creditProductSubtitle}>{product.description}</Text>
        </View>
        <View style={styles.creditProductRight}>
          <Text style={styles.creditProductPrice}>{priceLabel}</Text>
          <Text style={styles.creditProductChevron}>›</Text>
        </View>
      </Pressable>
    </Animated.View>
  );
}

function CreditHistoryRow({ entry }: { entry: CreditHistoryEntry }) {
  const positive = entry.amount > 0;
  const amount = `${positive ? "+" : ""}${entry.amount}`;
  const subtitle = getCreditHistorySubtitle(entry);
  const icon = getCreditHistoryIcon(entry);

  return (
    <View style={styles.creditHistoryRow}>
      <View style={styles.creditHistoryIconFrame}>
        <Text style={styles.creditHistoryIcon}>{icon}</Text>
      </View>
      <View style={styles.creditHistoryCopy}>
        <Text style={styles.creditHistoryTitle}>{entry.label}</Text>
        <Text style={styles.creditHistorySubtitle}>{subtitle}</Text>
      </View>
      <View style={styles.creditHistoryRight}>
        <Text
          style={[
            styles.creditHistoryAmount,
            !positive && styles.creditHistoryAmountNegative,
          ]}
        >
          {amount}
        </Text>
        <Text style={styles.creditHistoryDate}>{formatCreditHistoryDate(entry.createdAt)}</Text>
      </View>
    </View>
  );
}

function CreditHistoryEmptyState({
  balance,
  onSync,
  status,
}: {
  balance: number;
  onSync: () => void;
  status: CreditHistoryStatus;
}) {
  const actionLabel = getCreditHistoryActionLabel(status, balance);

  return (
    <View style={styles.creditHistoryEmptyWrap}>
      <Text style={styles.creditHistoryEmpty}>{getCreditHistoryEmptyText(status, balance)}</Text>
      {actionLabel == null ? null : (
        <Pressable
          accessibilityLabel={actionLabel}
          accessibilityRole="button"
          onPress={onSync}
          style={({ pressed }) => [
            styles.creditHistorySyncButton,
            pressed && styles.pressed,
          ]}
        >
          <Text style={styles.creditHistorySyncText}>{actionLabel}</Text>
        </Pressable>
      )}
    </View>
  );
}

function getCreditProductAccessibilityLabel(product: CreditProduct): string {
  if (product.bonusCredits > 0) {
    return `buy ${product.baseCredits} plus ${product.bonusCredits} bonus credits`;
  }

  return `buy ${product.totalCredits} credits`;
}

function getCreditHistorySubtitle(entry: CreditHistoryEntry): string {
  if (entry.syncing === true) {
    return "syncing";
  }

  if (entry.optimistic === true) {
    return "just now";
  }

  if (entry.kind === "purchase" || (entry.amount > 0 && entry.source === "revenuecat")) {
    return getCreditPurchaseSubtitle(entry);
  }

  if (entry.kind === "gift") {
    return "welcome credits";
  }

  if (entry.amount < 0) {
    if (entry.amount === -5) {
      return "full reflection";
    }

    if (entry.amount === -1) {
      return "reflection";
    }

    return "reflection";
  }

  return entry.source.length > 0 ? entry.source : entry.kind;
}

function getCreditPurchaseSubtitle(entry: CreditHistoryEntry): string {
  const packageId = readMetadataString(entry.metadata, "packageId");

  switch (packageId) {
    case "credits_22":
      return "22 credits";
    case "credits_88_bonus_11":
      return "88 + 11 bonus";
    case "credits_333_bonus_88":
      return "333 + 88 bonus";
    default:
      break;
  }

  switch (entry.amount) {
    case 22:
      return "22 credits";
    case 99:
      return "88 + 11 bonus";
    case 421:
      return "333 + 88 bonus";
    default:
      return `${entry.amount} credits`;
  }
}

function getCreditHistoryIcon(entry: CreditHistoryEntry): string {
  if (entry.kind === "purchase" || entry.kind === "gift" || entry.amount > 0) {
    return "◔";
  }

  return "✧";
}

function readMetadataString(metadata: unknown, key: string): string | null {
  if (typeof metadata !== "object" || metadata == null || !(key in metadata)) {
    return null;
  }

  const value = (metadata as Record<string, unknown>)[key];

  return typeof value === "string" && value.length > 0 ? value : null;
}

function formatCreditHistoryDate(value: string): string {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return "recent";
  }

  return date
    .toLocaleDateString(undefined, {
      day: "numeric",
      month: "short",
      year: "numeric",
    })
    .toLowerCase();
}

function getCreditHistoryEmptyText(status: CreditHistoryStatus, balance = 0): string {
  switch (status) {
    case "loading":
      return "loading history...";
    case "requires_account":
      return "sync your account to show credit history.";
    case "unavailable":
      return "history could not sync yet.";
    case "ready":
    default:
      if (balance > 0) {
        return "history has not synced yet.";
      }

      return "no credit history yet.";
  }
}

function getCreditHistoryActionLabel(status: CreditHistoryStatus, balance = 0): string | null {
  switch (status) {
    case "requires_account":
      return "sync account";
    case "unavailable":
      return "retry sync";
    case "ready":
      return balance > 0 ? "sync history" : null;
    case "loading":
    default:
      return null;
  }
}

function buildOptimisticPurchaseEntry(
  product: CreditProduct,
  transactionId: string,
): CreditHistoryEntry {
  return {
    amount: product.totalCredits,
    createdAt: new Date().toISOString(),
    id: `optimistic:${transactionId}`,
    kind: "purchase",
    label: "bought credits",
    optimistic: true,
    referenceId: transactionId,
    source: "revenuecat",
    syncing: false,
    userId: "local",
  };
}

function mergeServerLedgerEntries(
  serverEntries: CreditLedgerEntry[],
  currentEntries: CreditHistoryEntry[],
): CreditHistoryEntry[] {
  const serverIds = new Set(serverEntries.map((entry) => entry.id));
  const serverReferenceIds = new Set(
    serverEntries
      .map((entry) => entry.referenceId)
      .filter((referenceId): referenceId is string => referenceId != null && referenceId.length > 0),
  );
  const unresolvedOptimisticEntries = currentEntries.filter((entry) => {
    if (entry.optimistic !== true) {
      return false;
    }

    if (serverIds.has(entry.id)) {
      return false;
    }

    return entry.referenceId == null || !serverReferenceIds.has(entry.referenceId);
  });

  return [...serverEntries, ...unresolvedOptimisticEntries].sort(compareCreditHistoryEntries);
}

function compareCreditHistoryEntries(
  first: CreditHistoryEntry,
  second: CreditHistoryEntry,
): number {
  const firstTime = Date.parse(first.createdAt);
  const secondTime = Date.parse(second.createdAt);

  if (Number.isNaN(firstTime) && Number.isNaN(secondTime)) {
    return 0;
  }

  if (Number.isNaN(firstTime)) {
    return 1;
  }

  if (Number.isNaN(secondTime)) {
    return -1;
  }

  return secondTime - firstTime;
}

function getPurchaseHistoryTransactionId(
  result: Extract<Awaited<ReturnType<typeof purchaseCreditsPackage>>, { status: "completed" }>,
): string {
  const transactionId = result.transactionId.trim();

  if (transactionId.length > 0) {
    return transactionId;
  }

  return `${result.productId}:${result.purchasedAt}`;
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

function devCreditLog(message: string, payload?: unknown) {
  if (process.env.NODE_ENV === "production") {
    return;
  }

  if (payload === undefined) {
    console.log(`[credits] ${message}`);
    return;
  }

  console.log(`[credits] ${message}`, payload);
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

function formatProfileUtcDay(utcDay: number): string {
  const date = new Date(utcDay * 86_400_000);

  if (Number.isNaN(date.getTime())) {
    return `day ${utcDay}`;
  }

  return date.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
  }).toLowerCase();
}

function formatProfileProofStatus(status: string): string {
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

function SectionTitle({ label }: { label: string }) {
  return (
    <View style={styles.sectionTitleRow}>
      <Text style={styles.sectionTitle}>{label}</Text>
      <View style={styles.sectionTitleLine} />
    </View>
  );
}

function Pill({ label, variant = "normal" }: { label: string; variant?: RowVariant }) {
  return (
    <View
      style={[
        styles.pill,
        variant === "danger" && styles.pillDanger,
        variant === "highlight" && styles.pillHighlight,
      ]}
    >
      <Text
        style={[
          styles.pillText,
          variant === "danger" && styles.pillTextDanger,
          variant === "highlight" && styles.pillTextHighlight,
        ]}
      >
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

function formatBackupSummary(summary: ArchiveSummary): string {
  const parts = [
    `${summary.completeAnkys} ${summary.completeAnkys === 1 ? "anky" : "ankys"}`,
    summary.reflections > 0
      ? `${summary.reflections} ${summary.reflections === 1 ? "reflection" : "reflections"}`
      : null,
    summary.keepWritingThreads > 0
      ? `${summary.keepWritingThreads} ${summary.keepWritingThreads === 1 ? "conversation" : "conversations"}`
      : null,
    summary.sealReceipts > 0
      ? `${summary.sealReceipts} ${summary.sealReceipts === 1 ? "seal receipt" : "seal receipts"}`
      : null,
  ].filter((part): part is string => part != null);

  if (summary.ankyFiles === 0) {
    return "no local archive yet.";
  }

  return `${parts.join(" · ")} stored on this device.`;
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
      badge: "none",
      detail: hasWallet ? "this wallet has no loom yet." : "log in to see the loom for this wallet.",
      status: "no loom",
      variant: "normal",
    };
  }

  if (!hasWallet) {
    return {
      badge: "connect",
      detail: "log in to see the loom for this wallet.",
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

function getAccountLoginMethod({
  email,
  externalWalletLabel,
  user,
  walletKind,
}: {
  email: string | null;
  externalWalletLabel?: string;
  user: unknown;
  walletKind?: "embedded" | "external";
}): string {
  if (externalWalletLabel != null || walletKind === "external") {
    return externalWalletLabel?.toLowerCase() ?? "wallet";
  }

  const linkedAccounts = readArrayField(user, "linkedAccounts");

  if (linkedAccounts.some((account) => hasLinkedAccountProvider(account, "google"))) {
    return "google";
  }

  if (linkedAccounts.some((account) => hasLinkedAccountProvider(account, "apple"))) {
    return "apple";
  }

  if (email != null || linkedAccounts.some((account) => isEmail(readStringField(account, "email")))) {
    return "email";
  }

  return "privy";
}

function hasLinkedAccountProvider(value: unknown, provider: string): boolean {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  return Object.values(value as Record<string, unknown>).some(
    (field) => typeof field === "string" && field.toLowerCase().includes(provider),
  );
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
  backupSummary: {
    color: COPY,
    fontFamily: SERIF,
    fontSize: 15,
    lineHeight: 22,
    marginTop: 4,
    textAlign: "center",
    textTransform: "lowercase",
  },
  backupWarning: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 12,
    lineHeight: 18,
    marginTop: 16,
    textAlign: "center",
    textTransform: "lowercase",
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
    backgroundColor: "rgba(5, 5, 14, 0.34)",
  },
  creditHistoryAmount: {
    color: GOLD,
    fontFamily: SERIF,
    fontSize: 16,
    lineHeight: 20,
    textAlign: "right",
    textTransform: "lowercase",
  },
  creditHistoryAmountNegative: {
    color: "rgba(233, 190, 114, 0.78)",
  },
  creditHistoryCopy: {
    flex: 1,
    minWidth: 0,
    paddingRight: 12,
  },
  creditHistoryDate: {
    color: "rgba(216, 201, 212, 0.58)",
    fontFamily: SERIF,
    fontSize: 11.5,
    lineHeight: 15,
    marginTop: 2,
    textAlign: "right",
    textTransform: "lowercase",
  },
  creditHistoryEmpty: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 14,
    lineHeight: 19,
    textAlign: "center",
    textTransform: "lowercase",
  },
  creditHistoryEmptyWrap: {
    alignItems: "center",
    paddingVertical: 18,
  },
  creditHistoryIcon: {
    color: GOLD,
    fontSize: 20,
    lineHeight: 23,
    textAlign: "center",
  },
  creditHistoryIconFrame: {
    alignItems: "center",
    backgroundColor: "rgba(5, 6, 16, 0.54)",
    borderColor: "rgba(233, 190, 114, 0.48)",
    borderRadius: 21,
    borderWidth: 1,
    height: 42,
    justifyContent: "center",
    marginRight: 14,
    width: 42,
  },
  creditHistoryList: {
    borderTopColor: "rgba(233, 190, 114, 0.18)",
    borderTopWidth: StyleSheet.hairlineWidth,
  },
  creditHistoryRight: {
    alignItems: "flex-end",
    minWidth: 78,
  },
  creditHistoryRow: {
    alignItems: "center",
    borderBottomColor: "rgba(233, 190, 114, 0.16)",
    borderBottomWidth: StyleSheet.hairlineWidth,
    flexDirection: "row",
    minHeight: 66,
    paddingVertical: 10,
  },
  creditHistorySubtitle: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 14,
    lineHeight: 18,
    marginTop: 1,
    textTransform: "lowercase",
  },
  creditHistorySyncButton: {
    alignItems: "center",
    borderColor: "rgba(233, 190, 114, 0.34)",
    borderRadius: 999,
    borderWidth: 1,
    justifyContent: "center",
    marginTop: 10,
    minHeight: 34,
    paddingHorizontal: 16,
  },
  creditHistorySyncText: {
    color: GOLD,
    fontFamily: SERIF,
    fontSize: 13,
    lineHeight: 17,
    textTransform: "lowercase",
  },
  creditHistoryTitle: {
    color: "rgba(242, 211, 146, 0.96)",
    fontFamily: SERIF,
    fontSize: 17,
    lineHeight: 21,
    textTransform: "lowercase",
  },
  creditPackageList: {
    borderTopColor: "rgba(233, 190, 114, 0.18)",
    borderTopWidth: StyleSheet.hairlineWidth,
  },
  creditProductAura: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(242, 211, 146, 0.12)",
  },
  creditProductCopy: {
    flex: 1,
    minWidth: 0,
  },
  creditProductDimmed: {
    opacity: 0.38,
  },
  creditProductChevron: {
    color: "rgba(242, 211, 146, 0.82)",
    fontFamily: SERIF,
    fontSize: 24,
    lineHeight: 28,
    marginLeft: 8,
  },
  creditProductIconFrame: {
    alignItems: "center",
    backgroundColor: "rgba(5, 6, 16, 0.52)",
    borderColor: "rgba(233, 190, 114, 0.58)",
    borderRadius: 24,
    borderWidth: 1,
    height: 48,
    justifyContent: "center",
    marginRight: 14,
    width: 48,
  },
  creditProductIconGlyph: {
    color: GOLD_BRIGHT,
    fontSize: 28,
    lineHeight: 32,
    textAlign: "center",
  },
  creditProductPanel: {
    alignItems: "center",
    backgroundColor: "rgba(9, 8, 20, 0.18)",
    borderColor: "transparent",
    borderRadius: 18,
    borderWidth: 1,
    borderBottomColor: "rgba(233, 190, 114, 0.15)",
    flexDirection: "row",
    minHeight: 74,
    overflow: "hidden",
    paddingHorizontal: 8,
    paddingVertical: 10,
  },
  creditProductPrice: {
    color: GOLD,
    fontFamily: SERIF,
    fontSize: 16,
    lineHeight: 21,
    textAlign: "right",
    textTransform: "lowercase",
  },
  creditProductProcessing: {
    backgroundColor: "rgba(31, 21, 54, 0.38)",
    borderColor: "rgba(242, 211, 146, 0.48)",
    shadowColor: GOLD_BRIGHT,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.2,
    shadowRadius: 16,
  },
  creditProductRecommended: {
    backgroundColor: "rgba(18, 14, 32, 0.44)",
    borderBottomColor: "rgba(177, 83, 214, 0.42)",
    borderColor: "rgba(177, 83, 214, 0.56)",
  },
  creditProductRight: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "flex-end",
    marginLeft: 10,
    minWidth: 88,
  },
  creditProductShell: {
    borderRadius: 18,
    shadowColor: GOLD_BRIGHT,
    shadowOffset: { height: 0, width: 0 },
    shadowRadius: 14,
  },
  creditProductSubtitle: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 14,
    lineHeight: 18,
    marginTop: 3,
    textTransform: "lowercase",
  },
  creditProductSuccess: {
    backgroundColor: "rgba(28, 42, 31, 0.48)",
    borderColor: "rgba(139, 234, 166, 0.46)",
    shadowColor: "#8BEAA6",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.16,
    shadowRadius: 16,
  },
  creditProductTitle: {
    color: GOLD_BRIGHT,
    flexShrink: 1,
    fontFamily: SERIF,
    fontSize: 18,
    lineHeight: 23,
    textTransform: "lowercase",
  },
  creditProductTitleRow: {
    alignItems: "center",
    flexDirection: "row",
    flexWrap: "wrap",
    gap: 10,
  },
  creditRuleIcon: {
    color: GOLD,
    fontSize: 21,
    lineHeight: 25,
    textAlign: "center",
  },
  creditRuleIconFrame: {
    alignItems: "center",
    height: 38,
    justifyContent: "center",
    marginRight: 16,
    width: 34,
  },
  creditRuleRow: {
    alignItems: "center",
    flexDirection: "row",
    minHeight: 54,
  },
  creditRuleRowBorder: {
    borderBottomColor: "rgba(233, 190, 114, 0.18)",
    borderBottomWidth: StyleSheet.hairlineWidth,
  },
  creditRules: {
    marginTop: 22,
  },
  creditRuleText: {
    color: "rgba(244, 231, 206, 0.9)",
    flex: 1,
    fontFamily: SERIF,
    fontSize: 16,
    lineHeight: 21,
    textTransform: "lowercase",
  },
  creditsCosmosWash: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(2, 4, 12, 0.18)",
  },
  creditsHero: {
    alignItems: "center",
    marginTop: 10,
  },
  creditsHeroAvailable: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 24,
    lineHeight: 29,
    marginTop: -6,
    textTransform: "lowercase",
  },
  creditsHeroBalance: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 64,
    lineHeight: 72,
    maxWidth: 152,
    textShadowColor: "rgba(233, 190, 114, 0.22)",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 16,
    textTransform: "lowercase",
  },
  creditsHeroBalanceLoading: {
    opacity: 0.42,
    textShadowRadius: 22,
  },
  creditsHeroBalanceCopy: {
    alignItems: "flex-start",
    flexShrink: 1,
    justifyContent: "center",
    marginLeft: 22,
  },
  creditsHeroEmblem: {
    alignItems: "center",
    borderColor: "rgba(233, 190, 114, 0.12)",
    borderRadius: 50,
    borderWidth: 1,
    height: 100,
    justifyContent: "center",
    width: 100,
  },
  creditsHeroEmblemGlyph: {
    color: GOLD_BRIGHT,
    fontSize: 48,
    lineHeight: 56,
    textAlign: "center",
    textShadowColor: "rgba(177, 83, 214, 0.75)",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 10,
  },
  creditsHeroEmblemRing: {
    alignItems: "center",
    backgroundColor: "rgba(5, 6, 16, 0.45)",
    borderColor: GOLD,
    borderRadius: 36,
    borderWidth: 1,
    height: 72,
    justifyContent: "center",
    width: 72,
  },
  creditsHeroMain: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "center",
    width: "100%",
  },
  creditsHeroSubtitle: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 16,
    lineHeight: 23,
    marginTop: 14,
    maxWidth: 292,
    textAlign: "center",
    textTransform: "lowercase",
  },
  creditsScene: {
    paddingTop: 2,
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
  headerCredits: {
    minHeight: 66,
    paddingHorizontal: 22,
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
  headerSubtitleCredits: {
    color: "rgba(216, 201, 212, 0.72)",
    fontSize: 16,
    lineHeight: 21,
    marginTop: 2,
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
  headerTitleCredits: {
    color: GOLD,
    fontSize: 38,
    lineHeight: 45,
    textShadowColor: "rgba(233, 190, 114, 0.18)",
    textShadowRadius: 18,
  },
  headerTitleRow: {
    alignItems: "center",
    flexDirection: "row",
    gap: 8,
    justifyContent: "center",
    minWidth: 0,
  },
  helperText: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 13,
    lineHeight: 18,
    marginTop: 8,
    textTransform: "lowercase",
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
  pillHighlight: {
    backgroundColor: "rgba(18, 10, 29, 0.52)",
    borderColor: "rgba(177, 83, 214, 0.72)",
    borderRadius: 999,
    paddingHorizontal: 10,
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
  pillTextHighlight: {
    color: "rgba(225, 181, 243, 0.9)",
  },
  pointHistoryChevron: {
    color: GOLD_BRIGHT,
    fontSize: 24,
    marginLeft: 8,
  },
  pointHistoryCopy: {
    flex: 1,
    minWidth: 0,
  },
  pointHistoryList: {
    marginTop: 4,
  },
  pointHistoryMeta: {
    color: COPY_DIM,
    fontFamily: SERIF,
    fontSize: 12,
    lineHeight: 16,
    marginTop: 2,
    textTransform: "lowercase",
  },
  pointHistoryRow: {
    alignItems: "center",
    backgroundColor: "rgba(244, 241, 234, 0.055)",
    borderColor: "rgba(244, 241, 234, 0.13)",
    borderRadius: 8,
    borderWidth: 1,
    flexDirection: "row",
    marginTop: 8,
    minHeight: 58,
    paddingHorizontal: 12,
    paddingVertical: 8,
  },
  pointHistoryRowDisabled: {
    opacity: 0.56,
  },
  pointHistoryTitle: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 15,
    lineHeight: 19,
    textTransform: "lowercase",
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
  walletAddressText: {
    color: "rgba(242, 211, 146, 0.82)",
    fontFamily: SERIF,
    fontSize: 11,
    lineHeight: 16,
    marginTop: 8,
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
    color: GOLD,
    fontFamily: SERIF,
    fontSize: 17,
    lineHeight: 21,
    marginRight: 12,
    textTransform: "lowercase",
  },
  sectionTitleLine: {
    backgroundColor: "rgba(233, 190, 114, 0.16)",
    flex: 1,
    height: StyleSheet.hairlineWidth,
  },
  sectionTitleRow: {
    alignItems: "center",
    flexDirection: "row",
    marginBottom: 4,
    marginTop: 20,
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
