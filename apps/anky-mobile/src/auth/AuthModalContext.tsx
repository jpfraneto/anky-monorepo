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
import * as Clipboard from "expo-clipboard";

import {
  ANKY_APP_URL,
  getPrivySignInDomain,
  PRIVY_OAUTH_REDIRECT_PATH,
} from "../lib/auth/privyConfig";
import {
  type BackendWalletAuthProof,
  exchangePrivyAccessTokenForBackendSession,
  hasConfiguredBackend,
} from "../lib/auth/backendSession";
import { getAnkyApiClient } from "../lib/api/client";
import { configureRevenueCat } from "../lib/credits/revenueCatCredits";
import {
  type ExternalWalletProviderName,
  useExternalSolanaWallet,
} from "../lib/privy/ExternalSolanaWalletProvider";
import { toPrivySiwsSignature } from "../lib/privy/siwsSignature";
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

const CODE_LENGTH = 6;

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
  const { getAccessToken, isReady, user } = usePrivy();
  const oauth = useLoginWithOAuth();
  const emailLogin = useLoginWithEmail();
  const siws = useLoginWithSiws();
  const externalWallet = useExternalSolanaWallet();
  const attemptedWalletLoginRef = useRef<string | null>(null);
  const codeInputRef = useRef<TextInput>(null);
  const finishingVisibleLoginRef = useRef(false);
  const walletLoginInFlight = useRef(false);
  const [code, setCode] = useState("");
  const [email, setEmail] = useState("");
  const [emailCodeSent, setEmailCodeSent] = useState(false);
  const [message, setMessage] = useState("");
  const [pendingWallet, setPendingWallet] = useState<WalletProvider | null>(null);
  const [working, setWorking] = useState(false);

  const finishPrivyLogin = useCallback(async (walletProof?: BackendWalletAuthProof) => {
    const accessToken = await getAccessToken();

    if (accessToken != null && hasConfiguredBackend()) {
      try {
        const session = await exchangePrivyAccessTokenForBackendSession(accessToken, walletProof);
        await configureRevenueCat().catch((error: unknown) => {
          console.warn("RevenueCat identity sync failed after login.", error);
        });
        const api = getAnkyApiClient();
        if (api != null) {
          await api.claimWelcomeCreditGift(session.sessionToken).catch((error: unknown) => {
            console.warn("Welcome credits grant failed after login.", error);
          });
        }
      } catch (error) {
        console.warn("Backend session exchange failed after Privy login.", error);
      }
    }

    try {
      await afterSuccess?.();
    } catch (error) {
      console.warn("Auth success callback failed.", error);
    }

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
        const privySignature = toPrivySiwsSignature(signature);

        await siws.login({
          message: siwsMessage,
          signature: privySignature,
          wallet: {
            connectorType: "deeplink",
            walletClientType: walletProvider,
          },
        });
        setPendingWallet(null);
        await finishPrivyLogin({
          siwsMessage,
          siwsSignature: signature,
          walletAddress: address,
        });
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
    if (
      !visible ||
      !isReady ||
      user == null ||
      working ||
      walletLoginInFlight.current ||
      finishingVisibleLoginRef.current
    ) {
      return;
    }

    finishingVisibleLoginRef.current = true;
    setWorking(true);
    setMessage("connected.");

    void finishPrivyLogin().finally(() => {
      finishingVisibleLoginRef.current = false;
      setWorking(false);
    });
  }, [finishPrivyLogin, isReady, user, visible, working]);

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

  async function handlePasteCode() {
    try {
      const clipboardValue = await Clipboard.getStringAsync();
      const nextCode = clipboardValue.replace(/\D/g, "").slice(0, CODE_LENGTH);

      if (nextCode.length === 0) {
        setMessage("no code found on the clipboard.");
        return;
      }

      setCode(nextCode);
      setMessage("");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "could not read the clipboard.");
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

      await finishPrivyLogin();
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

  return (
    <Modal animationType="fade" onRequestClose={onClose} transparent visible={visible}>
      <KeyboardAvoidingView
        behavior={Platform.OS === "ios" ? "padding" : undefined}
        style={styles.modalRoot}
      >
        <View style={styles.authBackdrop}>
          <Pressable accessibilityRole="button" onPress={onClose} style={styles.scrim} />
          <View style={styles.sheet}>
            <View style={styles.sheetHandle} />
            <ScrollView
              contentContainerStyle={styles.sheetContent}
              keyboardShouldPersistTaps="handled"
              showsVerticalScrollIndicator={false}
            >
              {!isReady ? (
                <View style={styles.header}>
                  <Text style={styles.title}>opening anky</Text>
                  <Text style={styles.subtitle}>one moment.</Text>
                </View>
              ) : null}

              {isReady && isLoggedIn ? (
                <View style={styles.header}>
                  <Text style={styles.title}>connected</Text>
                  <Text style={styles.subtitle}>returning to your writing.</Text>
                </View>
              ) : null}

              {isReady && !isLoggedIn && !emailCodeSent ? (
                <>
                  <View style={styles.header}>
                    <Text style={styles.title}>log in to anky</Text>
                    <Text style={styles.subtitle}>
                      {reason ?? "continue your journey within"}
                    </Text>
                  </View>

                  <View style={styles.inputWrap}>
                    <Text style={styles.inputIcon}>✉</Text>
                    <TextInput
                      autoCapitalize="none"
                      autoCorrect={false}
                      keyboardType="email-address"
                      onChangeText={setEmail}
                      placeholder="enter your email"
                      placeholderTextColor="rgba(255, 240, 201, 0.48)"
                      style={styles.input}
                      textContentType="emailAddress"
                      value={email}
                    />
                  </View>
                  <Text style={styles.helperText}>
                    ✦ we’ll send a one-time passcode to your email ✦
                  </Text>
                  <AuthButton
                    disabled={isBusy}
                    icon="✉"
                    label={isBusy ? "sending code" : "send code"}
                    onPress={() => void handleSendEmailCode()}
                  />

                  <View style={styles.dividerRow}>
                    <View style={styles.dividerLine} />
                    <Text style={styles.dividerText}>or continue with</Text>
                    <View style={styles.dividerLine} />
                  </View>
                  <View style={styles.buttonGroup}>
                    <AuthButton
                      disabled={isBusy}
                      icon="G"
                      label="continue with google"
                      onPress={() => void handleOAuth("google")}
                      variant="secondary"
                    />
                    <AuthButton
                      disabled={isBusy}
                      icon=""
                      label="continue with apple"
                      onPress={() => void handleOAuth("apple")}
                      variant="secondary"
                    />
                    <AuthButton
                      disabled={isBusy}
                      icon="◥"
                      label="continue with phantom"
                      onPress={() => void handleWalletConnect("phantom")}
                      variant="secondary"
                    />
                  </View>
                </>
              ) : null}

              {isReady && !isLoggedIn && emailCodeSent ? (
                <>
                  <Pressable
                    accessibilityRole="button"
                    onPress={() => {
                      setCode("");
                      setEmailCodeSent(false);
                      setMessage("");
                    }}
                    style={styles.backButton}
                  >
                    <Text style={styles.backText}>‹ back</Text>
                  </Pressable>
                  <View style={styles.header}>
                    <Text style={styles.title}>enter your code</Text>
                    <Text numberOfLines={2} style={styles.subtitle}>
                      ✦ we sent a code to {email.trim()} ✦
                    </Text>
                  </View>

                  <Pressable
                    accessibilityRole="button"
                    onPress={() => codeInputRef.current?.focus()}
                    style={styles.codeRow}
                  >
                    {Array.from({ length: CODE_LENGTH }).map((_, index) => {
                      const digit = code[index] ?? "";
                      const active = digit.length === 0 && index === Math.min(code.length, CODE_LENGTH - 1);

                      return (
                        <View
                          key={index}
                          style={[styles.codeBox, active && styles.codeBoxActive]}
                        >
                          <Text style={styles.codeText}>{digit}</Text>
                        </View>
                      );
                    })}
                    <TextInput
                      ref={codeInputRef}
                      autoCapitalize="none"
                      autoFocus
                      caretHidden
                      keyboardType="number-pad"
                      maxLength={CODE_LENGTH}
                      onChangeText={(value) => setCode(value.replace(/\D/g, "").slice(0, CODE_LENGTH))}
                      style={styles.hiddenCodeInput}
                      textContentType="oneTimeCode"
                      value={code}
                    />
                  </Pressable>

                  <Pressable
                    accessibilityRole="button"
                    disabled={isBusy}
                    onPress={() => void handlePasteCode()}
                    style={({ pressed }) => [
                      styles.pasteButton,
                      isBusy && styles.disabled,
                      pressed && !isBusy && styles.pressed,
                    ]}
                  >
                    <Text style={styles.pasteText}>▣ paste code</Text>
                  </Pressable>

                  <AuthButton
                    disabled={isBusy || code.trim().length === 0}
                    icon="✉"
                    label={isBusy ? "verifying code" : "verify code"}
                    onPress={() => void handleEmailLogin()}
                  />
                  <Pressable
                    accessibilityRole="button"
                    disabled={isBusy}
                    onPress={() => void handleSendEmailCode()}
                    style={({ pressed }) => [
                      styles.resendButton,
                      isBusy && styles.disabled,
                      pressed && !isBusy && styles.pressed,
                    ]}
                  >
                    <Text style={styles.resendText}>resend code</Text>
                  </Pressable>
                </>
              ) : null}

              {isLoggedIn || connectedExternalWallet == null ? null : (
                <Text style={styles.note}>
                  {connectedExternalWallet.label} {shortAddress(connectedExternalWallet.address, 6)}{" "}
                  {needsWalletLogin
                    ? "connected. sign once to continue."
                    : "is ready for loom actions."}
                </Text>
              )}
              {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}

              <View style={styles.privyRow}>
                <Text style={styles.lockIcon}>▣</Text>
                <Text style={styles.privyText}>protected with privy</Text>
              </View>

              <Pressable accessibilityRole="button" onPress={onClose} style={styles.closeButton}>
                <Text style={styles.closeText}>not now</Text>
              </Pressable>
            </ScrollView>
          </View>
        </View>
      </KeyboardAvoidingView>
    </Modal>
  );
}

