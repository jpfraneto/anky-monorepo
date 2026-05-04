import { StyleSheet, Text, View } from "react-native";

import { Kingdom } from "../../lib/sojourn";
import { ankyColors, spacing } from "../../theme/tokens";

type Props = {
  kingdom: Kingdom;
  showEnergy?: boolean;
};

export function KingdomBadge({ kingdom, showEnergy = true }: Props) {
  return (
    <View style={[styles.badge, { borderColor: kingdom.accent }]}>
      <View style={[styles.dot, { backgroundColor: kingdom.accent }]} />
      <Text style={styles.name}>{kingdom.name}</Text>
      {showEnergy ? <Text style={styles.energy}>{kingdom.energy}</Text> : null}
    </View>
  );
}

const styles = StyleSheet.create({
  badge: {
    alignItems: "center",
    alignSelf: "center",
    borderRadius: 8,
    borderWidth: 1,
    flexDirection: "row",
    gap: spacing.sm,
    paddingHorizontal: 12,
    paddingVertical: 8,
  },
  dot: {
    borderRadius: 4,
    height: 8,
    width: 8,
  },
  energy: {
    color: ankyColors.textMuted,
    fontSize: 12,
    textTransform: "lowercase",
  },
  name: {
    color: ankyColors.text,
    fontSize: 13,
    fontWeight: "700",
  },
});
