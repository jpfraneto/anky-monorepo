import { ReactNode } from "react";
import { Pressable, StyleProp, StyleSheet, Text, View, ViewStyle } from "react-native";

import { ankyColors, radius, spacing } from "../../theme/tokens";

type Props = {
  disabled?: boolean;
  label: string;
  left?: ReactNode;
  onPress?: () => void;
  right?: ReactNode;
  style?: StyleProp<ViewStyle>;
  variant?: "danger" | "ghost" | "primary" | "secondary";
};

export function RitualButton({
  disabled = false,
  label,
  left,
  onPress,
  right,
  style,
  variant = "primary",
}: Props) {
  return (
    <Pressable
      accessibilityRole="button"
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => [
        styles.pressable,
        styles[variant],
        disabled && styles.disabled,
        pressed && !disabled && styles.pressed,
        style,
      ]}
    >
      <View style={styles.fill}>
        <ButtonContent label={label} left={left} right={right} variant={variant} />
      </View>
    </Pressable>
  );
}

function ButtonContent({
  label,
  left,
  right,
  variant,
}: {
  label: string;
  left?: ReactNode;
  right?: ReactNode;
  variant: NonNullable<Props["variant"]>;
}) {
  return (
    <View style={styles.content}>
      {left}
      <Text style={[styles.label, styles[`${variant}Label`]]}>{label}</Text>
      {right}
    </View>
  );
}

const styles = StyleSheet.create({
  content: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.sm,
    justifyContent: "center",
  },
  danger: {
    backgroundColor: "transparent",
    borderColor: ankyColors.danger,
    borderWidth: 1,
  },
  dangerLabel: {
    color: ankyColors.danger,
  },
  disabled: {
    opacity: 0.42,
  },
  fill: {
    borderRadius: radius.pill,
    paddingHorizontal: spacing.lg,
    paddingVertical: 14,
  },
  ghost: {
    backgroundColor: "transparent",
  },
  ghostLabel: {
    color: ankyColors.textMuted,
  },
  label: {
    fontSize: 16,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  pressable: {
    borderRadius: radius.pill,
    overflow: "hidden",
  },
  pressed: {
    opacity: 0.72,
  },
  primary: {
    backgroundColor: ankyColors.text,
  },
  primaryLabel: {
    color: ankyColors.bg,
  },
  secondary: {
    backgroundColor: "transparent",
    borderColor: ankyColors.border,
    borderWidth: 1,
  },
  secondaryLabel: {
    color: ankyColors.text,
  },
});
