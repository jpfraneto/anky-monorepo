import { ReactNode } from "react";
import { Pressable, StyleProp, StyleSheet, Text, View, ViewStyle } from "react-native";
import { LinearGradient } from "expo-linear-gradient";

import { ankyColors, glow, radius, spacing } from "../../theme/tokens";

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
  const isPrimary = variant === "primary";

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
      {isPrimary ? (
        <LinearGradient
          colors={[ankyColors.violet, ankyColors.magenta]}
          end={{ x: 1, y: 1 }}
          start={{ x: 0, y: 0 }}
          style={styles.fill}
        >
          <ButtonContent label={label} left={left} right={right} variant={variant} />
        </LinearGradient>
      ) : (
        <View style={styles.fill}>
          <ButtonContent label={label} left={left} right={right} variant={variant} />
        </View>
      )}
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
    backgroundColor: "rgba(255, 124, 159, 0.08)",
  },
  dangerLabel: {
    color: ankyColors.danger,
  },
  disabled: {
    opacity: 0.42,
  },
  fill: {
    borderRadius: radius.pill,
    paddingHorizontal: spacing.xl,
    paddingVertical: 16,
  },
  ghost: {
    backgroundColor: "transparent",
  },
  ghostLabel: {
    color: ankyColors.textMuted,
  },
  label: {
    fontSize: 16,
    fontWeight: "800",
    letterSpacing: 0.4,
    textTransform: "lowercase",
  },
  pressable: {
    borderRadius: radius.pill,
    overflow: "hidden",
  },
  pressed: {
    opacity: 0.82,
    transform: [{ scale: 0.99 }],
  },
  primary: {
    ...glow.magenta,
  },
  primaryLabel: {
    color: ankyColors.text,
  },
  secondary: {
    backgroundColor: "rgba(155, 92, 255, 0.10)",
    borderColor: ankyColors.border,
    borderWidth: 1,
  },
  secondaryLabel: {
    color: ankyColors.violetBright,
  },
});
