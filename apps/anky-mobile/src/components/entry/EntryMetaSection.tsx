import { Pressable, StyleSheet, Text, View } from "react-native";

import type { ProcessingReceiptSidecar } from "../../lib/ankyStorage";
import { ankySolanaConfig } from "../../lib/solana/ankySolanaConfig";
import { shortAddress } from "../../lib/solana/loomStorage";
import type { LoomSeal } from "../../lib/solana/types";
import { ankyColors, fontSize, spacing } from "../../theme/tokens";

type Props = {
  artifactKinds: string[];
  copiedHash: boolean;
  durationLabel: string;
  fileName: string;
  hash: string;
  hashMatches: boolean;
  localState: string;
  onCopyHash: () => void;
  onCopyRaw: () => void;
  onOpenExplorer: () => void;
  onToggleRaw: () => void;
  processingReceipt: ProcessingReceiptSidecar | null;
  raw: string;
  seal: LoomSeal | null;
  showRaw: boolean;
  valid: boolean;
};

export function EntryMetaSection({
  artifactKinds,
  copiedHash,
  durationLabel,
  fileName,
  hash,
  hashMatches,
  localState,
  onCopyHash,
  onCopyRaw,
  onOpenExplorer,
  onToggleRaw,
  processingReceipt,
  raw,
  seal,
  showRaw,
  valid,
}: Props) {
  return (
    <View style={styles.section}>
      <Text style={styles.sectionLabel}>local details</Text>

      <View style={styles.panel}>
        <MetaRow label="file" value={fileName} />
        <MetaRow label="state" value={localState} />
        <MetaRow label="duration" value={durationLabel} />
        <MetaRow label="integrity" value={valid && hashMatches ? "hash verified" : "needs attention"} />
        <MetaRow label="artifacts" value={artifactKinds.length > 0 ? artifactKinds.join(", ") : "none"} />
        {processingReceipt == null ? null : (
          <MetaRow
            label="reflection cost"
            value={`${processingReceipt.credits_spent} credit spent / ${processingReceipt.credits_remaining} remaining`}
          />
        )}
        {seal == null ? null : (
          <>
            <MetaRow label="seal network" value={seal.network ?? ankySolanaConfig.network} />
            <MetaRow label="loom" value={shortAddress(seal.loomId)} />
            <MetaRow label="writer" value={shortAddress(seal.writer)} />
            <MetaRow label="signature" value={shortAddress(seal.txSignature, 6)} />
          </>
        )}

        <Text selectable style={styles.hash}>
          {hash}
        </Text>

        <View style={styles.actions}>
          <MetaButton label={copiedHash ? "hash copied" : "copy hash"} onPress={onCopyHash} />
          <MetaButton label={showRaw ? "hide raw" : "show raw"} onPress={onToggleRaw} />
          <MetaButton label="copy raw" onPress={onCopyRaw} />
          {seal == null ? null : <MetaButton label="open explorer" onPress={onOpenExplorer} />}
        </View>

        {showRaw ? (
          <View style={styles.rawCard}>
            <Text selectable style={styles.rawText}>
              {raw}
            </Text>
          </View>
        ) : null}
      </View>
    </View>
  );
}

function MetaRow({ label, value }: { label: string; value: string }) {
  return (
    <View style={styles.row}>
      <Text style={styles.rowLabel}>{label}</Text>
      <Text style={styles.rowValue}>{value}</Text>
    </View>
  );
}

function MetaButton({ label, onPress }: { label: string; onPress: () => void }) {
  return (
    <Pressable accessibilityRole="button" onPress={onPress} style={styles.button}>
      <Text style={styles.buttonText}>{label}</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  actions: {
    flexDirection: "row",
    flexWrap: "wrap",
    gap: spacing.sm,
    marginTop: spacing.lg,
  },
  button: {
    backgroundColor: "rgba(16, 19, 24, 0.7)",
    borderColor: "rgba(244, 241, 234, 0.12)",
    borderRadius: 8,
    borderWidth: 1,
    paddingHorizontal: spacing.md,
    paddingVertical: 9,
  },
  buttonText: {
    color: ankyColors.text,
    fontSize: 12,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  hash: {
    color: ankyColors.gold,
    fontFamily: "monospace",
    fontSize: 12,
    lineHeight: 18,
    marginTop: spacing.md,
  },
  panel: {
    backgroundColor: "rgba(16, 19, 24, 0.5)",
    borderColor: "rgba(244, 241, 234, 0.09)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.sm,
    padding: spacing.lg,
  },
  rawCard: {
    backgroundColor: "rgba(8, 9, 11, 0.72)",
    borderColor: "rgba(215, 186, 115, 0.18)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.md,
    padding: spacing.md,
  },
  rawText: {
    color: ankyColors.gold,
    fontFamily: "monospace",
    fontSize: 12,
    lineHeight: 19,
  },
  row: {
    flexDirection: "row",
    gap: spacing.md,
    justifyContent: "space-between",
    paddingVertical: 5,
  },
  rowLabel: {
    color: ankyColors.textMuted,
    flex: 0.44,
    fontSize: fontSize.sm,
    textTransform: "lowercase",
  },
  rowValue: {
    color: ankyColors.text,
    flex: 0.56,
    fontSize: fontSize.sm,
    lineHeight: 19,
    textAlign: "right",
    textTransform: "lowercase",
  },
  section: {
    marginTop: spacing.xl,
  },
  sectionLabel: {
    color: ankyColors.textMuted,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0,
    textTransform: "uppercase",
  },
});
