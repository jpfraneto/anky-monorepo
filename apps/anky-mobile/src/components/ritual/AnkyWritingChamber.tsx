import { useEffect, useMemo, useState } from "react";
import {
  Image,
  Platform,
  StyleSheet,
  Text,
  useWindowDimensions,
  View,
} from "react-native";
import {
  Canvas,
  Circle,
  Group,
  Line,
  matchFont,
  Path,
  RadialGradient,
  Skia,
  Text as SkiaText,
} from "@shopify/react-native-skia";
import Animated, {
  Easing,
  useAnimatedStyle,
  useSharedValue,
  withRepeat,
  withSequence,
  withTiming,
} from "react-native-reanimated";
import Svg, {
  Circle as SvgCircle,
  G,
  Line as SvgLine,
  Polygon as SvgPolygon,
} from "react-native-svg";

const ANCHOR_COUNT = 96;
const END_SILENCE_MS = 8000;
const ECHO_LIFE_MS = 8000;
const GOLD = [223, 196, 122] as const;
const MAX_PAUSE_MS = 1800;
const MAX_THREADS = 1400;
const RITE_DURATION_MS = 8 * 60 * 1000;
const TURN_BUCKETS = 8;
const ANKY_WRITING_BACKGROUND = require("../../../assets/anky-writing-background.png");
const ANKY_WRITING_WITNESS = require("../../../assets/anky-writing-witness.png");

const KINGDOMS = [
  [356, 78, 58],
  [24, 86, 58],
  [48, 92, 62],
  [137, 48, 54],
  [201, 78, 62],
  [242, 56, 66],
  [282, 58, 68],
  [44, 18, 84],
] as const;

export type ChamberAcceptedGlyph = {
  acceptedAt: number;
  char: string;
  deltaMs: number;
  id: number;
};

export type ChamberKeyPulse = {
  acceptedAt: number;
  char: string;
  id: number;
  kind: "accepted" | "deletion";
};

type Props = {
  acceptedGlyphs: ChamberAcceptedGlyph[];
  keyboardHeight: number;
  keyPulses: ChamberKeyPulse[];
  lastVisualGlyph: ChamberKeyPulse | null;
  phase: "active" | "revealed" | "unraveling";
  progress: number;
  revealStartedAt: number | null;
  safeBottom: number;
  silenceProgress: number;
  startedAt: number | null;
};

type Point = {
  x: number;
  y: number;
};

type ChamberLayout = {
  center: Point;
  radius: number;
  vesselRadius: number;
  vesselSize: number;
  visibleHeight: number;
  voidRadius: number;
};

type ThreadSegment = {
  a: Point;
  b: Point;
  born: number;
  control: Point;
  glow: number;
  hue: number;
  index: number;
  path: string;
  pauseRatio: number;
  speed: number;
  weight: number;
};

type Spark = {
  born: number;
  hue: number;
  life: number;
  size: number;
  vx: number;
  vy: number;
  x: number;
  y: number;
};

type GlyphEcho = {
  born: number;
  hue: number;
  mergeX: number;
  mergeY: number;
  size: number;
  text: string;
  x: number;
  y: number;
};

type Breath = {
  born: number;
  hue: number;
  intensity: number;
  life: number;
};

export function AnkyWritingChamber({
  acceptedGlyphs,
  keyboardHeight,
  keyPulses,
  lastVisualGlyph,
  phase,
  progress,
  revealStartedAt,
  safeBottom,
  silenceProgress,
  startedAt,
}: Props) {
  const { height, width } = useWindowDimensions();
  const [frameNow, setFrameNow] = useState(() => Date.now());
  const layout = useMemo(
    () => makeChamberLayout(width, height, keyboardHeight),
    [height, keyboardHeight, width],
  );
  const daySeed = useMemo(() => getDaySeed(), []);
  const kingdom = KINGDOMS[daySeed % KINGDOMS.length];
  const field = useMemo(
    () => buildThreadField(acceptedGlyphs, layout, kingdom, daySeed),
    [acceptedGlyphs, daySeed, kingdom, layout],
  );

  useEffect(() => {
    let frame = 0;
    let mounted = true;

    function tick() {
      if (!mounted) {
        return;
      }

      setFrameNow(Date.now());
      frame = requestAnimationFrame(tick);
    }

    frame = requestAnimationFrame(tick);

    return () => {
      mounted = false;
      cancelAnimationFrame(frame);
    };
  }, []);

  return (
    <View pointerEvents="none" style={StyleSheet.absoluteFill}>
      <BackgroundLayer />
      <ThreadField
        acceptedGlyphs={acceptedGlyphs}
        field={field}
        frameNow={frameNow}
        kingdom={kingdom}
        layout={layout}
        phase={phase}
        revealStartedAt={revealStartedAt}
        silenceProgress={silenceProgress}
      />
      <VesselCircle
        acceptedGlyphs={acceptedGlyphs}
        frameNow={frameNow}
        kingdom={kingdom}
        layout={layout}
        progress={progress}
        startedAt={startedAt}
      />
      <KeyPulseLayer
        frameNow={frameNow}
        height={height}
        keyboardHeight={keyboardHeight}
        kingdom={kingdom}
        keyPulses={keyPulses}
        layout={layout}
        width={width}
      />
      <CurrentGlyph event={lastVisualGlyph} layout={layout} silenceProgress={silenceProgress} />
      <AnkyWitness keyboardHeight={keyboardHeight} safeBottom={safeBottom} width={width} />
    </View>
  );
}

