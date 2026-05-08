import { useEffect, useMemo, useState } from "react";
import { Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { GlassCard } from "../components/anky/GlassCard";
import { RitualButton } from "../components/anky/RitualButton";
import { SacredHeader } from "../components/anky/SacredHeader";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { CREDIT_COSTS, PROCESSING_TYPES, ProcessingType } from "../lib/api/types";
import { listSavedAnkyFiles, SavedAnkyFile } from "../lib/ankyStorage";
import { hasConfiguredBackend } from "../lib/auth/backendSession";
import {
  getReflectionCreditBalance,
  processReflectionWithMode,
} from "../lib/credits/processAnky";
import { ankyColors, fontSize, spacing } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Credits">;
type RunState = "loading" | "ready" | "running" | "done" | "error";

export function CreditsScreen({ navigation, route }: Props) {
  const [balance, setBalance] = useState<number>(0);
  const [files, setFiles] = useState<SavedAnkyFile[]>([]);
  const [message, setMessage] = useState("");
  const [processingType, setProcessingType] = useState<ProcessingType>(
    route.params?.processingType ?? "reflection",
  );
  const [receipt, setReceipt] = useState<{
    creditsRemaining: number;
    creditsSpent: number;
  } | null>(null);
  const [selectedFileName, setSelectedFileName] = useState<string | null>(
    route.params?.fileName ?? null,
  );
  const [state, setState] = useState<RunState>("loading");

  const selectedFile = useMemo(
    () => files.find((file) => file.fileName === selectedFileName) ?? files[0] ?? null,
    [files, selectedFileName],
  );

  useEffect(() => {
    let mounted = true;

    async function load() {
      try {
        setState("loading");
        const [nextFiles, nextBalance] = await Promise.all([
          listSavedAnkyFiles(),
          getReflectionCreditBalance(),
        ]);

        if (!mounted) {
          return;
        }

        setFiles(nextFiles);
        setBalance(nextBalance);
        if (selectedFileName == null && nextFiles[0] != null) {
          setSelectedFileName(nextFiles[0].fileName);
        }
        setState("ready");
      } catch (error) {
        console.error(error);
        if (mounted) {
          setMessage("Reflection failed. Your .anky is unchanged.");
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
    if (selectedFile == null || state === "running") {
      return;
    }

    if (!hasConfiguredBackend()) {
      setMessage("reflection needs the backend API URL. writing and copy still work locally.");
      return;
    }

    if (processingType === "full_anky") {
      setMessage("full reflection is coming soon.");
      return;
    }

    if (processingType !== "reflection") {
      setMessage("this mirror is not available in this build.");
      return;
    }

    if (balance < CREDIT_COSTS[processingType]) {
      setMessage("Not enough credits. Credits pay for mirrors, not for truth.");
      return;
    }

    try {
      setState("running");
      setMessage("");
      const result = await processReflectionWithMode(
        selectedFile.fileName,
        "simple",
      );

      setBalance(result.creditsRemaining);
      setReceipt({
        creditsRemaining: result.creditsRemaining,
        creditsSpent: result.creditsSpent,
      });
      setMessage("reflection saved beside this local anky.");
      setState("done");
    } catch (error) {
      console.error(error);
      setMessage(
        error instanceof Error ? error.message : "Reflection failed. Your .anky is unchanged.",
      );
      setState("error");
    }
  }

  async function handleRefreshCredits() {
    const nextBalance = await getReflectionCreditBalance();
    setBalance(nextBalance);
    setMessage("credit balance refreshed.");
  }

  return (
    <ScreenBackground variant="plain">
      <ScrollView contentContainerStyle={styles.content}>
        <SacredHeader
          compact
          subtitle="Credits pay for mirrors, not for truth."
          title="credits"
        />

        <GlassCard style={styles.card}>
          <Text style={styles.label}>balance</Text>
          <Text style={styles.balance}>Credits: {balance}</Text>
          <Text style={styles.note}>
            Credits are server-backed when the API URL is configured. Native purchases are not configured in this build.
          </Text>
        </GlassCard>

        <GlassCard style={styles.card}>
          <Text style={styles.label}>processing options</Text>
          <View style={styles.options}>
            {PROCESSING_TYPES.map((type) => (
              <Pressable
                accessibilityRole="button"
                key={type}
                onPress={() => setProcessingType(type)}
                style={[styles.option, processingType === type && styles.optionSelected]}
              >
                <Text style={styles.optionText}>{labelForProcessingType(type)}</Text>
                <Text style={styles.optionCost}>
                  {CREDIT_COSTS[type]} credits
                  {type === "reflection" ? "" : " · unavailable"}
                </Text>
              </Pressable>
            ))}
          </View>
        </GlassCard>

        <GlassCard style={styles.card}>
          <Text style={styles.label}>selected .anky</Text>
          <Text style={styles.note}>
            {selectedFile == null ? "no local .anky file" : selectedFile.fileName}
          </Text>
          <Text style={styles.cost}>
            {processingType === "full_anky"
              ? "full reflection is coming soon"
              : `${CREDIT_COSTS[processingType]} credits will be spent`}
          </Text>
        </GlassCard>

        <RitualButton
          disabled={selectedFile == null || state === "running"}
          label={state === "running" ? "reflecting" : "spend credits and mirror"}
          onPress={() => void handleRunProcessing()}
        />
        <RitualButton
          label="refresh balance"
          onPress={() => void handleRefreshCredits()}
          style={styles.secondaryButton}
          variant="secondary"
        />

        {receipt == null ? null : (
          <GlassCard style={styles.card}>
            <Text style={styles.label}>receipt</Text>
            <Text style={styles.receipt}>{receipt.creditsSpent} credits spent</Text>
            <Text style={styles.receipt}>{receipt.creditsRemaining} credits remaining</Text>
          </GlassCard>
        )}

        {selectedFile != null && state === "done" ? (
          <View style={styles.doneActions}>
            <RitualButton
              label="View entry"
              onPress={() => navigation.navigate("Entry", { fileName: selectedFile.fileName })}
            />
          </View>
        ) : null}

        {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}
      </ScrollView>
    </ScreenBackground>
  );
}

function labelForProcessingType(type: ProcessingType): string {
  switch (type) {
    case "reflection":
      return "Reflection";
    case "image":
      return "Image";
    case "full_anky":
      return "Full Anky coming soon";
    case "deep_mirror":
      return "Deep Mirror";
    case "full_sojourn_archive":
      return "Full Sojourn";
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
  doneActions: {
    gap: spacing.sm,
    marginTop: spacing.md,
  },
  label: {
    color: ankyColors.gold,
    fontSize: fontSize.xs,
    fontWeight: "800",
    letterSpacing: 0,
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
  secondaryButton: {
    marginTop: spacing.md,
  },
});
