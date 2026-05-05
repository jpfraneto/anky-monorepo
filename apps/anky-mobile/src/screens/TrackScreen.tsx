import { useEffect, useMemo, useState } from "react";
import { StyleSheet, View } from "react-native";
import type { NativeStackScreenProps } from "@react-navigation/native-stack";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import type { RootStackParamList } from "../../App";
import { ScreenBackground } from "../components/anky/ScreenBackground";
import { RootTabBar, RootTabName } from "../components/navigation/RootTabBar";
import { SojournMap } from "../components/sojourn/SojournMap";
import { buildSojournMapDays } from "../components/sojourn/sojournMapAdapter";
import type { SojournMapAnky } from "../components/sojourn/SojournMap.types";
import { listAnkySessionSummaries } from "../lib/ankySessionIndex";
import { listSavedAnkyFiles, type SavedAnkyFile } from "../lib/ankyStorage";
import {
  buildSojournDays,
  getCurrentSojournDay,
  SOJOURN_LENGTH_DAYS,
  type AnkySessionSummary,
} from "../lib/sojourn";
import { useAnkyPresenceScreen } from "../presence/useAnkyPresenceScreen";

type Props = NativeStackScreenProps<RootStackParamList, "Track">;

export function TrackScreen({ navigation }: Props) {
  const insets = useSafeAreaInsets();
  const [files, setFiles] = useState<SavedAnkyFile[]>([]);
  const [now, setNow] = useState(() => new Date());
  const [sessions, setSessions] = useState<AnkySessionSummary[]>([]);
  const currentDay = getCurrentSojournDay(now);
  const sojournDays = useMemo(() => buildSojournDays(sessions, now), [sessions, now]);
  const mapDays = useMemo(
    () => buildSojournMapDays({ currentDay, days: sojournDays, files }),
    [currentDay, files, sojournDays],
  );

  useAnkyPresenceScreen({
    emotion: "walking",
    preferredMode: "sigil",
    sequence: "walk_right",
  });

  useEffect(() => {
    let mounted = true;

    async function load() {
      try {
        const [nextSessions, nextFiles] = await Promise.all([
          listAnkySessionSummaries(),
          listSavedAnkyFiles(),
        ]);

        if (mounted) {
          setNow(new Date());
          setSessions(nextSessions);
          setFiles(nextFiles);
        }
      } catch (error) {
        console.error(error);
      }
    }

    void load();
    const unsubscribe = navigation.addListener("focus", () => {
      void load();
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [navigation]);

  function openAnky(anky: SojournMapAnky) {
    if (anky.fileName != null) {
      navigation.navigate("Entry", { fileName: anky.fileName });
      return;
    }

    navigation.navigate("DayChamber", { day: anky.day });
  }

  function selectTab(tab: RootTabName) {
    if (tab === "Track") {
      return;
    }

    if (tab === "Write") {
      navigation.navigate("Write");
      return;
    }

    navigation.navigate("You");
  }

  return (
    <ScreenBackground safe={false} variant="plain">
      <View style={styles.shell}>
        <SojournMap
          bottomInset={108 + insets.bottom}
          currentDay={currentDay}
          days={mapDays}
          initialSelectedDay={currentDay}
          onPressAnky={openAnky}
          sojournLength={SOJOURN_LENGTH_DAYS}
          sojournNumber={9}
        />
        <RootTabBar active="Track" onSelect={selectTab} />
      </View>
    </ScreenBackground>
  );
}

const styles = StyleSheet.create({
  shell: {
    flex: 1,
  },
});
