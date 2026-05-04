import type { ViewStyle } from "react-native";

export const ankyColors = {
  bg: "#08090B",
  bg2: "#0D0F12",
  bg3: "#14171C",
  card: "#101318",
  cardStrong: "#151922",
  border: "#262B35",
  borderStrong: "#3A414D",
  violet: "#8B7CF6",
  violetSoft: "#1B1E2A",
  violetBright: "#B8B2FF",
  magenta: "#B979E8",
  cyan: "#7DD3FC",
  gold: "#D7BA73",
  goldSoft: "#211D14",
  text: "#F4F1EA",
  textMuted: "#9CA3AF",
  danger: "#F87171",
  success: "#86EFAC",
} as const;

export const spacing = {
  xs: 4,
  sm: 8,
  md: 14,
  lg: 20,
  xl: 28,
  xxl: 40,
} as const;

export const radius = {
  sm: 6,
  md: 8,
  lg: 8,
  xl: 10,
  pill: 8,
} as const;

export const fontSize = {
  xs: 11,
  sm: 13,
  md: 16,
  lg: 20,
  xl: 28,
  xxl: 38,
  hero: 58,
} as const;

export const glow = {
  violet: {
    shadowColor: ankyColors.violet,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0,
    shadowRadius: 0,
  } satisfies ViewStyle,
  gold: {
    shadowColor: ankyColors.gold,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0,
    shadowRadius: 0,
  } satisfies ViewStyle,
  magenta: {
    shadowColor: ankyColors.magenta,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0,
    shadowRadius: 0,
  } satisfies ViewStyle,
} as const;

export const typography = {
  title: {
    color: ankyColors.gold,
    fontWeight: "600",
    letterSpacing: 0,
  },
  body: {
    color: ankyColors.text,
    letterSpacing: 0,
  },
  label: {
    color: ankyColors.textMuted,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
} as const;