function BackgroundLayer() {
  return (
    <View style={styles.backgroundLayer}>
      <Image
        accessibilityIgnoresInvertColors
        resizeMode="cover"
        source={ANKY_WRITING_BACKGROUND}
        style={styles.backgroundImage}
      />
      <View style={styles.backgroundInk} />
      <View style={styles.backgroundThreads} />
      <View style={styles.backgroundVignette} />
    </View>
  );
}

function ThreadField({
  acceptedGlyphs,
  field,
  frameNow,
  kingdom,
  layout,
  phase,
  revealStartedAt,
  silenceProgress,
}: {
  acceptedGlyphs: ChamberAcceptedGlyph[];
  field: ReturnType<typeof buildThreadField>;
  frameNow: number;
  kingdom: readonly [number, number, number];
  layout: ChamberLayout;
  phase: Props["phase"];
  revealStartedAt: number | null;
  silenceProgress: number;
}) {
  const revealAge = revealStartedAt == null ? 0 : Math.max(0, frameNow - revealStartedAt);
  const revealT = phase === "active" ? 0 : clamp(revealAge / 5200, 0, 1);
  const isRevealing = phase !== "active";
  const latestAge =
    acceptedGlyphs.length === 0 ? Infinity : frameNow - acceptedGlyphs[acceptedGlyphs.length - 1].acceptedAt;
  const typedGlow = acceptedGlyphs.length === 0 ? 0 : Math.max(0, 1 - latestAge / 1800);
  const shimmer = 0.52 + Math.sin(frameNow / 1800) * 0.48;
  const potential = makePotentialPath(field.threadHead, field.threadAngle, layout, silenceProgress, frameNow);
  const visibleThreads = isRevealing ? field.threads : field.threads.slice(-MAX_THREADS);
  const activeSparks = isRevealing
    ? []
    : field.sparks.filter((spark) => frameNow - spark.born >= 0 && frameNow - spark.born < spark.life);
  const activeEchoes = isRevealing
    ? []
    : field.echoes.filter((echo) => frameNow - echo.born >= 0 && frameNow - echo.born < ECHO_LIFE_MS);
  const activeBreaths = isRevealing
    ? []
    : field.breaths.filter((breath) => frameNow - breath.born >= 0 && frameNow - breath.born < breath.life);
  const echoFont = matchFont({
    fontFamily: Platform.select({ android: "serif", ios: "Georgia", default: "serif" }),
    fontSize: 22,
    fontStyle: "normal",
    fontWeight: "400",
  });

  return (
    <Canvas style={StyleSheet.absoluteFill}>
      <Circle cx={layout.center.x} cy={layout.center.y} r={layout.radius * 1.6} opacity={0.9}>
        <RadialGradient
          c={layout.center}
          colors={[
            hsla(kingdom[0], kingdom[1], kingdom[2], 0.03 + typedGlow * 0.1),
            hsla(kingdom[0], kingdom[1], kingdom[2], 0.012 + typedGlow * 0.04),
            "rgba(0, 0, 0, 0)",
          ]}
          positions={[0, 0.42, 1]}
          r={layout.radius * 1.6}
        />
      </Circle>

      <Group opacity={0.52 + shimmer * 0.1}>
        <Path
          color={`rgba(${GOLD[0]}, ${GOLD[1]}, ${GOLD[2]}, 0.15)`}
          path={makeOctagonPath(layout.center, layout.radius, Math.PI / 8)}
          strokeWidth={1}
          style="stroke"
        />
        <Path
          color={hsla(kingdom[0], kingdom[1], kingdom[2], 0.09)}
          path={makeOctagonPath(layout.center, layout.radius, Math.PI / 8)}
          strokeWidth={1}
          style="stroke"
        />
        {[0.34, 0.52, 0.7, 0.86].map((scale, index) => (
          <Path
            color={hsla(kingdom[0], kingdom[1], kingdom[2], 0.045 + index * 0.012)}
            key={scale}
            path={makeOctagonPath(
              layout.center,
              layout.radius * scale + Math.sin(frameNow / 2400 + index) * 2,
              Math.PI / 8 + index * 0.003,
            )}
            strokeWidth={1}
            style="stroke"
          />
        ))}
        {field.anchors.map((anchor, index) => (
          <Circle
            color={
              index % 12 === 0
                ? `rgba(${GOLD[0]}, ${GOLD[1]}, ${GOLD[2]}, 0.22)`
                : hsla(kingdom[0], kingdom[1], kingdom[2], 0.13)
            }
            cx={anchor.x}
            cy={anchor.y}
            key={index}
            r={index % 12 === 0 ? 1.35 : 0.72}
          />
        ))}
      </Group>

      {field.threadPath.length > 0 ? (
        <Group opacity={isRevealing ? 0.18 + smoothstep(0, 0.7, revealT) * 0.18 : 1}>
          <Path
            color={isRevealing ? "rgba(255, 210, 106, 0.16)" : hsla(kingdom[0], kingdom[1], kingdom[2], 0.08)}
            path={field.threadPath}
            strokeCap="round"
            strokeWidth={8}
            style="stroke"
          />
          <Path
            color={isRevealing ? "rgba(255, 231, 163, 0.24)" : "rgba(242, 234, 215, 0.15)"}
            path={field.threadPath}
            strokeCap="round"
            strokeWidth={1.15}
            style="stroke"
          />
        </Group>
      ) : null}

      {visibleThreads.map((thread) => {
        const age = frameNow - thread.born;
        const appear = isRevealing ? 1 : clamp(age / 520, 0, 1);
        const settle = 0.32 + Math.min(thread.index, 420) / 1600;
        const revealGold = smoothstep(0, 0.78, revealT);
        const alpha = Math.min(0.48, appear * settle) * (isRevealing ? 0.72 + revealGold * 0.55 : 1);

        if (alpha <= 0.002) {
          return null;
        }

        return (
          <Group key={thread.index}>
            <Path
              color={
                isRevealing
                  ? `rgba(255, 210, 106, ${alpha * 0.36})`
                  : hsla(thread.hue, 72, 64, alpha * (0.1 + thread.glow * 0.1))
              }
              path={thread.path}
              strokeCap="round"
              strokeWidth={thread.weight + 2.4 * thread.glow}
              style="stroke"
            />
            <Path
              color={
                isRevealing
                  ? `rgba(255, 231, 163, ${alpha * 0.34})`
                  : hsla(thread.hue, 68, 72, alpha * 0.34)
              }
              path={thread.path}
              strokeCap="round"
              strokeWidth={thread.weight}
              style="stroke"
            />
          </Group>
        );
      })}

      {field.threadHead != null && acceptedGlyphs.length > 0 && !isRevealing ? (
        <Group>
          <Circle color="rgba(242, 234, 215, 0.86)" cx={field.threadHead.x} cy={field.threadHead.y} r={2.6} />
          <Circle
            color={hsla(kingdom[0], kingdom[1], kingdom[2], 0.42)}
            cx={field.threadHead.x}
            cy={field.threadHead.y}
            r={6.5 + Math.sin(frameNow / 220) * 1.5}
            strokeWidth={1}
            style="stroke"
          />
        </Group>
      ) : null}

      {!isRevealing && potential != null ? (
        <Path
          color={hsla(kingdom[0], kingdom[1], kingdom[2], 0.06 + silenceProgress * 0.18)}
          path={potential}
          strokeCap="round"
          strokeWidth={1 + silenceProgress * 1.2}
          style="stroke"
        />
      ) : null}

      {activeBreaths.map((breath, index) => {
        const age = frameNow - breath.born;
        const t = clamp(age / breath.life, 0, 1);
        const alpha = Math.sin(t * Math.PI) * breath.intensity;

        return (
          <Circle
            color={hsla(breath.hue, kingdom[1], kingdom[2], alpha * 0.18)}
            cx={layout.center.x}
            cy={layout.center.y}
            key={`${breath.born}-${index}`}
            r={layout.voidRadius * 1.18 + layout.radius * 0.74 * t}
            strokeWidth={1 + breath.intensity * 1.2}
            style="stroke"
          />
        );
      })}

      {activeEchoes.map((echo, index) => {
        const age = frameNow - echo.born;
        const t = clamp(age / ECHO_LIFE_MS, 0, 1);
        const melt = smoothstep(0.68, 1, t);
        const appear = smoothstep(0, 0.12, t);
        const alpha = appear * (1 - smoothstep(0.52, 1, t));
        const x = lerp(echo.x, echo.mergeX, melt);
        const y = lerp(echo.y, echo.mergeY, melt);

        if (alpha <= 0.01) {
          return null;
        }

        return (
          <Group key={`${echo.born}-${index}`} opacity={alpha}>
            <SkiaText
              color={`rgba(242, 234, 215, ${0.7 * alpha})`}
              font={echoFont}
              text={echo.text}
              x={x - echo.size * 0.28}
              y={y}
            />
            {melt > 0 ? (
              <Line
                color={hsla(echo.hue, 78, 72, 0.14 * melt)}
                p1={{ x, y }}
                p2={{ x: echo.mergeX, y: echo.mergeY }}
                strokeWidth={1}
              />
            ) : null}
          </Group>
        );
      })}

      {activeSparks.map((spark, index) => {
        const age = frameNow - spark.born;
        const t = clamp(age / spark.life, 0, 1);
        const ticks = age / 16;
        const x = spark.x + spark.vx * ticks;
        const y = spark.y + spark.vy * ticks;

        return (
          <Circle
            color={hsla(spark.hue, 76, 70, (1 - t) * 0.42)}
            cx={x}
            cy={y}
            key={`${spark.born}-${index}`}
            r={spark.size * (1 - t * 0.42)}
          />
        );
      })}

      <Circle cx={layout.center.x} cy={layout.center.y} r={layout.voidRadius * 1.36}>
        <RadialGradient
          c={layout.center}
          colors={["rgba(2, 2, 8, 0.98)", "rgba(3, 3, 10, 0.94)", "rgba(3, 3, 10, 0)"]}
          positions={[0, 0.68, 1]}
          r={layout.voidRadius * 1.36}
        />
      </Circle>
      <Circle color="rgba(4, 4, 10, 0.92)" cx={layout.center.x} cy={layout.center.y} r={layout.voidRadius} />
      <Circle
        color={`rgba(${GOLD[0]}, ${GOLD[1]}, ${GOLD[2]}, ${0.16 + typedGlow * 0.24})`}
        cx={layout.center.x}
        cy={layout.center.y}
        r={layout.voidRadius}
        strokeWidth={1}
        style="stroke"
      />
    </Canvas>
  );
}

