import { Pressable, StyleSheet, Text, View } from "react-native";

import type { AnkySessionSummary, DayState } from "../../lib/sojourn";
import { ankyColors } from "../../theme/tokens";
import { ThreadCount } from "./ThreadCount";

type Props = {
  day: DayState;
  compact?: boolean;
  onPress?: () => void;
  showLabel?: boolean;
};

export function DayStone({ compact = false, day, onPress, showLabel = true }: Props) {
  const isToday = day.status === "today_open" || day.status === "today_sealed";
  const isFuture = day.status === "future";
  const sessions = [
    day.dailySeal,
    ...day.extraThreads,
    ...day.fragments,
  ].filter((session): session is AnkySessionSummary => session != null);
  const hasComplete = day.dailySeal != null || day.extraThreads.length > 0;
  const hasFragment = day.fragments.length > 0;
  const hasReflection = sessions.some((session) => session.reflectionId != null);
  const hasSeal = sessions.some((session) => session.sealedOnchain === true);
  const hasThread = sessions.some((session) => session.hasThread === true);
  const isFilled = hasComplete || hasFragment;
  const size = compact ? 34 : 58;
  const stoneColor = hasComplete
    ? day.kingdom.accent
    : hasFragment
      ? "rgba(215, 115, 73, 0.72)"
      : "rgba(255, 255, 255, 0.035)";

  return (
    <Pressable
      accessibilityLabel={`day ${day.day}`}
      accessibilityRole="button"
      disabled={onPress == null}
      onPress={onPress}
      style={({ pressed }) => [styles.pressable, pressed && styles.pressed]}
    >
      <View
        style={[
          styles.stone,
          {
            backgroundColor: stoneColor,
            borderColor: hasSeal ? ankyColors.gold : hasFragment ? "rgba(215, 115, 73, 0.9)" : day.kingdom.accent,
            height: size,
            opacity: isFuture ? 0.36 : day.status === "unwoven" ? 0.58 : 1,
            shadowColor: day.kingdom.accent,
            shadowOpacity: isToday ? 0.55 : isFilled ? 0.2 : 0,
            shadowRadius: isToday ? 18 : isFilled ? 10 : 0,
            width: size,
          },
          isToday && styles.today,
          hasFragment && !hasComplete && styles.fragmentStone,
          hasSeal && styles.sealedStone,
        ]}
      >
        <Text
          style={[
            styles.number,
            compact && styles.compactNumber,
            { color: isFilled ? ankyColors.bg : day.kingdom.accent },
          ]}
        >
          {day.day}
        </Text>
        {hasReflection ? <View style={styles.mirror} /> : null}
        {hasThread ? <View style={styles.tail} /> : null}
        {day.extraThreads.length > 0 ? <View style={styles.knot} /> : null}
      </View>

      {showLabel ? (
        <View style={styles.labelRow}>
          <Text style={[styles.label, isToday && { color: day.kingdom.accent }]}>
            {getLabel(day)}
          </Text>
          <ThreadCount count={day.threadCount} />
        </View>
      ) : null}
    </Pressable>
  );
}

function getLabel(day: DayState): string {
  switch (day.status) {
    case "today_open":
      return "today";
    case "today_sealed":
      return "sealed";
    case "sealed":
      return "woven";
    case "unwoven":
      return day.fragments.length > 0 ? "fragment" : "quiet";
    case "future":
      return "not yet";
  }
}

const styles = StyleSheet.create({
  compactNumber: {
    fontSize: 12,
  },
  fragmentStone: {
    borderStyle: "dashed",
  },
  knot: {
    backgroundColor: ankyColors.gold,
    borderRadius: 3,
    height: 6,
    position: "absolute",
    right: 7,
    top: 7,
    width: 6,
  },
  mirror: {
    backgroundColor: "rgba(244, 241, 234, 0.88)",
    borderRadius: 4,
    height: 8,
    left: 7,
    opacity: 0.86,
    position: "absolute",
    top: 7,
    width: 8,
  },
  label: {
    color: ankyColors.textMuted,
    fontSize: 11,
    textTransform: "lowercase",
  },
  labelRow: {
    alignItems: "center",
    flexDirection: "row",
    gap: 6,
    justifyContent: "center",
    marginTop: 8,
    minHeight: 18,
  },
  number: {
    fontSize: 18,
    fontWeight: "800",
  },
  pressable: {
    alignItems: "center",
  },
  pressed: {
    opacity: 0.72,
  },
  sealedStone: {
    borderWidth: 2.4,
  },
  stone: {
    alignItems: "center",
    borderRadius: 999,
    borderWidth: 1.5,
    justifyContent: "center",
    shadowOffset: { height: 0, width: 0 },
  },
  today: {
    borderWidth: 2,
    transform: [{ scale: 1.08 }],
  },
  tail: {
    backgroundColor: ankyColors.gold,
    borderRadius: 2,
    bottom: -3,
    height: 8,
    position: "absolute",
    right: 5,
    transform: [{ rotate: "35deg" }],
    width: 4,
  },
});
