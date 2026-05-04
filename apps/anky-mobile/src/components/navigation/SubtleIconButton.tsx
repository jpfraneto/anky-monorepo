import { ReactNode } from "react";
import { Platform, Pressable, StyleProp, StyleSheet, Text, ViewStyle } from "react-native";

import { ankyColors } from "../../theme/tokens";

type Props = {
  accessibilityLabel: string;
  disabled?: boolean;
  icon: ReactNode;
  onPress: () => void;
  style?: StyleProp<ViewStyle>;
};

export function SubtleIconButton({
  accessibilityLabel,
  disabled = false,
  icon,
  onPress,
  style,
}: Props) {
  return (
    <Pressable
      accessibilityLabel={accessibilityLabel}
      accessibilityRole="button"
      disabled={disabled}
      hitSlop={10}
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        disabled && styles.disabled,
        pressed && !disabled && styles.pressed,
        style,
      ]}
    >
      {typeof icon === "string" ? <Text style={styles.icon}>{icon}</Text> : icon}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  button: {
    alignItems: "center",
    backgroundColor: Platform.select({
      ios: "rgba(244, 241, 234, 0.12)",
      default: "rgba(244, 241, 234, 0.1)",
    }),
    borderColor: "rgba(244, 241, 234, 0.07)",
    borderRadius: 18,
    borderWidth: StyleSheet.hairlineWidth,
    height: 36,
    justifyContent: "center",
    width: 36,
  },
  disabled: {
    opacity: 0.42,
  },
  icon: {
    color: ankyColors.text,
    fontSize: 20,
    fontWeight: "500",
    lineHeight: 24,
    textAlign: "center",
  },
  pressed: {
    opacity: 0.62,
    transform: [{ scale: 0.97 }],
  },
});
