import { Pressable, StyleSheet, Text, View } from "react-native";
import { LinearGradient } from "expo-linear-gradient";

import { ankyColors, glow, radius, spacing } from "../../theme/tokens";
import { AnkyGlyph } from "./AnkyGlyph";

type Props = {
  disabled?: boolean;
  label?: string;
  onSeal: () => void;
};

export function SealAction({ disabled = false, label = "slide to seal", onSeal }: Props) {
  return (
    <Pressable
      accessibilityRole="button"
      disabled={disabled}
      onPress={onSeal}
      style={({ pressed }) => [
        styles.track,
        disabled && styles.disabled,
        pressed && !disabled && styles.pressed,
      ]}
    >
      <LinearGradient
        colors={["rgba(155, 92, 255, 0.24)", "rgba(223, 92, 255, 0.13)"]}
        end={{ x: 1, y: 0 }}
        start={{ x: 0, y: 0 }}
        style={styles.gradient}
      >
        <View style={styles.thumb}>
          <AnkyGlyph size={32} />
        </View>
        <Text style={styles.label}>{label}</Text>
        <Text style={styles.chevrons}>›››</Text>
      </LinearGradient>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  chevrons: {
    color: ankyColors.violetBright,
    fontSize: 18,
    letterSpacing: -2,
    opacity: 0.8,
  },
  disabled: {
    opacity: 0.42,
  },
  gradient: {
    alignItems: "center",
    borderRadius: radius.pill,
    flexDirection: "row",
    gap: spacing.md,
    padding: 8,
    paddingRight: spacing.lg,
  },
  label: {
    color: ankyColors.text,
    flex: 1,
    fontSize: 16,
    fontWeight: "800",
    letterSpacing: 0.5,
    textAlign: "center",
    textTransform: "lowercase",
  },
  pressed: {
    opacity: 0.86,
  },
  thumb: {
    alignItems: "center",
    backgroundColor: "rgba(245, 238, 248, 0.08)",
    borderColor: ankyColors.borderStrong,
    borderRadius: 24,
    borderWidth: 1,
    height: 48,
    justifyContent: "center",
    width: 48,
  },
  track: {
    ...glow.magenta,
    borderColor: ankyColors.borderStrong,
    borderRadius: radius.pill,
    borderWidth: 1,
    overflow: "hidden",
  },
});
