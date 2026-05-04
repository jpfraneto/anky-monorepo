import { Pressable, StyleSheet, Text, View } from "react-native";

import { AnkyGlyph } from "../anky/AnkyGlyph";
import { ankyColors, fontSize, spacing } from "../../theme/tokens";

type Props = {
  expanded: boolean;
  onPress: () => void;
  reflection: string;
};

const reflectionGold = "#F1C776";

export function EntryReflectionCard({ expanded, onPress, reflection }: Props) {
  return (
    <Pressable accessibilityRole="button" onPress={onPress} style={styles.card}>
      <View style={styles.header}>
        <AnkyGlyph size={28} />
        <View style={styles.labelWrap}>
          <Text style={styles.label}>reflection</Text>
          <Text style={styles.hint}>{expanded ? "less" : "more"}</Text>
        </View>
      </View>

      <Text selectable numberOfLines={expanded ? undefined : 7} style={styles.reflection}>
        {reflection}
      </Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  card: {
    backgroundColor: "rgba(21, 17, 10, 0.58)",
    borderColor: "rgba(215, 186, 115, 0.38)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.lg,
    padding: spacing.lg,
  },
  header: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.sm,
  },
  hint: {
    color: ankyColors.textMuted,
    fontSize: 12,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  label: {
    color: ankyColors.gold,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
  labelWrap: {
    alignItems: "center",
    flex: 1,
    flexDirection: "row",
    justifyContent: "space-between",
  },
  reflection: {
    color: reflectionGold,
    fontSize: fontSize.md,
    lineHeight: 25,
    marginTop: spacing.md,
  },
});
