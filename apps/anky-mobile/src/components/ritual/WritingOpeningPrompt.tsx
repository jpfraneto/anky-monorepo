import { StyleSheet, Text, View } from "react-native";

import { ankyColors, fontSize, spacing } from "../../theme/tokens";

type Props = {
  keyboardHeight: number;
  prompt?: string | null;
  safeBottom: number;
  visible: boolean;
};

export function WritingOpeningPrompt({
  keyboardHeight,
  prompt,
  safeBottom,
  visible,
}: Props) {
  const hasPrompt = prompt != null;

  if (!visible) {
    return null;
  }

  return (
    <View
      style={[
        styles.wrap,
        {
          bottom: keyboardHeight > 0 ? 12 : safeBottom + 18,
        },
      ]}
    >
      <View style={styles.card}>
        <View style={styles.copy}>
          <Text numberOfLines={hasPrompt ? 2 : undefined} style={styles.title}>
            {prompt ?? "write for 8 minutes."}
          </Text>
          {hasPrompt ? null : (
            <Text style={styles.body}>i am here to witness how you find the thread of your truth.</Text>
          )}
        </View>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  body: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 18,
    marginTop: 4,
    textTransform: "lowercase",
  },
  card: {
    overflow: "hidden",
    padding: spacing.md,
  },
  copy: {
    flex: 1,
    minWidth: 0,
    width: "70%",
    marginLeft: "auto",
  },
  title: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  wrap: {
    left: spacing.lg,
    position: "absolute",
    right: spacing.lg,
    zIndex: 6,
  },
});
