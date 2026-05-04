import { Image, StyleSheet, Text, useWindowDimensions, View } from "react-native";

import { ankyColors, fontSize, spacing } from "../../theme/tokens";

type Props = {
  keyboardHeight: number;
  prompt?: string | null;
  safeBottom: number;
  visible: boolean;
};

const ANKY_THREAD_IMAGE = require("../../../assets/anky_thread.png");
const ANKY_PROMPT_IMAGE = require("../../../assets/anky-onboarding-welcome.png");

export function WritingOpeningPrompt({
  keyboardHeight,
  prompt,
  safeBottom,
  visible,
}: Props) {
  const { width } = useWindowDimensions();
  const hasPrompt = prompt != null;
  const imageSize = hasPrompt
    ? Math.min(92, Math.max(70, width * 0.22))
    : Math.min(118, Math.max(82, width * 0.5));

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
        <Image
          accessibilityIgnoresInvertColors
          resizeMode="contain"
          source={hasPrompt ? ANKY_PROMPT_IMAGE : ANKY_THREAD_IMAGE}
          style={[
            styles.image,
            hasPrompt && styles.promptImage,
            { height: hasPrompt ? imageSize * 1.22 : imageSize * 1.1, width: imageSize },
          ]}
        />

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
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.md,
    overflow: "hidden",
    padding: spacing.md,
  },
  copy: {
    flex: 1,
    minWidth: 0,
  },
  image: {
    marginBottom: -10,
    marginLeft: -8,
    marginTop: -12,
  },
  promptImage: {
    marginBottom: -6,
    marginLeft: -4,
    marginRight: 2,
    marginTop: -6,
    opacity: 0.86,
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