function VesselCircle({
  acceptedGlyphs,
  frameNow,
  kingdom,
  layout,
  progress,
  startedAt,
}: {
  acceptedGlyphs: ChamberAcceptedGlyph[];
  frameNow: number;
  kingdom: readonly [number, number, number];
  layout: ChamberLayout;
  progress: number;
  startedAt: number | null;
}) {
  const size = layout.vesselSize;
  const center = size / 2;
  const radius = layout.vesselRadius;
  const safeProgress = clamp(progress, 0, 1);
  const progressPath = useMemo(() => {
    const path = Skia.Path.Make();
    if (safeProgress <= 0) {
      return path;
    }

    path.addArc(Skia.XYWHRect(center - radius, center - radius, radius * 2, radius * 2), -90, safeProgress * 360);
    return path;
  }, [center, radius, safeProgress]);
  const stitches = useMemo(() => {
    if (startedAt == null) {
      return [];
    }

    return acceptedGlyphs.map((event) => ({
      id: event.id,
      progress: clamp((event.acceptedAt - startedAt) / RITE_DURATION_MS, 0, 1),
      recent: frameNow - event.acceptedAt < 900,
    }));
  }, [acceptedGlyphs, frameNow, startedAt]);

  return (
    <View
      style={[
        styles.vesselCircle,
        {
          height: size,
          left: layout.center.x - size / 2,
          top: layout.center.y - size / 2,
          width: size,
        },
      ]}
    >
      <WovenVesselAsset kingdom={kingdom} size={size} />
      <Canvas style={StyleSheet.absoluteFill}>
        <Circle
          color={`rgba(${GOLD[0]}, ${GOLD[1]}, ${GOLD[2]}, 0.13)`}
          cx={center}
          cy={center}
          r={radius}
          strokeWidth={1.15}
          style="stroke"
        />
        <Path
          color={hsla(kingdom[0], kingdom[1], kingdom[2], 0.52)}
          path={progressPath}
          strokeCap="round"
          strokeWidth={2.4}
          style="stroke"
        />
        {stitches.map((stitch) => {
          const angle = -Math.PI / 2 + stitch.progress * Math.PI * 2;
          const x = center + Math.cos(angle) * radius;
          const y = center + Math.sin(angle) * radius;

          return (
            <Circle
              color={stitch.recent ? "rgba(255, 235, 168, 0.95)" : hsla(kingdom[0], 70, 70, 0.42)}
              cx={x}
              cy={y}
              key={stitch.id}
              r={stitch.recent ? 2.4 : 1.25}
            />
          );
        })}
      </Canvas>
    </View>
  );
}

