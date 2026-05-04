export const sojournMapTokens = {
  card: {
    avatar: 58,
    minHeight: 86,
  },
  colors: {
    background: "#070712",
    backgroundDeep: "#03030A",
    gold: "#D8B06B",
    goldBright: "#FFD98A",
    panel: "rgba(13, 12, 31, 0.94)",
    panelBorder: "rgba(216, 176, 107, 0.42)",
    panelSoft: "rgba(20, 17, 44, 0.72)",
    textMuted: "#8E826A",
    textPrimary: "#F0D9A6",
    textSecondary: "#C7B692",
    track: "rgba(255, 255, 255, 0.09)",
    trackGold: "rgba(216, 176, 107, 0.18)",
    white: "#EAE6FF",
  },
  dayNode: {
    dotGap: 7,
    dotSize: 10,
    rowHeight: 78,
    sizeFuture: 40,
    sizeSelected: 70,
    sizeWritten: 50,
  },
  kingdomColors: {
    blue: "#50B8FF",
    green: "#89D85B",
    indigo: "#6E7CFF",
    orange: "#F4A23E",
    red: "#FF745C",
    violet: "#AA70FF",
    white: "#E9E6FF",
    yellow: "#E9CA4F",
  },
  radius: {
    lg: 26,
    md: 18,
    pill: 999,
    sm: 12,
  },
  spacing: {
    lg: 16,
    md: 12,
    sm: 8,
    xl: 20,
    xs: 4,
    xxl: 28,
    xxxl: 36,
  },
  typography: {
    body: 14,
    caption: 13,
    cardTitle: 18,
    sheetTitle: 30,
    subtitle: 17,
    title: 34,
  },
} as const;

export const SOJOURN_KINGDOM_ORDER = [
  "red",
  "orange",
  "yellow",
  "green",
  "blue",
  "indigo",
  "violet",
  "white",
] as const;

export type SojournMapKingdom = (typeof SOJOURN_KINGDOM_ORDER)[number];

export function getMapKingdomForDay(day: number) {
  const safeDay = Math.max(1, Math.floor(day));
  const key = SOJOURN_KINGDOM_ORDER[(safeDay - 1) % SOJOURN_KINGDOM_ORDER.length];

  return {
    color: sojournMapTokens.kingdomColors[key],
    key,
  };
}
