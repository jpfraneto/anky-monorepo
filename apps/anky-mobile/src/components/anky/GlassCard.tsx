import { ReactNode } from "react";
import { StyleProp, StyleSheet, View, ViewStyle } from "react-native";

import { ankyColors, glow, radius, spacing } from "../../theme/tokens";

type Props = {
  border?: boolean;
  children: ReactNode;
  contentStyle?: StyleProp<ViewStyle>;
  glow?: boolean;
  style?: StyleProp<ViewStyle>;
};

export function GlassCard({
  border = true,
  children,
  contentStyle,
  glow: glowEnabled = false,
  style,
}: Props) {
  return (
    <View
      style={[
        styles.card,
        border && styles.border,
        glowEnabled && styles.glow,
        style,
      ]}
    >
      <View style={contentStyle}>{children}</View>
    </View>
  );
}

const styles = StyleSheet.create({
  border: {
    borderColor: ankyColors.border,
    borderWidth: 1,
  },
  card: {
    backgroundColor: ankyColors.card,
    borderRadius: radius.lg,
    overflow: "hidden",
    padding: spacing.lg,
  },
  glow: {
    ...glow.violet,
  },
});
