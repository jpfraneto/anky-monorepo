import { Fragment, type ReactNode } from "react";
import { Platform, StyleSheet, Text, type StyleProp, type TextStyle, View } from "react-native";

type Props = {
  selectable?: boolean;
  text: string;
  textStyle?: StyleProp<TextStyle>;
};

const SERIF = Platform.select({ android: "serif", default: "Georgia", ios: "Georgia" });
const GOLD = "#E8C879";
const MUTED = "rgba(244, 241, 234, 0.68)";

export function SimpleMarkdownText({ selectable = true, text, textStyle }: Props) {
  const lines = text.replace(/\r\n/g, "\n").split("\n");

  return (
    <View style={styles.wrap}>
      {lines.map((line, index) => (
        <MarkdownLine
          key={`${index}-${line.slice(0, 12)}`}
          line={line}
          selectable={selectable}
          textStyle={textStyle}
        />
      ))}
    </View>
  );
}

function MarkdownLine({
  line,
  selectable,
  textStyle,
}: {
  line: string;
  selectable: boolean;
  textStyle?: StyleProp<TextStyle>;
}) {
  const trimmed = line.trim();

  if (trimmed.length === 0) {
    return <View style={styles.blankLine} />;
  }

  const heading = /^(#{1,3})\s+(.+)$/.exec(trimmed);

  if (heading != null) {
    return (
      <Text selectable={selectable} style={[styles.baseText, textStyle, styles.heading]}>
        {renderInline(heading[2])}
      </Text>
    );
  }

  const bullet = /^[-*]\s+(.+)$/.exec(trimmed);

  if (bullet != null) {
    return (
      <View style={styles.listRow}>
        <Text style={[styles.baseText, textStyle, styles.bulletMarker]}>•</Text>
        <Text selectable={selectable} style={[styles.baseText, textStyle, styles.listText]}>
          {renderInline(bullet[1])}
        </Text>
      </View>
    );
  }

  const numbered = /^(\d+)\.\s+(.+)$/.exec(trimmed);

  if (numbered != null) {
    return (
      <View style={styles.listRow}>
        <Text style={[styles.baseText, textStyle, styles.numberMarker]}>{numbered[1]}.</Text>
        <Text selectable={selectable} style={[styles.baseText, textStyle, styles.listText]}>
          {renderInline(numbered[2])}
        </Text>
      </View>
    );
  }

  const quote = /^>\s?(.+)$/.exec(trimmed);

  if (quote != null) {
    return (
      <View style={styles.quote}>
        <Text selectable={selectable} style={[styles.baseText, textStyle, styles.quoteText]}>
          {renderInline(quote[1])}
        </Text>
      </View>
    );
  }

  return (
    <Text selectable={selectable} style={[styles.baseText, textStyle, styles.paragraph]}>
      {renderInline(trimmed)}
    </Text>
  );
}

function renderInline(value: string): ReactNode[] {
  const parts = value.split(/(\*\*[^*]+\*\*|`[^`]+`)/g).filter((part) => part.length > 0);

  return parts.map((part, index) => {
    if (part.startsWith("**") && part.endsWith("**")) {
      return (
        <Text key={index} style={styles.strong}>
          {part.slice(2, -2)}
        </Text>
      );
    }

    if (part.startsWith("`") && part.endsWith("`")) {
      return (
        <Text key={index} style={styles.code}>
          {part.slice(1, -1)}
        </Text>
      );
    }

    return <Fragment key={index}>{part}</Fragment>;
  });
}

const styles = StyleSheet.create({
  baseText: {
    fontFamily: SERIF,
  },
  blankLine: {
    height: 9,
  },
  bulletMarker: {
    color: GOLD,
    lineHeight: 24,
    width: 18,
  },
  code: {
    backgroundColor: "rgba(244, 241, 234, 0.09)",
    color: GOLD,
    fontFamily: Platform.select({ android: "monospace", default: "Courier", ios: "Courier" }),
  },
  heading: {
    color: GOLD,
    fontSize: 20,
    fontWeight: "700",
    lineHeight: 27,
    marginBottom: 5,
    marginTop: 4,
  },
  listRow: {
    alignItems: "flex-start",
    flexDirection: "row",
    marginTop: 4,
  },
  listText: {
    flex: 1,
    minWidth: 0,
  },
  numberMarker: {
    color: GOLD,
    lineHeight: 24,
    width: 28,
  },
  paragraph: {
    marginTop: 5,
  },
  quote: {
    borderLeftColor: "rgba(232, 200, 121, 0.36)",
    borderLeftWidth: 2,
    marginTop: 6,
    paddingLeft: 10,
  },
  quoteText: {
    color: MUTED,
    fontStyle: "italic",
  },
  strong: {
    color: GOLD,
    fontWeight: "700",
  },
  wrap: {
    width: "100%",
  },
});
