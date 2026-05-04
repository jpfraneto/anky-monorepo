import { StyleSheet, Text, View } from "react-native";

import { ankyColors } from "../../theme/tokens";

type Props = {
  count: number;
};

export function ThreadCount({ count }: Props) {
  if (count <= 1) {
    return null;
  }

  return (
    <View style={styles.wrap}>
      <Text style={styles.text}>+{count - 1}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  text: {
    color: ankyColors.bg,
    fontSize: 10,
    fontWeight: "800",
  },
  wrap: {
    alignItems: "center",
    backgroundColor: ankyColors.gold,
    borderRadius: 8,
    minWidth: 18,
    paddingHorizontal: 4,
    paddingVertical: 2,
  },
});
