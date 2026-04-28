import { useEffect, useMemo, useState } from "react";
import { Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import * as Clipboard from "expo-clipboard";

import type { RootStackParamList } from "../../App";
import { GlassCard } from "../components/anky/GlassCard";
import { RitualButton } from "../components/anky/RitualButton";
import { SacredHeader } from "../components/anky/SacredHeader";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { SealAction } from "../components/anky/SealAction";
import { computeSessionHash, parseAnky, reconstructText, verifyHash } from "../lib/ankyProtocol";
import {
  appendLoomSeal,
  clearActiveDraft,
  clearPendingReveal,
  deleteSavedAnkyFile,
  readAnkyFile,
  readPendingReveal,
  saveClosedSession,
} from "../lib/ankyStorage";
import { getSelectedLoom, sealAnky } from "../lib/solana/loomClient";
import type { LoomSeal } from "../lib/solana/types";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Reveal">;

type ActionState = "idle" | "copying" | "sealing" | "releasing" | "error";

export function RevealScreen({ navigation, route }: Props) {
  const [actionState, setActionState] = useState<ActionState>("idle");
  const [copied, setCopied] = useState(false);
  const [fileName, setFileName] = useState<string | null>(route.params?.fileName ?? null);
  const [hash, setHash] = useState("");
  const [hashMatches, setHashMatches] = useState(false);
  const [raw, setRaw] = useState<string | null>(null);
  const [releaseArmed, setReleaseArmed] = useState(false);
  const [seal, setSeal] = useState<LoomSeal | null>(null);

  const reconstructed = useMemo(() => (raw == null ? "" : reconstructText(raw)), [raw]);
  const parsed = useMemo(() => (raw == null ? null : parseAnky(raw)), [raw]);
  const canAct =
    raw != null &&
    parsed?.valid === true &&
    hashMatches &&
    actionState !== "sealing" &&
    actionState !== "releasing";

  useEffect(() => {
    let mounted = true;

    async function loadRaw() {
      const nextFileName = route.params?.fileName ?? null;
      const nextRaw =
        nextFileName == null ? await readPendingReveal() : await readAnkyFile(nextFileName);

      if (!mounted) {
        return;
      }

      setRaw(nextRaw);
      setFileName(nextFileName);

      if (nextRaw != null) {
        const nextHash = nextFileName?.replace(/\.anky$/, "") ?? (await computeSessionHash(nextRaw));
        const nextHashMatches = await verifyHash(nextRaw, nextHash);

        if (mounted) {
          setHash(nextHash);
          setHashMatches(nextHashMatches);
        }
      }
    }

    void loadRaw().catch((error) => {
      console.error(error);
      if (mounted) {
        setActionState("error");
      }
    });

    return () => {
      mounted = false;
    };
  }, [route.params?.fileName]);

  async function handleCopy() {
    if (reconstructed.length === 0) {
      return;
    }

    try {
      setActionState("copying");
      await Clipboard.setStringAsync(reconstructed);
      setCopied(true);
      setActionState("idle");
      setTimeout(() => setCopied(false), 1600);
    } catch (error) {
      console.error(error);
      setActionState("error");
    }
  }

  async function handleSeal() {
    if (!canAct || raw == null) {
      return;
    }

    try {
      setActionState("sealing");
      const saved = await saveClosedSession(raw);
      const loom = await getSelectedLoom();

      if (loom == null) {
        throw new Error("A valid Anky Sojourn 9 Loom is required to seal.");
      }

      const result = await sealAnky({
        loomId: loom.id,
        sessionHash: saved.hash,
      });
      const sealRecord = {
        ...result,
        createdAt: new Date((result.blockTime ?? Math.floor(Date.now() / 1000)) * 1000).toISOString(),
      };

      await appendLoomSeal(sealRecord);
      await clearActiveDraft();
      await clearPendingReveal();
      setSeal(sealRecord);
      navigation.replace("Entry", { fileName: saved.fileName });
    } catch (error) {
      console.error(error);
      setActionState("error");
    }
  }

  async function handleRelease() {
    if (raw == null) {
      navigation.replace("AnkyverseTrail");
      return;
    }

    if (!releaseArmed) {
      setReleaseArmed(true);
      return;
    }

    try {
      setActionState("releasing");
      await clearPendingReveal();
      await clearActiveDraft();
      if (fileName != null) {
        await deleteSavedAnkyFile(fileName);
      }
      navigation.replace("AnkyverseTrail");
    } catch (error) {
      console.error(error);
      setActionState("error");
    }
  }

  return (
    <ScreenBackground variant="cosmic">
      <ScrollView contentContainerStyle={styles.content}>
        <SacredHeader
          subtitle="saved on this device"
          title="the thread closed"
        />

        <View style={styles.cardTop}>
          <Text style={styles.cardLabel}>the mirror</Text>
          <Pressable accessibilityRole="button" onPress={handleCopy}>
            <Text style={styles.copyText}>{copied ? "copied" : "tap to copy"}</Text>
          </Pressable>
        </View>
        <GlassCard glow style={styles.card}>
          <ScrollView nestedScrollEnabled style={styles.writingScroll}>
            <Text selectable style={styles.writing}>
              {raw == null
                ? "No closed .anky is waiting to be revealed."
                : reconstructed.length > 0
                  ? reconstructed
                  : "No visible characters reconstructed."}
            </Text>
          </ScrollView>
        </GlassCard>

        <GlassCard style={styles.hashCard}>
          <Text style={styles.hashLabel}>session hash</Text>
          <Text selectable style={styles.hash}>
            {hash.length > 0 ? hash : "not computed"}
          </Text>
          <Text style={styles.verifiedText}>
            {hashMatches ? "hash verified" : "hash not verified"}
          </Text>
        </GlassCard>

        {parsed != null && !parsed.valid ? (
          <Text style={styles.errorText}>This pending .anky is invalid and cannot be sealed.</Text>
        ) : null}

        <SealAction
          disabled={!canAct}
          label={actionState === "sealing" ? "sealing" : "seal through loom"}
          onSeal={handleSeal}
        />
        <Text style={styles.helper}>
          your writing stayed on this device. only its hash will be sealed through a loom.
        </Text>

        <RitualButton
          disabled={hash.length === 0}
          label="mirror with credits"
          onPress={() =>
            navigation.navigate("Credits", {
              fileName: fileName ?? undefined,
              processingType: "reflection",
            })
          }
          style={styles.reflectButton}
          variant="secondary"
        />
        <RitualButton
          disabled={hash.length === 0}
          label="full anky · 5 credits"
          onPress={() =>
            navigation.navigate("Credits", {
              fileName: fileName ?? undefined,
              processingType: "full_anky",
            })
          }
          style={styles.reflectButton}
          variant="secondary"
        />

        <RitualButton
          disabled={actionState === "sealing" || actionState === "releasing"}
          label={
            actionState === "releasing"
              ? "releasing"
              : releaseArmed
                ? "tap again to release"
                : "release this writing"
          }
          onPress={handleRelease}
          style={styles.releaseButton}
          variant="danger"
        />
        <Text style={styles.releaseHelper}>release deletes this local .anky file from this device.</Text>

        {actionState === "error" ? (
          <Text style={styles.errorText}>Something failed locally. The .anky was not sent anywhere.</Text>
        ) : null}
        {seal == null ? null : (
          <Text selectable style={styles.sealText}>
            sealed through loom · {seal.txSignature}
          </Text>
        )}
      </ScrollView>
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  card: {
    minHeight: 260,
  },
  cardLabel: {
    color: ankyColors.violetBright,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 1.4,
    textTransform: "uppercase",
  },
  cardTop: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "space-between",
    marginBottom: spacing.sm,
  },
  copyText: {
    color: ankyColors.gold,
    fontSize: fontSize.sm,
    letterSpacing: 0.8,
    textTransform: "lowercase",
  },
  content: {
    flexGrow: 1,
    padding: spacing.xl,
    paddingBottom: 46,
  },
  errorText: {
    color: ankyColors.danger,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: 16,
  },
  hash: {
    color: ankyColors.text,
    fontSize: 11,
    lineHeight: 17,
    marginTop: 8,
  },
  hashCard: {
    marginTop: spacing.lg,
    padding: spacing.md,
  },
  hashLabel: {
    color: ankyColors.gold,
    fontSize: 11,
    letterSpacing: 1.3,
    textTransform: "uppercase",
  },
  helper: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: 12,
    textAlign: "center",
  },
  reflectButton: {
    marginTop: spacing.xl,
  },
  releaseButton: {
    marginTop: spacing.md,
  },
  releaseHelper: {
    color: ankyColors.textMuted,
    fontSize: 12,
    lineHeight: 18,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  sealText: {
    color: ankyColors.textMuted,
    fontSize: 12,
    lineHeight: 18,
    marginTop: spacing.md,
    textAlign: "center",
  },
  verifiedText: {
    color: ankyColors.success,
    fontSize: 12,
    marginTop: spacing.sm,
    textTransform: "lowercase",
  },
  writing: {
    color: ankyColors.text,
    fontSize: 21,
    lineHeight: 32,
  },
  writingScroll: {
    maxHeight: 390,
  },
});
