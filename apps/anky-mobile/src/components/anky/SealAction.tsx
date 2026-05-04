import { Pressable, StyleSheet, Text, View } from "react-native";

import { ankyColors, radius, spacing } from "../../theme/tokens";
import { AnkyGlyph } from "./AnkyGlyph";

type Props = {
  disabled?: boolean;
  label?: string;
  onSeal: () => void;
};

export function SealAction({ disabled = false, label = "seal", onSeal }: Props) {
  return (
    <Pressable
      accessibilityRole="button"
      disabled={disabled}
      onPress={onSeal}
      style={({ pressed }) => [
        styles.button,
        disabled && styles.disabled,
        pressed && !disabled && styles.pressed,
      ]}
    >
      <View style={styles.content}>
        <View style={styles.thumb}>
          <AnkyGlyph size={32} />
        </View>
        <Text style={styles.label}>{label}</Text>
      </View>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  button: {
    backgroundColor: ankyColors.text,
    borderRadius: radius.pill,
    overflow: "hidden",
  },
  disabled: {
    opacity: 0.42,
  },
  content: {
    alignItems: "center",
    borderRadius: radius.pill,
    flexDirection: "row",
    gap: spacing.sm,
    paddingHorizontal: spacing.md,
    paddingVertical: 10,
  },
  label: {
    color: ankyColors.bg,
    flex: 1,
    fontSize: 16,
    fontWeight: "700",
    letterSpacing: 0,
    textAlign: "center",
    textTransform: "lowercase",
  },
  pressed: {
    opacity: 0.72,
  },
  thumb: {
    alignItems: "center",
    backgroundColor: ankyColors.bg,
    borderRadius: 8,
    height: 48,
    justifyContent: "center",
    width: 48,
  },
});
