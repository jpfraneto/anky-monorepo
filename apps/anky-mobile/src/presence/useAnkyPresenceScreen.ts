import { useCallback, useMemo } from "react";
import { useFocusEffect } from "@react-navigation/native";

import {
  useAnkyPresence,
  type AnkyPresenceScreenConfig,
} from "./AnkyPresenceContext";

export function useAnkyPresenceScreen(config: AnkyPresenceScreenConfig) {
  const { clearScreenPresence, setScreenPresence } = useAnkyPresence();
  const stableConfig = useMemo(
    () => config,
    [
      config.avoidKeyboard,
      config.emotion,
      config.intensity,
      config.maxMode,
      config.placement,
      config.preferredMode,
      config.sequence,
    ],
  );

  useFocusEffect(
    useCallback(() => {
      setScreenPresence(stableConfig);

      return () => {
        clearScreenPresence();
      };
    }, [clearScreenPresence, setScreenPresence, stableConfig]),
  );
}
