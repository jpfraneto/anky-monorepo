import { useEffect, useRef, type ReactNode } from "react";
import {
  Animated,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from "react-native";

import { SubtleIconButton } from "../navigation/SubtleIconButton";
import { ankyColors, fontSize, spacing } from "../../theme/tokens";

export type SessionReflectionMode = "full" | "simple";

type Props = {
  canContinue?: boolean;
  canCopy: boolean;
  canFullReflect: boolean;
  canSimpleReflect: boolean;
  dateLabel: string;
  durationLabel: string;
  errorText?: string;
  isComplete: boolean;
  isProcessing?: boolean;
  message?: string;
  onBack: () => void;
  onContinue?: () => void;
  onCopy: () => void;
  onFullReflect: () => void;
  onSimpleReflect: () => void;
  onTryAgain?: () => void;
  reflection?: string | null;
  sealAction?: ReactNode;
  text: string;
  timeLabel?: string;
};

const SERIF = Platform.select({ android: "serif", default: "Georgia", ios: "Georgia" });
const GOLD = "#E8C879";
const GOLD_SOFT = "rgba(232, 200, 121, 0.72)";
const PANEL = "rgba(9, 8, 20, 0.84)";
const PANEL_LIGHT = "rgba(22, 18, 38, 0.72)";
const BORDER = "rgba(232, 200, 121, 0.24)";

export function AnkySessionSurface({
  canContinue = false,
  canCopy,
  canFullReflect,
  canSimpleReflect,
  dateLabel,
  durationLabel,
  errorText,
  isComplete,
  isProcessing = false,
  message = "",
  onBack,
  onContinue,
  onCopy,
  onFullReflect,
  onSimpleReflect,
  onTryAgain,
  reflection,
  sealAction,
  text,
  timeLabel,
}: Props) {
  const hasReflection = reflection != null && reflection.trim().length > 0;

  return (
    <View style={styles.surface}>
      <SessionBackgroundTexture />
      <View style={styles.topBar}>
        <SubtleIconButton accessibilityLabel="go back" icon="←" onPress={onBack} />
        <View style={styles.topMeta}>
          <Text style={styles.topMetaText}>{dateLabel}</Text>
          {timeLabel == null ? null : <Text style={styles.topMetaSubtext}>{timeLabel}</Text>}
        </View>
        <View style={styles.topSide} />
      </View>

      <ScrollView
        contentContainerStyle={styles.content}
        showsVerticalScrollIndicator={false}
      >
        <View style={styles.statusRow}>
          <Text style={styles.statusText}>
            {isComplete ? "complete anky" : "fragment"}
          </Text>
          <Text style={styles.durationText}>{durationLabel}</Text>
        </View>

        <View style={styles.artifact}>
          <Text selectable style={styles.writingText}>
            {text.length > 0 ? text : " "}
          </Text>
        </View>

        {hasReflection ? (
          <View style={styles.replyCard}>
            <Text style={styles.replyLabel}>anky</Text>
            <Text selectable style={styles.replyText}>
              {reflection}
            </Text>
          </View>
        ) : isComplete ? (
          <></>
        ) : (
          <View style={styles.consentCard}>
            <Text style={styles.consentTitle}>a fragment arrived</Text>
            <Text style={styles.consentText}>
              fragments can be read and copied. only complete ankys can open a conversation.
            </Text>
          </View>
        )}

        {isProcessing ? (
          <View style={styles.processingRow}>
            <GoldenThreadSpinner />
            <Text style={styles.processingText}>anky is reading</Text>
          </View>
        ) : null}

        {sealAction == null ? null : <View style={styles.sealActionWrap}>{sealAction}</View>}

        <View style={styles.actionGrid}>
          <SessionActionButton
            disabled={!canCopy}
            label="copy"
            onPress={onCopy}
            variant="secondary"
          />
          {isComplete && !hasReflection ? (
            <>
              <SessionActionButton
                disabled={!canSimpleReflect}
                label="simple reflection"
                onPress={onSimpleReflect}
                variant="primary"
              />
              <SessionActionButton
                disabled={!canFullReflect}
                label="full reflection"
                onPress={onFullReflect}
                variant="secondary"
              />
            </>
          ) : null}
          {!isComplete && onTryAgain != null ? (
            <SessionActionButton
              label="try again"
              onPress={onTryAgain}
              variant="primary"
            />
          ) : null}
          {hasReflection && canContinue && onContinue != null ? (
            <SessionActionButton
              label="keep writing"
              onPress={onContinue}
              variant="primary"
            />
          ) : null}
        </View>

        {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}
        {errorText == null || errorText.length === 0 ? null : (
          <Text style={styles.errorText}>{errorText}</Text>
        )}
      </ScrollView>
    </View>
  );
}

export function GoldenThreadSpinner({ label }: { label?: string }) {
  const rotation = useRef(new Animated.Value(0)).current;

  useEffect(() => {
    const loop = Animated.loop(
      Animated.timing(rotation, {
        duration: 1080,
        toValue: 1,
        useNativeDriver: true,
      }),
    );

    loop.start();

    return () => {
      loop.stop();
    };
  }, [rotation]);

  const rotate = rotation.interpolate({
    inputRange: [0, 1],
    outputRange: ["0deg", "360deg"],
  });

  const spinner = (
    <Animated.View style={[styles.spinner, { transform: [{ rotate }] }]}>
      <View style={styles.spinnerGap} />
    </Animated.View>
  );

  if (label == null) {
    return spinner;
  }

  return (
    <View style={styles.spinnerWithLabel}>
      {spinner}
      <Text style={styles.processingText}>{label}</Text>
    </View>
  );
}

function SessionActionButton({
  disabled = false,
  label,
  onPress,
  variant,
}: {
  disabled?: boolean;
  label: string;
  onPress: () => void;
  variant: "primary" | "secondary";
}) {
  return (
    <Pressable
      accessibilityRole="button"
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => [
        styles.actionButton,
        variant === "primary" && styles.actionButtonPrimary,
        disabled && styles.disabled,
        pressed && !disabled && styles.pressed,
      ]}
    >
      <Text style={[styles.actionText, variant === "primary" && styles.actionTextPrimary]}>
        {label}
      </Text>
    </Pressable>
  );
}

