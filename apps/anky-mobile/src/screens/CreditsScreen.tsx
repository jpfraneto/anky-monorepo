import { useEffect, useMemo, useState } from "react";
import { Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { GlassCard } from "../components/anky/GlassCard";
import { RitualButton } from "../components/anky/RitualButton";
import { SacredHeader } from "../components/anky/SacredHeader";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { createAnkyApiClient } from "../lib/api/ankyApi";
import {
  AppConfigResponse,
  CREDIT_COSTS,
  CreditReceipt,
  PROCESSING_TYPES,
  ProcessingType,
} from "../lib/api/types";
import {
  buildCarpetFromAnkyStrings,
  createProcessingCarpetPayload,
  createProcessingTicketRequest,
} from "../lib/processing/carpet";
import {
  listSavedAnkyFiles,
  readAnkyFile,
  SavedAnkyFile,
  writeProcessingArtifacts,
} from "../lib/ankyStorage";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Credits">;

type RunState = "idle" | "loading" | "ready" | "running" | "done" | "error";

declare const process:
  | {
      env?: Record<string, string | undefined>;
    }
  | undefined;

const API_BASE_URL =
  typeof process === "undefined" ? "" : (process.env?.EXPO_PUBLIC_ANKY_API_URL ?? "");

export function CreditsScreen({ route }: Props) {
  const [balance, setBalance] = useState<number | null>(null);
  const [config, setConfig] = useState<AppConfigResponse | null>(null);
  const [files, setFiles] = useState<SavedAnkyFile[]>([]);
  const [message, setMessage] = useState("");
  const [processingType, setProcessingType] = useState<ProcessingType>(
    route.params?.processingType ?? "reflection",
  );
  const [receipt, setReceipt] = useState<CreditReceipt | null>(null);
  const [selectedFileName, setSelectedFileName] = useState<string | null>(
    route.params?.fileName ?? null,
  );
  const [state, setState] = useState<RunState>("loading");

  const selectedFile = useMemo(
    () => files.find((file) => file.fileName === selectedFileName) ?? files[0] ?? null,
    [files, selectedFileName],
  );
  const selectedFiles =
    processingType === "full_sojourn_archive" ? files : selectedFile == null ? [] : [selectedFile];

  useEffect(() => {
    let mounted = true;

    async function load() {
      setState("loading");

      try {
        const localFiles = await listSavedAnkyFiles();

        if (!mounted) {
          return;
        }

        setFiles(localFiles);

        if (selectedFileName == null && localFiles[0] != null) {
          setSelectedFileName(localFiles[0].fileName);
        }

        if (API_BASE_URL.length === 0) {
          setMessage("backend not configured; credit processing is unavailable");
          setState("ready");
          return;
        }

        const api = createAnkyApiClient({ baseUrl: API_BASE_URL });
        const [nextConfig, nextBalance] = await Promise.all([
          api.getConfig(),
          api.getCreditBalance(),
        ]);

        if (mounted) {
          setConfig(nextConfig);
          setBalance(nextBalance.creditsRemaining);
          setState("ready");
        }
      } catch (error) {
        console.error(error);

        if (mounted) {
          setMessage("could not load credit state");
          setState("error");
        }
      }
    }

    void load();

    return () => {
      mounted = false;
    };
  }, [selectedFileName]);

  async function handleRunProcessing() {
    if (selectedFiles.length === 0 || state === "running") {
      return;
    }

    if (API_BASE_URL.length === 0 || config == null) {
      setMessage("backend not configured; no credits were spent");
      return;
    }

    try {
      setState("running");
      setMessage("");

      const rawEntries = await Promise.all(
        selectedFiles.map((file) => readAnkyFile(file.fileName)),
      );
      const carpet = buildCarpetFromAnkyStrings(processingType, rawEntries);
      const carpetPayload = createProcessingCarpetPayload(carpet, config.processing);
      const api = createAnkyApiClient({ baseUrl: API_BASE_URL });
      const ticket = await api.createProcessingTicket(createProcessingTicketRequest(carpet));

      setReceipt(ticket.receipt);
      setBalance(ticket.receipt.creditsRemaining);

      const response = await api.runProcessing({
        encryptedCarpet: carpetPayload.encryptedCarpet,
        encryptionScheme: carpetPayload.encryptionScheme,
        receipt: ticket.receipt,
      });

      await writeProcessingArtifacts(response.artifacts);
      setMessage("derived artifacts saved beside local .anky files");
      setState("done");
    } catch (error) {
      console.error(error);
      setMessage("processing did not complete");
      setState("error");
    }
  }

  return (
    <ScreenBackground variant="cosmic">
      <ScrollView contentContainerStyle={styles.content}>
        <SacredHeader
          compact
          subtitle="credits pay for mirrors, not the canonical archive"
          title="credits"
        />

        <GlassCard style={styles.card}>
          <Text style={styles.label}>balance</Text>
          <Text style={styles.balance}>{balance == null ? "not loaded" : `${balance} credits`}</Text>
          <Text style={styles.note}>processing sends a temporary carpet only after this action.</Text>
        </GlassCard>

        <GlassCard style={styles.card}>
          <Text style={styles.label}>processing</Text>
          <View style={styles.options}>
            {PROCESSING_TYPES.map((type) => (
              <Pressable
                accessibilityRole="button"
                key={type}
                onPress={() => setProcessingType(type)}
                style={[styles.option, processingType === type && styles.optionSelected]}
              >
                <Text style={styles.optionText}>{labelForProcessingType(type)}</Text>
                <Text style={styles.optionCost}>{CREDIT_COSTS[type]} credits</Text>
              </Pressable>
            ))}
          </View>
        </GlassCard>

        <GlassCard style={styles.card}>
          <Text style={styles.label}>selected .anky</Text>
          <Text style={styles.note}>
            {processingType === "full_sojourn_archive"
              ? `${files.length} local .anky files`
              : selectedFile == null
                ? "no local .anky file"
                : selectedFile.fileName}
          </Text>
          <Text style={styles.cost}>
            {CREDIT_COSTS[processingType]} credits will be spent
          </Text>
        </GlassCard>

        <RitualButton
          disabled={selectedFiles.length === 0 || state === "running"}
          label={state === "running" ? "processing" : "spend credits and mirror"}
          onPress={handleRunProcessing}
        />

        {receipt == null ? null : (
          <GlassCard style={styles.card}>
            <Text style={styles.label}>receipt</Text>
            <Text style={styles.receipt}>{receipt.creditsSpent} credits spent</Text>
            <Text style={styles.receipt}>{receipt.creditsRemaining} credits remaining</Text>
          </GlassCard>
        )}

        {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}
      </ScrollView>
    </ScreenBackground>
  );
}

function labelForProcessingType(type: ProcessingType): string {
  switch (type) {
    case "reflection":
      return "reflection";
    case "image":
      return "image";
    case "full_anky":
      return "full anky";
    case "deep_mirror":
      return "deep mirror";
    case "full_sojourn_archive":
      return "full sojourn archive";
  }
}

const styles = StyleSheet.create({
  balance: {
    color: ankyColors.gold,
    fontSize: 28,
    fontWeight: "800",
    marginTop: spacing.sm,
  },
  card: {
    marginTop: spacing.lg,
  },
  content: {
    padding: spacing.xl,
    paddingBottom: 44,
  },
  cost: {
    color: ankyColors.gold,
    fontSize: fontSize.md,
    fontWeight: "700",
    marginTop: spacing.md,
  },
  label: {
    color: ankyColors.violetBright,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 1.3,
    textTransform: "uppercase",
  },
  message: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.lg,
    textAlign: "center",
  },
  note: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.sm,
  },
  option: {
    borderColor: ankyColors.border,
    borderRadius: 8,
    borderWidth: 1,
    padding: spacing.md,
  },
  optionCost: {
    color: ankyColors.textMuted,
    fontSize: 12,
    marginTop: 4,
  },
  optionSelected: {
    borderColor: ankyColors.gold,
  },
  optionText: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  options: {
    gap: spacing.sm,
    marginTop: spacing.md,
  },
  receipt: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    lineHeight: 23,
    marginTop: spacing.sm,
  },
});
