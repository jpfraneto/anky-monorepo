import { useEffect, useState } from "react";
import { FlatList, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { GlassCard } from "../components/anky/GlassCard";
import { SacredHeader } from "../components/anky/SacredHeader";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { TraceCard } from "../components/anky/TraceCard";
import { listSavedAnkyFiles, SavedAnkyFile } from "../lib/ankyStorage";
import { hydrateMobileSealReceiptsForHashes } from "../lib/solana/mobileSealReceipts";
import { useAnkyPresenceScreen } from "../presence/useAnkyPresenceScreen";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Past">;

export function PastScreen({ navigation }: Props) {
  const [files, setFiles] = useState<SavedAnkyFile[]>([]);
  const [loading, setLoading] = useState(true);

  useAnkyPresenceScreen({
    emotion: "idle",
    preferredMode: "sigil",
    sequence: "seated",
  });

  useEffect(() => {
    let mounted = true;

    async function loadFiles() {
      setLoading(true);

      try {
        const nextFiles = await listSavedAnkyFiles();

        if (mounted) {
          setFiles(nextFiles);
        }

        await hydrateMobileSealReceiptsForHashes(nextFiles.map((file) => file.hash));

        const hydratedFiles = await listSavedAnkyFiles();

        if (mounted) {
          setFiles(hydratedFiles);
        }
      } catch (error) {
        console.error(error);
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    }

    void loadFiles();
    const unsubscribe = navigation.addListener("focus", () => {
      void loadFiles();
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  return (
    <ScreenBackground variant="cosmic">
      <View style={styles.headerWrap}>
        <SacredHeader
          compact
          subtitle="local .anky files; backend storage is not canonical"
          title="local archive"
        />
      </View>

      <FlatList
        contentContainerStyle={files.length === 0 ? styles.emptyList : styles.list}
        data={files}
        keyExtractor={(item) => item.fileName}
        ListEmptyComponent={
          <GlassCard glow style={styles.emptyCard}>
            <Text style={styles.emptyTitle}>
              {loading ? "reading local files" : "no local .anky files yet"}
            </Text>
            <Text style={styles.empty}>
              {loading
                ? "looking for exact .anky files on this device"
                : "write, reveal, and choose what remains on this device"}
            </Text>
          </GlassCard>
        }
        ListFooterComponent={
          <Text style={styles.footer}>sealed means a hash anchor, not an upload</Text>
        }
        renderItem={({ item }) => {
          const badges = [
            "local",
            item.sealCount > 0 ? "sealed" : null,
            item.artifactKinds.includes("reflection") ? "mirrored" : null,
            item.artifactKinds.includes("conversation") ? "keep writing" : null,
          ].filter(Boolean).join(" · ");

          return (
            <TraceCard
              hash={item.hash.slice(0, 12)}
              onPress={() => navigation.navigate("Entry", { fileName: item.fileName })}
              preview={item.preview.length > 0 ? item.preview : "No visible text."}
              status={item.localState}
              subtitle={item.valid && item.hashMatches ? badges : "invalid .anky"}
              title={`trace ${item.hash.slice(0, 8)}`}
            />
          );
        }}
      />
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  empty: {
    color: ankyColors.textMuted,
    fontSize: fontSize.md,
    lineHeight: 23,
    marginTop: spacing.sm,
    textAlign: "center",
  },
  emptyCard: {
    marginTop: spacing.xl,
  },
  emptyList: {
    flexGrow: 1,
    padding: spacing.xl,
    paddingTop: spacing.lg,
  },
  emptyTitle: {
    color: ankyColors.gold,
    fontSize: fontSize.lg,
    fontWeight: "700",
    textAlign: "center",
  },
  footer: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    marginTop: spacing.md,
    paddingBottom: spacing.xl,
    textAlign: "center",
  },
  headerWrap: {
    paddingHorizontal: spacing.xl,
    paddingTop: spacing.lg,
  },
  list: {
    padding: spacing.xl,
    paddingTop: spacing.lg,
  },
});
