import { useEffect, useRef, useState } from "react";
import { StyleSheet, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";

import type { RootStackParamList } from "../../App";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { AnkyOnboardingSheet } from "../components/onboarding/AnkyOnboardingSheet";
import { hasTerminalLine, parseAnky } from "../lib/ankyProtocol";
import { listAnkySessionSummaries } from "../lib/ankySessionIndex";
import {
  readActiveDraft,
  readPendingReveal,
  saveClosedSession,
  stageTerminalDraftForReveal,
} from "../lib/ankyStorage";
import { getCurrentSojournDay, getNextSessionKindForToday } from "../lib/sojourn";
import { ankyColors } from "../theme/tokens";

type Props = NativeStackScreenProps<RootStackParamList, "Write">;

export function WriteRootScreen({ navigation, route }: Props) {
  const hasShownOpeningOnboardingRef = useRef(false);
  const startingRef = useRef(false);
  const [hasDraft, setHasDraft] = useState(false);
  const [onboardingVisible, setOnboardingVisible] = useState(false);
  const [sessionKind, setSessionKind] = useState<"daily_seal" | "extra_thread">("daily_seal");
  const [today, setToday] = useState(() => new Date());

  useEffect(() => {
    let mounted = true;

    async function load() {
      const [draft, pendingReveal, sessions] = await Promise.all([
        readActiveDraft(),
        readPendingReveal(),
        listAnkySessionSummaries(),
      ]);

      if (!mounted) {
        return;
      }

      const now = new Date();
      const nextSessionKind = getNextSessionKindForToday(sessions, now);
      const nextHasDraft = draft != null && !hasTerminalLine(draft);
      const hasValidPendingReveal = pendingReveal != null && parseAnky(pendingReveal).valid;
      const hasValidTerminalDraft =
        draft != null && hasTerminalLine(draft) && parseAnky(draft).valid;
      const shouldShowOnboarding =
        !hasValidPendingReveal &&
        !hasValidTerminalDraft &&
        (route.params?.replayOnboarding === true || !hasShownOpeningOnboardingRef.current);

      setToday(now);
      setSessionKind(nextSessionKind);
      setHasDraft(nextHasDraft);

      if (hasValidPendingReveal && pendingReveal != null) {
        // Recovery priority:
        // 1. A non-terminal active draft is the only non-durable artifact, so recover it first.
        //    The pending reveal is saved by hash before active writing can overwrite pending.anky.
        // 2. If active.anky.draft is terminal too, save it by hash before Reveal clears the draft.
        // 3. Otherwise pending.anky is already the safest reveal handoff artifact.
        await saveClosedSession(pendingReveal);

        if (hasValidTerminalDraft && draft != null && draft !== pendingReveal) {
          await saveClosedSession(draft);
        }

        if (!mounted) {
          return;
        }

        if (!nextHasDraft) {
          startReveal();
          return;
        }
      }

      if (!hasValidPendingReveal && hasValidTerminalDraft && draft != null) {
        await stageTerminalDraftForReveal(draft);

        if (!mounted) {
          return;
        }

        startReveal();
        return;
      }

      if (shouldShowOnboarding) {
        hasShownOpeningOnboardingRef.current = true;
        setOnboardingVisible(true);
        return;
      }

      startWriting(nextHasDraft, now, nextSessionKind);
    }

    void load().catch((error) => {
      console.error(error);
    });
    const unsubscribe = navigation.addListener("focus", () => {
      startingRef.current = false;
      void load().catch((error) => {
        console.error(error);
      });
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation, route.params?.replayOnboarding]);

  function startWriting(
    recoverDraft = hasDraft,
    writingDate = today,
    writingSessionKind = sessionKind,
  ) {
    if (startingRef.current) {
      return;
    }

    startingRef.current = true;
    setOnboardingVisible(false);

    try {
      navigation.setParams({ replayOnboarding: false });
      navigation.replace("ActiveWriting", {
        dayNumber: getCurrentSojournDay(writingDate),
        isoDate: writingDate.toISOString().slice(0, 10),
        recoverDraft,
        sessionKind: writingSessionKind,
        sojourn: 9,
      });
    } catch (error) {
      startingRef.current = false;
      throw error;
    }
  }

  function startReveal() {
    if (startingRef.current) {
      return;
    }

    startingRef.current = true;
    setOnboardingVisible(false);

    try {
      navigation.setParams({ replayOnboarding: false });
      navigation.replace("Reveal");
    } catch (error) {
      startingRef.current = false;
      throw error;
    }
  }

  return (
    <ScreenBackground safe={false} variant="plain">
      <View style={styles.root} />

      <AnkyOnboardingSheet
        onBegin={() => startWriting(hasDraft)}
        visible={onboardingVisible}
      />
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  root: {
    backgroundColor: ankyColors.bg,
    flex: 1,
  },
});
