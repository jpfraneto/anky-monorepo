import { ReactNode } from "react";
import { StyleSheet, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";

import { ankyColors } from "../../theme/tokens";

type Props = {
  children: ReactNode;
  safe?: boolean;
  variant?: "centerGlow" | "cosmic" | "plain";
};

export function ScreenBackground({ children, safe = true, variant = "cosmic" }: Props) {
  const Content = safe ? SafeAreaView : View;

  return (
    <View style={[styles.root, variant === "plain" && styles.plain]}>
      <Content style={styles.content}>{children}</Content>
    </View>
  );
}

const styles = StyleSheet.create({
  content: {
    flex: 1,
  },
  plain: {
    backgroundColor: ankyColors.bg,
  },
  root: {
    backgroundColor: ankyColors.bg,
    flex: 1,
  },
});
