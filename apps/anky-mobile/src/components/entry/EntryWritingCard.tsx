import { Platform, StyleSheet, Text, View } from "react-native";

import { ankyColors, fontSize, spacing } from "../../theme/tokens";

type Props = {
  isFragment: boolean;
  maxPreviewLines?: number;
  text: string;
};

const textureRows = Array.from({ length: 10 }, (_, index) => index);

export function EntryWritingCard({ isFragment, maxPreviewLines, text }: Props) {
  return (
    <View style={styles.card}>
      <ManuscriptTexture />
      <View style={styles.cardTop}>
        <View style={styles.rule} />
        <Text style={styles.label}>{isFragment ? "fragment" : "writing"}</Text>
        <View style={styles.rule} />
      </View>
      <Text selectable numberOfLines={maxPreviewLines} style={styles.body}>
        {text.length > 0 ? text : "No visible text."}
      </Text>
    </View>
  );
}

function ManuscriptTexture() {
  return (
    <View pointerEvents="none" style={styles.texture}>
      {textureRows.map((row) => (
        <View
          key={row}
          style={[
            styles.textureLine,
            {
              opacity: row % 3 === 0 ? 0.08 : 0.045,
              top: 30 + row * 46,
              width: row % 2 === 0 ? "76%" : "58%",
            },
          ]}
        />
      ))}
      <View style={styles.innerGlow} />
    </View>
  );
}

const styles = StyleSheet.create({
  body: {
    color: "#F7EEDB",
    fontFamily: Platform.select({ android: "serif", ios: "Georgia" }),
    fontSize: 20,
    letterSpacing: 0,
    lineHeight: 33,
    marginTop: spacing.lg,
  },
  card: {
    backgroundColor: "#0D0D17",
    borderColor: "rgba(215, 186, 115, 0.34)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.lg,
    overflow: "hidden",
    padding: spacing.xl,
  },
  cardTop: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.md,
  },
  innerGlow: {
    backgroundColor: "rgba(215, 186, 115, 0.035)",
    borderRadius: 8,
    bottom: spacing.md,
    left: spacing.md,
    position: "absolute",
    right: spacing.md,
    top: spacing.md,
  },
  label: {
    color: ankyColors.gold,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
  rule: {
    backgroundColor: "rgba(215, 186, 115, 0.24)",
    flex: 1,
    height: StyleSheet.hairlineWidth,
  },
  texture: {
    ...StyleSheet.absoluteFillObject,
  },
  textureLine: {
    backgroundColor: ankyColors.gold,
    height: StyleSheet.hairlineWidth,
    left: spacing.xl,
    position: "absolute",
  },
});
