import { useEffect, useMemo, useRef, useState } from "react";
import { ScrollView, StyleSheet, Text, useWindowDimensions, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { RitualButton } from "../components/anky/RitualButton";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { TrailPath, TRAIL_ROW_HEIGHT } from "../components/sojourn/TrailPath";
import { listAnkySessionSummaries } from "../lib/ankySessionIndex";
import {
  AnkySessionSummary,
  buildSojournDays,
  DayState,
  getCurrentSojournDay,
  SOJOURN_LENGTH_DAYS,
} from "../lib/sojourn";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Trail">;

export function TrailScreen({ navigation }: Props) {
  const scrollRef = useRef<ScrollView>(null);
  const { height, width } = useWindowDimensions();
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);
  const [now, setNow] = useState(() => new Date());
  const days = useMemo(() => buildSojournDays(sessions, now), [sessions, now]);
  const trailDays = useMemo(() => [...days].reverse(), [days]);
  const currentDay = getCurrentSojournDay(now);
  const today = days[currentDay - 1];

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

  useEffect(() => {
    const timer = setTimeout(() => {
      const index = trailDays.findIndex((day) => day.day === currentDay);

      if (index >= 0) {
        scrollRef.current?.scrollTo({
          animated: false,
          y: Math.max(0, index * TRAIL_ROW_HEIGHT - height * 0.42),
        });
      }
    }, 80);

    return () => clearTimeout(timer);
  }, [currentDay, height, trailDays]);

  function openDay(day: DayState) {
    navigation.navigate("DayChamber", { day: day.day });
  }

  return (
    <ScreenBackground variant="plain">
      <View style={styles.header}>
        <View>
          <Text style={styles.title}>trail</Text>
          <Text style={styles.subtitle}>day {today.day} of {SOJOURN_LENGTH_DAYS}</Text>
        </View>
        <RitualButton
          label={today.status === "today_sealed" ? "write again" : "write"}
          onPress={() =>
            navigation.navigate("ActiveWriting", {
              dayNumber: today.day,
              isoDate: today.dateUtc.slice(0, 10),
              sessionKind: today.status === "today_sealed" ? "extra_thread" : "daily_seal",
              sojourn: 9,
            })
          }
          style={styles.headerButton}
          variant="secondary"
        />
      </View>

      <ScrollView
        ref={scrollRef}
        contentContainerStyle={styles.content}
        showsVerticalScrollIndicator={false}
      >
        <TrailPath days={trailDays} onPressDay={openDay} width={width - spacing.xl * 2} />
      </ScrollView>
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  content: {
    paddingBottom: 72,
    paddingHorizontal: spacing.xl,
    paddingTop: 28,
  },
  header: {
    alignItems: "center",
    borderBottomColor: ankyColors.border,
    borderBottomWidth: 1,
    flexDirection: "row",
    justifyContent: "space-between",
    paddingBottom: spacing.md,
    paddingHorizontal: spacing.xl,
    paddingTop: spacing.lg,
  },
  headerButton: {
    minWidth: 116,
  },
  subtitle: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    marginTop: 4,
    textTransform: "lowercase",
  },
  title: {
    color: ankyColors.gold,
    fontSize: fontSize.xl,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
});
