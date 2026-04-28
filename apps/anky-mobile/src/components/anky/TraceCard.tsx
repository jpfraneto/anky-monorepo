import { Pressable, StyleSheet, Text, View } from "react-native";

import { ankyColors, radius, spacing } from "../../theme/tokens";
import type { AnkyLocalState } from "../../lib/ankyState";
import { AnkyGlyph } from "./AnkyGlyph";

type Props = {
  hash?: string;
  onPress?: () => void;
  preview?: string;
  status?: AnkyLocalState | "pending" | "released";
  subtitle?: string;
  title: string;
};

export function TraceCard({
  hash,
  onPress,
  preview,
  status = "sealed",
  subtitle,
  title,
}: Props) {
  return (
    <Pressable accessibilityRole="button" onPress={onPress} style={styles.card}>
      <AnkyGlyph glow={status === "sealed" || status === "processed"} size={40} />
      <View style={styles.body}>
        <View style={styles.row}>
          <Text style={styles.title}>{title}</Text>
          <Text style={styles.status}>{status}</Text>
        </View>
        {subtitle == null ? null : <Text style={styles.subtitle}>{subtitle}</Text>}
        {preview == null ? null : (
          <Text numberOfLines={3} style={styles.preview}>
            {preview}
          </Text>
        )}
        {hash == null ? null : <Text style={styles.hash}>{hash}</Text>}
      </View>
      <Text style={styles.chevron}>›</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  body: {
    flex: 1,
  },
  card: {
    alignItems: "center",
    backgroundColor: ankyColors.card,
    borderColor: ankyColors.border,
    borderRadius: radius.lg,
    borderWidth: 1,
    flexDirection: "row",
    gap: spacing.md,
    marginBottom: spacing.md,
    padding: spacing.lg,
  },
  chevron: {
    color: ankyColors.violetBright,
    fontSize: 28,
    opacity: 0.66,
  },
  hash: {
    color: ankyColors.gold,
    fontSize: 11,
    letterSpacing: 1,
    marginTop: spacing.sm,
  },
  preview: {
    color: ankyColors.text,
    fontSize: 16,
    lineHeight: 23,
    marginTop: spacing.sm,
  },
  row: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "space-between",
  },
  status: {
    color: ankyColors.success,
    fontSize: 10,
    fontWeight: "800",
    letterSpacing: 1,
    textTransform: "uppercase",
  },
  subtitle: {
    color: ankyColors.textMuted,
    fontSize: 12,
    marginTop: 3,
  },
  title: {
    color: ankyColors.gold,
    fontSize: 17,
    fontWeight: "700",
  },
});
