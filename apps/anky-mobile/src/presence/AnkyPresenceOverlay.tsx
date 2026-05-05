import { useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  Animated,
  Easing,
  Image,
  Keyboard,
  PanResponder,
  Platform,
  Pressable,
  StyleSheet,
  useWindowDimensions,
  View,
} from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import { AnkySprite } from "./AnkySprite";
import { useAnkyPresence } from "./AnkyPresenceContext";
import { getFpsForSequence, type AnkySequenceName } from "./ankySequences";

const ANKY_SIGIL = require("../../assets/anky_sigil_smal.png");

/**
 * Anky Presence Rule:
 * Anky is a witness, not a mascot.
 * It should be available, never interruptive.
 * During writing, Anky becomes almost silent.
 */

export function AnkyPresenceOverlay() {
  const insets = useSafeAreaInsets();
  const { height, width } = useWindowDimensions();
  const {
    cycleSequence,
    effectiveMode,
    intensity,
    screenConfig,
    sequence,
    setMode,
    togglePresence,
  } = useAnkyPresence();
  const [keyboardHeight, setKeyboardHeight] = useState(0);
  const [dragPosition, setDragPosition] = useState<PresencePoint | null>(null);
  const [outgoingCompanion, setOutgoingCompanion] = useState<OutgoingCompanion | null>(null);
  const floatY = useRef(new Animated.Value(0)).current;
  const fold = useRef(new Animated.Value(effectiveMode === "companion" ? 1 : 0)).current;
  const outgoingOpacity = useRef(new Animated.Value(0)).current;
  const panStartRef = useRef<PresencePoint | null>(null);
  const previousCompanionRef = useRef<OutgoingCompanion | null>(null);
  const previousModeRef = useRef(effectiveMode);
  const currentPositionRef = useRef<PresencePoint>({ x: 0, y: 0 });
  const suppressPressRef = useRef(false);
  const isMinimal = intensity === "minimal" || screenConfig?.maxMode === "sigil";
  const shouldAvoidKeyboard = screenConfig?.avoidKeyboard === true;

  useEffect(() => {
    const showEvent = Platform.OS === "ios" ? "keyboardWillShow" : "keyboardDidShow";
    const hideEvent = Platform.OS === "ios" ? "keyboardWillHide" : "keyboardDidHide";
    const showSubscription = Keyboard.addListener(showEvent, (event) => {
      setKeyboardHeight(event.endCoordinates.height);
    });
    const hideSubscription = Keyboard.addListener(hideEvent, () => {
      setKeyboardHeight(0);
    });

    return () => {
      showSubscription.remove();
      hideSubscription.remove();
    };
  }, []);

  useEffect(() => {
    const animation = Animated.loop(
      Animated.sequence([
        Animated.timing(floatY, {
          duration: 3200,
          easing: Easing.inOut(Easing.quad),
          toValue: isMinimal ? -1 : -3,
          useNativeDriver: true,
        }),
        Animated.timing(floatY, {
          duration: 3200,
          easing: Easing.inOut(Easing.quad),
          toValue: 0,
          useNativeDriver: true,
        }),
      ]),
    );

    animation.start();

    return () => animation.stop();
  }, [floatY, isMinimal]);

  useEffect(() => {
    Animated.timing(fold, {
      duration: 380,
      easing: Easing.out(Easing.cubic),
      toValue: effectiveMode === "companion" ? 1 : 0,
      useNativeDriver: true,
    }).start();
  }, [effectiveMode, fold]);

  const visual = useMemo(() => {
    if (effectiveMode === "companion") {
      return {
        fps: getFpsForSequence(sequence, effectiveMode),
        opacity: 1,
        size: isMinimal ? 72 : 84,
      };
    }

    return {
      fps: getFpsForSequence(sequence, "sigil"),
      opacity: isMinimal ? 0.42 : 0.7,
      size: isMinimal ? 72 : 84,
    };
  }, [effectiveMode, isMinimal, sequence]);

  useEffect(() => {
    if (effectiveMode === "companion") {
      previousCompanionRef.current = {
        fps: visual.fps,
        opacity: visual.opacity,
        sequence,
        size: visual.size,
      };
    }
  }, [effectiveMode, sequence, visual.fps, visual.opacity, visual.size]);

  useEffect(() => {
    const previousMode = previousModeRef.current;

    if (previousMode === "companion" && effectiveMode !== "companion") {
      const outgoing = previousCompanionRef.current;

      if (outgoing != null) {
        outgoingOpacity.stopAnimation();
        outgoingOpacity.setValue(outgoing.opacity);
        setOutgoingCompanion(outgoing);
        Animated.timing(outgoingOpacity, {
          duration: 420,
          easing: Easing.out(Easing.quad),
          toValue: 0,
          useNativeDriver: true,
        }).start(({ finished }) => {
          if (finished) {
            setOutgoingCompanion(null);
          }
        });
      }
    }

    if (effectiveMode === "companion") {
      outgoingOpacity.stopAnimation();
      setOutgoingCompanion(null);
    }

    previousModeRef.current = effectiveMode;
  }, [effectiveMode, outgoingOpacity]);

  const scale = fold.interpolate({
    inputRange: [0, 1],
    outputRange: [0.86, 1],
  });
  const restingBottomOffset = insets.bottom + (isMinimal ? 18 : 106);
  const keyboardBottomOffset =
    shouldAvoidKeyboard && keyboardHeight > 0 ? keyboardHeight + (isMinimal ? 12 : 18) : 0;
  const bottomOffset = Math.max(restingBottomOffset, keyboardBottomOffset);
  const touchSize = Math.max(48, visual.size + 8);
  const bounds = useMemo(
    () => getPresenceBounds({ bottomOffset, height, insets, touchSize, width }),
    [bottomOffset, height, insets, touchSize, width],
  );
  const defaultPosition = useMemo(
    () =>
      getDefaultPresencePosition({
        bounds,
        placement: screenConfig?.placement ?? "right",
      }),
    [bounds, screenConfig?.placement],
  );
  const currentPosition = useMemo(
    () => clampPresencePoint(dragPosition ?? defaultPosition, bounds),
    [bounds, defaultPosition, dragPosition],
  );

  useEffect(() => {
    currentPositionRef.current = currentPosition;
  }, [currentPosition]);

  const panResponder = useMemo(
    () =>
      PanResponder.create({
        onMoveShouldSetPanResponder: (_, gesture) =>
          Math.abs(gesture.dx) > 4 || Math.abs(gesture.dy) > 4,
        onPanResponderGrant: () => {
          panStartRef.current = currentPositionRef.current;
        },
        onPanResponderMove: (_, gesture) => {
          if (panStartRef.current == null) {
            return;
          }

          setDragPosition(
            clampPresencePoint(
              {
                x: panStartRef.current.x + gesture.dx,
                y: panStartRef.current.y + gesture.dy,
              },
              bounds,
            ),
          );
        },
        onPanResponderRelease: (_, gesture) => {
          if (panStartRef.current != null) {
            setDragPosition(
              clampPresencePoint(
                {
                  x: panStartRef.current.x + gesture.dx,
                  y: panStartRef.current.y + gesture.dy,
                },
                bounds,
              ),
            );
          }

          panStartRef.current = null;
          suppressPressRef.current = true;
          setTimeout(() => {
            suppressPressRef.current = false;
          }, 80);
        },
        onPanResponderTerminate: () => {
          panStartRef.current = null;
        },
        onStartShouldSetPanResponder: () => false,
      }),
    [bounds],
  );

  if (effectiveMode === "hidden") {
    return null;
  }

  return (
    <View pointerEvents="box-none" style={StyleSheet.absoluteFill}>
      <Animated.View
        {...panResponder.panHandlers}
        style={[
          styles.touchTarget,
          {
            height: touchSize,
            left: currentPosition.x,
            top: currentPosition.y,
            width: touchSize,
          },
        ]}
      >
        <Pressable
          accessibilityLabel="Anky presence"
          accessibilityRole="button"
          hitSlop={12}
          onLongPress={() => {
            Alert.alert("Anky presence", undefined, [
              { onPress: () => setMode("companion"), text: "Show Anky" },
              { onPress: () => setMode("sigil"), text: "Sigil only" },
              { onPress: () => setMode("hidden"), text: "Hide Anky" },
              { onPress: cycleSequence, text: "Change motion / cycle sequence" },
              { style: "cancel", text: "Cancel" },
            ]);
          }}
          onPress={() => {
            if (!suppressPressRef.current) {
              togglePresence();
            }
          }}
          style={styles.pressTarget}
        >
          <Animated.View
            pointerEvents="none"
            style={[
              styles.spriteWrap,
              {
                transform: [{ translateY: floatY }, { scale }],
              },
            ]}
          >
            {effectiveMode === "sigil" ? (
              <Image
                accessibilityIgnoresInvertColors
                resizeMode="contain"
                source={ANKY_SIGIL}
                style={{
                  height: visual.size,
                  opacity: visual.opacity,
                  width: visual.size,
                }}
              />
            ) : (
              <AnkySprite
                fps={visual.fps}
                opacity={visual.opacity}
                sequence={sequence}
                size={visual.size}
              />
            )}
            {outgoingCompanion != null ? (
              <Animated.View
                pointerEvents="none"
                style={[
                  styles.outgoingSprite,
                  {
                    height: outgoingCompanion.size,
                    left: (visual.size - outgoingCompanion.size) / 2,
                    opacity: outgoingOpacity,
                    top: (visual.size - outgoingCompanion.size) / 2,
                    width: outgoingCompanion.size,
                  },
                ]}
              >
                <AnkySprite
                  fps={outgoingCompanion.fps}
                  opacity={1}
                  sequence={outgoingCompanion.sequence}
                  size={outgoingCompanion.size}
                />
              </Animated.View>
            ) : null}
          </Animated.View>
        </Pressable>
      </Animated.View>
    </View>
  );
}