function WovenVesselAsset({
  kingdom,
  size,
}: {
  kingdom: readonly [number, number, number];
  size: number;
}) {
  const center = size / 2;
  const radius = size * 0.45;
  const anchors = useMemo(() => makeAnchors({ x: center, y: center }, radius, size * 0.1), [center, radius, size]);
  const sides = useMemo(() => makeOctagonVertices({ x: center, y: center }, radius, Math.PI / 8), [center, radius]);
  const sideColors = ["#ff3b30", "#ff8a00", "#ffd60a", "#34c759", "#0a84ff", "#5e5ce6", "#bf5af2", "#f2f2f7"];

  return (
    <Svg height={size} viewBox={`0 0 ${size} ${size}`} width={size}>
      <G opacity={0.48}>
        {[0.38, 0.56, 0.73, 0.9].map((scale, index) => (
          <SvgCircle
            cx={center}
            cy={center}
            fill="none"
            key={scale}
            r={radius * scale}
            stroke={index % 2 === 0 ? "#ffffff" : hsla(kingdom[0], kingdom[1], kingdom[2], 0.8)}
            strokeOpacity={0.04 + index * 0.012}
            strokeWidth={1}
          />
        ))}
      </G>
      <SvgPolygon
        fill="none"
        points={sides.map((point) => `${point.x},${point.y}`).join(" ")}
        stroke="#ffffff"
        strokeOpacity={0.16}
        strokeWidth={1.1}
      />
      {sides.map((point, index) => {
        const next = sides[(index + 1) % sides.length];

        return (
          <SvgLine
            key={index}
            stroke={sideColors[index]}
            strokeLinecap="round"
            strokeOpacity={0.34}
            strokeWidth={2.2}
            x1={point.x}
            x2={next.x}
            y1={point.y}
            y2={next.y}
          />
        );
      })}
      {anchors.map((anchor, index) => {
        const side = Math.floor(index / (ANCHOR_COUNT / 8));
        const color = sideColors[side] ?? "#ffffff";

        return (
          <G key={index}>
            <SvgCircle cx={anchor.x} cy={anchor.y} fill={color} fillOpacity={0.16} r={2.5} />
            <SvgCircle cx={anchor.x} cy={anchor.y} fill={color} fillOpacity={0.7} r={0.9} />
          </G>
        );
      })}
    </Svg>
  );
}

