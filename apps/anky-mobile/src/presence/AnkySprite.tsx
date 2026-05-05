import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  Animated,
  Easing,
  Image,
  type ImageStyle,
  type StyleProp,
  type ViewStyle,
} from "react-native";

import { ANKY_FRAMES } from "./ankyFrameAssets";
import { resolveAnkySequenceFrames, type AnkySequenceName } from "./ankySequences";

/**
 * Anky Presence Rule:
 * Anky is a witness, not a mascot.
 * It should be available, never interruptive.
 * During writing, Anky becomes almost silent.
 */

export type AnkySpriteProps = {
  fps?: number;
  imageStyle?: StyleProp<ImageStyle>;
  loop?: boolean;
  opacity?: number;
  paused?: boolean;
  sequence?: AnkySequenceName;
  size?: number;
  style?: StyleProp<ViewStyle>;
};

export const AnkySprite = React.memo(function AnkySprite({
  fps = 4,
  imageStyle,
  loop = true,
  opacity = 1,
  paused = false,
  sequence = "idle_front",
  size = 88,
  style,
}: AnkySpriteProps) {
  const frames = useMemo(() => resolveAnkySequenceFrames(sequence), [sequence]);
  const [cursor, setCursor] = useState(0);
  const breath = useRef(new Animated.Value(1)).current;
  const fade = useRef(new Animated.Value(opacity)).current;

  useEffect(() => {
    setCursor(0);

    if (paused || fps <= 0 || frames.length <= 1) {
      return undefined;
    }

    const frameMs = Math.max(80, Math.round(1000 / fps));
    const interval = setInterval(() => {
      setCursor((current) => {
        const next = current + 1;

        if (next < frames.length) {
          return next;
        }

        return loop ? 0 : current;
      });
    }, frameMs);

    return () => clearInterval(interval);
  }, [fps, frames, loop, paused]);

  useEffect(() => {
    Animated.timing(fade, {
      duration: 220,
      easing: Easing.out(Easing.quad),
      toValue: opacity,
      useNativeDriver: true,
    }).start();
  }, [fade, opacity]);

  useEffect(() => {
    const animation = Animated.loop(
      Animated.sequence([
        Animated.timing(breath, {
          duration: 1800,
          easing: Easing.inOut(Easing.quad),
          toValue: 1.018,
          useNativeDriver: true,
        }),
        Animated.timing(breath, {
          duration: 1800,
          easing: Easing.inOut(Easing.quad),
          toValue: 1,
          useNativeDriver: true,
        }),
      ]),
    );

    animation.start();

    return () => animation.stop();
  }, [breath]);

  const frameId = frames[Math.min(cursor, frames.length - 1)];
  const source = ANKY_FRAMES[frameId];

  return (
    <Animated.View
      pointerEvents="none"
      style={[
        {
          alignItems: "center",
          height: size,
          justifyContent: "center",
          opacity: fade,
          transform: [{ scale: breath }],
          width: size,
        },
        style,
      ]}
    >
      <Image
        accessibilityIgnoresInvertColors
        resizeMode="contain"
        source={source}
        style={[
          {
            height: size,
            width: size,
          },
          imageStyle,
        ]}
      />
    </Animated.View>
  );
});