type PresencePoint = {
  x: number;
  y: number;
};

type PresenceBounds = {
  maxX: number;
  maxY: number;
  minX: number;
  minY: number;
};

type OutgoingCompanion = {
  fps: number;
  opacity: number;
  sequence: AnkySequenceName;
  size: number;
};

function getPresenceBounds({
  bottomOffset,
  height,
  insets,
  touchSize,
  width,
}: {
  bottomOffset: number;
  height: number;
  insets: { bottom: number; left: number; right: number; top: number };
  touchSize: number;
  width: number;
}): PresenceBounds {
  const minX = insets.left + 8;
  const minY = insets.top + 8;

  return {
    maxX: Math.max(minX, width - insets.right - touchSize - 8),
    maxY: Math.max(minY, height - bottomOffset - touchSize),
    minX,
    minY,
  };
}

function getDefaultPresencePosition({
  bounds,
  placement,
}: {
  bounds: PresenceBounds;
  placement: "left" | "right";
}): PresencePoint {
  return {
    x: placement === "left" ? bounds.minX + 6 : bounds.maxX - 6,
    y: bounds.maxY,
  };
}

function clampPresencePoint(point: PresencePoint, bounds: PresenceBounds): PresencePoint {
  return {
    x: Math.min(bounds.maxX, Math.max(bounds.minX, point.x)),
    y: Math.min(bounds.maxY, Math.max(bounds.minY, point.y)),
  };
}

const styles = StyleSheet.create({
  pressTarget: {
    alignItems: "center",
    height: "100%",
    justifyContent: "center",
    width: "100%",
  },
  outgoingSprite: {
    alignItems: "center",
    justifyContent: "center",
    position: "absolute",
  },
  spriteWrap: {
    alignItems: "center",
    justifyContent: "center",
    position: "relative",
  },
  touchTarget: {
    alignItems: "center",
    justifyContent: "center",
    position: "absolute",
    zIndex: 100,
  },
});
