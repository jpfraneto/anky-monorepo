import { useEffect, useState } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import { usePrivy } from "@privy-io/expo";

import type { RootStackParamList } from "../../App";
import { GlassCard } from "../components/anky/GlassCard";
import { RitualButton } from "../components/anky/RitualButton";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import {
  hasCompletedOnboarding,
  markOnboardingComplete,
} from "../lib/onboarding/onboardingStorage";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Onboarding">;

type OnboardingStep = {
  body: string;
  step?: string;
  subtitle?: string;
  title: string;
};

const STEPS: OnboardingStep[] = [
  {
    subtitle: "a journey inward",
    title: "anky",
    body: "",
  },
  {
    step: "2 of 6",
    title: "Welcome, traveler.",
    body: "Anky is a companion for meeting the page.",
  },
  {
    step: "3 of 6",
    title: "A daily practice of presence.",
    body: "Write for 8 minutes.\nNothing is posted.\nThe writing stays with you.",
  },
  {
    step: "4 of 6",
    title: "Express. Let go.",
    body: "Move forward.\nNo backspace.\nThe moment passes. The trace remains.",
  },
  {
    step: "5 of 6",
    title: "Receive a reflection.",
    body: "After writing, you can ask for a mirror.",
  },
  {
    step: "6 of 6",
    title: "This is for you.",
    body: "No followers. No feed.\nJust you and the page.",
  },
];

export function OnboardingScreen({ navigation }: Props) {
  const { isReady, user } = usePrivy();
  const [index, setIndex] = useState(0);
  const [checking, setChecking] = useState(true);
  const step = STEPS[index];
  const isCover = index === 0;
  const isLast = index === STEPS.length - 1;

  useEffect(() => {
    let mounted = true;

    async function load() {
      const complete = await hasCompletedOnboarding();

      if (!mounted) {
        return;
      }

      if (complete || (isReady && user != null)) {
        navigation.replace("Track");
        return;
      }

      setChecking(false);
    }

    void load();

    return () => {
      mounted = false;
    };
  }, [isReady, navigation, user]);

  async function finishToWrite() {
    await markOnboardingComplete();
    navigation.replace("ActiveWriting", { sojourn: 9 });
  }

  async function finishToAuth() {
    await markOnboardingComplete();
    navigation.navigate("Auth");
  }

  function next() {
    setIndex((value) => Math.min(value + 1, STEPS.length - 1));
  }

  if (checking) {
    return (
      <ScreenBackground variant="plain">
        <View style={styles.center}>
          <Text style={styles.muted}>anky</Text>
        </View>
      </ScreenBackground>
    );
  }

  return (
    <ScreenBackground variant="plain">
      <View style={styles.root}>
        <GlassCard style={styles.card}>
          {step.step == null ? null : <Text style={styles.step}>{step.step}</Text>}
          <Text style={isCover ? styles.coverTitle : styles.title}>{step.title}</Text>
          {step.subtitle == null ? null : <Text style={styles.subtitle}>{step.subtitle}</Text>}
          {step.body.length === 0 ? null : <Text style={styles.body}>{step.body}</Text>}
        </GlassCard>

        <View style={styles.dots}>
          {STEPS.map((item, dotIndex) => (
            <Pressable
              accessibilityLabel={`show onboarding step ${dotIndex + 1}`}
              accessibilityRole="button"
              key={`${item.title}-${dotIndex}`}
              onPress={() => setIndex(dotIndex)}
              style={[styles.dot, dotIndex === index && styles.dotActive]}
            />
          ))}
        </View>

        {isLast ? (
          <View style={styles.actions}>
            <RitualButton label="Write now" onPress={() => void finishToWrite()} />
            <RitualButton
              label="Create account"
              onPress={() => void finishToAuth()}
              variant="secondary"
            />
            <RitualButton
              label="I already have an account"
              onPress={() => void finishToAuth()}
              variant="ghost"
            />
          </View>
        ) : (
          <RitualButton label={isCover ? "begin" : "next"} onPress={next} />
        )}
      </View>
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  actions: {
    gap: spacing.sm,
    width: "100%",
  },
  body: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    lineHeight: 25,
    marginTop: spacing.lg,
    textAlign: "center",
  },
  card: {
    alignItems: "center",
    minHeight: 300,
    justifyContent: "center",
  },
  center: {
    alignItems: "center",
    flex: 1,
    justifyContent: "center",
  },
  coverTitle: {
    color: ankyColors.gold,
    fontSize: 56,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  dot: {
    backgroundColor: ankyColors.borderStrong,
    borderRadius: 4,
    height: 8,
    width: 8,
  },
  dotActive: {
    backgroundColor: ankyColors.gold,
    width: 24,
  },
  dots: {
    flexDirection: "row",
    gap: spacing.sm,
    justifyContent: "center",
    marginVertical: spacing.xl,
  },
  muted: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
  },
  root: {
    flex: 1,
    justifyContent: "center",
    padding: spacing.xl,
  },
  step: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    fontWeight: "700",
    marginBottom: spacing.lg,
  },
  subtitle: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
    letterSpacing: 0,
    marginTop: spacing.sm,
    textTransform: "lowercase",
  },
  title: {
    color: ankyColors.gold,
    fontSize: fontSize.xl,
    fontWeight: "700",
    letterSpacing: 0,
    lineHeight: 34,
    textAlign: "center",
  },
});