function AuthButton({
  disabled = false,
  icon,
  label,
  onPress,
  variant = "primary",
}: {
  disabled?: boolean;
  icon?: string;
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
      {icon == null ? null : (
        <Text style={[styles.authButtonIcon, variant === "secondary" && styles.authButtonIconSecondary]}>
          {icon}
        </Text>
      )}
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
  authBackdrop: {
    flex: 1,
    justifyContent: "flex-end",
  },
  authButton: {
    alignItems: "center",
    backgroundColor: "rgba(99, 45, 160, 0.42)",
    borderColor: "rgba(255, 216, 116, 0.82)",
    borderRadius: 8,
    borderWidth: 1,
    flexDirection: "row",
    gap: spacing.sm,
    justifyContent: "center",
    minHeight: 58,
    paddingHorizontal: spacing.md,
  },
  authButtonIcon: {
    color: ankyColors.gold,
    fontSize: 22,
    lineHeight: 26,
    minWidth: 26,
    textAlign: "center",
  },
  authButtonIconSecondary: {
    color: ankyColors.gold,
  },
  authButtonSecondary: {
    backgroundColor: "rgba(12, 11, 31, 0.72)",
    borderColor: "rgba(232, 142, 83, 0.35)",
  },
  authButtonText: {
    color: ankyColors.gold,
    flexShrink: 1,
    fontSize: 18,
    fontWeight: "700",
    textAlign: "center",
    textTransform: "lowercase",
  },
  authButtonTextSecondary: {
    color: ankyColors.gold,
  },
  backButton: {
    alignSelf: "flex-start",
    paddingBottom: spacing.sm,
    paddingHorizontal: spacing.xs,
    paddingTop: spacing.xs,
  },
  backText: {
    color: ankyColors.gold,
    fontSize: fontSize.md,
    textTransform: "lowercase",
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
  codeBox: {
    alignItems: "center",
    backgroundColor: "rgba(12, 11, 31, 0.82)",
    borderColor: "rgba(185, 121, 232, 0.66)",
    borderRadius: 8,
    borderWidth: 1,
    flex: 1,
    height: 58,
    justifyContent: "center",
    maxWidth: 64,
  },
  codeBoxActive: {
    borderColor: "rgba(255, 216, 116, 0.94)",
  },
  codeRow: {
    flexDirection: "row",
    gap: spacing.sm,
    justifyContent: "center",
    marginBottom: spacing.md,
    marginTop: spacing.md,
  },
  codeText: {
    color: ankyColors.gold,
    fontSize: 25,
    fontWeight: "700",
  },
  disabled: {
    opacity: 0.48,
  },
  dividerLine: {
    backgroundColor: "rgba(215, 186, 115, 0.42)",
    flex: 1,
    height: StyleSheet.hairlineWidth,
  },
  dividerRow: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.md,
    marginTop: spacing.lg,
  },
  dividerText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    textTransform: "lowercase",
  },
  header: {
    alignItems: "center",
    marginBottom: spacing.lg,
  },
  helperText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginBottom: spacing.md,
    marginTop: spacing.md,
    textAlign: "center",
    textTransform: "lowercase",
  },
  hiddenCodeInput: {
    height: 1,
    opacity: 0,
    position: "absolute",
    width: 1,
  },
  input: {
    color: ankyColors.text,
    flex: 1,
    fontSize: fontSize.md,
    paddingVertical: 0,
  },
  inputIcon: {
    color: ankyColors.gold,
    fontSize: 25,
    lineHeight: 28,
    minWidth: 30,
    textAlign: "center",
  },
  inputWrap: {
    alignItems: "center",
    backgroundColor: "rgba(15, 12, 34, 0.76)",
    borderColor: "rgba(185, 121, 232, 0.72)",
    borderRadius: 8,
    borderWidth: 1,
    flexDirection: "row",
    gap: spacing.sm,
    minHeight: 58,
    paddingHorizontal: spacing.md,
  },
  label: {
    color: ankyColors.gold,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
  lockIcon: {
    color: ankyColors.gold,
    fontSize: 18,
    lineHeight: 20,
  },
  message: {
    color: ankyColors.gold,
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
  pasteButton: {
    alignItems: "center",
    alignSelf: "center",
    backgroundColor: "rgba(12, 11, 31, 0.72)",
    borderColor: "rgba(232, 142, 83, 0.42)",
    borderRadius: 8,
    borderWidth: 1,
    marginBottom: spacing.lg,
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.sm,
  },
  pasteText: {
    color: ankyColors.gold,
    fontSize: fontSize.sm,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  pressed: {
    opacity: 0.72,
  },
  privyRow: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.sm,
    justifyContent: "center",
    marginTop: spacing.lg,
  },
  privyText: {
    color: "rgba(184, 178, 255, 0.72)",
    fontSize: fontSize.md,
    textTransform: "lowercase",
  },
  resendButton: {
    alignItems: "center",
    backgroundColor: "rgba(12, 11, 31, 0.5)",
    borderColor: "rgba(156, 163, 175, 0.24)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.md,
    minHeight: 46,
    justifyContent: "center",
  },
  resendText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
    textTransform: "lowercase",
  },
  scrim: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(0, 0, 0, 0.42)",
    zIndex: 0,
  },
  sheet: {
    backgroundColor: "rgba(8, 7, 24, 0.96)",
    borderColor: "rgba(255, 216, 116, 0.64)",
    borderTopLeftRadius: 24,
    borderTopRightRadius: 24,
    borderWidth: 1,
    maxHeight: "78%",
    overflow: "hidden",
    zIndex: 2,
  },
  sheetContent: {
    paddingHorizontal: spacing.xl,
    paddingBottom: spacing.lg,
    paddingTop: spacing.md,
  },
  sheetHandle: {
    alignSelf: "center",
    backgroundColor: "rgba(184, 178, 255, 0.7)",
    borderRadius: 8,
    height: 6,
    marginTop: spacing.md,
    width: 62,
  },
  subtitle: {
    color: "rgba(244, 241, 234, 0.74)",
    fontSize: fontSize.md,
    lineHeight: 22,
    marginTop: spacing.sm,
    maxWidth: 340,
    textAlign: "center",
    textTransform: "lowercase",
  },
  title: {
    color: ankyColors.gold,
    fontFamily: Platform.select({ android: "serif", default: "Georgia", ios: "Georgia" }),
    fontSize: 34,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
});