function KeyPulseLayer({
  frameNow,
  height,
  keyboardHeight,
  kingdom,
  keyPulses,
  layout,
  width,
}: {
  frameNow: number;
  height: number;
  keyboardHeight: number;
  kingdom: readonly [number, number, number];
  keyPulses: ChamberKeyPulse[];
  layout: ChamberLayout;
  width: number;
}) {
  const activePulses = keyPulses.filter((pulse) => frameNow - pulse.acceptedAt >= 0 && frameNow - pulse.acceptedAt < 820);

  return (
    <Canvas style={StyleSheet.absoluteFill}>
      {activePulses.map((pulse) => {
        const age = frameNow - pulse.acceptedAt;
        const t = clamp(age / 820, 0, 1);
        const from = estimateKeyboardPoint(pulse.char, width, height, keyboardHeight);
        const control = {
          x: lerp(from.x, layout.center.x, 0.5),
          y: Math.min(from.y, layout.center.y) - 44 - smoothstep(0, 1, t) * 28,
        };
        const path = `M ${from.x} ${from.y} Q ${control.x} ${control.y} ${layout.center.x} ${layout.center.y}`;
        const alpha = 1 - smoothstep(0.2, 1, t);

        return (
          <Group key={pulse.id}>
            <Circle
              color={
                pulse.kind === "deletion"
                  ? `rgba(248, 113, 113, ${0.22 * alpha})`
                  : hsla(kingdom[0], kingdom[1], kingdom[2], 0.2 * alpha)
              }
              cx={from.x}
              cy={from.y}
              r={22 + t * 18}
            />
            <Circle
              color={pulse.kind === "deletion" ? `rgba(248, 113, 113, ${0.55 * alpha})` : `rgba(255, 235, 168, ${0.52 * alpha})`}
              cx={from.x}
              cy={from.y}
              r={5 + t * 2}
            />
            <Path
              color={
                pulse.kind === "deletion"
                  ? `rgba(248, 113, 113, ${0.35 * alpha})`
                  : hsla(kingdom[0], 82, 72, 0.34 * alpha)
              }
              end={smoothstep(0, 0.82, t)}
              path={path}
              start={0}
              strokeCap="round"
              strokeWidth={1.7}
              style="stroke"
            />
          </Group>
        );
      })}
    </Canvas>
  );
}

function CurrentGlyph({
  event,
  layout,
  silenceProgress,
}: {
  event: ChamberKeyPulse | null;
  layout: ChamberLayout;
  silenceProgress: number;
}) {
  const scale = useSharedValue(0.96);
  const opacity = useSharedValue(0);
  const silence = useSharedValue(silenceProgress);
  const glyph = event == null ? "" : visibleGlyph(event.char);
  const fontSize = Math.max(58, layout.voidRadius * 1.44);
  const boxSize = layout.voidRadius * 2.42;

  useEffect(() => {
    silence.value = withTiming(silenceProgress, { duration: 90, easing: Easing.out(Easing.quad) });
  }, [silence, silenceProgress]);

  useEffect(() => {
    if (event == null) {
      opacity.value = withTiming(0, { duration: 240 });
      return;
    }

    scale.value = withSequence(
      withTiming(1.13, { duration: 90, easing: Easing.out(Easing.cubic) }),
      withTiming(1, { duration: 220, easing: Easing.out(Easing.cubic) }),
    );
    opacity.value = withTiming(event.kind === "deletion" ? 0.78 : 0.96, { duration: 80 });

    if (event.kind === "deletion") {
      opacity.value = withSequence(
        withTiming(0.78, { duration: 80 }),
        withTiming(0, { duration: 520, easing: Easing.out(Easing.quad) }),
      );
    }
  }, [event, opacity, scale]);

  const animatedStyle = useAnimatedStyle(() => ({
    opacity: Math.max(0, opacity.value * (1 - silence.value * 0.98)),
    transform: [{ scale: scale.value }],
  }));

  if (event == null) {
    return null;
  }

  return (
    <Animated.View
      style={[
        styles.currentGlyphShell,
        {
          height: boxSize,
          left: layout.center.x - boxSize / 2,
          top: layout.center.y - boxSize / 2,
          width: boxSize,
        },
        animatedStyle,
      ]}
    >
      <Text
        adjustsFontSizeToFit
        numberOfLines={1}
        style={[
          styles.currentGlyph,
          {
            fontSize,
            lineHeight: fontSize * 1.08,
          },
        ]}
      >
        {glyph}
      </Text>
    </Animated.View>
  );
}

function AnkyWitness({
  keyboardHeight,
  safeBottom,
  width,
}: {
  keyboardHeight: number;
  safeBottom: number;
  width: number;
}) {
  const breath = useSharedValue(1);
  const witnessHeight = Math.max(104, Math.min(148, width * 0.32));
  const witnessWidth = witnessHeight * 0.86;
  const bottom = keyboardHeight > 0 ? keyboardHeight - 2 : safeBottom + 10;

  useEffect(() => {
    breath.value = withRepeat(
      withSequence(
        withTiming(1.025, { duration: 1900, easing: Easing.inOut(Easing.sin) }),
        withTiming(0.985, { duration: 1900, easing: Easing.inOut(Easing.sin) }),
      ),
      -1,
      true,
    );
  }, [breath]);

  const animatedStyle = useAnimatedStyle(() => ({
    opacity: 0.5 + (breath.value - 0.985) * 3.8,
    transform: [{ scale: breath.value }],
  }));

  return (
    <Animated.Image
      accessibilityIgnoresInvertColors
      resizeMode="contain"
      source={ANKY_WRITING_WITNESS}
      style={[
        styles.witness,
        {
          bottom,
          height: witnessHeight,
          left: width / 2 - witnessWidth / 2,
          width: witnessWidth,
        },
        animatedStyle,
      ]}
    />
  );
}

