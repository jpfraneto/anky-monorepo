import {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
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
import {
  useLoginWithEmail,
  useLoginWithOAuth,
  useLoginWithSiws,
  usePrivy,
} from "@privy-io/expo";

import {
  ANKY_APP_URL,
  getPrivySignInDomain,
  PRIVY_OAUTH_REDIRECT_PATH,
} from "../lib/auth/privyConfig";
import {
  clearBackendAuthSession,
  exchangePrivyAccessTokenForBackendSession,
  hasConfiguredBackend,
} from "../lib/auth/backendSession";
import {
  type ExternalWalletProviderName,
  useExternalSolanaWallet,
} from "../lib/privy/ExternalSolanaWalletProvider";
import { useAnkyPrivyWallet } from "../lib/privy/useAnkyPrivyWallet";
import { shortAddress } from "../lib/solana/loomStorage";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type AuthModalOptions = {
  afterSuccess?: () => void | Promise<void>;
  reason?: string;
};

type AuthModalContextValue = {
  closeAuthModal: () => void;
  openAuthModal: (options?: AuthModalOptions) => void;
};

type OAuthProvider = "apple" | "google";
type WalletProvider = ExternalWalletProviderName;

const AuthModalContext = createContext<AuthModalContextValue | null>(null);

export function AuthModalProvider({ children }: { children: ReactNode }) {
  const [request, setRequest] = useState<AuthModalOptions | null>(null);

  const closeAuthModal = useCallback(() => {
    setRequest(null);
  }, []);

  const openAuthModal = useCallback((options: AuthModalOptions = {}) => {
    setRequest(options);
  }, []);

  const value = useMemo(
    () => ({
      closeAuthModal,
      openAuthModal,
    }),
    [closeAuthModal, openAuthModal],
  );

  return (
    <AuthModalContext.Provider value={value}>
      {children}
      <AuthModal
        afterSuccess={request?.afterSuccess}
        onClose={closeAuthModal}
        reason={request?.reason}
        visible={request != null}
      />
    </AuthModalContext.Provider>
  );
}

export function useAuthModal(): AuthModalContextValue {
  const value = useContext(AuthModalContext);

  if (value == null) {
    throw new Error("useAuthModal must be used inside AuthModalProvider.");
  }

  return value;
}

function AuthModal({
  afterSuccess,
  onClose,
  reason,
  visible,
}: {
  afterSuccess?: () => void | Promise<void>;
  onClose: () => void;
  reason?: string;
  visible: boolean;
}) {
  const { getAccessToken, isReady, logout, user } = usePrivy();
  const oauth = useLoginWithOAuth();
  const emailLogin = useLoginWithEmail();
  const siws = useLoginWithSiws();
  const externalWallet = useExternalSolanaWallet();
  const privyWallet = useAnkyPrivyWallet();
  const attemptedWalletLoginRef = useRef<string | null>(null);
  const walletLoginInFlight = useRef(false);
  const [code, setCode] = useState("");
  const [email, setEmail] = useState("");
  const [emailCodeSent, setEmailCodeSent] = useState(false);
  const [message, setMessage] = useState("");
  const [pendingWallet, setPendingWallet] = useState<WalletProvider | null>(null);
  const [working, setWorking] = useState(false);

  const finishPrivyLogin = useCallback(async () => {
    const accessToken = await getAccessToken();

    if (accessToken != null && hasConfiguredBackend()) {
      await exchangePrivyAccessTokenForBackendSession(accessToken);
    }

    await afterSuccess?.();
    setMessage("connected.");
    onClose();
  }, [afterSuccess, getAccessToken, onClose]);

  const completeWalletLogin = useCallback(
    async (
      walletProvider: WalletProvider,
      address: string,
      signMessage: (message: string) => Promise<{ signature: string }>,
    ) => {
      if (walletLoginInFlight.current) {
        return;
      }

      walletLoginInFlight.current = true;
      setWorking(true);
      setMessage("sign the wallet message to continue.");

      try {
        const { message: siwsMessage } = await siws.generateMessage({
          from: {
            domain: getPrivySignInDomain(),
            uri: ANKY_APP_URL,
          },
          wallet: { address },
        });
        const { signature } = await signMessage(siwsMessage);

        await siws.login({
          message: siwsMessage,
          signature,
          wallet: {
            connectorType: "deeplink",
            walletClientType: walletProvider,
          },
        });
        setPendingWallet(null);
        await finishPrivyLogin();
      } catch (error) {
        setPendingWallet(null);
        setMessage(error instanceof Error ? error.message : "wallet login failed.");
      } finally {
        walletLoginInFlight.current = false;
        setWorking(false);
      }
    },
    [finishPrivyLogin, siws],
  );

  useEffect(() => {
    if (!visible) {
      setCode("");
      setEmail("");
      setEmailCodeSent(false);
      setMessage("");
      setPendingWallet(null);
      setWorking(false);
      attemptedWalletLoginRef.current = null;
    }
  }, [visible]);

  useEffect(() => {
    if (!visible || !isReady || user != null || walletLoginInFlight.current) {
      return;
    }

    const wallet =
      pendingWallet == null
        ? externalWallet.activeWallet
        : externalWallet.wallets[pendingWallet];

    if (wallet == null) {
      return;
    }

    const loginKey = getWalletLoginKey(wallet.provider, wallet.address);

    if (pendingWallet == null && attemptedWalletLoginRef.current === loginKey) {
      return;
    }

    attemptedWalletLoginRef.current = loginKey;
    void completeWalletLogin(wallet.provider, wallet.address, wallet.signMessage);
  }, [
    completeWalletLogin,
    externalWallet.activeWallet,
    externalWallet.wallets,
    isReady,
    pendingWallet,
    user,
    visible,
  ]);

  const connectedExternalWallet = externalWallet.activeWallet;
  const isBusy =
    working ||
    oauth.state.status === "loading" ||
    emailLogin.state.status === "sending-code" ||
    emailLogin.state.status === "submitting-code" ||
    pendingWallet != null;
  const isLoggedIn = user != null;
  const needsWalletLogin = !isLoggedIn && connectedExternalWallet != null;

  async function handleOAuth(provider: OAuthProvider) {
    setWorking(true);
    setMessage("");

    try {
      await oauth.login({
        provider,
        redirectUri: PRIVY_OAUTH_REDIRECT_PATH,
      });
      await finishPrivyLogin();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : `${provider} login failed.`);
    } finally {
      setWorking(false);
    }
  }

  async function handleSendEmailCode() {
    const trimmedEmail = email.trim();

    if (trimmedEmail.length === 0) {
      setMessage("enter an email first.");
      return;
    }

    setWorking(true);
    setMessage("");

    try {
      await emailLogin.sendCode({ email: trimmedEmail });
      setEmailCodeSent(true);
      setMessage("check your email for the code.");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "could not send email code.");
    } finally {
      setWorking(false);
    }
  }

  async function handleEmailLogin() {
    const trimmedEmail = email.trim();
    const trimmedCode = code.trim();

    if (trimmedEmail.length === 0 || trimmedCode.length === 0) {
      setMessage("enter the email code first.");
      return;
    }

    setWorking(true);
    setMessage("");

    try {
      await emailLogin.loginWithCode({
        code: trimmedCode,
        email: trimmedEmail,
      });
      await finishPrivyLogin();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "email login failed.");
    } finally {
      setWorking(false);
    }
  }

  async function handleWalletConnect(provider: WalletProvider) {
    const connectedWallet = externalWallet.wallets[provider];

    if (connectedWallet != null) {
      externalWallet.setActiveProvider(provider);

      if (user == null) {
        attemptedWalletLoginRef.current = getWalletLoginKey(
          provider,
          connectedWallet.address,
        );
        await completeWalletLogin(provider, connectedWallet.address, connectedWallet.signMessage);
        return;
      }

      setMessage(`${connectedWallet.label} connected.`);
      return;
    }

    setWorking(true);
    setPendingWallet(provider);
    setMessage(`opening ${provider}.`);

    try {
      await externalWallet.connectWallet(provider);
    } catch (error) {
      setPendingWallet(null);
      setMessage(error instanceof Error ? error.message : `${provider} did not connect.`);
    } finally {
      setWorking(false);
    }
  }

  async function handleCreateEmbeddedWallet() {
    setWorking(true);
    setMessage("");

    try {
      await privyWallet.createWallet();
      setMessage("embedded solana wallet ready.");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "wallet creation failed.");
    } finally {
      setWorking(false);
    }
  }

  async function handleLogout() {
    setWorking(true);
    setMessage("");

    try {
      await Promise.allSettled(
        (["phantom", "backpack"] as const).map((provider) =>
          externalWallet.wallets[provider] == null
            ? Promise.resolve()
            : externalWallet.disconnectWallet(provider),
        ),
      );
      await clearBackendAuthSession();
      await logout();
      await afterSuccess?.();
      setMessage("logged out.");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "logout failed.");
    } finally {
      setWorking(false);
    }
  }

  return (
    <Modal animationType="fade" onRequestClose={onClose} transparent visible={visible}>
      <KeyboardAvoidingView
        behavior={Platform.OS === "ios" ? "padding" : undefined}
        style={styles.modalRoot}
      >
        <Pressable accessibilityRole="button" onPress={onClose} style={styles.scrim} />
        <View style={styles.sheet}>
          <ScrollView
            contentContainerStyle={styles.sheetContent}
            keyboardShouldPersistTaps="handled"
            showsVerticalScrollIndicator={false}
          >
            <View style={styles.header}>
              <Text style={styles.title}>enter anky</Text>
              <Text style={styles.subtitle}>
                {reason ??
                  "your writing stays on this device unless you ask anky to process it."}
              </Text>
            </View>

            {isReady && isLoggedIn ? (
              <View style={styles.panel}>
                <Text style={styles.label}>signed in</Text>
                <Text numberOfLines={2} style={styles.connected}>
                  {user.id}
                </Text>
                <Text style={styles.note}>
                  {privyWallet.hasEmbeddedWallet
                    ? `embedded wallet ${shortAddress(privyWallet.embeddedPublicKey ?? "", 6)}`
                    : "embedded wallet not created."}
                </Text>
                <View style={styles.buttonGroup}>
                  {!privyWallet.hasEmbeddedWallet ? (
                    <AuthButton
                      disabled={isBusy}
                      label="create embedded wallet"
                      onPress={() => void handleCreateEmbeddedWallet()}
                    />
                  ) : null}
                  <AuthButton disabled={isBusy} label="log out" onPress={() => void handleLogout()} />
                </View>
              </View>
            ) : null}

            {isReady && !isLoggedIn ? (
              <View style={styles.panel}>
                <Text style={styles.label}>email</Text>
                <TextInput
                  autoCapitalize="none"
                  autoCorrect={false}
                  keyboardType="email-address"
                  onChangeText={setEmail}
                  placeholder="email"
                  placeholderTextColor="rgba(255, 240, 201, 0.42)"
                  style={styles.input}
                  textContentType="emailAddress"
                  value={email}
                />
                {emailCodeSent ? (
                  <TextInput
                    autoCapitalize="none"
                    keyboardType="number-pad"
                    onChangeText={setCode}
                    placeholder="code"
                    placeholderTextColor="rgba(255, 240, 201, 0.42)"
                    style={styles.input}
                    textContentType="oneTimeCode"
                    value={code}
                  />
                ) : null}
                <AuthButton
                  disabled={isBusy}
                  label={emailCodeSent ? "verify code" : "continue with email"}
                  onPress={() => void (emailCodeSent ? handleEmailLogin() : handleSendEmailCode())}
                />
              </View>
            ) : null}

            {isReady && !isLoggedIn ? (
              <View style={styles.buttonGroup}>
                <AuthButton
                  disabled={isBusy}
                  label="continue with apple"
                  onPress={() => void handleOAuth("apple")}
                  variant="secondary"
                />
                <AuthButton
                  disabled={isBusy}
                  label="continue with google"
                  onPress={() => void handleOAuth("google")}
                  variant="secondary"
                />
                <AuthButton
                  disabled={
                    isBusy || (isLoggedIn && connectedExternalWallet?.provider === "phantom")
                  }
                  label={
                    connectedExternalWallet?.provider === "phantom"
                      ? isLoggedIn
                        ? "phantom connected"
                        : "finish phantom login"
                      : "connect phantom"
                  }
                  onPress={() => void handleWalletConnect("phantom")}
                  variant="secondary"
                />
              </View>
            ) : null}

            {connectedExternalWallet == null ? null : (
              <Text style={styles.note}>
                {connectedExternalWallet.label} {shortAddress(connectedExternalWallet.address, 6)}{" "}
                {needsWalletLogin
                  ? "connected. sign once to finish login."
                  : "is ready for loom actions."}
              </Text>
            )}
            {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}

            <Pressable accessibilityRole="button" onPress={onClose} style={styles.closeButton}>
              <Text style={styles.closeText}>stay local</Text>
            </Pressable>
          </ScrollView>
        </View>
      </KeyboardAvoidingView>
    </Modal>
  );
}

