import { ReactNode } from "react";
import { StyleSheet, Text, View } from "react-native";
import Svg, { Circle } from "react-native-svg";

import { ankyColors, fontSize, glow } from "../../theme/tokens";

type Props = {
  children?: ReactNode;
  label?: string;
  remainingMs: number;
  size?: number;
  totalMs: number;
};

export function RiteTimer({ children, label, remainingMs, size = 280, totalMs }: Props) {
  const strokeWidth = 5;
  const center = size / 2;
  const radius = center - 18;
  const circumference = 2 * Math.PI * radius;
  const progress = totalMs <= 0 ? 0 : Math.max(0, Math.min(1, remainingMs / totalMs));
  const dashOffset = circumference * (1 - progress);

  return (
    <View style={[styles.root, { height: size, width: size }]}>
      <View style={[styles.innerGlow, { borderRadius: size / 2 }]} />
      <Svg height={size} width={size} viewBox={`0 0 ${size} ${size}`}>
        <Circle
          cx={center}
          cy={center}
          fill="rgba(5, 8, 22, 0.72)"
          r={radius - 18}
          stroke="rgba(200, 162, 255, 0.08)"
          strokeWidth="1"
        />
        <Circle
          cx={center}
          cy={center}
          fill="transparent"
          opacity="0.4"
          r={radius}
          stroke={ankyColors.violetSoft}
          strokeWidth="14"
        />
        <Circle
          cx={center}
          cy={center}
          fill="transparent"
          r={radius}
          stroke="rgba(245, 238, 248, 0.10)"
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
      {label == null ? null : <Text style={styles.label}>{label}</Text>}
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
  innerGlow: {
    ...StyleSheet.absoluteFillObject,
    ...glow.violet,
    backgroundColor: "rgba(155, 92, 255, 0.05)",
  },
  label: {
    bottom: 48,
    color: ankyColors.gold,
    fontSize: fontSize.md,
    fontWeight: "700",
    left: 0,
    letterSpacing: 1.2,
    position: "absolute",
    right: 0,
    textAlign: "center",
  },
  root: {
    alignItems: "center",
    justifyContent: "center",
  },
});
