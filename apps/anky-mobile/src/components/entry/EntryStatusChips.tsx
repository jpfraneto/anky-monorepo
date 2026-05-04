import { StyleSheet, Text, View } from "react-native";

import { ankyColors, spacing } from "../../theme/tokens";

type Props = {
  hasReflection: boolean;
  hasThread: boolean;
  isFragment: boolean;
  isSealed: boolean;
};

type Chip = {
  label: string;
  tone: "gold" | "indigo" | "purple";
};

export function EntryStatusChips({ hasReflection, hasThread, isFragment, isSealed }: Props) {
  const chips: Chip[] = [
    { label: isFragment ? "fragment" : "complete", tone: isFragment ? "purple" : "gold" },
    { label: "local", tone: "indigo" },
    hasReflection ? { label: "reflected", tone: "gold" } : null,
    isSealed ? { label: "sealed", tone: "gold" } : null,
    hasThread ? { label: "thread", tone: "purple" } : null,
  ].filter((chip): chip is Chip => chip != null);

  return (
    <View style={styles.row}>
      {chips.map((chip) => (
        <View key={chip.label} style={[styles.chip, styles[chip.tone]]}>
          <Text style={[styles.label, styles[`${chip.tone}Label`]]}>{chip.label}</Text>
        </View>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  chip: {
    borderRadius: 8,
    borderWidth: 1,
    paddingHorizontal: spacing.md,
    paddingVertical: 7,
  },
  gold: {
    backgroundColor: "rgba(215, 186, 115, 0.1)",
    borderColor: "rgba(215, 186, 115, 0.42)",
  },
  goldLabel: {
    color: ankyColors.gold,
  },
  indigo: {
    backgroundColor: "rgba(16, 19, 24, 0.78)",
    borderColor: "rgba(244, 241, 234, 0.12)",
  },
  indigoLabel: {
    color: ankyColors.text,
  },
  label: {
    fontSize: 12,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  purple: {
    backgroundColor: "rgba(139, 124, 246, 0.13)",
    borderColor: "rgba(139, 124, 246, 0.34)",
  },
  purpleLabel: {
    color: ankyColors.violetBright,
  },
  row: {
    flexDirection: "row",
    flexWrap: "wrap",
    gap: spacing.sm,
    justifyContent: "center",
    marginTop: spacing.lg,
  },
});