function AuthButton({
  disabled = false,
  label,
  onPress,
  variant = "primary",
}: {
  disabled?: boolean;
  label: string;
  onPress: () => void;
  variant?: "primary" | "secondary";
}) {
  return (
    <Pressable
      accessibilityRole="button"
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => [
        styles.authButton,
        variant === "secondary" && styles.authButtonSecondary,
        disabled && styles.disabled,
        pressed && !disabled && styles.pressed,
      ]}
    >
      <Text style={[styles.authButtonText, variant === "secondary" && styles.authButtonTextSecondary]}>
        {label}
      </Text>
    </Pressable>
  );
}

function getWalletLoginKey(provider: WalletProvider, address: string): string {
  return `${provider}:${address}`;
}

const styles = StyleSheet.create({
  authButton: {
    alignItems: "center",
    backgroundColor: "rgba(232, 200, 121, 0.18)",
    borderColor: "rgba(232, 200, 121, 0.54)",
    borderRadius: 8,
    borderWidth: 1,
    minHeight: 50,
    justifyContent: "center",
    paddingHorizontal: spacing.md,
  },
  authButtonSecondary: {
    backgroundColor: "rgba(255,255,255,0.045)",
    borderColor: "rgba(232, 200, 121, 0.22)",
  },
  authButtonText: {
    color: ankyColors.gold,
    fontSize: fontSize.md,
    fontWeight: "700",
    textAlign: "center",
    textTransform: "lowercase",
  },
  authButtonTextSecondary: {
    color: ankyColors.text,
  },
  buttonGroup: {
    gap: spacing.sm,
    marginTop: spacing.md,
  },
  closeButton: {
    alignItems: "center",
    paddingVertical: spacing.md,
  },
  closeText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  connected: {
    color: ankyColors.text,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.xs,
  },
  disabled: {
    opacity: 0.48,
  },
  header: {
    alignItems: "center",
    marginBottom: spacing.lg,
  },
  input: {
    backgroundColor: "rgba(255,255,255,0.045)",
    borderColor: "rgba(232, 200, 121, 0.22)",
    borderRadius: 8,
    borderWidth: 1,
    color: ankyColors.text,
    fontSize: fontSize.md,
    marginTop: spacing.md,
    paddingHorizontal: spacing.md,
    paddingVertical: 13,
  },
  label: {
    color: ankyColors.gold,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
  message: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
    textAlign: "center",
  },
  modalRoot: {
    flex: 1,
    justifyContent: "flex-end",
  },
  note: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  panel: {
    backgroundColor: "rgba(10, 9, 25, 0.94)",
    borderColor: "rgba(232, 200, 121, 0.22)",
    borderRadius: 8,
    borderWidth: 1,
    padding: spacing.md,
  },
  pressed: {
    opacity: 0.72,
  },
  scrim: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(0, 0, 0, 0.62)",
  },
  sheet: {
    backgroundColor: "rgba(8, 7, 19, 0.98)",
    borderColor: "rgba(232, 200, 121, 0.28)",
    borderTopLeftRadius: 20,
    borderTopRightRadius: 20,
    borderWidth: 1,
    maxHeight: "88%",
    overflow: "hidden",
  },
  sheetContent: {
    padding: spacing.xl,
    paddingBottom: spacing.lg,
  },
  subtitle: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 21,
    marginTop: spacing.sm,
    maxWidth: 300,
    textAlign: "center",
    textTransform: "lowercase",
  },
  title: {
    color: ankyColors.gold,
    fontSize: fontSize.xxl,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
});
