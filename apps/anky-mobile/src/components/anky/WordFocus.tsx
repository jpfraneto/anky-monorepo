import { StyleSheet, Text, View } from "react-native";

import { ankyColors, fontSize } from "../../theme/tokens";

type Props = {
  lastCharGlow?: boolean;
  placeholder?: string;
  word: string;
};

export function WordFocus({ lastCharGlow = true, placeholder = "", word }: Props) {
  const characters = Array.from(word);
  const last = characters.at(-1) ?? "";

  if (word.length === 0) {
    return (
      <View style={styles.emptyWrap}>
        <Text style={styles.placeholder}>{placeholder}</Text>
        <View style={styles.caret} />
      </View>
    );
  }

  return (
    <Text adjustsFontSizeToFit numberOfLines={1} style={styles.word}>
      <Text style={lastCharGlow ? styles.lastGlow : styles.last}>{last}</Text>
    </Text>
  );
}

const styles = StyleSheet.create({
  caret: {
    backgroundColor: ankyColors.violetBright,
    height: 42,
    opacity: 0.5,
    width: 2,
  },
  emptyWrap: {
    alignItems: "center",
    minHeight: 70,
  },
  last: {
    color: ankyColors.gold,
  },
  lastGlow: {
    color: ankyColors.gold,
    textShadowColor: ankyColors.magenta,
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 18,
  },
  placeholder: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
    marginBottom: 8,
  },
  word: {
    color: ankyColors.text,
    fontSize: fontSize.hero,
    letterSpacing: 0.4,
    lineHeight: 72,
    maxWidth: 220,
    textAlign: "center",
  },
});
