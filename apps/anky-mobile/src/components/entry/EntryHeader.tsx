import { StyleSheet, Text, View } from "react-native";

import { AnkyGlyph } from "../anky/AnkyGlyph";
import { SubtleIconButton } from "../navigation/SubtleIconButton";
import { ankyColors, fontSize } from "../../theme/tokens";

type Props = {
  onBack: () => void;
};

export function EntryHeader({ onBack }: Props) {
  return (
    <View style={styles.header}>
      <View style={styles.side}>
        <SubtleIconButton accessibilityLabel="back" icon="←" onPress={onBack} />
      </View>

      <View style={styles.center}>
        <Text style={styles.title}>entry</Text>
      </View>

      <View style={[styles.side, styles.witnessSide]}>
        <View style={styles.witness}>
          <AnkyGlyph size={34} />
        </View>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  center: {
    alignItems: "center",
    flex: 1,
  },
  header: {
    alignItems: "center",
    flexDirection: "row",
    minHeight: 46,
  },
  side: {
    flexBasis: 82,
  },
  title: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  witness: {
    alignItems: "center",
    backgroundColor: "rgba(16, 19, 24, 0.72)",
    borderColor: "rgba(139, 124, 246, 0.28)",
    borderRadius: 8,
    borderWidth: 1,
    height: 42,
    justifyContent: "center",
    width: 42,
  },
  witnessSide: {
    alignItems: "flex-end",
  },
});
