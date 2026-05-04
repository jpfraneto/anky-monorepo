import { useMemo, useRef, useState } from "react";
import {
  Animated,
  PanResponder,
  StyleSheet,
  Text,
  View,
} from "react-native";

import { AnkyGlyph } from "../anky/AnkyGlyph";
import { ankyColors, spacing } from "../../theme/tokens";

type Props = {
  disabled?: boolean;
  error?: string;
  isSealing?: boolean;
  onSeal: () => Promise<void> | void;
  sealed?: boolean;
};

const KNOB_SIZE = 54;
const COMPLETE_THRESHOLD = 0.72;

export function SwipeToSealAction({
  disabled = false,
  error,
  isSealing = false,
  onSeal,
  sealed = false,
}: Props) {
  const translateX = useRef(new Animated.Value(0)).current;
  const progressRef = useRef(sealed ? 1 : 0);
  const [progress, setProgress] = useState(sealed ? 1 : 0);
  const [trackWidth, setTrackWidth] = useState(0);
  const maxSwipe = Math.max(0, trackWidth - KNOB_SIZE - 8);
  const unavailable = disabled && !sealed && !isSealing;

  const panResponder = useMemo(
    () =>
      PanResponder.create({
        onMoveShouldSetPanResponder: () => !disabled && !isSealing && !sealed,
        onStartShouldSetPanResponder: () => !disabled && !isSealing && !sealed,
        onPanResponderMove: (_event, gesture) => {
          const nextX = clamp(gesture.dx, 0, maxSwipe);
          const nextProgress = maxSwipe === 0 ? 0 : nextX / maxSwipe;

          translateX.setValue(nextX);
          progressRef.current = nextProgress;
          setProgress(nextProgress);
        },
        onPanResponderRelease: () => {
          if (progressRef.current >= COMPLETE_THRESHOLD) {
            Animated.timing(translateX, {
              duration: 120,
              toValue: maxSwipe,
              useNativeDriver: true,
            }).start(() => {
              progressRef.current = 1;
              setProgress(1);
              void Promise.resolve(onSeal()).finally(() => {
                if (!sealed) {
                  Animated.timing(translateX, {
                    duration: 180,
                    toValue: 0,
                    useNativeDriver: true,
                  }).start(() => {
                    progressRef.current = 0;
                    setProgress(0);
                  });
                }
              });
            });
            return;
          }

          Animated.timing(translateX, {
            duration: 180,
            toValue: 0,
            useNativeDriver: true,
          }).start(() => {
            progressRef.current = 0;
            setProgress(0);
          });
        },
      }),
    [disabled, isSealing, maxSwipe, onSeal, sealed, translateX],
  );

  return (
    <View style={styles.root}>
      <View
        onLayout={(event) => setTrackWidth(event.nativeEvent.layout.width)}
        style={[styles.track, unavailable && styles.unavailable, sealed && styles.sealedTrack]}
      >
        <View style={[styles.fill, { width: `${Math.max(progress, sealed ? 1 : 0) * 100}%` }]} />
        <View style={styles.copy}>
          <Text style={[styles.label, unavailable && styles.mutedLabel]}>
            {sealed
              ? "sealed"
              : isSealing
                ? "sealing"
                : unavailable
                  ? "sealing unavailable"
                  : "swipe to seal"}
          </Text>
          <Text style={styles.detail}>hash only • local writing stays private</Text>
        </View>
        <Animated.View
          {...panResponder.panHandlers}
          style={[
            styles.knob,
            {
              transform: [{ translateX: sealed ? maxSwipe : translateX }],
            },
          ]}
        >
          <AnkyGlyph size={30} />
        </Animated.View>
      </View>
      {unavailable ? <Text style={styles.quiet}>your writing is still local</Text> : null}
      {error == null || error.length === 0 ? null : <Text style={styles.error}>{error}</Text>}
    </View>
  );
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

const styles = StyleSheet.create({
  copy: {
    alignItems: "center",
    bottom: 0,
    justifyContent: "center",
    left: KNOB_SIZE + spacing.sm,
    position: "absolute",
    right: spacing.md,
    top: 0,
  },
  detail: {
    color: ankyColors.textMuted,
    fontSize: 11,
    fontWeight: "700",
    marginTop: 3,
    textTransform: "lowercase",
  },
  error: {
    color: ankyColors.danger,
    fontSize: 12,
    lineHeight: 18,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  fill: {
    backgroundColor: "rgba(215, 186, 115, 0.16)",
    bottom: 0,
    left: 0,
    position: "absolute",
    top: 0,
  },
  knob: {
    alignItems: "center",
    backgroundColor: ankyColors.bg,
    borderColor: ankyColors.borderStrong,
    borderRadius: 8,
    borderWidth: 1,
    height: KNOB_SIZE,
    justifyContent: "center",
    left: 4,
    position: "absolute",
    top: 4,
    width: KNOB_SIZE,
    zIndex: 2,
  },
  label: {
    color: ankyColors.text,
    fontSize: 16,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  mutedLabel: {
    color: ankyColors.textMuted,
  },
  quiet: {
    color: ankyColors.textMuted,
    fontSize: 12,
    marginTop: spacing.sm,
    textAlign: "center",
    textTransform: "lowercase",
  },
  root: {
    marginTop: spacing.lg,
  },
  sealedTrack: {
    borderColor: ankyColors.success,
  },
  track: {
    backgroundColor: ankyColors.bg3,
    borderColor: ankyColors.border,
    borderRadius: 8,
    borderWidth: 1,
    height: KNOB_SIZE + 8,
    overflow: "hidden",
  },
  unavailable: {
    opacity: 0.7,
  },
});
