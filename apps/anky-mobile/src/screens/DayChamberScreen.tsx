import { useEffect, useState } from "react";
import { ScrollView, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { RitualButton } from "../components/anky/RitualButton";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { KingdomBadge } from "../components/sojourn/KingdomBadge";
import { listAnkySessionSummaries } from "../lib/ankySessionIndex";
import {
  AnkySessionSummary,
  getCurrentSojournDay,
  getDayState,
  getNextSessionKindForToday,
} from "../lib/sojourn";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "DayChamber">;

export function DayChamberScreen({ navigation, route }: Props) {
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);
  const [now, setNow] = useState(() => new Date());
  const currentDay = getCurrentSojournDay(now);
  const day = getDayState(route.params.day, sessions, now);
  const isToday = day.day === currentDay;
  const nextKind = getNextSessionKindForToday(sessions, now);
  const rows = [
    day.dailySeal == null ? null : { label: "main anky", session: day.dailySeal },
    ...day.extraThreads.map((session) => ({ label: "extra anky", session })),
    ...day.fragments.map((session) => ({ label: "fragment", session })),
  ].filter((row): row is { label: string; session: AnkySessionSummary } => row != null);

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

  function writeAnotherThread() {
    navigation.navigate("ActiveWriting", {
      dayNumber: day.day,
      isoDate: day.dateUtc.slice(0, 10),
      sessionKind: nextKind,
      sojourn: 9,
    });
  }

  return (
    <ScreenBackground variant="plain">
      <ScrollView contentContainerStyle={styles.content}>
        <Text style={styles.sojourn}>Sojourn 9</Text>
        <Text style={styles.title}>day {day.day}</Text>
        <Text style={styles.date}>{day.dateUtc.slice(0, 10)}</Text>
        <KingdomBadge kingdom={day.kingdom} />
        <Text style={styles.energy}>{day.kingdom.energy}</Text>

        <View style={styles.statusBlock}>
          <Text style={styles.status}>{statusLabel(day)}</Text>
          <Text style={styles.note}>
            {dayStatusCopy(day)}
          </Text>
          <Text style={styles.threadCount}>
            {day.threadCount} {day.threadCount === 1 ? "trace" : "traces"}
          </Text>
        </View>

        <View style={styles.rows}>
          {rows.length === 0 ? (
            <View style={styles.emptyRow}>
              <Text style={styles.rowTitle}>no trace</Text>
              <Text style={styles.rowMeta}>hollow chamber</Text>
            </View>
          ) : (
            rows.map((row) => (
              <ThreadRow
                key={row.session.id}
                label={row.label}
                onOpen={() => {
                  const fileName = getFileName(row.session);

                  if (fileName != null) {
                    navigation.navigate("Entry", { fileName });
                  }
                }}
                session={row.session}
              />
            ))
          )}
        </View>

        <View style={styles.actions}>
          <RitualButton label="return to map" onPress={() => navigation.navigate("Track")} />
          {isToday && day.status === "today_open" ? (
            <RitualButton
              label="write"
              onPress={writeAnotherThread}
              variant="secondary"
            />
          ) : null}
          {isToday && day.status === "today_sealed" ? (
            <RitualButton
              label="write again"
              onPress={writeAnotherThread}
              variant="secondary"
            />
          ) : null}
        </View>
      </ScrollView>
    </ScreenBackground>
  );
}

function ThreadRow({
  label,
  onOpen,
  session,
}: {
  label: string;
  onOpen: () => void;
  session: AnkySessionSummary;
}) {
  const canReveal = session.localFileUri != null;
  const hasReflection = session.reflectionId != null;

  return (
    <View style={styles.row}>
      <View style={styles.rowHeader}>
        <Text style={styles.rowTitle}>{label}</Text>
        <Text style={styles.rowTime}>{formatTime(session.createdAt)}</Text>
      </View>
      <Text style={styles.rowMeta}>
        {[
          session.kind === "fragment" ? "fragment" : "complete anky",
          hasReflection ? "reflection saved" : null,
          session.hasThread ? "conversation" : null,
        ]
          .filter(Boolean)
          .join(" · ")}
      </Text>
      {canReveal ? (
        <RitualButton
          label="open"
          onPress={onOpen}
          style={styles.rowAction}
          variant="ghost"
        />
      ) : null}
      {hasReflection ? (
        <RitualButton
          label="view reflection"
          onPress={onOpen}
          style={styles.rowAction}
          variant="secondary"
        />
      ) : null}
    </View>
  );
}

function statusLabel(day: ReturnType<typeof getDayState>): string {
  switch (day.status) {
    case "future":
      return "not yet open";
    case "sealed":
      return "sealed";
    case "today_open":
      return "today is open";
    case "today_sealed":
      return "today is sealed";
    case "unwoven":
      return day.fragments.length > 0 ? "fragment" : "quiet";
  }
}

function dayStatusCopy(day: ReturnType<typeof getDayState>): string {
  switch (day.status) {
    case "future":
      return "this day has not arrived.";
    case "today_open":
      return "today is open.";
    case "today_sealed":
      return "today has a complete anky.";
    case "sealed":
      return day.threadCount === 0 ? "this day stayed quiet." : "something was written here.";
    case "unwoven":
      return day.fragments.length === 0 ? "this day stayed quiet." : "a fragment lives here.";
  }
}

function formatTime(value: string): string {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return "";
  }

  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }).toLowerCase();
}

function getFileName(session: AnkySessionSummary): string | null {
  return session.sessionHash == null ? null : `${session.sessionHash}.anky`;
}

const styles = StyleSheet.create({
  actions: {
    gap: spacing.sm,
    marginTop: spacing.xl,
  },
  content: {
    padding: spacing.xl,
    paddingBottom: 44,
  },
  date: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    marginBottom: spacing.lg,
    textAlign: "center",
  },
  emptyRow: {
    borderColor: ankyColors.border,
    borderRadius: 8,
    borderWidth: 1,
    padding: spacing.lg,
  },
  energy: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    marginTop: spacing.sm,
    textAlign: "center",
    textTransform: "lowercase",
  },
  note: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
    lineHeight: 24,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  row: {
    borderColor: ankyColors.border,
    borderRadius: 8,
    borderWidth: 1,
    padding: spacing.lg,
  },
  rowAction: {
    marginTop: spacing.sm,
  },
  rowHeader: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "space-between",
  },
  rowMeta: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.sm,
  },
  rowTime: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
  },
  rowTitle: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  rows: {
    gap: spacing.md,
    marginTop: spacing.xl,
  },
  sojourn: {
    color: ankyColors.textMuted,
    fontSize: 12,
    fontWeight: "700",
    letterSpacing: 0,
    textAlign: "center",
    textTransform: "uppercase",
  },
  status: {
    color: ankyColors.gold,
    fontSize: fontSize.lg,
    fontWeight: "700",
    textAlign: "center",
    textTransform: "lowercase",
  },
  statusBlock: {
    marginTop: spacing.xl,
  },
  threadCount: {
    color: ankyColors.text,
    fontSize: fontSize.sm,
    marginTop: spacing.md,
    textAlign: "center",
    textTransform: "lowercase",
  },
  title: {
    color: ankyColors.text,
    fontSize: 42,
    fontWeight: "700",
    letterSpacing: 0,
    marginTop: spacing.md,
    textAlign: "center",
    textTransform: "lowercase",
  },
});