function SessionBackgroundTexture() {
  return (
    <View pointerEvents="none" style={styles.backgroundTexture}>
      <View style={[styles.backgroundLine, { top: 88, width: "58%" }]} />
      <View style={[styles.backgroundLine, { top: 196, width: "78%" }]} />
      <View style={[styles.backgroundLine, { top: 330, width: "48%" }]} />
      <View style={[styles.backgroundLine, { bottom: 128, width: "68%" }]} />
    </View>
  );
}

const styles = StyleSheet.create({
  actionButton: {
    alignItems: "center",
    backgroundColor: "rgba(255,255,255,0.035)",
    borderColor: BORDER,
    borderRadius: 8,
    borderWidth: 1,
    flexGrow: 1,
    minHeight: 48,
    minWidth: "45%",
    paddingHorizontal: spacing.md,
    paddingVertical: 13,
  },
  actionButtonPrimary: {
    backgroundColor: "rgba(232, 200, 121, 0.18)",
    borderColor: "rgba(232, 200, 121, 0.52)",
  },
  actionGrid: {
    flexDirection: "row",
    flexWrap: "wrap",
    gap: spacing.sm,
    marginTop: spacing.md,
  },
  actionText: {
    color: ankyColors.text,
    fontSize: fontSize.sm,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  actionTextPrimary: {
    color: GOLD,
  },
  artifact: {
    backgroundColor: PANEL,
    borderColor: "rgba(232, 200, 121, 0.18)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.md,
    minHeight: 260,
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.xl,
  },
  backgroundLine: {
    alignSelf: "center",
    backgroundColor: "rgba(215, 186, 115, 0.044)",
    height: StyleSheet.hairlineWidth,
    position: "absolute",
  },
  backgroundTexture: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "#090A12",
  },
  consentCard: {
    backgroundColor: PANEL_LIGHT,
    borderColor: BORDER,
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.md,
    padding: spacing.lg,
  },
  consentText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: 6,
  },
  consentTitle: {
    color: GOLD,
    fontSize: fontSize.md,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  content: {
    padding: spacing.xl,
    paddingBottom: 58,
    paddingTop: spacing.md,
  },
  disabled: {
    opacity: 0.42,
  },
  durationText: {
    color: GOLD_SOFT,
    fontSize: fontSize.sm,
    fontVariant: ["tabular-nums"],
  },
  errorText: {
    color: "#F19A72",
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
    textAlign: "center",
  },
  message: {
    color: GOLD,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
    textAlign: "center",
    textTransform: "lowercase",
  },
  pressed: {
    opacity: 0.72,
    transform: [{ scale: 0.99 }],
  },
  processingRow: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.sm,
    justifyContent: "center",
    marginTop: spacing.md,
  },
  processingText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    textTransform: "lowercase",
  },
  replyCard: {
    backgroundColor: "rgba(232, 200, 121, 0.09)",
    borderColor: "rgba(232, 200, 121, 0.24)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.md,
    padding: spacing.lg,
  },
  replyLabel: {
    color: GOLD,
    fontSize: fontSize.sm,
    fontWeight: "800",
    letterSpacing: 0,
    marginBottom: spacing.sm,
    textTransform: "lowercase",
  },
  replyText: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    lineHeight: 25,
  },
  sealActionWrap: {
    marginTop: spacing.md,
  },
  spinner: {
    borderColor: "rgba(232, 200, 121, 0.22)",
    borderLeftColor: GOLD,
    borderRadius: 11,
    borderTopColor: GOLD,
    borderWidth: 2,
    height: 22,
    width: 22,
  },
  spinnerGap: {
    backgroundColor: "#090A12",
    borderRadius: 5,
    height: 10,
    left: 5,
    position: "absolute",
    top: -3,
    width: 10,
  },
  spinnerWithLabel: {
    alignItems: "center",
    gap: spacing.sm,
    justifyContent: "center",
  },
  statusRow: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "space-between",
  },
  statusText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  surface: {
    flex: 1,
  },
  topBar: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "space-between",
    paddingHorizontal: spacing.lg,
    paddingTop: spacing.md,
  },
  topMeta: {
    alignItems: "center",
    flex: 1,
  },
  topMetaSubtext: {
    color: GOLD_SOFT,
    fontSize: fontSize.xs,
    marginTop: 2,
    textTransform: "lowercase",
  },
  topMetaText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    textTransform: "lowercase",
  },
  topSide: {
    width: 42,
  },
  writingText: {
    color: ankyColors.text,
    fontFamily: SERIF,
    fontSize: 19,
    lineHeight: 31,
  },
});