function buildThreadField(
  acceptedGlyphs: ChamberAcceptedGlyph[],
  layout: ChamberLayout,
  kingdom: readonly [number, number, number],
  daySeed: number,
) {
  const anchors = makeAnchors(layout.center, layout.radius, layout.voidRadius);
  const threads: ThreadSegment[] = [];
  const sparks: Spark[] = [];
  const echoes: GlyphEcho[] = [];
  const breaths: Breath[] = [];
  let threadHead = pointFromAngle(layout.center, -Math.PI / 2, layout.voidRadius * 1.34);
  let threadAngle = -Math.PI / 2;
  let previousGlyphCode = 0;
  let lastBreathAt = 0;

  acceptedGlyphs.forEach((event, eventIndex) => {
    const index = eventIndex + 1;
    const delta = Math.max(24, event.deltaMs);
    const speed = clamp(1 - delta / 1250, 0, 1);
    const code = glyphCode(event.char);
    const a = keepInsideAnnulus(threadHead, layout);
    const codeDelta = code - previousGlyphCode;
    const direction = codeDelta === 0 ? (index % 2 === 0 ? 1 : -1) : Math.sign(codeDelta);
    const pauseRatio = clamp(delta / MAX_PAUSE_MS, 0, 1);
    const turnBucket = Math.round(pauseRatio * TURN_BUCKETS);
    const quantizedTurn = turnBucket * (Math.PI / TURN_BUCKETS);
    const characterBias = clamp(Math.abs(codeDelta) / 56, 0, 1) * (Math.PI / 18);
    const breathDrift = Math.sin(index * 0.41 + daySeed) * (Math.PI / 72);
    const length =
      layout.radius * (0.018 + easeOutCubic(pauseRatio) * 0.15) + Math.min(Math.abs(codeDelta), 72) * 0.09;

    threadAngle += direction * (quantizedTurn + characterBias) + breathDrift;

    let b = pointFromAngle(a, threadAngle, length);
    if (distance(b, layout.center) > layout.radius * 0.9) {
      const inward = Math.atan2(layout.center.y - a.y, layout.center.x - a.x);
      threadAngle = inward + direction * (Math.PI / 14 + characterBias * 0.45);
      b = pointFromAngle(a, threadAngle, length * 0.74);
    }
    if (distance(b, layout.center) < layout.voidRadius * 1.2) {
      const tangent = Math.atan2(a.y - layout.center.y, a.x - layout.center.x) + direction * Math.PI / 2;
      threadAngle = tangent + direction * (Math.PI / 18 + pauseRatio * Math.PI / 10);
      b = pointFromAngle(a, threadAngle, length * 0.92);
    }

    b = keepInsideAnnulus(b, layout);

    const normal = normalize({ x: -(b.y - a.y), y: b.x - a.x });
    const curve = direction * length * (0.08 + pauseRatio * 0.34);
    const control = keepInsideAnnulus(
      {
        x: (a.x + b.x) / 2 + normal.x * curve,
        y: (a.y + b.y) / 2 + normal.y * curve,
      },
      layout,
    );
    const hue = positiveMod(kingdom[0] + turnBucket * 8 + codeDelta * 0.28 + speed * 12, 360);
    const thread = {
      a,
      b,
      born: event.acceptedAt,
      control,
      glow: 0.2 + pauseRatio * 0.35 + speed * 0.18,
      hue,
      index,
      path: `M ${a.x} ${a.y} Q ${control.x} ${control.y} ${b.x} ${b.y}`,
      pauseRatio,
      speed,
      weight: 0.5 + pauseRatio * 0.55 + speed * 0.45,
    };

    threads.push(thread);
    echoes.push(createGlyphEcho(visibleGlyph(event.char), thread));
    sparks.push(...createSparks(thread, event.id, speed));

    if (delta > 1450 && event.acceptedAt - lastBreathAt > 600) {
      lastBreathAt = event.acceptedAt;
      breaths.push({
        born: event.acceptedAt,
        hue: kingdom[0],
        intensity: clamp(delta / 4500, 0.24, 1),
        life: 2600 + clamp(delta / 4500, 0.24, 1) * 1800,
      });
    }

    threadHead = b;
    previousGlyphCode = code;
  });

  const visibleThreads = threads.slice(-MAX_THREADS);
  const threadPath =
    visibleThreads.length === 0
      ? ""
      : visibleThreads.reduce((path, thread, index) => {
          if (index === 0) {
            return `M ${thread.a.x} ${thread.a.y} Q ${thread.control.x} ${thread.control.y} ${thread.b.x} ${thread.b.y}`;
          }

          return `${path} Q ${thread.control.x} ${thread.control.y} ${thread.b.x} ${thread.b.y}`;
        }, "");

  return {
    anchors,
    breaths,
    echoes,
    sparks,
    threadAngle,
    threadHead,
    threadPath,
    threads: visibleThreads,
  };
}

