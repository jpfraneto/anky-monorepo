import { useEffect, useMemo, useRef, useState } from "react";
import {
  FlatList,
  ListRenderItemInfo,
  Pressable,
  StyleSheet,
  Text,
  useWindowDimensions,
  View,
} from "react-native";
import Svg, { Path } from "react-native-svg";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { RitualButton } from "../components/anky/RitualButton";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import {
  buildLegacySojournDays,
  DAYS_PER_KINGDOM,
  getSojournDayIndex,
  KINGDOM_COLORS,
  SOJOURN_LENGTH_DAYS,
  SojournDay,
} from "../lib/sojourn";
import { ankyColors, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "AnkyverseTrail">;

const ROW_HEIGHT = 104;
const NODE_CLUSTER_WIDTH = 124;
const NODE_SIZE = 58;
const POSITIONS = ["center", "right", "center", "left"] as const;

type NodePosition = (typeof POSITIONS)[number];

export function AnkyverseTrailScreen({ navigation }: Props) {
  const { width } = useWindowDimensions();
  const listRef = useRef<FlatList<SojournDay>>(null);
  const nowMs = useMemo(() => Date.now(), []);
  const days = useMemo(() => buildLegacySojournDays(nowMs), [nowMs]);
  const trailDays = useMemo(() => [...days].reverse(), [days]);
  const todayIndex = getSojournDayIndex(nowMs);
  const today = days.find((day) => day.status === "today") ?? null;
  const safeInitialIndex =
    todayIndex >= 0 && todayIndex < SOJOURN_LENGTH_DAYS
      ? SOJOURN_LENGTH_DAYS - 1 - todayIndex
      : todayIndex < 0
        ? SOJOURN_LENGTH_DAYS - 1
        : 0;
  const [notice, setNotice] = useState("");

  useEffect(() => {
    if (today == null) {
      return;
    }

    const timer = setTimeout(() => {
      listRef.current?.scrollToIndex({
        animated: false,
        index: SOJOURN_LENGTH_DAYS - 1 - today.index,
        viewPosition: 0.46,
      });
    }, 80);

    return () => clearTimeout(timer);
  }, [today]);

  function handlePressDay(day: SojournDay) {
    if (day.status === "today") {
      navigateToTodayWrite(day);
      return;
    }

    if (day.status === "future") {
      flashNotice("not yet");
      return;
    }

    flashNotice("already walked");
  }

  function handlePressWrite() {
    if (today == null) {
      flashNotice("no writable day");
      return;
    }

    navigateToTodayWrite(today);
  }

  function navigateToTodayWrite(day: SojournDay) {
    navigation.navigate("ActiveWriting", {
      dayNumber: day.dayNumber,
      isoDate: day.isoDate,
      sojourn: 9,
    });
  }

  function flashNotice(message: string) {
    setNotice(message);
    setTimeout(() => setNotice(""), 1200);
  }

  const headerSubtitle =
    today == null
      ? todayIndex < 0
        ? "the trail opens 2026-03-03"
        : "sojourn complete"
      : `day ${today.dayNumber} of ${SOJOURN_LENGTH_DAYS}`;
  const headerHint = today == null ? "no writable day" : "write today's anky";

  return (
    <ScreenBackground variant="cosmic">
      <View style={styles.header}>
        <View>
          <Text style={styles.title}>9th sojourn</Text>
          <Text style={styles.subtitle}>{headerSubtitle}</Text>
        </View>
        <Text style={styles.headerHint}>{headerHint}</Text>
      </View>

      <FlatList
        ref={listRef}
        contentContainerStyle={styles.listContent}
        data={trailDays}
        getItemLayout={(_, index) => ({
          index,
          length: ROW_HEIGHT,
          offset: ROW_HEIGHT * index,
        })}
        initialScrollIndex={safeInitialIndex}
        keyExtractor={(item) => String(item.dayNumber)}
        onScrollToIndexFailed={(info) => {
          setTimeout(() => {
            listRef.current?.scrollToIndex({
              animated: false,
              index: Math.min(info.index, trailDays.length - 1),
              viewPosition: 0.46,
            });
          }, 120);
        }}
        renderItem={(info: ListRenderItemInfo<SojournDay>) => (
          <TrailDayNode
            day={info.item}
            onPress={() => handlePressDay(info.item)}
            pathWidth={width - 48}
          />
        )}
        showsVerticalScrollIndicator={false}
      />

      <View style={styles.writeDock}>
        <RitualButton
          label="Write 8 Minutes"
          onPress={handlePressWrite}
          style={today == null && styles.writeButtonDisabled}
        />
      </View>

      {notice.length > 0 ? (
        <View pointerEvents="none" style={styles.notice}>
          <Text style={styles.noticeText}>{notice}</Text>
        </View>
      ) : null}
    </ScreenBackground>
  );
}

function TrailDayNode({
  day,
  onPress,
  pathWidth,
}: {
  day: SojournDay;
  onPress: () => void;
  pathWidth: number;
}) {
  const color = KINGDOM_COLORS[day.kingdomIndex];
  const position = getNodePosition(day.index);
  const previousVisualPosition = getNodePosition(Math.min(SOJOURN_LENGTH_DAYS - 1, day.index + 1));
  const nextVisualPosition = getNodePosition(Math.max(0, day.index - 1));
  const currentX = getPathX(position, pathWidth);
  const previousX =
    day.index === SOJOURN_LENGTH_DAYS - 1 ? currentX : getPathX(previousVisualPosition, pathWidth);
  const nextX = day.index === 0 ? currentX : getPathX(nextVisualPosition, pathWidth);
  const isToday = day.status === "today";
  const isFuture = day.status === "future";
  const isPast = day.status === "past";
  const showKingdomThreshold = day.index % DAYS_PER_KINGDOM === 0;

  return (
    <View style={styles.dayRow}>
      <Svg height={ROW_HEIGHT} pointerEvents="none" style={styles.thread} width={pathWidth}>
        <Path
          d={`M ${previousX} 0 C ${previousX} 28 ${currentX} 24 ${currentX} 52 C ${currentX} 80 ${nextX} 76 ${nextX} ${ROW_HEIGHT}`}
          fill="none"
          opacity={isFuture ? 0.18 : isPast ? 0.28 : 0.86}
          stroke={isToday ? color : ankyColors.gold}
          strokeLinecap="round"
          strokeWidth={isToday ? 3 : 1.5}
        />
      </Svg>

      {showKingdomThreshold ? (
        <Text style={[styles.kingdomText, { color }]}>kingdom {day.kingdomIndex + 1}</Text>
      ) : null}

      <Pressable
        accessibilityState={{ disabled: !isToday }}
        accessibilityRole="button"
        onPress={onPress}
        style={[
          styles.nodeCluster,
          position === "left" && styles.nodeLeft,
          position === "center" && styles.nodeCenter,
          position === "right" && styles.nodeRight,
        ]}
      >
        <View
          style={[
            styles.node,
            {
              borderColor: color,
              opacity: isFuture ? 0.32 : isPast ? 0.58 : 1,
              shadowColor: color,
              shadowOpacity: isToday ? 0.95 : 0,
              shadowRadius: isToday ? 24 : 0,
            },
            isToday && styles.todayNode,
          ]}
        >
          <Text style={[styles.nodeNumber, { color }]}>{day.dayNumber}</Text>
        </View>

        <Text style={[styles.dayLabel, isToday && { color }, isFuture && styles.futureText]}>
          {isToday ? "today" : `day ${day.dayNumber}`}
        </Text>
        <Text style={styles.dateText}>{day.isoDate}</Text>
        {isToday ? <Text style={[styles.writeHint, { color }]}>write</Text> : null}
        {isFuture ? <Text style={styles.lockHint}>locked</Text> : null}
      </Pressable>
    </View>
  );
}

function getNodePosition(index: number): NodePosition {
  return POSITIONS[index % POSITIONS.length];
}

function getPathX(position: NodePosition, width: number): number {
  if (position === "left") {
    return NODE_CLUSTER_WIDTH / 2;
  }

  if (position === "right") {
    return width - NODE_CLUSTER_WIDTH / 2;
  }

  return width / 2;
}

const styles = StyleSheet.create({
  dateText: {
    color: ankyColors.textMuted,
    fontSize: 10,
    marginTop: 2,
  },
  dayLabel: {
    color: ankyColors.text,
    fontSize: 12,
    marginTop: 8,
    textTransform: "lowercase",
  },
  dayRow: {
    height: ROW_HEIGHT,
    justifyContent: "center",
  },
  futureText: {
    color: ankyColors.textMuted,
  },
  header: {
    alignItems: "flex-end",
    backgroundColor: "rgba(5, 8, 22, 0.74)",
    borderBottomColor: ankyColors.border,
    borderBottomWidth: 1,
    flexDirection: "row",
    justifyContent: "space-between",
    paddingBottom: 14,
    paddingHorizontal: 24,
    paddingTop: 18,
  },
  headerHint: {
    color: ankyColors.violetBright,
    fontSize: 12,
    letterSpacing: 0,
    paddingBottom: 3,
    textTransform: "lowercase",
  },
  kingdomText: {
    fontSize: 10,
    left: 0,
    letterSpacing: 0,
    opacity: 0.52,
    position: "absolute",
    textTransform: "uppercase",
    top: 2,
  },
  listContent: {
    paddingBottom: 164,
    paddingHorizontal: 24,
    paddingTop: 28,
  },
  lockHint: {
    color: ankyColors.textMuted,
    fontSize: 10,
    marginTop: 2,
    textTransform: "lowercase",
  },
  node: {
    alignItems: "center",
    backgroundColor: "rgba(255, 255, 255, 0.04)",
    borderRadius: NODE_SIZE / 2,
    borderWidth: 2,
    height: NODE_SIZE,
    justifyContent: "center",
    shadowOffset: { height: 0, width: 0 },
    width: NODE_SIZE,
  },
  nodeCenter: {
    alignSelf: "center",
  },
  nodeCluster: {
    alignItems: "center",
    width: NODE_CLUSTER_WIDTH,
  },
  nodeLeft: {
    alignSelf: "flex-start",
  },
  nodeNumber: {
    fontSize: 18,
    fontWeight: "700",
  },
  nodeRight: {
    alignSelf: "flex-end",
  },
  notice: {
    alignSelf: "center",
    backgroundColor: "rgba(5, 8, 22, 0.88)",
    borderColor: ankyColors.border,
    borderRadius: 999,
    borderWidth: 1,
    bottom: 26,
    paddingHorizontal: 18,
    paddingVertical: 10,
    position: "absolute",
  },
  noticeText: {
    color: ankyColors.gold,
    fontSize: 12,
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  subtitle: {
    color: ankyColors.textMuted,
    fontSize: 14,
    marginTop: 4,
  },
  thread: {
    left: 0,
    position: "absolute",
    top: 0,
  },
  title: {
    color: ankyColors.gold,
    fontSize: 28,
    fontWeight: "600",
  },
  todayNode: {
    backgroundColor: "rgba(255, 255, 255, 0.09)",
    transform: [{ scale: 1.14 }],
  },
  writeHint: {
    fontSize: 11,
    fontWeight: "600",
    marginTop: 2,
    textTransform: "lowercase",
  },
  writeButtonDisabled: {
    opacity: 0.42,
  },
  writeDock: {
    backgroundColor: "rgba(5, 8, 22, 0.82)",
    borderTopColor: ankyColors.border,
    borderTopWidth: 1,
    bottom: 0,
    left: 0,
    paddingBottom: spacing.lg,
    paddingHorizontal: spacing.xl,
    paddingTop: spacing.md,
    position: "absolute",
    right: 0,
  },
});
