import type { ImageSourcePropType } from "react-native";

export type SojournMapAnky = {
  avatar: ImageSourcePropType;
  createdAt: string;
  day: number;
  durationLabel: string;
  fileName?: string;
  firstLine: string;
  id: string;
  kind: "anky" | "fragment";
  sessionHash?: string;
  title: string;
};

export type SojournMapDay = {
  ankyCount: number;
  ankys: SojournMapAnky[];
  day: number;
  fragmentCount?: number;
  isCurrent?: boolean;
  isFuture?: boolean;
};
