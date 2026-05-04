import { Pressable, StyleSheet, View } from "react-native";
import Svg, { Circle, G } from "react-native-svg";
import { LocalSvg } from "react-native-svg/css";

import { ANKY_KINGDOMS, DayState, SOJOURN_LENGTH_DAYS } from "../../lib/sojourn";
import { ankyColors } from "../../theme/tokens";

type Props = {
  days: DayState[];
  onPressDay?: (day: DayState) => void;
  size?: number;
};

const VIEWBOX = 1200;
const MARK_RADIUS = 8.4;
const BASE_LOOM_ASSET = require("../../../assets/anky_base_loom.svg");
const SIDE_PIN_RANGES = [
  { end: { x: 739.24, y: 202.73 }, start: { x: 460.76, y: 202.73 } },
  { end: { x: 979.37, y: 417.55 }, start: { x: 782.45, y: 220.63 } },
  { end: { x: 997.27, y: 739.24 }, start: { x: 997.27, y: 460.76 } },
  { end: { x: 782.45, y: 979.37 }, start: { x: 979.37, y: 782.45 } },
  { end: { x: 460.76, y: 997.27 }, start: { x: 739.24, y: 997.27 } },
  { end: { x: 220.63, y: 782.45 }, start: { x: 417.55, y: 979.37 } },
  { end: { x: 202.73, y: 460.76 }, start: { x: 202.73, y: 739.24 } },
  { end: { x: 417.55, y: 220.63 }, start: { x: 220.63, y: 417.55 } },
] as const;

export function ChakanaLoom({ days, onPressDay, size = 320 }: Props) {
  return (
    <View style={[styles.wrap, { height: size, width: size }]}>
      <LocalSvg asset={BASE_LOOM_ASSET} height={size} width={size} />
      <Svg
        height={size}
        pointerEvents="none"
        style={StyleSheet.absoluteFill}
        viewBox={`0 0 ${VIEWBOX} ${VIEWBOX}`}
        width={size}
      >
        {ANKY_KINGDOMS.map((kingdom, kingdomIndex) => {
          const regionDays = days.filter((day) => day.kingdom.index === kingdom.index);

          return (
            <G key={kingdom.name}>
              {regionDays.map((day, index) => {
                const point = getMarkPoint(kingdomIndex, index);
                const isToday = day.status === "today_open" || day.status === "today_sealed";
                const isFilled = day.status === "sealed" || day.status === "today_sealed";
                const isFuture = day.status === "future";

                return (
                  <G key={day.day}>
                    {isToday ? (
                      <Circle
                        cx={point.x}
                        cy={point.y}
                        fill={ankyColors.gold}
                        opacity="0.16"
                        r="22"
                      />
                    ) : null}
                    <Circle
                      cx={point.x}
                      cy={point.y}
                      fill={isFilled ? kingdom.accent : ankyColors.bg}
                      opacity={isFuture ? 0.24 : day.status === "unwoven" ? 0.48 : 1}
                      r={isToday ? MARK_RADIUS + 2.2 : MARK_RADIUS}
                      stroke={isToday ? ankyColors.gold : kingdom.accent}
                      strokeWidth={isToday ? 2 : 1}
                    />
                  </G>
                );
              })}
              {regionDays
                .filter((day) => day.extraThreads.length > 0)
                .map((day) => {
                  const point = getMarkPoint(kingdomIndex, (day.day - kingdom.startDay) % 12);

                  return (
                    <Circle
                      cx={point.x + 11}
                      cy={point.y - 11}
                      fill={ankyColors.gold}
                      key={`knot-${day.day}`}
                      r="4.4"
                    />
                  );
                })}
            </G>
          );
        })}
      </Svg>

      {onPressDay == null
        ? null
        : days.map((day) => {
            const kingdomIndex = day.kingdom.index - 1;
            const dayIndex = (day.day - day.kingdom.startDay) % 12;
            const point = getMarkPoint(kingdomIndex, dayIndex, size / VIEWBOX);

            return (
              <Pressable
                accessibilityLabel={`open day ${day.day}`}
                accessibilityRole="button"
                key={`press-${day.day}`}
                onPress={() => onPressDay(day)}
                style={[
                  styles.hit,
                  {
                    left: point.x - 12,
                    top: point.y - 12,
                  },
                ]}
              />
            );
          })}
    </View>
  );
}

function getMarkPoint(kingdomIndex: number, index: number, scale = 1): { x: number; y: number } {
  const range = SIDE_PIN_RANGES[kingdomIndex];
  const t = index / 11;

  return {
    x: (range.start.x + (range.end.x - range.start.x) * t) * scale,
    y: (range.start.y + (range.end.y - range.start.y) * t) * scale,
  };
}

export function getLoomCompletion(days: DayState[]): number {
  return days.filter((day) => day.dailySeal != null).length / SOJOURN_LENGTH_DAYS;
}

const styles = StyleSheet.create({
  hit: {
    height: 24,
    position: "absolute",
    width: 24,
  },
  wrap: {
    alignItems: "center",
    alignSelf: "center",
    justifyContent: "center",
  },
});
