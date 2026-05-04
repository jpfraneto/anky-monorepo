import { Modal, Pressable, StyleSheet, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import { fontSize, spacing } from "../../theme/tokens";

export const REFLECTION_COST_CREDITS = 1;

type Props = {
  creditsAvailable: number;
  isProcessing?: boolean;
  onBuyCredits: () => void;
  onDismiss: () => void;
  onReflectNow: () => void;
  reflectionCost?: number;
  visible: boolean;
};

export function ReflectCreditSheet({
  creditsAvailable,
  isProcessing = false,
  onBuyCredits,
  onDismiss,
  onReflectNow,
  reflectionCost = REFLECTION_COST_CREDITS,
  visible,
}: Props) {
  const insets = useSafeAreaInsets();
  const canReflect = creditsAvailable >= reflectionCost && !isProcessing;
  const costLabel = `${reflectionCost} credit${reflectionCost === 1 ? "" : "s"}`;

  return (
    <Modal animationType="slide" onRequestClose={onDismiss} transparent visible={visible}>
      <View style={styles.backdrop}>
        <Pressable accessibilityRole="button" onPress={onDismiss} style={styles.scrim} />
        <View style={[styles.sheet, { paddingBottom: spacing.xl + Math.max(8, insets.bottom) }]}>
          <View style={styles.handle} />
          <Text style={styles.title}>reflect on this anky</Text>

          <View style={styles.rows}>
            <View style={styles.row}>
              <Text style={styles.rowLabel}>reflection cost</Text>
              <Text style={styles.rowValue}>{costLabel}</Text>
            </View>
            <View style={styles.row}>
              <Text style={styles.rowLabel}>available credits</Text>
              <Text style={styles.rowValue}>{creditsAvailable}</Text>
            </View>
          </View>

          <Text style={styles.consent}>
            your writing is sent for processing only when you choose reflect now.
          </Text>

          <View style={styles.actions}>
            <Pressable
              accessibilityRole="button"
              onPress={onBuyCredits}
              style={({ pressed }) => [styles.buyButton, pressed && styles.pressed]}
            >
              <Text style={styles.buyButtonText}>buy more credits</Text>
            </Pressable>
            <Pressable
              accessibilityRole="button"
              disabled={!canReflect}
              onPress={onReflectNow}
              style={({ pressed }) => [
                styles.reflectButton,
                !canReflect && styles.disabledButton,
                pressed && canReflect && styles.pressed,
              ]}
            >
              <Text style={styles.reflectButtonText}>
                {isProcessing ? "reflecting" : "reflect now"}
              </Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  actions: {
    flexDirection: "row",
    gap: spacing.sm,
    marginTop: spacing.lg,
  },
  backdrop: {
    flex: 1,
    justifyContent: "flex-end",
  },
  buyButton: {
    alignItems: "center",
    borderColor: "rgba(232, 200, 121, 0.24)",
    borderRadius: 8,
    borderWidth: 1,
    flex: 1,
    paddingVertical: 15,
  },
  buyButtonText: {
    color: "rgba(232, 200, 121, 0.72)",
    fontSize: fontSize.sm,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  consent: {
    color: "rgba(255, 240, 201, 0.54)",
    fontSize: fontSize.sm,
    lineHeight: 19,
    marginTop: spacing.lg,
    textAlign: "center",
    textTransform: "lowercase",
  },
  disabledButton: {
    backgroundColor: "rgba(123, 77, 255, 0.24)",
  },
  handle: {
    alignSelf: "center",
    backgroundColor: "rgba(255, 240, 201, 0.22)",
    borderRadius: 8,
    height: 4,
    marginBottom: spacing.lg,
    width: 44,
  },
  pressed: {
    opacity: 0.72,
  },
  reflectButton: {
    alignItems: "center",
    backgroundColor: "#7B4DFF",
    borderRadius: 8,
    flex: 1,
    paddingVertical: 15,
    shadowColor: "#7B4DFF",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.28,
    shadowRadius: 16,
  },
  reflectButtonText: {
    color: "#FFFFFF",
    fontSize: fontSize.sm,
    fontWeight: "800",
    textTransform: "lowercase",
  },
  row: {
    alignItems: "center",
    borderBottomColor: "rgba(232, 200, 121, 0.16)",
    borderBottomWidth: StyleSheet.hairlineWidth,
    flexDirection: "row",
    justifyContent: "space-between",
    minHeight: 48,
    paddingHorizontal: spacing.md,
  },
  rowLabel: {
    color: "rgba(255, 240, 201, 0.62)",
    fontSize: fontSize.sm,
    textTransform: "lowercase",
  },
  rows: {
    borderColor: "rgba(232, 200, 121, 0.18)",
    borderRadius: 8,
    borderWidth: 1,
    overflow: "hidden",
  },
  rowValue: {
    color: "#FFF0C9",
    fontSize: 15,
    fontWeight: "700",
  },
  scrim: {
    backgroundColor: "rgba(0, 0, 0, 0.66)",
    bottom: 0,
    left: 0,
    position: "absolute",
    right: 0,
    top: 0,
  },
  sheet: {
    backgroundColor: "rgba(16, 12, 31, 0.98)",
    borderColor: "rgba(232, 200, 121, 0.34)",
    borderRadius: 8,
    borderWidth: 1,
    margin: 12,
    padding: spacing.xl,
    paddingTop: spacing.md,
  },
  title: {
    color: "#E8C879",
    fontSize: 25,
    fontWeight: "700",
    letterSpacing: 0,
    marginBottom: spacing.lg,
    textAlign: "center",
    textTransform: "lowercase",
  },
});
