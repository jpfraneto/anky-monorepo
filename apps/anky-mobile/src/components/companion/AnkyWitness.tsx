import { StyleSheet, View } from "react-native";

import { ankyColors } from "../../theme/tokens";
import { AnkyGlyph } from "../anky/AnkyGlyph";

type Props = {
  mood?: "listening" | "quiet" | "warm";
  size?: "sheet" | "small" | "tiny";
};

const GLYPH_SIZE = {
  sheet: 46,
  small: 34,
  tiny: 24,
} as const;

export function AnkyWitness({ mood = "quiet", size = "small" }: Props) {
  const glyphSize = GLYPH_SIZE[size];

  return (
    <View style={[styles.wrap, styles[size], styles[mood]]}>
      <AnkyGlyph size={glyphSize} />
    </View>
  );
}

const styles = StyleSheet.create({
  listening: {
    borderColor: ankyColors.cyan,
  },
  quiet: {
    borderColor: ankyColors.border,
  },
  sheet: {
    borderRadius: 30,
    padding: 7,
  },
  small: {
    borderRadius: 24,
    padding: 6,
  },
  tiny: {
    borderRadius: 18,
    padding: 5,
  },
  warm: {
    borderColor: ankyColors.gold,
  },
  wrap: {
    alignItems: "center",
    backgroundColor: "rgba(8, 9, 11, 0.82)",
    borderWidth: 1,
    justifyContent: "center",
  },
});
