import { useEffect, useState } from "react";
import { ScrollView, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { GlassCard } from "../components/anky/GlassCard";
import { SacredHeader } from "../components/anky/SacredHeader";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { parseAnky, reconstructText, verifyHash } from "../lib/ankyProtocol";
import { readSavedAnkyFile } from "../lib/ankyStorage";
import type { AnkyLocalState } from "../lib/ankyState";
import type { LoomSeal } from "../lib/solana/types";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Entry">;

type EntryState = {
  hash: string;
  hashMatches: boolean;
  artifactKinds: string[];
  latestSeal?: LoomSeal;
  localState: AnkyLocalState;
  raw: string;
  sealCount: number;
  text: string;
  valid: boolean;
};

export function EntryScreen({ route }: Props) {
  const [entry, setEntry] = useState<EntryState | null>(null);

  useEffect(() => {
    let mounted = true;

    async function loadEntry() {
      const saved = await readSavedAnkyFile(route.params.fileName);
      const raw = saved.raw;
      const hash = saved.hash;
      const parsed = parseAnky(raw);
      const hashMatches = await verifyHash(raw, hash);

      if (mounted) {
        setEntry({
          artifactKinds: saved.artifactKinds,
          hash,
          hashMatches,
          latestSeal: saved.latestSeal,
          localState: saved.localState,
          raw,
          sealCount: saved.sealCount,
          text: reconstructText(raw),
          valid: parsed.valid,
        });
      }
    }

    void loadEntry().catch((error) => {
      console.error(error);
    });

    return () => {
      mounted = false;
    };
  }, [route.params.fileName]);

  if (entry == null) {
    return (
      <ScreenBackground variant="plain">
        <Text style={styles.loading}>Reading .anky file...</Text>
      </ScreenBackground>
    );
  }

  return (
    <ScreenBackground variant="cosmic">
      <ScrollView contentContainerStyle={styles.content}>
        <SacredHeader
          compact
          subtitle={`hash ${entry.hash.slice(0, 8)} · ${entry.localState}`}
          title="local .anky"
        />

        <GlassCard glow style={styles.textCard}>
          <Text style={styles.cardLabel}>reconstructed</Text>
          <Text selectable style={styles.body}>
            {entry.text.length > 0 ? entry.text : "No visible text."}
          </Text>
        </GlassCard>

        <GlassCard style={styles.validityCard}>
          <Text style={styles.validityText}>
            {entry.valid && entry.hashMatches ? "Valid .anky file" : "Invalid .anky file"}
          </Text>
          <Text style={styles.meta}>
            terminal 8000: {entry.valid ? "present" : "failed"} · hash:{" "}
            {entry.hashMatches ? "matches" : "mismatch"}
          </Text>
          <Text selectable style={styles.hash}>
            {entry.hash}
          </Text>
          <Text style={styles.note}>this .anky lives on this device. its hash names the file.</Text>
        </GlassCard>

        <GlassCard style={styles.validityCard}>
          <Text style={styles.validityText}>
            {entry.latestSeal == null ? "Not sealed through loom" : "Sealed through loom"}
          </Text>
          <Text style={styles.meta}>
            loom lineage count: {entry.sealCount} · private text is not public
          </Text>
          {entry.latestSeal == null ? null : (
            <>
              <Text selectable style={styles.hash}>
                {entry.latestSeal.txSignature}
              </Text>
              <Text style={styles.note}>
                {entry.latestSeal.createdAt ?? "Solana time unavailable"} · loom{" "}
                {entry.latestSeal.loomId}
              </Text>
            </>
          )}
        </GlassCard>

        <GlassCard style={styles.validityCard}>
          <Text style={styles.validityText}>Derived artifacts</Text>
          <Text style={styles.meta}>
            {entry.artifactKinds.length > 0
              ? entry.artifactKinds.join(", ")
              : "No local sidecar artifacts."}
          </Text>
          <Text style={styles.note}>derived artifacts are fruit. .anky is seed.</Text>
        </GlassCard>

        <Text style={styles.label}>Canonical .anky file</Text>
        <GlassCard style={styles.rawCard}>
          <Text selectable style={styles.rawText}>
            {entry.raw}
          </Text>
        </GlassCard>
      </ScrollView>
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  body: {
    color: ankyColors.text,
    fontSize: 22,
    lineHeight: 33,
    marginTop: spacing.md,
  },
  cardLabel: {
    color: ankyColors.violetBright,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 1.4,
    textTransform: "uppercase",
  },
  content: {
    padding: spacing.xl,
    paddingBottom: 44,
  },
  hash: {
    color: ankyColors.text,
    fontSize: 12,
    lineHeight: 18,
    marginTop: spacing.md,
  },
  label: {
    color: ankyColors.gold,
    fontSize: 13,
    fontWeight: "700",
    letterSpacing: 1.4,
    marginTop: 22,
    textTransform: "uppercase",
  },
  loading: {
    color: ankyColors.textMuted,
    fontSize: 16,
    margin: spacing.xl,
  },
  meta: {
    color: ankyColors.textMuted,
    fontSize: 13,
    lineHeight: 20,
    marginTop: 8,
  },
  note: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
  },
  rawCard: {
    marginTop: 12,
  },
  rawText: {
    color: ankyColors.gold,
    fontFamily: "monospace",
    fontSize: 13,
    lineHeight: 20,
  },
  textCard: {
    marginTop: spacing.sm,
  },
  validityCard: {
    marginTop: spacing.lg,
  },
  validityText: {
    color: ankyColors.gold,
    fontSize: 18,
    fontWeight: "700",
  },
});
