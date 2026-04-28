import { StyleSheet, Text, View } from "react-native";

import { ankyColors, fontSize, spacing } from "../../theme/tokens";

type Props = {
  align?: "center" | "left";
  compact?: boolean;
  eyebrow?: string;
  subtitle?: string;
  title: string;
};

export function SacredHeader({ align = "left", compact = false, eyebrow, subtitle, title }: Props) {
  const centered = align === "center";

  return (
    <View style={[styles.root, compact && styles.compact, centered && styles.centered]}>
      {eyebrow == null ? null : (
        <Text style={[styles.eyebrow, centered && styles.textCenter]}>{eyebrow}</Text>
      )}
      <Text style={[styles.title, compact && styles.titleCompact, centered && styles.textCenter]}>
        {title}
      </Text>
      {subtitle == null ? null : (
        <Text style={[styles.subtitle, centered && styles.textCenter]}>{subtitle}</Text>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  centered: {
    alignItems: "center",
  },
  compact: {
    marginBottom: spacing.md,
  },
  eyebrow: {
    color: ankyColors.violetBright,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 1.8,
    marginBottom: spacing.sm,
    textTransform: "uppercase",
  },
  root: {
    marginBottom: spacing.xl,
  },
  subtitle: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
    lineHeight: 23,
    marginTop: spacing.sm,
    maxWidth: 310,
  },
  textCenter: {
    textAlign: "center",
  },
  title: {
    color: ankyColors.gold,
    fontSize: fontSize.xxl,
    fontWeight: "600",
    letterSpacing: 0.9,
    lineHeight: 46,
  },
  titleCompact: {
    fontSize: fontSize.xl,
    lineHeight: 34,
  },
});
