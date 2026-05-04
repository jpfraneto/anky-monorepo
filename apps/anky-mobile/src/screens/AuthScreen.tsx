import { useCallback, useEffect, useRef, useState } from "react";
import {
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import { useLoginWithEmail, useLoginWithOAuth, useLoginWithSiws, usePrivy } from "@privy-io/expo";

import type { RootStackParamList } from "../../App";
import { GlassCard } from "../components/anky/GlassCard";
import { RitualButton } from "../components/anky/RitualButton";
import { ScreenBackground } from "../components/anky/ScreenBackground";
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

type Props = NativeStackScreenProps<RootStackParamList, "Auth">;
type OAuthProvider = "apple" | "google";
type WalletProvider = ExternalWalletProviderName;

export function AuthScreen({ navigation }: Props) {
  const { getAccessToken, isReady, logout, user } = usePrivy();
  const oauth = useLoginWithOAuth();
  const emailLogin = useLoginWithEmail();
  const siws = useLoginWithSiws();
  const externalWallet = useExternalSolanaWallet();
  const privyWallet = useAnkyPrivyWallet();
  const walletLoginInFlight = useRef(false);
  const [code, setCode] = useState("");
  const [email, setEmail] = useState("");
  const [emailCodeSent, setEmailCodeSent] = useState(false);
  const [message, setMessage] = useState("");
  const [pendingWallet, setPendingWallet] = useState<WalletProvider | null>(null);
  const [working, setWorking] = useState(false);

  const finishPrivyLogin = useCallback(async () => {
    const accessToken = await getAccessToken();

    if (accessToken == null) {
      setMessage("connected to Privy. backend session is not ready yet.");
      return;
    }

    if (!hasConfiguredBackend()) {
      setMessage("connected to Privy. backend URL is not configured.");
      return;
    }

    await exchangePrivyAccessTokenForBackendSession(accessToken);
    setMessage("connected");
  }, [getAccessToken]);

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
      setMessage("sign the wallet message to continue");

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
        await finishPrivyLogin();
        setPendingWallet(null);
        navigation.replace("You");
      } catch (error) {
        setPendingWallet(null);
        setMessage(error instanceof Error ? error.message : "wallet login failed");
      } finally {
        walletLoginInFlight.current = false;
        setWorking(false);
      }
    },
    [finishPrivyLogin, navigation, siws],
  );

  useEffect(() => {
    if (pendingWallet == null) {
      return;
    }

    const wallet = externalWallet.wallets[pendingWallet];

    if (wallet == null) {
      return;
    }

    if (user != null) {
      setMessage(`${wallet.label} connected for Loom signing.`);
      setPendingWallet(null);
      return;
    }

    void completeWalletLogin(pendingWallet, wallet.address, wallet.signMessage);
  }, [completeWalletLogin, externalWallet.wallets, pendingWallet, user]);

  async function handleOAuth(provider: OAuthProvider) {
    setWorking(true);
    setMessage("");

    try {
      await oauth.login({
        provider,
        redirectUri: PRIVY_OAUTH_REDIRECT_PATH,
      });
      await finishPrivyLogin();
      navigation.replace("You");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : `${provider} login failed`);
    } finally {
      setWorking(false);
    }
  }

  async function handleSendEmailCode() {
    const trimmedEmail = email.trim();

    if (trimmedEmail.length === 0) {
      setMessage("enter an email first");
      return;
    }

    setWorking(true);
    setMessage("");

    try {
      await emailLogin.sendCode({ email: trimmedEmail });
      setEmailCodeSent(true);
      setMessage("check your email for the code");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "could not send email code");
    } finally {
      setWorking(false);
    }
  }

  async function handleEmailLogin() {
    const trimmedEmail = email.trim();
    const trimmedCode = code.trim();

    if (trimmedEmail.length === 0 || trimmedCode.length === 0) {
      setMessage("enter the email code first");
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
      navigation.replace("You");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "email login failed");
    } finally {
      setWorking(false);
    }
  }

  async function handleWalletConnect(provider: WalletProvider) {
    setWorking(true);
    setPendingWallet(provider);
    setMessage(`opening ${provider}`);

    try {
      await externalWallet.connectWallet(provider);
    } catch (error) {
      setPendingWallet(null);
      setMessage(error instanceof Error ? error.message : `${provider} did not connect`);
    } finally {
      setWorking(false);
    }
  }

  async function handleLogout() {
    setWorking(true);

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
      setMessage("logged out");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "logout failed");
    } finally {
      setWorking(false);
    }
  }

  async function handleCreateEmbeddedWallet() {
    setWorking(true);
    setMessage("");

    try {
      await privyWallet.createWallet();
      setMessage("embedded Solana wallet ready");
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : "Wallet connection failed. You can still write.",
      );
    } finally {
      setWorking(false);
    }
  }

  async function handleWalletDisconnect(provider: WalletProvider) {
    setWorking(true);
    setMessage("");

    try {
      await externalWallet.disconnectWallet(provider);
      setMessage(`${provider} disconnected`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : `${provider} did not disconnect`);
    } finally {
      setWorking(false);
    }
  }

  const isBusy =
    working ||
    oauth.state.status === "loading" ||
    emailLogin.state.status === "sending-code" ||
    emailLogin.state.status === "submitting-code" ||
    pendingWallet != null;
  const connectedExternalWallet = externalWallet.activeWallet;
  const phantomWallet = externalWallet.wallets.phantom;
  const backpackWallet = externalWallet.wallets.backpack;
  const embeddedWalletCopy = privyWallet.hasEmbeddedWallet
    ? `embedded wallet ${
        privyWallet.embeddedPublicKey == null
          ? "ready"
          : shortAddress(privyWallet.embeddedPublicKey, 6)
      }`
    : "embedded wallet not created. use this path for apple, google, or email sign-in.";
  const externalWalletCopy =
    connectedExternalWallet == null
      ? "use Phantom or Backpack for Loom minting and sealing. embedded wallets are for apple, google, or email sign-in."
      : `${connectedExternalWallet.label} ${shortAddress(
          connectedExternalWallet.address,
          6,
        )} connected. Loom actions will ask this wallet to sign.`;

  return (
    <ScreenBackground variant="plain">
      <KeyboardAvoidingView
        behavior={Platform.OS === "ios" ? "padding" : undefined}
        style={styles.keyboard}
      >
        <ScrollView contentContainerStyle={styles.content} keyboardShouldPersistTaps="handled">
          <View style={styles.header}>
            <Text style={styles.title}>connect</Text>
            <Text style={styles.subtitle}>
              use Privy to connect a wallet or account. writing still stays local.
            </Text>
          </View>

          {isReady && user != null ? (
            <GlassCard style={styles.card}>
              <Text style={styles.label}>privy session</Text>
              <Text style={styles.connected}>{user.id}</Text>
              <Text style={styles.note}>{embeddedWalletCopy}</Text>
              {connectedExternalWallet == null ? null : (
                <Text style={styles.note}>
                  {connectedExternalWallet.label} is active for Loom signing.
                </Text>
              )}
              <View style={styles.buttonGroup}>
                <RitualButton label="continue" onPress={() => navigation.replace("You")} />
                {!privyWallet.hasEmbeddedWallet ? (
                  <RitualButton
                    disabled={isBusy}
                    label="create embedded wallet"
                    onPress={() => void handleCreateEmbeddedWallet()}
                    variant="secondary"
                  />
                ) : null}
                <RitualButton
                  disabled={isBusy}
                  label="log out"
                  onPress={handleLogout}
                  variant="secondary"
                />
              </View>
            </GlassCard>
          ) : null}

          <GlassCard style={styles.card}>
            <Text style={styles.label}>external wallet</Text>
            <Text style={styles.note}>{externalWalletCopy}</Text>
            <View style={styles.buttonGroup}>
              <RitualButton
                disabled={isBusy || connectedExternalWallet?.provider === "phantom"}
                label={
                  phantomWallet == null
                    ? "phantom"
                    : connectedExternalWallet?.provider === "phantom"
                      ? "phantom connected"
                      : "use phantom"
                }
                onPress={() =>
                  void (phantomWallet == null
                    ? handleWalletConnect("phantom")
                    : externalWallet.setActiveProvider("phantom"))
                }
              />
              <RitualButton
                disabled={isBusy || connectedExternalWallet?.provider === "backpack"}
                label={
                  backpackWallet == null
                    ? "backpack"
                    : connectedExternalWallet?.provider === "backpack"
                      ? "backpack connected"
                      : "use backpack"
                }
                onPress={() =>
                  void (backpackWallet == null
                    ? handleWalletConnect("backpack")
                    : externalWallet.setActiveProvider("backpack"))
                }
                variant="secondary"
              />
              {connectedExternalWallet == null ? null : (
                <RitualButton
                  disabled={isBusy}
                  label={`disconnect ${connectedExternalWallet.label.toLowerCase()}`}
                  onPress={() => void handleWalletDisconnect(connectedExternalWallet.provider)}
                  variant="ghost"
                />
              )}
            </View>
          </GlassCard>

          {isReady && user == null ? (
            <>
              <GlassCard style={styles.card}>
                <Text style={styles.label}>email</Text>
                <TextInput
                  autoCapitalize="none"
                  autoCorrect={false}
                  keyboardType="email-address"
                  onChangeText={setEmail}
                  placeholder="email"
                  placeholderTextColor={ankyColors.textMuted}
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
                    placeholderTextColor={ankyColors.textMuted}
                    style={styles.input}
                    textContentType="oneTimeCode"
                    value={code}
                  />
                ) : null}
                <RitualButton
                  disabled={isBusy}
                  label={emailCodeSent ? "verify code" : "email code"}
                  onPress={() => void (emailCodeSent ? handleEmailLogin() : handleSendEmailCode())}
                  variant="secondary"
                />
              </GlassCard>

              <GlassCard style={styles.card}>
                <Text style={styles.label}>social</Text>
                <View style={styles.buttonGroup}>
                  <RitualButton
                    disabled={isBusy}
                    label="google"
                    onPress={() => void handleOAuth("google")}
                    variant="secondary"
                  />
                  <RitualButton
                    disabled={isBusy}
                    label="apple"
                    onPress={() => void handleOAuth("apple")}
                    variant="secondary"
                  />
                </View>
              </GlassCard>
            </>
          ) : null}

          {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}

          <Pressable
            accessibilityRole="button"
            onPress={() => navigation.replace("Write")}
            style={styles.localLink}
          >
            <Text style={styles.localLinkText}>continue locally</Text>
          </Pressable>
        </ScrollView>
      </KeyboardAvoidingView>
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  buttonGroup: {
    gap: spacing.sm,
    marginTop: spacing.md,
  },
  card: {
    marginTop: spacing.lg,
  },
  connected: {
    color: ankyColors.text,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.sm,
  },
  content: {
    flexGrow: 1,
    justifyContent: "center",
    padding: spacing.xl,
  },
  header: {
    alignItems: "center",
    marginBottom: spacing.lg,
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
    paddingVertical: 14,
  },
  keyboard: {
    flex: 1,
  },
  label: {
    color: ankyColors.gold,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
  localLink: {
    alignItems: "center",
    paddingTop: spacing.lg,
  },
  localLinkText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    fontWeight: "700",
  },
  message: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.lg,
    textAlign: "center",
  },
  note: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.sm,
  },
  subtitle: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
    lineHeight: 24,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  title: {
    color: ankyColors.gold,
    fontSize: fontSize.xxl,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
});
