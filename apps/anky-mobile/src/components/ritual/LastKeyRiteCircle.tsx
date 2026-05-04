import { StyleSheet, Text, View } from "react-native";
import Svg, { Circle } from "react-native-svg";

import { ankyColors } from "../../theme/tokens";

type Props = {
  isActive: boolean;
  lastKey: string | null;
  progress: number;
  silenceProgress?: number;
  size?: number;
};

export function LastKeyRiteCircle({
  isActive,
  lastKey,
  progress,
  silenceProgress = 0,
  size = 280,
}: Props) {
  const strokeWidth = 6;
  const center = size / 2;
  const radius = center - 18;
  const circumference = 2 * Math.PI * radius;
  const safeProgress = clamp(progress);
  const safeSilence = clamp(silenceProgress);
  const dashOffset = circumference * (1 - safeProgress);
  const display = formatKey(lastKey);
  const isSpace = lastKey === " ";
  const keyOpacity = display.length === 0 ? 0.45 : Math.max(0.16, 1 - safeSilence * 0.84);
  const keyFontSize = Math.max(54, Math.min(84, size * 0.3));
  const keyLineHeight = keyFontSize * 1.14;
  const spaceFontSize = Math.max(24, Math.min(34, size * 0.14));

  return (
    <View style={[styles.root, { height: size, width: size }]}>
      <Svg height={size} viewBox={`0 0 ${size} ${size}`} width={size}>
        <Circle
          cx={center}
          cy={center}
          fill="rgba(20, 23, 28, 0.58)"
          r={radius - 20}
          stroke={ankyColors.border}
          strokeWidth="1"
        />
        <Circle
          cx={center}
          cy={center}
          fill="transparent"
          opacity={isActive ? 0.58 : 0.28}
          r={radius}
          stroke={ankyColors.violetSoft}
          strokeWidth="12"
        />
        <Circle
          cx={center}
          cy={center}
          fill="transparent"
          opacity={0.8 - safeSilence * 0.36}
          r={radius}
          stroke={ankyColors.borderStrong}
          strokeWidth={strokeWidth}
        />
        <Circle
          cx={center}
          cy={center}
          fill="transparent"
          originX={center}
          originY={center}
          r={radius}
          rotation="-90"
          stroke={safeSilence > 0.82 ? ankyColors.gold : ankyColors.violetBright}
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
              cx={x}
              cy={y}
              fill={angle === 0 ? ankyColors.gold : ankyColors.cyan}
              key={angle}
              opacity={angle === 0 ? 0.9 : 0.42}
              r={angle === 0 ? 3 : 2}
            />
          );
        })}
      </Svg>

      <View style={styles.center}>
        <Text
          adjustsFontSizeToFit
          numberOfLines={1}
          style={[
            styles.key,
            isSpace && styles.spaceKey,
            {
              fontSize: isSpace ? spaceFontSize : keyFontSize,
              lineHeight: isSpace ? spaceFontSize * 1.24 : keyLineHeight,
              maxWidth: size * 0.62,
              opacity: keyOpacity,
            },
          ]}
        >
          {display.length === 0 ? "|" : display}
        </Text>
      </View>
    </View>
  );
}

function formatKey(value: string | null): string {
  if (value == null || value.length === 0) {
    return "";
  }

  if (value === " ") {
    return "space";
  }

  return value;
}

function clamp(value: number): number {
  return Math.max(0, Math.min(1, value));
}

const styles = StyleSheet.create({
  center: {
    alignItems: "center",
    bottom: 34,
    justifyContent: "center",
    left: 34,
    position: "absolute",
    right: 34,
    top: 34,
  },
  key: {
    color: ankyColors.gold,
    fontWeight: "700",
    letterSpacing: 0,
    textAlign: "center",
  },
  root: {
    alignItems: "center",
    justifyContent: "center",
  },
  spaceKey: {
    color: ankyColors.text,
  },
});
