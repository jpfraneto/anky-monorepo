import { ReactNode } from "react";
import { StyleSheet, View } from "react-native";
import { LinearGradient } from "expo-linear-gradient";
import { SafeAreaView } from "react-native-safe-area-context";

import { ankyColors } from "../../theme/tokens";

type Props = {
  children: ReactNode;
  safe?: boolean;
  variant?: "centerGlow" | "cosmic" | "plain";
};

const STAR_POSITIONS = [
  { left: "12%", top: "16%" },
  { left: "82%", top: "14%" },
  { left: "68%", top: "31%" },
  { left: "18%", top: "46%" },
  { left: "88%", top: "58%" },
  { left: "35%", top: "76%" },
] as const;

export function ScreenBackground({ children, safe = true, variant = "cosmic" }: Props) {
  const Content = safe ? SafeAreaView : View;

  return (
    <LinearGradient
      colors={[ankyColors.bg, ankyColors.bg2, "#03040B"]}
      end={{ x: 1, y: 1 }}
      start={{ x: 0, y: 0 }}
      style={styles.root}
    >
      {variant !== "plain" ? (
        <>
          <View style={[styles.glow, styles.glowTop]} />
          <View style={[styles.glow, styles.glowBottom]} />
          {variant === "centerGlow" ? <View style={[styles.glow, styles.glowCenter]} /> : null}
          {STAR_POSITIONS.map((position, index) => (
            <View
              key={`${position.left}-${position.top}`}
              style={[
                styles.star,
                position,
                index % 2 === 0 ? styles.starGold : styles.starViolet,
              ]}
            />
          ))}
        </>
      ) : null}

      <Content style={styles.content}>{children}</Content>
    </LinearGradient>
  );
}

const styles = StyleSheet.create({
  content: {
    flex: 1,
  },
  glow: {
    borderRadius: 999,
    opacity: 0.72,
    position: "absolute",
  },
  glowBottom: {
    backgroundColor: "rgba(223, 92, 255, 0.16)",
    bottom: -90,
    height: 240,
    right: -120,
    width: 240,
  },
  glowCenter: {
    alignSelf: "center",
    backgroundColor: "rgba(155, 92, 255, 0.16)",
    height: 260,
    top: "26%",
    width: 260,
  },
  glowTop: {
    backgroundColor: "rgba(99, 230, 255, 0.12)",
    height: 220,
    left: -110,
    top: -80,
    width: 220,
  },
  root: {
    flex: 1,
  },
  star: {
    borderRadius: 999,
    height: 2,
    opacity: 0.7,
    position: "absolute",
    width: 2,
  },
  starGold: {
    backgroundColor: ankyColors.gold,
  },
  starViolet: {
    backgroundColor: ankyColors.violetBright,
  },
});
