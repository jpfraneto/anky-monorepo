import { useEffect, useMemo, useState } from "react";
import {
  Image,
  ImageBackground,
  ImageSourcePropType,
  Linking,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import { usePrivy } from "@privy-io/expo";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import type { RootStackParamList } from "../../App";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { RootTabBar, RootTabName } from "../components/navigation/RootTabBar";
import { parseAnky } from "../lib/ankyProtocol";
import { listAnkySessionSummaries } from "../lib/ankySessionIndex";
import { listSavedAnkyFiles, SavedAnkyFile } from "../lib/ankyStorage";
import { getReflectionCreditBalance } from "../lib/credits/processAnky";
import { useAnkyPrivyWallet } from "../lib/privy/useAnkyPrivyWallet";
import { getCurrentSojournDay, SOJOURN_LENGTH_DAYS } from "../lib/sojourn";
import type { AnkySessionSummary } from "../lib/sojourn";
import { shortAddress } from "../lib/solana/loomStorage";
import {
  getRiteDurationMs,
  isCompleteRawAnky,
} from "../lib/thread/threadLogic";
import { fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "You">;
type MenuIcon = "account" | "chevronRight" | "credits" | "export" | "loom" | "privacy";
type StatIcon = "clockStat" | "featherStat" | "flameStat";

const assets = {
  avatar: require("../../assets/anky-you/avatar-anky.png"),
  background: require("../../assets/anky-you/bg-cosmos.png"),
  icons: {
    account: require("../../assets/anky-you/icons/account.png"),
    chevronRight: require("../../assets/anky-you/icons/chevron-right.png"),
    clockStat: require("../../assets/anky-you/icons/clock-stat.png"),
    credits: require("../../assets/anky-you/icons/credits.png"),
    export: require("../../assets/anky-you/icons/export.png"),
    featherStat: require("../../assets/anky-you/icons/feather-stat.png"),
    flameStat: require("../../assets/anky-you/icons/flame-stat.png"),
    loom: require("../../assets/anky-you/icons/loom.png"),
    privacy: require("../../assets/anky-you/icons/privacy.png"),
  } satisfies Record<MenuIcon | StatIcon, ImageSourcePropType>,
};

const GOLD = "#E9BE72";
const GOLD_BRIGHT = "#F2D392";
const GOLD_DIM = "rgba(214, 147, 68, 0.55)";
const COPY = "#D8C9D4";
const PANEL = "rgba(13, 12, 27, 0.74)";
const PANEL_DEEP = "rgba(9, 8, 20, 0.86)";
const SERIF = Platform.select({ android: "serif", default: "Georgia", ios: "Georgia" });
const PRIVACY_POLICY_URL = "https://www.anky.app/privacy-policy.md";

export function YouScreen({ navigation }: Props) {
  const insets = useSafeAreaInsets();
  const { user } = usePrivy();
  const wallet = useAnkyPrivyWallet();
  const [credits, setCredits] = useState(0);
  const [files, setFiles] = useState<SavedAnkyFile[]>([]);
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);
  const stats = useMemo(() => buildStats(files, sessions), [files, sessions]);
  const accountLabel =
    wallet.publicKey != null
      ? `connected ${wallet.walletLabel ?? "wallet"} ${shortAddress(wallet.publicKey, 6)}`
      : user == null
        ? "local-first. no login required to write."
        : `connected ${shortAddress(user.id, 8)}`;

  useEffect(() => {
    let mounted = true;

    async function load() {
      const [nextSessions, nextFiles, nextCredits] = await Promise.all([
        listAnkySessionSummaries(),
        listSavedAnkyFiles(),
        getReflectionCreditBalance(),
      ]);

      if (mounted) {
        setSessions(nextSessions);
        setFiles(nextFiles);
        setCredits(nextCredits);
      }
    }

    void load().catch((error) => {
      console.error(error);
    });
    const unsubscribe = navigation.addListener("focus", () => {
      void load().catch((error) => {
        console.error(error);
      });
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  function selectTab(tab: RootTabName) {
    if (tab === "You") {
      return;
    }

    if (tab === "Write") {
      navigation.replace("Write");
      return;
    }

    navigation.replace("Track");
  }

  return (
    <ScreenBackground safe={false} variant="plain">
      <ImageBackground resizeMode="cover" source={assets.background} style={styles.screen}>
        <View pointerEvents="none" style={styles.cosmosWash} />
        <View style={[styles.shell, { paddingTop: insets.top + 10 }]}>
          <ScrollView contentContainerStyle={styles.content} showsVerticalScrollIndicator={false}>
            <View style={styles.topRow}>
              <View style={styles.topSide} />
              <View style={styles.titleBlock}>
                <Text style={styles.pageTitle}>you</Text>
                <Text style={styles.pageSubtitle}>your story. your uniqueness.</Text>
              </View>
              <View style={styles.topSide} />
            </View>

            <ProfileHero />

            <View style={styles.statsCard}>
              <StatCell icon="featherStat" label="ankys" value={stats.completeAnkys} />
              <View style={styles.statDivider} />
              <StatCell icon="clockStat" label="minutes" value={stats.minutes} />
              <View style={styles.statDivider} />
              <StatCell icon="flameStat" label="streak" value={stats.streak} />
            </View>

            <View style={styles.menuCard}>
              <MenuRow
                icon="account"
                onPress={() => navigation.navigate("Account")}
                subtitle={accountLabel}
                title="account"
              />
              <MenuRow
                icon="privacy"
                onPress={() => {
                  void Linking.openURL(PRIVACY_POLICY_URL);
                }}
                subtitle="local by default. processing asks first."
                title="privacy"
              />
              <MenuRow
                icon="export"
                onPress={() => navigation.navigate("ExportData")}
                subtitle="open your local archive and export paths."
                title="export data"
              />
              <MenuRow
                icon="credits"
                last
                onPress={() => navigation.navigate("CreditsInfo")}
                subtitle={`${credits} credit${credits === 1 ? "" : "s"} available. writing is free.`}
                title="credits"
              />
            </View>

            <Pressable
              accessibilityRole="button"
              onPress={() => navigation.navigate("LoomInfo")}
              style={({ pressed }) => [styles.loomCard, pressed && styles.pressed]}
            >
              <Image accessibilityIgnoresInvertColors source={assets.icons.loom} style={styles.menuIcon} />
              <View style={styles.menuCopy}>
                <View style={styles.loomTitleRow}>
                  <Text style={styles.menuTitle}>loom</Text>
                  <View style={styles.optionalPill}>
                    <Text style={styles.optionalText}>optional</Text>
                  </View>
                </View>
                <Text style={styles.menuSubtitle}>seal hashes when you choose. writing never requires it.</Text>
              </View>
              <Image accessibilityIgnoresInvertColors source={assets.icons.chevronRight} style={styles.chevron} />
            </Pressable>

            <View style={styles.privacySeal}>
              <Image accessibilityIgnoresInvertColors source={assets.icons.privacy} style={styles.privacySealIcon} />
              <View style={styles.privacySealCopy}>
                <Text style={styles.privacySealTitle}>Your writing belongs to you.</Text>
                <Text style={styles.privacySealText}>100% local-first • Private • Sovereign</Text>
              </View>
            </View>

            <View style={styles.bottomOrnament}>
              <View style={styles.bottomLine} />
              <View style={styles.bottomDiamond} />
              <View style={styles.bottomLine} />
            </View>
          </ScrollView>

          <RootTabBar active="You" onSelect={selectTab} />
        </View>
      </ImageBackground>
    </ScreenBackground>
  );
}

function ProfileHero() {
  return (
    <View style={styles.hero}>
      <View style={styles.avatarOuterRing}>
        <View style={styles.avatarInnerRing}>
          <Image accessibilityIgnoresInvertColors source={assets.avatar} style={styles.avatar} />
        </View>
        <View style={[styles.ringDiamond, styles.ringDiamondTop]} />
        <View style={[styles.ringDiamond, styles.ringDiamondBottom]} />
        <View style={[styles.ringDiamond, styles.ringDiamondLeft]} />
        <View style={[styles.ringDiamond, styles.ringDiamondRight]} />
      </View>

     
    </View>
  );
}

function StatCell({ icon, label, value }: { icon: StatIcon; label: string; value: number }) {
  return (
    <View style={styles.statCell}>
      <Image accessibilityIgnoresInvertColors source={assets.icons[icon]} style={styles.statIcon} />
      <Text style={styles.statValue}>{value}</Text>
      <Text style={styles.statLabel}>{label}</Text>
    </View>
  );
}

function MenuRow({
  icon,
  last = false,
  onPress,
  subtitle,
  title,
}: {
  icon: MenuIcon;
  last?: boolean;
  onPress: () => void;
  subtitle: string;
  title: string;
}) {
  return (
    <Pressable accessibilityRole="button" onPress={onPress} style={({ pressed }) => [styles.menuRow, pressed && styles.pressed]}>
      <Image accessibilityIgnoresInvertColors source={assets.icons[icon]} style={styles.menuIcon} />
      <View style={styles.menuCopy}>
        <Text style={styles.menuTitle}>{title}</Text>
        <Text numberOfLines={2} style={styles.menuSubtitle}>
          {subtitle}
        </Text>
      </View>
      <Image accessibilityIgnoresInvertColors source={assets.icons.chevronRight} style={styles.chevron} />
      {last ? null : <View style={styles.rowDivider} />}
    </Pressable>
  );
}

function buildStats(files: SavedAnkyFile[], sessions: AnkySessionSummary[]) {
  const durations = files.map((file) => getRiteDurationMs(parseAnky(file.raw)) ?? 0);
  const completeAnkys = files.filter((file) => isCompleteRawAnky(file.raw)).length;
  const minutes = Math.round(durations.reduce((total, duration) => total + duration, 0) / 60000);
  const touchedDays = new Set(sessions.map((session) => session.sojournDay));
  const today = getCurrentSojournDay();
  const touched = new Set([...touchedDays].filter((day) => day >= 1 && day <= SOJOURN_LENGTH_DAYS));
  let streak = 0;

  for (let day = today; day >= 1; day -= 1) {
    if (!touched.has(day)) {
      break;
    }

    streak += 1;
  }

  return {
    completeAnkys,
    daysTouched: touched.size,
    fragments: sessions.filter((session) => session.kind === "fragment").length,
    minutes,
    reflections: sessions.filter((session) => session.reflectionId != null).length,
    sealed: sessions.filter((session) => session.sealedOnchain === true).length,
    streak,
  };
}

const styles = StyleSheet.create({
  avatar: {
    height: "100%",
    width: "100%",
  },
  avatarInnerRing: {
    backgroundColor: "#171022",
    borderColor: "rgba(237, 180, 91, 0.78)",
    borderRadius: 58,
    borderWidth: 1,
    height: 116,
    overflow: "hidden",
    width: 116,
  },
  avatarOuterRing: {
    alignItems: "center",
    borderColor: GOLD_DIM,
    borderRadius: 62,
    borderWidth: 1,
    height: 124,
    justifyContent: "center",
    shadowColor: "#E5A550",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.23,
    shadowRadius: 15,
    width: 124,
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
    width: 112,
  },
  bottomOrnament: {
    alignItems: "center",
    flexDirection: "row",
    height: 34,
    justifyContent: "center",
    marginTop: 8,
  },
  chevron: {
    height: 24,
    marginLeft: spacing.sm,
    opacity: 0.84,
    width: 24,
  },
  content: {
    paddingBottom: 112,
    paddingHorizontal: 22,
  },
  cosmosWash: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(5, 5, 14, 0.18)",
  },
  hero: {
    alignItems: "center",
    marginTop: -6,
  },
  loomCard: {
    alignItems: "center",
    backgroundColor: PANEL_DEEP,
    borderColor: "rgba(232, 113, 207, 0.68)",
    borderRadius: 16,
    borderWidth: 1,
    flexDirection: "row",
    marginTop: 8,
    minHeight: 56,
    overflow: "hidden",
    paddingLeft: 16,
    paddingRight: 12,
    shadowColor: "#CB65D7",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.16,
    shadowRadius: 10,
  },
  loomTitleRow: {
    alignItems: "center",
    flexDirection: "row",
  },
  memoryCard: {
    backgroundColor: "rgba(18, 16, 34, 0.66)",
    borderColor: "rgba(217, 143, 63, 0.5)",
    borderRadius: 18,
    borderWidth: 1,
    marginTop: 8,
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
  },
  memoryCopy: {
    color: COPY,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: 4,
    textTransform: "lowercase",
  },
  memoryTitle: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 22,
    lineHeight: 27,
    textTransform: "lowercase",
  },
  menuCard: {
    backgroundColor: PANEL,
    borderColor: "rgba(217, 143, 63, 0.62)",
    borderRadius: 18,
    borderWidth: 1,
    marginTop: 10,
    overflow: "hidden",
  },
  menuCopy: {
    flex: 1,
    justifyContent: "center",
  },
  menuIcon: {
    height: 34,
    marginRight: 10,
    width: 34,
  },
  menuRow: {
    alignItems: "center",
    flexDirection: "row",
    minHeight: 58,
    paddingLeft: 12,
    paddingRight: 10,
  },
  menuSubtitle: {
    color: COPY,
    fontFamily: SERIF,
    fontSize: 12.5,
    lineHeight: 16,
    textTransform: "lowercase",
  },
  menuTitle: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 18,
    lineHeight: 22,
    textTransform: "lowercase",
  },
  nameDiamond: {
    borderColor: GOLD,
    borderWidth: 1,
    height: 8,
    marginHorizontal: 8,
    transform: [{ rotate: "45deg" }],
    width: 8,
  },
  nameDivider: {
    alignItems: "center",
    flexDirection: "row",
    marginTop: 5,
    width: 86,
  },
  nameLine: {
    backgroundColor: GOLD_DIM,
    flex: 1,
    height: 1,
  },
  optionalPill: {
    backgroundColor: "rgba(11, 10, 22, 0.55)",
    borderColor: "rgba(233, 190, 114, 0.48)",
    borderRadius: 8,
    borderWidth: 1,
    marginLeft: 10,
    paddingHorizontal: 8,
    paddingVertical: 2,
  },
  optionalText: {
    color: "rgba(233, 213, 170, 0.88)",
    fontFamily: SERIF,
    fontSize: 10.5,
    textTransform: "lowercase",
  },
  pageSubtitle: {
    color: "rgba(223, 209, 213, 0.78)",
    fontFamily: SERIF,
    fontSize: 13,
    marginTop: -6,
    textAlign: "center",
    textTransform: "lowercase",
  },
  pageTitle: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 32,
    lineHeight: 40,
    textAlign: "center",
    textShadowColor: "rgba(237, 179, 94, 0.24)",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 14,
    textTransform: "lowercase",
  },
  pressed: {
    opacity: 0.72,
  },
  privacySeal: {
    alignItems: "center",
    borderBottomColor: "rgba(219, 143, 63, 0.36)",
    borderBottomWidth: StyleSheet.hairlineWidth,
    borderRadius: 10,
    borderTopColor: "rgba(219, 143, 63, 0.28)",
    borderTopWidth: StyleSheet.hairlineWidth,
    flexDirection: "row",
    justifyContent: "center",
    marginHorizontal: 2,
    marginTop: 8,
    minHeight: 42,
    paddingHorizontal: 10,
  },
  privacySealCopy: {
    minWidth: 0,
  },
  privacySealIcon: {
    height: 18,
    marginRight: 7,
    opacity: 0.92,
    width: 18,
  },
  privacySealText: {
    color: "rgba(242, 211, 146, 0.9)",
    fontFamily: SERIF,
    fontSize: 11,
    lineHeight: 14,
  },
  privacySealTitle: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 12,
    lineHeight: 15,
  },
  profileName: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 24,
    lineHeight: 30,
    marginTop: 6,
    textShadowColor: "rgba(237, 178, 86, 0.18)",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 10,
    textTransform: "lowercase",
  },
  profileSubtitle: {
    color: "#D8B8EA",
    fontFamily: SERIF,
    fontSize: 13,
    lineHeight: 17,
    marginTop: -3,
    textTransform: "lowercase",
  },
  ringDiamond: {
    backgroundColor: "#15101f",
    borderColor: GOLD,
    borderWidth: 1,
    height: 9,
    position: "absolute",
    transform: [{ rotate: "45deg" }],
    width: 9,
  },
  ringDiamondBottom: {
    bottom: -4,
  },
  ringDiamondLeft: {
    left: -4,
    top: 58,
  },
  ringDiamondRight: {
    right: -4,
    top: 58,
  },
  ringDiamondTop: {
    top: -4,
  },
  rowDivider: {
    backgroundColor: "rgba(219, 143, 63, 0.45)",
    bottom: 0,
    height: StyleSheet.hairlineWidth,
    left: 0,
    position: "absolute",
    right: 0,
  },
  screen: {
    backgroundColor: "#070812",
    flex: 1,
  },
  shell: {
    flex: 1,
  },
  statCell: {
    alignItems: "center",
    flex: 1,
    justifyContent: "center",
    paddingTop: 4,
  },
  statDivider: {
    backgroundColor: "rgba(178, 80, 129, 0.48)",
    width: 1,
  },
  statIcon: {
    height: 20,
    marginBottom: -2,
    width: 20,
  },
  statLabel: {
    color: "#D8B8EA",
    fontFamily: SERIF,
    fontSize: 11,
    marginTop: 0,
    textTransform: "lowercase",
  },
  statsCard: {
    backgroundColor: "rgba(18, 16, 34, 0.76)",
    borderColor: "rgba(217, 143, 63, 0.62)",
    borderRadius: 18,
    borderWidth: 1,
    flexDirection: "row",
    height: 68,
    marginTop: 10,
    overflow: "hidden",
  },
  statValue: {
    color: GOLD_BRIGHT,
    fontFamily: SERIF,
    fontSize: 22,
    lineHeight: 26,
  },
  titleBlock: {
    alignItems: "center",
    flex: 1,
  },
  topRow: {
    alignItems: "flex-start",
    flexDirection: "row",
    justifyContent: "space-between",
    minHeight: 48,
  },
  topSide: {
    width: 48,
  },
});
