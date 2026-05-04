import { useEffect, useRef } from "react";
import { Animated, Easing, StyleSheet, Text, View } from "react-native";

import { ankyColors, fontSize } from "../../theme/tokens";

type Props = {
  lastCharGlow?: boolean;
  lastCharOpacity?: number;
  word: string;
};

export function WordFocus({ lastCharGlow = true, lastCharOpacity = 1, word }: Props) {
  const caretOpacity = useRef(new Animated.Value(1)).current;
  const characters = Array.from(word);
  const last = characters.at(-1) ?? "";

  useEffect(() => {
    const animation = Animated.loop(
      Animated.sequence([
        Animated.timing(caretOpacity, {
          duration: 520,
          easing: Easing.inOut(Easing.quad),
          toValue: 0.16,
          useNativeDriver: true,
        }),
        Animated.timing(caretOpacity, {
          duration: 520,
          easing: Easing.inOut(Easing.quad),
          toValue: 1,
          useNativeDriver: true,
        }),
      ]),
    );

    animation.start();

    return () => animation.stop();
  }, [caretOpacity]);

  if (word.length === 0) {
    return (
      <View style={styles.emptyWrap}>
        <Animated.View style={[styles.caret, { opacity: caretOpacity }]} />
      </View>
    );
  }

  return (
    <Text adjustsFontSizeToFit numberOfLines={1} style={[styles.word, { opacity: lastCharOpacity }]}>
      <Text style={lastCharGlow ? styles.lastGlow : styles.last}>{last}</Text>
    </Text>
  );
}

const styles = StyleSheet.create({
  caret: {
    backgroundColor: ankyColors.violetBright,
    height: 42,
    width: 2,
  },
  emptyWrap: {
    alignItems: "center",
    justifyContent: "center",
    minHeight: 70,
  },
  last: {
    color: ankyColors.gold,
  },
  lastGlow: {
    color: ankyColors.gold,
  },
  word: {
    color: ankyColors.text,
    fontSize: fontSize.hero,
    letterSpacing: 0,
    lineHeight: 72,
    maxWidth: 220,
    textAlign: "center",
  },
});
