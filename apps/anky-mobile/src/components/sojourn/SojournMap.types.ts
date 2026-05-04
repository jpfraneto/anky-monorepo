import type { ImageSourcePropType } from "react-native";

export type SojournMapAnky = {
  avatar: ImageSourcePropType;
  day: number;
  durationLabel: string;
  fileName?: string;
  firstLine: string;
  id: string;
  sessionHash?: string;
  title: string;
};

export type SojournMapDay = {
  ankyCount: number;
  ankys: SojournMapAnky[];
  day: number;
  isCurrent?: boolean;
  isFuture?: boolean;
};