function createGlyphEcho(value: string, thread: ThreadSegment): GlyphEcho {
  const position = quadraticPoint(thread.a, thread.control, thread.b, 0.66);
  const normal = normalize({ x: -(thread.b.y - thread.a.y), y: thread.b.x - thread.a.x });
  const offset = 8 + thread.pauseRatio * 18;

  return {
    born: thread.born,
    hue: thread.hue,
    mergeX: thread.b.x,
    mergeY: thread.b.y,
    size: 18 + thread.pauseRatio * 16,
    text: value,
    x: position.x + normal.x * offset,
    y: position.y + normal.y * offset,
  };
}

function createSparks(thread: ThreadSegment, seed: number, speed: number): Spark[] {
  const count = 3 + Math.round(speed * 5);
  const random = seededRandom(seed * 9973 + glyphCode(String(thread.index)));

  return Array.from({ length: count }, () => {
    const t = random();
    const position = quadraticPoint(thread.a, thread.control, thread.b, t);
    const angle = random() * Math.PI * 2;
    const velocity = 0.18 + random() * 0.42 + speed * 0.52;

    return {
      born: thread.born,
      hue: thread.hue,
      life: 640 + random() * 780,
      size: 0.6 + random() * 1.8,
      vx: Math.cos(angle) * velocity,
      vy: Math.sin(angle) * velocity,
      x: position.x,
      y: position.y,
    };
  });
}

function makeChamberLayout(width: number, height: number, keyboardHeight: number): ChamberLayout {
  const visibleHeight = Math.max(260, keyboardHeight > 0 ? height - keyboardHeight : height);
  const center = {
    x: width / 2,
    y: Math.max(142, Math.min(visibleHeight * 0.52, visibleHeight - 118)),
  };
  const base = Math.min(width, visibleHeight);
  const radius = Math.max(118, base * 0.39);
  const voidRadius = Math.max(36, base * 0.105);
  const vesselRadius = Math.max(voidRadius * 1.22, radius * 0.63);
  const vesselSize = vesselRadius * 2 + Math.max(34, width * 0.06);

  return {
    center,
    radius,
    vesselRadius,
    vesselSize,
    visibleHeight,
    voidRadius,
  };
}

function makePotentialPath(
  threadHead: Point | null,
  threadAngle: number,
  layout: ChamberLayout,
  silenceProgress: number,
  now: number,
): string | null {
  if (threadHead == null) {
    return null;
  }

  const pauseRatio = clamp(silenceProgress * (END_SILENCE_MS / MAX_PAUSE_MS), 0, 1);
  const turnBucket = Math.round(pauseRatio * TURN_BUCKETS);
  const reach = layout.radius * (0.018 + easeOutCubic(pauseRatio) * 0.15);
  const drift = Math.sin(now / 900) * (Math.PI / 56);
  const angle = threadAngle + drift + turnBucket * (Math.PI / TURN_BUCKETS) * 0.12;
  const end = keepInsideAnnulus(pointFromAngle(threadHead, angle, reach), layout);

  return `M ${threadHead.x} ${threadHead.y} L ${end.x} ${end.y}`;
}

function makeAnchors(center: Point, radius: number, voidRadius: number): Point[] {
  const points: Point[] = [];
  const sides = 8;
  const perSide = ANCHOR_COUNT / sides;
  const vertices = makeOctagonVertices(center, radius, Math.PI / 8);

  for (let side = 0; side < sides; side += 1) {
    const a = vertices[side];
    const b = vertices[(side + 1) % sides];

    for (let step = 0; step < perSide; step += 1) {
      const t = (step + 0.5) / perSide;

      points.push({
        x: lerp(a.x, b.x, t),
        y: lerp(a.y, b.y, t),
      });
    }
  }

  return points.map((point) => keepOutsideVoid(point, center, voidRadius));
}

