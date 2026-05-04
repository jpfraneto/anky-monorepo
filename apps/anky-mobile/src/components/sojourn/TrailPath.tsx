import { StyleSheet, Text, View } from "react-native";
import Svg, { Path } from "react-native-svg";

import { DayState } from "../../lib/sojourn";
import { ankyColors } from "../../theme/tokens";
import { DayStone } from "./DayStone";

type Props = {
  days: DayState[];
  onPressDay: (day: DayState) => void;
  width: number;
};

const ROW_HEIGHT = 112;
const POSITIONS = ["center", "right", "center", "left"] as const;

type NodePosition = (typeof POSITIONS)[number];

export function TrailPath({ days, onPressDay, width }: Props) {
  return (
    <View style={styles.root}>
      {days.map((day, index) => (
        <TrailRow
          day={day}
          index={index}
          key={day.day}
          onPress={() => onPressDay(day)}
          pathWidth={width}
        />
      ))}
    </View>
  );
}

function TrailRow({
  day,
  index,
  onPress,
  pathWidth,
}: {
  day: DayState;
  index: number;
  onPress: () => void;
  pathWidth: number;
}) {
  const position = getNodePosition(index);
  const previousPosition = getNodePosition(Math.max(0, index - 1));
  const nextPosition = getNodePosition(index + 1);
  const currentX = getPathX(position, pathWidth);
  const previousX = index === 0 ? currentX : getPathX(previousPosition, pathWidth);
  const nextX = getPathX(nextPosition, pathWidth);
  const isToday = day.status === "today_open" || day.status === "today_sealed";
  const isFuture = day.status === "future";
  const showKingdom = day.day === day.kingdom.startDay;

  return (
    <View style={styles.row}>
      <Svg height={ROW_HEIGHT} pointerEvents="none" style={styles.thread} width={pathWidth}>
        <Path
          d={`M ${previousX} 0 C ${previousX} 28 ${currentX} 26 ${currentX} 56 C ${currentX} 86 ${nextX} 84 ${nextX} ${ROW_HEIGHT}`}
          fill="none"
          opacity={isFuture ? 0.16 : isToday ? 0.72 : 0.3}
          stroke={isToday ? day.kingdom.accent : ankyColors.borderStrong}
          strokeLinecap="round"
          strokeWidth={isToday ? 3 : 1.5}
        />
      </Svg>

      {showKingdom ? (
        <Text style={[styles.kingdom, { color: day.kingdom.accent }]}>{day.kingdom.name}</Text>
      ) : null}

      <View
        style={[
          styles.nodeWrap,
          position === "left" && styles.left,
          position === "center" && styles.center,
          position === "right" && styles.right,
        ]}
      >
        <DayStone day={day} onPress={onPress} />
        <Text style={styles.date}>{day.dateUtc.slice(0, 10)}</Text>
      </View>
    </View>
  );
}

function getNodePosition(index: number): NodePosition {
  return POSITIONS[index % POSITIONS.length];
}

function getPathX(position: NodePosition, width: number): number {
  if (position === "left") {
    return 58;
  }

  if (position === "right") {
    return width - 58;
  }

  return width / 2;
}

export const TRAIL_ROW_HEIGHT = ROW_HEIGHT;

const styles = StyleSheet.create({
  center: {
    alignSelf: "center",
  },
  date: {
    color: ankyColors.textMuted,
    fontSize: 10,
    marginTop: 2,
    opacity: 0.72,
  },
  kingdom: {
    fontSize: 10,
    fontWeight: "800",
    left: 0,
    letterSpacing: 0,
    opacity: 0.62,
    position: "absolute",
    textTransform: "uppercase",
    top: 4,
  },
  left: {
    alignSelf: "flex-start",
  },
  nodeWrap: {
    alignItems: "center",
    width: 116,
  },
  right: {
    alignSelf: "flex-end",
  },
  root: {
    width: "100%",
  },
  row: {
    height: ROW_HEIGHT,
    justifyContent: "center",
  },
  thread: {
    left: 0,
    position: "absolute",
    top: 0,
  },
});
