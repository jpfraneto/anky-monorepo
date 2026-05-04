import { useEffect, useState } from "react";
import { Image, Modal, Pressable, StyleSheet, Text, useWindowDimensions, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import { ankyColors, fontSize, spacing } from "../../theme/tokens";

type Props = {
  onBegin: () => void;
  visible: boolean;
};

type SheetStep = "theory" | "welcome";

const BULLETS = [
  "8 quiet minutes",
  "local by default",
  "you choose what leaves",
];

const ANKY_WELCOME_IMAGE = require("../../../assets/anky-onboarding-welcome.png");

export function AnkyOnboardingSheet({ onBegin, visible }: Props) {
  const insets = useSafeAreaInsets();
  const { height, width } = useWindowDimensions();
  const [step, setStep] = useState<SheetStep>("welcome");
  const sheetHeight = Math.max(320, Math.round(height * 0.5));
  const ankySize = Math.min(250, Math.max(178, width * 0.54));

  useEffect(() => {
    if (visible) {
      setStep("welcome");
    }
  }, [visible]);

  return (
    <Modal animationType="slide" onRequestClose={onBegin} transparent visible={visible}>
      <View style={styles.backdrop}>
        <Pressable
          accessibilityLabel="begin writing"
          accessibilityRole="button"
          onPress={onBegin}
          style={styles.scrim}
        />
        <View
          style={[
            styles.sheet,
            {
              height: sheetHeight,
              paddingBottom: spacing.lg + Math.max(8, insets.bottom),
            },
          ]}
        >
          <View pointerEvents="none" style={styles.anky}>
            <Image
              resizeMode="contain"
              source={ANKY_WELCOME_IMAGE}
              style={{
                height: ankySize * 1.18,
                width: ankySize,
              }}
            />
          </View>

          {step === "welcome" ? (
            <View style={styles.copyBlock}>
              <Text style={styles.title}>hey, i'm anky.</Text>
              <Text style={styles.copy}>
                quiet the mind.{"\n"}
                open the heart.{"\n"}
                i witness while you write.
              </Text>

              <View style={styles.bullets}>
                {BULLETS.map((bullet, index) => (
                  <View key={bullet} style={styles.bulletRow}>
                    <View style={styles.bulletMark}>
                      <Text style={styles.bulletMarkText}>{index + 1}</Text>
                    </View>
                    <Text style={styles.bulletText}>{bullet}</Text>
                  </View>
                ))}
              </View>
            </View>
          ) : (
            <View style={styles.copyBlock}>
              <Text style={styles.title}>privacy.</Text>
              <Text style={styles.theory}>
                writing stays on this device unless you choose reflection.
              </Text>
            </View>
          )}

          <View style={styles.actions}>
            <SheetButton label="i'm ready" onPress={onBegin} variant="primary" />
            {step === "welcome" ? (
              <SheetButton label="privacy" onPress={() => setStep("theory")} />
            ) : null}
          </View>
        </View>
      </View>
    </Modal>
  );
}

function SheetButton({
  label,
  onPress,
  variant = "ghost",
}: {
  label: string;
  onPress: () => void;
  variant?: "ghost" | "primary";
}) {
  return (
    <Pressable
      accessibilityRole="button"
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        variant === "primary" && styles.buttonPrimary,
        pressed && styles.pressed,
      ]}
    >
      <Text style={[styles.buttonLabel, variant === "primary" && styles.buttonPrimaryLabel]}>
        {label}
      </Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  actions: {
    gap: 2,
    marginTop: "auto",
    paddingRight: 12,
    width: "50%",
    zIndex: 2,
  },
  anky: {
    bottom: -22,
    position: "absolute",
    right: -26,
    zIndex: 1,
  },
  backdrop: {
    flex: 1,
    justifyContent: "flex-end",
  },
  bulletMark: {
    alignItems: "center",
    borderColor: "rgba(215, 186, 115, 0.46)",
    borderRadius: 9,
    borderWidth: 1,
    height: 18,
    justifyContent: "center",
    marginTop: 1,
    width: 18,
  },
  bulletMarkText: {
    color: ankyColors.gold,
    fontSize: 10,
    fontWeight: "800",
    lineHeight: 12,
  },
  bulletRow: {
    alignItems: "flex-start",
    flexDirection: "row",
    gap: spacing.sm,
  },
  bulletText: {
    color: ankyColors.text,
    flex: 1,
    fontSize: fontSize.sm,
    lineHeight: 18,
    textTransform: "lowercase",
  },
  bullets: {
    gap: 8,
    marginTop: spacing.lg,
  },
  button: {
    alignItems: "center",
    borderRadius: 8,
    justifyContent: "center",
    minHeight: 34,
    paddingHorizontal: spacing.md,
    paddingVertical: 8,
  },
  buttonLabel: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  buttonPrimary: {
    backgroundColor: "#9D4E92",
    borderColor: "rgba(255, 182, 213, 0.48)",
    borderWidth: 1,
  },
  buttonPrimaryLabel: {
    color: ankyColors.text,
  },
  copy: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 18,
    marginTop: spacing.sm,
    maxWidth: 190,
    textTransform: "lowercase",
  },
  copyBlock: {
    maxWidth: 230,
    zIndex: 2,
  },
  pressed: {
    opacity: 0.72,
  },
  scrim: {
    backgroundColor: "rgba(0, 0, 0, 0.34)",
    bottom: 0,
    left: 0,
    position: "absolute",
    right: 0,
    top: 0,
  },
  sheet: {
    backgroundColor: "#0D1020",
    borderColor: "rgba(185, 121, 232, 0.2)",
    borderTopLeftRadius: 8,
    borderTopRightRadius: 8,
    borderWidth: 1,
    overflow: "hidden",
    paddingHorizontal: spacing.lg,
    paddingTop: spacing.lg,
  },
  theory: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    lineHeight: 23,
    marginTop: spacing.md,
    maxWidth: 230,
    textTransform: "lowercase",
  },
  title: {
    color: ankyColors.gold,
    fontSize: fontSize.lg,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
});
