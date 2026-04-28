import { Image, StyleSheet } from "react-native";

const logo = require("../../assets/adaptive-icon.png");

type AnkyMarkProps = {
  size?: number;
};

export function AnkyMark({ size = 148 }: AnkyMarkProps) {
  return (
    <Image
      accessibilityIgnoresInvertColors
      source={logo}
      style={[styles.image, { height: size, width: size }]}
    />
  );
}

const styles = StyleSheet.create({
  image: {
    resizeMode: "contain",
  },
});
