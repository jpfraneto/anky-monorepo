import { ReactNode } from "react";
import { StyleSheet, View } from "react-native";
import Svg, { Circle } from "react-native-svg";

import { ankyColors } from "../../theme/tokens";

type Props = {
  children?: ReactNode;
  remainingMs: number;
  size?: number;
  totalMs: number;
};

export function RiteTimer({ children, remainingMs, size = 280, totalMs }: Props) {
  const strokeWidth = 5;
  const center = size / 2;
  const radius = center - 18;
  const circumference = 2 * Math.PI * radius;
  const progress =
    totalMs <= 0 ? 1 : Math.max(0, Math.min(1, (totalMs - remainingMs) / totalMs));
  const dashOffset = circumference * (1 - progress);

  return (
    <View style={[styles.root, { height: size, width: size }]}>
      <Svg height={size} width={size} viewBox={`0 0 ${size} ${size}`}>
        <Circle
          cx={center}
          cy={center}
          fill={ankyColors.bg}
          r={radius - 18}
          stroke={ankyColors.border}
          strokeWidth="1"
        />
        <Circle
          cx={center}
          cy={center}
          fill="transparent"
          r={radius}
          stroke={ankyColors.violetSoft}
          strokeWidth="10"
        />
        <Circle
          cx={center}
          cy={center}
          fill="transparent"
          r={radius}
          stroke={ankyColors.border}
          strokeWidth={strokeWidth}
        />
        <Circle
          cx={center}
          cy={center}
          fill="transparent"
          r={radius}
          rotation="-90"
          originX={center}
          originY={center}
          stroke={ankyColors.violetBright}
          strokeDasharray={`${circumference} ${circumference}`}
          strokeDashoffset={dashOffset}
          strokeLinecap="round"
          strokeWidth={strokeWidth}
        />
        {[0, 90, 180, 270].map((angle) => {
          const radians = ((angle - 90) * Math.PI) / 180;
          const x = center + Math.cos(radians) * radius;
          const y = center + Math.sin(radians) * radius;

          return (
            <Circle
              key={angle}
              cx={x}
              cy={y}
              fill={angle === 0 ? ankyColors.gold : ankyColors.violetBright}
              opacity="0.86"
              r={angle === 0 ? 3 : 2}
            />
          );
        })}
      </Svg>
      <View style={styles.center}>{children}</View>
    </View>
  );
}

const styles = StyleSheet.create({
  center: {
    alignItems: "center",
    bottom: 28,
    justifyContent: "center",
    left: 28,
    position: "absolute",
    right: 28,
    top: 28,
  },
  root: {
    alignItems: "center",
    justifyContent: "center",
  },
});
