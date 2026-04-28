import { StyleSheet, View } from "react-native";
import Svg, { Circle, Path } from "react-native-svg";

import { ankyColors, glow } from "../../theme/tokens";

type Props = {
  glow?: boolean;
  size?: number;
};

export function AnkyGlyph({ glow: glowEnabled = true, size = 34 }: Props) {
  return (
    <View
      style={[
        styles.wrap,
        {
          borderRadius: size / 2,
          height: size,
          width: size,
        },
        glowEnabled && styles.glow,
      ]}
    >
      <Svg height={size} viewBox="0 0 48 48" width={size}>
        <Circle
          cx="24"
          cy="24"
          fill="rgba(155, 92, 255, 0.12)"
          r="18"
          stroke={ankyColors.borderStrong}
          strokeWidth="1.2"
        />
        <Path
          d="M24 8 C33 17 33 28 24 40 C15 28 15 17 24 8 Z"
          fill="rgba(99, 230, 255, 0.10)"
          stroke={ankyColors.violetBright}
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="2"
        />
        <Path
          d="M15 30 C20 26 28 26 33 30"
          fill="none"
          stroke={ankyColors.gold}
          strokeLinecap="round"
          strokeWidth="1.4"
        />
        <Circle cx="24" cy="24" fill={ankyColors.cyan} r="2.2" />
      </Svg>
    </View>
  );
}

const styles = StyleSheet.create({
  glow: {
    ...glow.violet,
  },
  wrap: {
    alignItems: "center",
    justifyContent: "center",
  },
});
