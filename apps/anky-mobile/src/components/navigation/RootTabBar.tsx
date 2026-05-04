import { Image, ImageSourcePropType, Pressable, StyleSheet, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import { ankyColors, spacing } from "../../theme/tokens";

export type RootTabName = "Track" | "Write" | "You";

type Props = {
  active: RootTabName;
  onSelect: (tab: RootTabName) => void;
};

const TAB_ICONS = {
  Track: require("../../../assets/anky-you/icons/map.png"),
  Write: require("../../../assets/anky-you/icons/write.png"),
  You: require("../../../assets/anky-you/icons/you.png"),
} satisfies Record<RootTabName, ImageSourcePropType>;

const TABS: { label: string; name: RootTabName }[] = [
  { label: "write", name: "Write" },
  { label: "map", name: "Track" },
  { label: "you", name: "You" },
];

export function RootTabBar({ active, onSelect }: Props) {
  const insets = useSafeAreaInsets();

  return (
    <View
      pointerEvents="box-none"
      style={[styles.outer, { paddingBottom: Math.max(10, insets.bottom + 6) }]}
    >
      <View style={styles.wrap}>
        {TABS.map((tab) => {
          const selected = active === tab.name;

          return (
            <Pressable
              accessibilityRole="tab"
              accessibilityState={{ selected }}
              key={tab.name}
              onPress={() => {
                if (!selected) {
                  onSelect(tab.name);
                }
              }}
              style={({ pressed }) => [
                styles.tab,
                selected && styles.selected,
                pressed && styles.pressed,
              ]}
            >
              <View style={[styles.iconHalo, selected && styles.iconHaloSelected]}>
                <Image
                  accessibilityIgnoresInvertColors
                  source={TAB_ICONS[tab.name]}
                  style={[styles.icon, !selected && styles.iconInactive]}
                />
              </View>
              <Text style={[styles.label, selected && styles.labelSelected]}>{tab.label}</Text>
            </Pressable>
          );
        })}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  icon: {
    height: 38,
    width: 38,
  },
  iconHalo: {
    alignItems: "center",
    borderRadius: 25,
    height: 50,
    justifyContent: "center",
    width: 50,
  },
  iconHaloSelected: {
    backgroundColor: "rgba(233, 190, 114, 0.13)",
    shadowColor: "#F1C47B",
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.35,
    shadowRadius: 13,
  },
  iconInactive: {
    opacity: 0.46,
  },
  label: {
    color: "rgba(216, 201, 212, 0.72)",
    fontSize: 13,
    fontWeight: "600",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
  labelSelected: {
    color: "#F2D392",
  },
  outer: {
    backgroundColor: "transparent",
    bottom: 0,
    left: 0,
    paddingHorizontal: spacing.md,
    paddingTop: spacing.sm,
    position: "absolute",
    right: 0,
    zIndex: 20,
  },
  pressed: {
    opacity: 0.7,
  },
  selected: {
    opacity: 1,
  },
  tab: {
    alignItems: "center",
    flex: 1,
    height: "100%",
    justifyContent: "center",
  },
  wrap: {
    alignItems: "center",
    backgroundColor: "rgba(10, 9, 25, 0.93)",
    borderColor: "rgba(219, 143, 63, 0.44)",
    borderRadius: 24,
    borderWidth: 1,
    flexDirection: "row",
    height: 84,
    overflow: "hidden",
    shadowColor: ankyColors.gold,
    shadowOffset: { height: 0, width: 0 },
    shadowOpacity: 0.1,
    shadowRadius: 16,
  },
});