function makeOctagonPath(center: Point, radius: number, rotation: number): string {
  const vertices = makeOctagonVertices(center, radius, rotation);

  return `${vertices.map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`).join(" ")} Z`;
}

function makeOctagonVertices(center: Point, radius: number, rotation: number): Point[] {
  return Array.from({ length: 8 }, (_, index) => {
    const angle = rotation + (index * Math.PI * 2) / 8;

    return {
      x: center.x + Math.cos(angle) * radius,
      y: center.y + Math.sin(angle) * radius,
    };
  });
}

function estimateKeyboardPoint(char: string, width: number, height: number, keyboardHeight: number): Point {
  const top = keyboardHeight > 0 ? height - keyboardHeight : height - Math.min(330, height * 0.38);
  const effectiveHeight = keyboardHeight > 0 ? keyboardHeight : Math.min(330, height * 0.38);
  const lower = char.toLowerCase();
  const rows = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

  if (char === " ") {
    return { x: width * 0.5, y: top + effectiveHeight * 0.74 };
  }

  if (char === "\n") {
    return { x: width * 0.84, y: top + effectiveHeight * 0.74 };
  }

  if (char === "\b") {
    return { x: width * 0.9, y: top + effectiveHeight * 0.52 };
  }

  for (let row = 0; row < rows.length; row += 1) {
    const index = rows[row].indexOf(lower);

    if (index >= 0) {
      const rowLength = rows[row].length;
      const rowWidth = width * (row === 0 ? 0.9 : row === 1 ? 0.8 : 0.64);
      const start = (width - rowWidth) / 2;
      const keyWidth = rowWidth / rowLength;

      return {
        x: start + keyWidth * (index + 0.5),
        y: top + effectiveHeight * (0.18 + row * 0.18),
      };
    }
  }

  if (/[0-9]/.test(lower)) {
    const index = Number(lower);
    const rowWidth = width * 0.9;
    const keyWidth = rowWidth / 10;

    return {
      x: (width - rowWidth) / 2 + keyWidth * (index + 0.5),
      y: top + effectiveHeight * 0.18,
    };
  }

  return {
    x: width * 0.5,
    y: top + effectiveHeight * 0.5,
  };
}

function visibleGlyph(value: string): string {
  if (value === " ") {
    return ".";
  }

  if (value === "\n") {
    return "|";
  }

  if (value === "\b") {
    return "<";
  }

  return value;
}

function getDaySeed(): number {
  const today = new Date();

  return Math.floor(Date.UTC(today.getFullYear(), today.getMonth(), today.getDate()) / 86400000);
}

function keepInsideAnnulus(point: Point, layout: ChamberLayout): Point {
  const maxDistance = layout.radius * 0.9;
  const minDistance = layout.voidRadius * 1.2;
  const currentDistance = distance(point, layout.center);

  if (currentDistance <= maxDistance && currentDistance >= minDistance) {
    return point;
  }

  const angle = Math.atan2(point.y - layout.center.y, point.x - layout.center.x);
  const targetDistance = currentDistance < minDistance ? minDistance : maxDistance;

  return {
    x: layout.center.x + Math.cos(angle) * targetDistance,
    y: layout.center.y + Math.sin(angle) * targetDistance,
  };
}

function keepOutsideVoid(point: Point, center: Point, voidRadius: number): Point {
  const currentDistance = distance(point, center);

  if (currentDistance > voidRadius * 1.2) {
    return point;
  }

  const angle = Math.atan2(point.y - center.y, point.x - center.x);

  return {
    x: center.x + Math.cos(angle) * voidRadius * 1.2,
    y: center.y + Math.sin(angle) * voidRadius * 1.2,
  };
}

function quadraticPoint(a: Point, c: Point, b: Point, t: number): Point {
  const mt = 1 - t;

  return {
    x: mt * mt * a.x + 2 * mt * t * c.x + t * t * b.x,
    y: mt * mt * a.y + 2 * mt * t * c.y + t * t * b.y,
  };
}

function pointFromAngle(origin: Point, angle: number, length: number): Point {
  return {
    x: origin.x + Math.cos(angle) * length,
    y: origin.y + Math.sin(angle) * length,
  };
}

function distance(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function normalize(vector: Point): Point {
  const length = Math.hypot(vector.x, vector.y) || 1;

  return {
    x: vector.x / length,
    y: vector.y / length,
  };
}

function glyphCode(value: string): number {
  if (value.length === 0) {
    return 0;
  }

  return value.codePointAt(0) ?? 0;
}

function easeOutCubic(value: number): number {
  return 1 - Math.pow(1 - value, 3);
}

function smoothstep(edge0: number, edge1: number, value: number): number {
  const t = clamp((value - edge0) / (edge1 - edge0), 0, 1);

  return t * t * (3 - 2 * t);
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

function positiveMod(value: number, divisor: number): number {
  return ((value % divisor) + divisor) % divisor;
}

function clamp(value: number, min = 0, max = 1): number {
  return Math.min(max, Math.max(min, value));
}

function hsla(h: number, s: number, l: number, a: number): string {
  return `hsla(${h}, ${s}%, ${l}%, ${a})`;
}

function seededRandom(seed: number): () => number {
  let value = seed % 2147483647;

  if (value <= 0) {
    value += 2147483646;
  }

  return () => {
    value = (value * 16807) % 2147483647;

    return (value - 1) / 2147483646;
  };
}

const styles = StyleSheet.create({
  backgroundImage: {
    height: "100%",
    opacity: 0.32,
    width: "100%",
  },
  backgroundInk: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(2, 3, 12, 0.72)",
  },
  backgroundLayer: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "#03030B",
  },
  backgroundThreads: {
    ...StyleSheet.absoluteFillObject,
    borderColor: "rgba(223, 196, 122, 0.05)",
    borderWidth: 1,
    opacity: 0.5,
    transform: [{ rotate: "-3deg" }, { scale: 1.16 }],
  },
  backgroundVignette: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(0, 0, 0, 0.18)",
  },
  currentGlyph: {
    color: "#F2EAD7",
    fontFamily: Platform.select({ android: "serif", ios: "Georgia", default: "serif" }),
    fontWeight: "400",
    letterSpacing: 0,
    textAlign: "center",
    textShadowColor: "rgba(223, 196, 122, 0.48)",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 24,
  },
  currentGlyphShell: {
    alignItems: "center",
    justifyContent: "center",
    position: "absolute",
  },
  vesselCircle: {
    opacity: 0.72,
    position: "absolute",
  },
  witness: {
    opacity: 0.62,
    position: "absolute",
  },
});
