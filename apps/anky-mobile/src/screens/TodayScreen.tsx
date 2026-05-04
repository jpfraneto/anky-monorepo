import { useEffect, useMemo, useState } from "react";
import { Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { AnkyGlyph } from "../components/anky/AnkyGlyph";
import { RitualButton } from "../components/anky/RitualButton";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { DayStone } from "../components/sojourn/DayStone";
import { KingdomBadge } from "../components/sojourn/KingdomBadge";
import { listAnkySessionSummaries } from "../lib/ankySessionIndex";
import {
  AnkySessionSummary,
  buildSojournDays,
  getCurrentSojournDay,
  getNextSessionKindForToday,
  SOJOURN_LENGTH_DAYS,
} from "../lib/sojourn";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Today">;

export function TodayScreen({ navigation }: Props) {
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [now, setNow] = useState(() => new Date());
  const days = useMemo(() => buildSojournDays(sessions, now), [sessions, now]);
  const currentDay = getCurrentSojournDay(now);
  const today = days[currentDay - 1];
  const todaySealed = today.status === "today_sealed";
  const nextKind = getNextSessionKindForToday(sessions, now);

  useEffect(() => {
    let mounted = true;

    async function load() {
      try {
        setNow(new Date());
        const nextSessions = await listAnkySessionSummaries();

        if (mounted) {
          setSessions(nextSessions);
        }
      } catch (error) {
        console.error(error);
      } finally {
        if (mounted) {
          setLoading(false);
        }
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
  }, [navigation]);

  function writeToday() {
    navigation.navigate("ActiveWriting", {
      dayNumber: today.day,
      isoDate: today.dateUtc.slice(0, 10),
      sessionKind: nextKind,
      sojourn: 9,
    });
  }

  return (
    <ScreenBackground variant="plain">
      <ScrollView contentContainerStyle={styles.content}>
        <View style={styles.top}>
          <Text style={styles.sojourn}>Sojourn 9</Text>
          <Pressable
            accessibilityLabel="settings"
            accessibilityRole="button"
            onPress={() => navigation.navigate("Auth")}
            style={styles.settings}
          >
            <View style={styles.settingsLine} />
            <View style={styles.settingsLineShort} />
            <View style={styles.settingsLine} />
          </Pressable>
        </View>

        <View style={styles.hero}>
          <Text style={styles.dayText}>
            {todaySealed
              ? `day ${today.day} sealed`
              : `day ${today.day} of ${SOJOURN_LENGTH_DAYS}`}
          </Text>
          <KingdomBadge kingdom={today.kingdom} />
          <Text style={styles.energy}>{today.kingdom.energy}</Text>

          <View style={styles.glyphWrap}>
            <DayStone day={today} showLabel={false} />
            <View style={styles.glyph}>
              <AnkyGlyph size={34} />
            </View>
          </View>

          <Text style={styles.opening}>
            {todaySealed ? "today's anky is in your loom" : "today's opening"}
          </Text>
          <Text style={styles.status}>
            {loading
              ? "reading the loom"
              : todaySealed
                ? "it was woven"
                : "your loom is waiting"}
          </Text>
        </View>

        <View style={styles.actions}>
          <RitualButton
            label={todaySealed ? "write again" : "write 8 minutes"}
            onPress={writeToday}
          />
          <RitualButton
            label={todaySealed ? "visit the trail" : "not now"}
            onPress={() => navigation.navigate("Trail")}
            variant="secondary"
          />
          <RitualButton
            label="see my loom"
            onPress={() => navigation.navigate("Loom")}
            variant="ghost"
          />
        </View>
      </ScrollView>
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  actions: {
    gap: spacing.sm,
    marginTop: spacing.xl,
  },
  content: {
    flexGrow: 1,
    justifyContent: "space-between",
    padding: spacing.xl,
    paddingBottom: 36,
  },
  dayText: {
    color: ankyColors.text,
    fontSize: fontSize.xl,
    fontWeight: "700",
    letterSpacing: 0,
    marginBottom: spacing.md,
    textAlign: "center",
    textTransform: "lowercase",
  },
  energy: {
    color: ankyColors.textMuted,
    fontSize: 14,
    marginTop: spacing.sm,
    textAlign: "center",
    textTransform: "lowercase",
  },
  glyph: {
    alignItems: "center",
    backgroundColor: ankyColors.bg,
    borderColor: ankyColors.border,
    borderRadius: 22,
    borderWidth: 1,
    height: 44,
    justifyContent: "center",
    position: "absolute",
    width: 44,
  },
  glyphWrap: {
    alignItems: "center",
    justifyContent: "center",
    marginVertical: 52,
  },
  hero: {
    alignItems: "center",
    justifyContent: "center",
    paddingVertical: spacing.xxl,
  },
  opening: {
    color: ankyColors.gold,
    fontSize: fontSize.lg,
    fontWeight: "700",
    letterSpacing: 0,
    textAlign: "center",
    textTransform: "lowercase",
  },
  settings: {
    alignItems: "flex-end",
    gap: 5,
    height: 34,
    justifyContent: "center",
    width: 34,
  },
  settingsLine: {
    backgroundColor: ankyColors.textMuted,
    borderRadius: 1,
    height: 2,
    width: 18,
  },
  settingsLineShort: {
    backgroundColor: ankyColors.textMuted,
    borderRadius: 1,
    height: 2,
    width: 12,
  },
  sojourn: {
    color: ankyColors.textMuted,
    fontSize: 13,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
  status: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
    lineHeight: 24,
    marginTop: spacing.sm,
    textAlign: "center",
    textTransform: "lowercase",
  },
  top: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "space-between",
  },
});
