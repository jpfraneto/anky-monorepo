import type { AnkyFrameId } from "./ankyFrameAssets";

export type AnkyPresenceMode = "hidden" | "sigil" | "companion";

export type AnkyEmotion =
  | "idle"
  | "welcome"
  | "walking"
  | "listening"
  | "complete"
  | "sleeping"
  | "softConcern";

export type AnkySequenceName =
  | "finding_thread"
  | "idle_front"
  | "idle_blink"
  | "wave_front"
  | "walk_right"
  | "celebrate"
  | "seated"
  | "sleeping"
  | "soft_concern"
  | "shy_listening";

export const ANKY_SEQUENCE_ORDER: AnkySequenceName[] = [
  "idle_front",
  "idle_blink",
  "wave_front",
  "walk_right",
  "celebrate",
  "seated",
  "sleeping",
  "soft_concern",
  "shy_listening",
];

export const ANKY_FRAME_SEQUENCES = {
  celebrate: [27, 28, 29, 30, 31],
  finding_thread: [
    "thread_001",
    "thread_002",
    "thread_003",
    "thread_004",
    "thread_005",
    "thread_006",
    "thread_007",
    "thread_008",
  ],
  idle_blink: [40, 41, 42, 43, 44, 45],
  idle_front: [1, 2, 3, 4, 5, 6],
  seated: [46, 47, 48, 49, 50, 51],
  shy_listening: [52, 53, 54, 55, 56, 57],
  sleeping: [36, 37, 38, 39],
  soft_concern: [32, 33, 34, 35],
  walk_right: [7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22],
  wave_front: [23, 24, 25, 26],
} as const satisfies Record<AnkySequenceName, readonly AnkyFrameId[]>;

export const DEFAULT_ANKY_SEQUENCE: AnkySequenceName = "idle_front";

export function resolveAnkySequenceFrames(
  requested: AnkySequenceName | undefined,
): readonly AnkyFrameId[] {
  const fallbackNames: AnkySequenceName[] = [
    requested ?? DEFAULT_ANKY_SEQUENCE,
    "idle_blink",
    "idle_front",
  ];

  for (const name of fallbackNames) {
    const frames = ANKY_FRAME_SEQUENCES[name];

    if (frames.length > 0) {
      return frames;
    }
  }

  return ANKY_FRAME_SEQUENCES[ANKY_SEQUENCE_ORDER[0]];
}

export function getEmotionForSequence(sequence: AnkySequenceName): AnkyEmotion {
  switch (sequence) {
    case "celebrate":
      return "complete";
    case "finding_thread":
      return "welcome";
    case "seated":
      return "idle";
    case "shy_listening":
      return "listening";
    case "sleeping":
      return "sleeping";
    case "soft_concern":
      return "softConcern";
    case "walk_right":
      return "walking";
    case "wave_front":
      return "welcome";
    case "idle_blink":
    case "idle_front":
      return "idle";
  }
}

export function getFpsForSequence(sequence: AnkySequenceName, mode: AnkyPresenceMode): number {
  if (mode === "sigil") {
    return sequence === "walk_right" ? 5 : 3;
  }

  switch (sequence) {
    case "finding_thread":
      return 5;
    case "walk_right":
      return 8;
    case "celebrate":
    case "wave_front":
      return 5;
    case "sleeping":
      return 2;
    default:
      return 4;
  }
}

export function getNextAnkySequence(current: AnkySequenceName): AnkySequenceName {
  const index = ANKY_SEQUENCE_ORDER.indexOf(current);
  const nextIndex = index < 0 ? 0 : (index + 1) % ANKY_SEQUENCE_ORDER.length;

  return ANKY_SEQUENCE_ORDER[nextIndex];
}
