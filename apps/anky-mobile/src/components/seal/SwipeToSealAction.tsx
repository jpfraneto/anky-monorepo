import { useEffect, useMemo, useRef, useState } from "react";
import {
  ActivityIndicator,
  Animated,
  Linking,
  PanResponder,
  Pressable,
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
  sealNetwork?: "devnet" | "mainnet-beta";
  sealSignature?: string;
  sealed?: boolean;
  walletKind?: "embedded" | "external";
};

const KNOB_SIZE = 56;
const TRACK_PADDING = 4;
const COMPLETE_THRESHOLD = 0.86;
const FAST_SWIPE_VELOCITY = 1.1;

export function SwipeToSealAction({
  disabled = false,
  error,
  isSealing = false,
  onSeal,
  sealNetwork,
  sealSignature,
  sealed = false,
  walletKind,
}: Props) {
  const translateX = useRef(new Animated.Value(0)).current;
  const fillProgress = useRef(new Animated.Value(sealed ? 1 : 0)).current;
  const panStartXRef = useRef(0);
  const progressRef = useRef(sealed ? 1 : 0);
  const [trackWidth, setTrackWidth] = useState(0);
  const maxSwipe = Math.max(0, trackWidth - KNOB_SIZE - TRACK_PADDING * 2);
  const unavailable = disabled && !sealed && !isSealing;
  const state = sealed
    ? "sealed"
    : isSealing
      ? "sealing"
      : error != null && error.length > 0
        ? "error"
        : unavailable
          ? "unavailable"
          : "ready";
  const label = getSealLabel(state, walletKind);
  const detail = getSealDetail(state, error, walletKind);
  const hasSealProof = sealed && sealSignature != null && sealSignature.length > 0;
  const trackHeight = hasSealProof ? 104 : KNOB_SIZE + TRACK_PADDING * 2;
  const knobTop = (trackHeight - KNOB_SIZE) / 2;
  const orbUrl = hasSealProof ? getOrbTxUrl(sealSignature, sealNetwork) : null;
  const fillWidth =
    trackWidth <= 0
      ? KNOB_SIZE
      : fillProgress.interpolate({
          inputRange: [0, 1],
          outputRange: [KNOB_SIZE + TRACK_PADDING, trackWidth],
        });

  const panResponder = useMemo(
    () =>
      PanResponder.create({
        onMoveShouldSetPanResponder: (_event, gesture) =>
          !disabled && !isSealing && !sealed && Math.abs(gesture.dx) > 2,
        onStartShouldSetPanResponder: () => !disabled && !isSealing && !sealed,
        onPanResponderGrant: () => {
          fillProgress.stopAnimation();
          translateX.stopAnimation((value) => {
            panStartXRef.current = typeof value === "number" ? value : 0;
          });
        },
        onPanResponderMove: (_event, gesture) => {
          const nextX = clamp(panStartXRef.current + gesture.dx, 0, maxSwipe);
          const nextProgress = maxSwipe === 0 ? 0 : nextX / maxSwipe;

          translateX.setValue(nextX);
          fillProgress.setValue(nextProgress);
          progressRef.current = nextProgress;
        },
        onPanResponderRelease: (_event, gesture) => {
          const shouldComplete =
            progressRef.current >= COMPLETE_THRESHOLD || gesture.vx > FAST_SWIPE_VELOCITY;

          if (shouldComplete) {
            animateSwipeTo({
              fillProgress,
              progress: 1,
              translateX,
              x: maxSwipe,
            }).start(() => {
              progressRef.current = 1;
              void Promise.resolve(onSeal()).catch(() => undefined);
            });
            return;
          }

          animateSwipeTo({
            fillProgress,
            progress: 0,
            translateX,
            x: 0,
          }).start(() => {
            progressRef.current = 0;
          });
        },
        onPanResponderTerminate: () => {
          animateSwipeTo({
            fillProgress,
            progress: 0,
            translateX,
            x: 0,
          }).start(() => {
            progressRef.current = 0;
          });
        },
      }),
    [disabled, fillProgress, isSealing, maxSwipe, onSeal, sealed, translateX],
  );

  useEffect(() => {
    if (trackWidth === 0) {
      return;
    }

    if (sealed || isSealing) {
      progressRef.current = 1;
      animateSwipeTo({
        fillProgress,
        progress: 1,
        translateX,
        x: maxSwipe,
      }).start();
      return;
    }

    if (error != null && error.length > 0) {
      progressRef.current = 0;
      animateSwipeTo({
        fillProgress,
        progress: 0,
        translateX,
        x: 0,
      }).start();
      return;
    }
  }, [error, fillProgress, isSealing, maxSwipe, sealed, trackWidth, translateX]);

  return (
    <View style={styles.root}>
      <View
        {...panResponder.panHandlers}
        onLayout={(event) => setTrackWidth(event.nativeEvent.layout.width)}
        style={[
          styles.track,
          { height: trackHeight },
          state === "unavailable" && styles.unavailable,
          state === "error" && styles.errorTrack,
          sealed && styles.sealedTrack,
        ]}
      >
        <Animated.View
          style={[
            styles.fill,
            (state === "sealing" || state === "sealed") && styles.fillStrong,
            state === "error" && styles.fillError,
            { width: fillWidth },
          ]}
        />
        <View style={styles.copy}>
          <Text
            adjustsFontSizeToFit
            numberOfLines={1}
            style={[
              styles.label,
              state === "unavailable" && styles.mutedLabel,
              state === "error" && styles.errorLabel,
            ]}
          >
            {label}
          </Text>
          {hasSealProof ? (
            <View style={styles.proofBlock}>
              <Text numberOfLines={1} style={styles.txHash}>
                tx hash {shortSignature(sealSignature)}
              </Text>
              <Pressable
                accessibilityRole="link"
                disabled={orbUrl == null}
                onPress={() => {
                  if (orbUrl != null) {
                    void Linking.openURL(orbUrl).catch(() => undefined);
                  }
                }}
                style={({ pressed }) => [styles.orbLink, pressed && styles.linkPressed]}
              >
                <Text style={styles.orbText}>view on orb</Text>
              </Pressable>
            </View>
          ) : (
            <Text numberOfLines={2} style={styles.detail}>
              {detail}
            </Text>
          )}
        </View>
        <Animated.View
          style={[
            styles.knob,
            {
              top: knobTop,
              transform: [{ translateX: sealed ? maxSwipe : translateX }],
            },
          ]}
        >
          {isSealing ? <ActivityIndicator color={ankyColors.gold} size="small" /> : <AnkyGlyph size={30} />}
        </Animated.View>
      </View>
    </View>
  );
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function getOrbTxUrl(
  signature: string,
  network?: "devnet" | "mainnet-beta",
): string {
  const cluster = network === "devnet" ? "?cluster=devnet" : "";
  return `https://orbmarkets.io/tx/${encodeURIComponent(signature)}${cluster}`;
}

function shortSignature(signature: string): string {
  if (signature.length <= 18) {
    return signature;
  }

  return `${signature.slice(0, 8)}...${signature.slice(-8)}`;
}

function animateSwipeTo({
  fillProgress,
  progress,
  translateX,
  x,
}: {
  fillProgress: Animated.Value;
  progress: number;
  translateX: Animated.Value;
  x: number;
}) {
  return Animated.parallel([
    Animated.spring(translateX, {
      damping: progress === 1 ? 18 : 20,
      mass: 0.72,
      stiffness: progress === 1 ? 230 : 250,
      toValue: x,
      useNativeDriver: true,
    }),
    Animated.spring(fillProgress, {
      damping: progress === 1 ? 18 : 20,
      mass: 0.72,
      stiffness: progress === 1 ? 230 : 250,
      toValue: progress,
      useNativeDriver: false,
    }),
  ]);
}

function getSealLabel(
  state: "error" | "ready" | "sealed" | "sealing" | "unavailable",
  walletKind?: "embedded" | "external",
): string {
  switch (state) {
    case "error":
      return "seal did not complete";
    case "sealed":
      return "sealed with loom";
    case "sealing":
      return walletKind === "external" ? "waiting for wallet" : "sealing hash";
    case "unavailable":
      return "sealing unavailable";
    case "ready":
      return "swipe to seal";
  }
}

function getSealDetail(
  state: "error" | "ready" | "sealed" | "sealing" | "unavailable",
  error?: string,
  walletKind?: "embedded" | "external",
): string {
  switch (state) {
    case "error":
      return error ?? "your writing is still local";
    case "sealed":
      return "hash only • local writing stayed private";
    case "sealing":
      return walletKind === "external"
        ? "approve the hash-only seal in your wallet"
        : "hash only • local writing stays private";
    case "unavailable":
      return "your writing is still local";
    case "ready":
      return "hash only • local writing stays private";
  }
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
    maxWidth: "100%",
    paddingHorizontal: spacing.xs,
    textAlign: "center",
    textTransform: "lowercase",
  },
  errorLabel: {
    color: "#F19A72",
  },
  errorTrack: {
    borderColor: "rgba(241, 154, 114, 0.42)",
  },
  fill: {
    backgroundColor: "rgba(139, 124, 246, 0.12)",
    bottom: 0,
    left: 0,
    position: "absolute",
    top: 0,
  },
  fillError: {
    backgroundColor: "rgba(241, 154, 114, 0.1)",
  },
  fillStrong: {
    backgroundColor: "rgba(215, 186, 115, 0.18)",
  },
  knob: {
    alignItems: "center",
    backgroundColor: "#0B0920",
    borderColor: "rgba(232, 200, 121, 0.54)",
    borderRadius: 8,
    borderWidth: 1,
    height: KNOB_SIZE,
    justifyContent: "center",
    left: TRACK_PADDING,
    position: "absolute",
    width: KNOB_SIZE,
    zIndex: 2,
  },
  label: {
    color: ankyColors.gold,
    fontSize: 16,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  mutedLabel: {
    color: ankyColors.textMuted,
  },
  linkPressed: {
    opacity: 0.7,
  },
  proofBlock: {
    alignItems: "center",
    marginTop: 5,
  },
  quiet: {
    color: ankyColors.textMuted,
    fontSize: 12,
    marginTop: spacing.sm,
    textAlign: "center",
    textTransform: "lowercase",
  },
  root: {
    width: "100%",
  },
  sealedTrack: {
    borderColor: "rgba(134, 239, 172, 0.42)",
  },
  orbLink: {
    marginTop: 2,
    paddingHorizontal: spacing.sm,
    paddingVertical: 2,
  },
  orbText: {
    color: ankyColors.violetBright,
    fontSize: 12,
    fontWeight: "800",
    textTransform: "lowercase",
  },
  track: {
    backgroundColor: "rgba(15, 12, 34, 0.96)",
    borderColor: "rgba(232, 200, 121, 0.26)",
    borderRadius: 8,
    borderWidth: 1,
    overflow: "hidden",
  },
  txHash: {
    color: ankyColors.text,
    fontSize: 12,
    fontWeight: "700",
  },
  unavailable: {
    opacity: 0.7,
  },
});
