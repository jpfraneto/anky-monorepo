import { Pressable, StyleSheet, Text, View } from "react-native";

import { ankyColors, spacing } from "../../theme/tokens";

type Props = {
  canReflect?: boolean;
  canTalkToAnky?: boolean;
  onCopy: () => void;
  onReflect: () => void;
  onTalkToAnky: () => void;
};

export function EntryActionRow({
  canReflect = true,
  canTalkToAnky = true,
  onCopy,
  onReflect,
  onTalkToAnky,
}: Props) {
  return (
    <View style={styles.row}>
      {canReflect ? <ActionPill label="reflect" onPress={onReflect} tone="quiet" /> : null}
      {canTalkToAnky ? (
        <ActionPill label="talk to anky" onPress={onTalkToAnky} tone="primary" />
      ) : null}
      <ActionPill label="copy" onPress={onCopy} tone="quiet" />
    </View>
  );
}

function ActionPill({
  label,
  onPress,
  tone,
}: {
  label: string;
  onPress: () => void;
  tone: "primary" | "quiet";
}) {
  return (
    <Pressable
      accessibilityRole="button"
      onPress={onPress}
      style={({ pressed }) => [
        styles.pill,
        tone === "primary" ? styles.primary : styles.quiet,
        pressed && styles.pressed,
      ]}
    >
      <Text style={[styles.label, tone === "primary" ? styles.primaryLabel : styles.quietLabel]}>
        {label}
      </Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  label: {
    fontSize: 14,
    fontWeight: "800",
    letterSpacing: 0,
    textAlign: "center",
    textTransform: "lowercase",
  },
  pill: {
    alignItems: "center",
    borderRadius: 8,
    borderWidth: 1,
    flexGrow: 1,
    justifyContent: "center",
    minHeight: 46,
    minWidth: 82,
    paddingHorizontal: spacing.md,
  },
  pressed: {
    opacity: 0.72,
  },
  primary: {
    backgroundColor: ankyColors.gold,
    borderColor: ankyColors.gold,
    flexGrow: 1.4,
  },
  primaryLabel: {
    color: ankyColors.bg,
  },
  quiet: {
    backgroundColor: "rgba(16, 19, 24, 0.72)",
    borderColor: "rgba(139, 124, 246, 0.28)",
  },
  quietLabel: {
    color: ankyColors.text,
  },
  row: {
    flexDirection: "row",
    flexWrap: "wrap",
    gap: spacing.sm,
    marginTop: spacing.lg,
  },
});
