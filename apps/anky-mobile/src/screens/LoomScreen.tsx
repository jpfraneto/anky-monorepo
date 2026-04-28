import { useEffect, useState } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { AnkyMark } from "../components/AnkyMark";
import { GlassCard } from "../components/anky/GlassCard";
import { RitualButton } from "../components/anky/RitualButton";
import { SacredHeader } from "../components/anky/SacredHeader";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { hasTerminalLine, parseAnky, reconstructText } from "../lib/ankyProtocol";
import {
  clearActiveDraft,
  getAnkyDirectoryUri,
  listLocalLoomSeals,
  readActiveDraft,
  readPendingReveal,
  saveClosedSession,
  writePendingReveal,
} from "../lib/ankyStorage";
import { getOwnedLooms } from "../lib/solana/loomClient";
import type { Loom, LoomSeal } from "../lib/solana/types";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Loom">;

type DraftState = {
  closed: boolean;
  fileName?: string;
  preview: string;
  raw: string;
};

export function LoomScreen({ navigation }: Props) {
  const [draft, setDraft] = useState<DraftState | null>(null);
  const [looms, setLooms] = useState<Loom[]>([]);
  const [seals, setSeals] = useState<LoomSeal[]>([]);
  const [storageUri, setStorageUri] = useState("");

  useEffect(() => {
    let mounted = true;

    async function refreshDraft() {
      try {
        setStorageUri(getAnkyDirectoryUri());
        const pendingReveal = await readPendingReveal();

        if (pendingReveal != null) {
          const saved = parseAnky(pendingReveal).valid
            ? await saveClosedSession(pendingReveal)
            : null;

          if (mounted) {
            setDraft({
              closed: true,
              fileName: saved?.fileName,
              raw: pendingReveal,
              preview: reconstructText(pendingReveal).slice(0, 80),
            });
          }
          return;
        }

        const raw = await readActiveDraft();

        if (raw == null) {
          if (mounted) {
            setDraft(null);
          }
          return;
        }

        if (hasTerminalLine(raw) && parseAnky(raw).valid) {
          const saved = await saveClosedSession(raw);
          await writePendingReveal(raw);
          await clearActiveDraft();

          if (mounted) {
            setDraft({
              closed: true,
              fileName: saved.fileName,
              raw,
              preview: reconstructText(raw).slice(0, 80),
            });
          }
          return;
        }

        if (mounted) {
          setDraft({
            closed: false,
            raw,
            preview: reconstructText(raw).slice(0, 80),
          });
        }
      } catch (error) {
        console.error(error);
      }
    }

    void refreshDraft();

    async function refreshLooms() {
      try {
        const [nextLooms, nextSeals] = await Promise.all([getOwnedLooms(), listLocalLoomSeals()]);

        if (mounted) {
          setLooms(nextLooms);
          setSeals(nextSeals);
        }
      } catch (error) {
        console.error(error);
      }
    }

    void refreshLooms();
    const unsubscribe = navigation.addListener("focus", () => {
      void refreshDraft();
      void refreshLooms();
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  const openWrite = () => {
    navigation.navigate("AnkyverseTrail");
  };
  const selectedLoom = looms[0] ?? null;
  const selectedLoomSeals =
    selectedLoom == null ? [] : seals.filter((seal) => seal.loomId === selectedLoom.id);

  return (
    <ScreenBackground variant="centerGlow">
      <View style={styles.header}>
        <View style={styles.eyesWrap}>
          <AnkyMark size={118} />
          <AnkyMark size={118} />
        </View>
        <SacredHeader
          align="center"
          compact
          subtitle="the archive lives with the writer"
          title="Anky Sojourn 9 Loom"
        />
        <Text style={styles.line}>Forward-only .anky capture.</Text>
      </View>

      <View style={styles.actions}>
        {selectedLoom == null ? null : (
          <GlassCard style={styles.loomCard}>
            <Text style={styles.draftLabel}>selected loom</Text>
            <Text style={styles.draftPreview}>{selectedLoom.name}</Text>
            <Text style={styles.loomMeta}>
              local seal lineage: {selectedLoomSeals.length} · private text is not public
            </Text>
          </GlassCard>
        )}

        {draft == null ? null : (
          <Pressable
            accessibilityRole="button"
            onPress={() =>
              draft.closed
                ? navigation.navigate("Reveal", { fileName: draft.fileName })
                : navigation.navigate("Write", { recoverDraft: true })
            }
          >
            <GlassCard glow style={styles.draftCard}>
              <Text style={styles.draftLabel}>
                {draft.closed ? "Closed writing ready to reveal" : "Unfinished active draft"}
              </Text>
              <Text numberOfLines={2} style={styles.draftPreview}>
                {draft.preview.length > 0 ? draft.preview : "No visible characters reconstructed."}
              </Text>
            </GlassCard>
          </Pressable>
        )}

        <RitualButton label="Write 8 Minutes" onPress={openWrite} />

        <RitualButton
          label="Local Archive"
          onPress={() => navigation.navigate("Past")}
          variant="secondary"
        />

        <RitualButton
          label="Credits / Mirror"
          onPress={() => navigation.navigate("Credits")}
          variant="secondary"
        />
      </View>

      <Text numberOfLines={1} style={styles.storage}>
        {storageUri}
      </Text>
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  actions: {
    gap: 14,
    paddingHorizontal: spacing.xl,
    width: "100%",
  },
  draftCard: {
    marginBottom: 2,
  },
  draftLabel: {
    color: ankyColors.gold,
    fontSize: 13,
    letterSpacing: 1.2,
    marginBottom: 10,
    textTransform: "uppercase",
  },
  draftPreview: {
    color: ankyColors.text,
    fontSize: 18,
    lineHeight: 26,
  },
  line: {
    color: ankyColors.text,
    fontSize: fontSize.lg,
    lineHeight: 32,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  loomCard: {
    marginBottom: 2,
  },
  loomMeta: {
    color: ankyColors.textMuted,
    fontSize: 12,
    lineHeight: 18,
    marginTop: spacing.sm,
  },
  eyesWrap: {
    alignItems: "center",
    flexDirection: "row",
    gap: 18,
    marginBottom: 24,
  },
  storage: {
    color: ankyColors.textMuted,
    fontSize: 11,
    marginTop: 18,
    paddingHorizontal: spacing.xl,
  },
  header: {
    flex: 1,
    justifyContent: "center",
    paddingHorizontal: spacing.xl,
  },
});
