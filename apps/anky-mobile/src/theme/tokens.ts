import type { ViewStyle } from "react-native";

export const ankyColors = {
  bg: "#050816",
  bg2: "#080B1D",
  bg3: "#120A24",
  card: "rgba(29, 18, 54, 0.56)",
  cardStrong: "rgba(42, 26, 76, 0.74)",
  border: "rgba(168, 113, 255, 0.24)",
  borderStrong: "rgba(200, 162, 255, 0.48)",
  violet: "#9B5CFF",
  violetSoft: "rgba(155, 92, 255, 0.18)",
  violetBright: "#C8A2FF",
  magenta: "#DF5CFF",
  cyan: "#63E6FF",
  gold: "#E8C982",
  goldSoft: "rgba(232, 201, 130, 0.18)",
  text: "#F5EEF8",
  textMuted: "rgba(245, 238, 248, 0.62)",
  danger: "#FF7C9F",
  success: "#A8F5C0",
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
  sm: 12,
  md: 18,
  lg: 26,
  xl: 34,
  pill: 999,
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
    shadowOpacity: 0.44,
    shadowRadius: 22,
  } satisfies ViewStyle,
  gold: {
    shadowColor: ankyColors.gold,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.34,
    shadowRadius: 18,
  } satisfies ViewStyle,
  magenta: {
    shadowColor: ankyColors.magenta,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.38,
    shadowRadius: 20,
  } satisfies ViewStyle,
} as const;

export const typography = {
  title: {
    color: ankyColors.gold,
    fontWeight: "600",
    letterSpacing: 0.8,
  },
  body: {
    color: ankyColors.text,
    letterSpacing: 0.1,
  },
  label: {
    color: ankyColors.textMuted,
    fontWeight: "700",
    letterSpacing: 1.2,
    textTransform: "uppercase",
  },
} as const;
