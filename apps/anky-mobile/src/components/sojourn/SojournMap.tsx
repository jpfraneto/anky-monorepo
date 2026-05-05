import { useEffect, useMemo, useRef, useState } from "react";
import {
  Image,
  ImageBackground,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  useWindowDimensions,
  View,
} from "react-native";

import {
  getMapKingdomForDay,
  sojournMapTokens,
} from "../../theme/sojournMapTokens";
import type { SojournMapAnky, SojournMapDay } from "./SojournMap.types";

type Props = {
  bottomInset?: number;
  currentDay: number;
  days: SojournMapDay[];
  initialSelectedDay?: number;
  onPressAnky?: (anky: SojournMapAnky) => void;
  onSelectDay?: (day: SojournMapDay) => void;
  sojournLength?: number;
  sojournNumber?: number;
};

const background = require("../../../assets/sojourn-map/backgrounds/sojourn-cosmos.png");
const SERIF = Platform.select({ android: "serif", default: "Georgia", ios: "Georgia" });

export function SojournMap({
  bottomInset = 0,
  currentDay,
  days,
  initialSelectedDay,
  onPressAnky,
  onSelectDay,
  sojournLength = 96,
  sojournNumber = 9,
}: Props) {
  const scrollRef = useRef<ScrollView>(null);
  const didInitialScrollRef = useRef(false);
  const { height } = useWindowDimensions();
  const [selectedDayNumber, setSelectedDayNumber] = useState(initialSelectedDay ?? currentDay);
  const orderedDays = useMemo(() => [...days].sort((a, b) => b.day - a.day), [days]);
  const selectedDay = useMemo(
    () =>
      days.find((day) => day.day === selectedDayNumber) ??
      days.find((day) => day.day === currentDay) ??
      days[0],
    [currentDay, days, selectedDayNumber],
  );

  useEffect(() => {
    if (didInitialScrollRef.current || orderedDays.length === 0) {
      return;
    }

    const timer = setTimeout(() => {
      const index = orderedDays.findIndex((day) => day.day === selectedDayNumber);

      if (index < 0) {
        return;
      }

      scrollRef.current?.scrollTo({
        animated: false,
        y: Math.max(0, index * sojournMapTokens.dayNode.rowHeight - height * 0.23),
      });
      didInitialScrollRef.current = true;
    }, 80);

    return () => clearTimeout(timer);
  }, [height, orderedDays, selectedDayNumber]);

  function selectDay(day: SojournMapDay) {
    setSelectedDayNumber(day.day);
    onSelectDay?.(day);
  }

  return (
    <ImageBackground resizeMode="cover" source={background} style={styles.background}>
      <View style={styles.container}>
        <View style={styles.header}>
          <Text style={styles.title}>Sojourn {romanize(sojournNumber)}</Text>
          <Text style={styles.subtitle}>
            Day {currentDay} of {sojournLength}
          </Text>
          <View style={styles.headerOrnament}>
            <View style={styles.headerLine} />
            <View style={styles.headerDiamond} />
            <View style={styles.headerLine} />
          </View>
        </View>

        <View style={styles.mapArea}>
          <ScrollView
            ref={scrollRef}
            contentContainerStyle={styles.scrollContent}
            showsVerticalScrollIndicator={false}
          >
            <View style={styles.centerLine} />
            {orderedDays.map((day) => (
              <DayRow
                currentDay={currentDay}
                day={day}
                isSelected={selectedDay?.day === day.day}
                key={day.day}
                onPress={() => selectDay(day)}
              />
            ))}
          </ScrollView>
        </View>

        {selectedDay == null ? null : (
          <SelectedDayPanel
            bottomInset={bottomInset}
            day={selectedDay}
            onPressAnky={onPressAnky}
          />
        )}
      </View>
    </ImageBackground>
  );
}

function DayRow({
  currentDay,
  day,
  isSelected,
  onPress,
}: {
  currentDay: number;
  day: SojournMapDay;
  isSelected: boolean;
  onPress: () => void;
}) {
  const { color } = getMapKingdomForDay(day.day);
  const fragmentCount = day.fragmentCount ?? 0;
  const hasAnkys = day.ankyCount > 0;
  const hasFragments = fragmentCount > 0;
  const hasAnyWriting = hasAnkys || hasFragments;
  const isFuture = day.day > currentDay;
  const size = isSelected
    ? sojournMapTokens.dayNode.sizeSelected
    : hasAnkys
      ? sojournMapTokens.dayNode.sizeWritten
      : hasFragments
        ? sojournMapTokens.dayNode.sizeWritten - 10
      : sojournMapTokens.dayNode.sizeFuture;

  return (
    <View style={styles.dayRow}>
      <AnkyCountLabel
        color={color}
        count={day.ankyCount}
        fragmentCount={fragmentCount}
        selected={isSelected}
      />

      <Pressable
        accessibilityLabel={`day ${day.day}${
          hasAnyWriting
            ? `, ${day.ankyCount} ankys, ${fragmentCount} fragments`
            : ", no ankys"
        }`}
        accessibilityRole="button"
        onPress={onPress}
        style={({ pressed }) => [
          styles.dayNode,
          {
            borderColor: withAlpha(color, hasAnyWriting || isSelected ? "E6" : "52"),
            height: size,
            shadowColor: color,
            width: size,
          },
          hasAnkys && {
            backgroundColor: withAlpha(color, "13"),
            shadowOpacity: 0.55,
            shadowRadius: 12,
          },
          !hasAnkys && hasFragments && {
            backgroundColor: withAlpha(color, "09"),
            opacity: 0.72,
            shadowOpacity: 0.22,
            shadowRadius: 8,
          },
          isFuture && !hasAnyWriting && styles.futureDay,
          isSelected && {
            backgroundColor: withAlpha(color, "24"),
            shadowOpacity: 0.95,
            shadowRadius: 24,
          },
          pressed && styles.pressed,
        ]}
      >
        <Text style={[styles.dayNodeText, isSelected && styles.dayNodeTextSelected]}>
          {day.day}
        </Text>
      </Pressable>

      <View style={styles.dayRowRightSpacer} />
    </View>
  );
}

function AnkyCountLabel({
  color,
  count,
  fragmentCount,
  selected,
}: {
  color: string;
  count: number;
  fragmentCount: number;
  selected: boolean;
}) {
  return (
    <View style={styles.dotRail}>
      {count > 0 ? (
        <Text style={[styles.ankyCount, { color, transform: [{ scale: selected ? 1.08 : 1 }] }]}>
          {count}
        </Text>
      ) : fragmentCount > 0 ? (
        <Text style={[styles.fragmentCount, { color }]}>•</Text>
      ) : null}
    </View>
  );
}

function SelectedDayPanel({
  bottomInset,
  day,
  onPressAnky,
}: {
  bottomInset: number;
  day: SojournMapDay;
  onPressAnky?: (anky: SojournMapAnky) => void;
}) {
  return (
    <View style={[styles.panel, { paddingBottom: sojournMapTokens.spacing.lg + bottomInset }]}>
      <View style={styles.panelTopDiamond} />
      <Text style={styles.panelTitle}>Day {day.day}</Text>
      <Text style={styles.panelSubtitle}>
        {formatPanelCount(day)}
      </Text>

      {day.ankys.length === 0 ? (
        <View style={styles.emptyState}>
          <Text style={styles.emptyTitle}>
            {day.isFuture ? "not open yet" : "nothing here yet"}
          </Text>
          <Text style={styles.emptyCopy}>
            {day.isFuture ? "this day has not arrived." : "write when the day calls."}
          </Text>
        </View>
      ) : (
        <ScrollView style={styles.ankyList} nestedScrollEnabled showsVerticalScrollIndicator={false}>
          {day.ankys.map((anky) => (
            <Pressable
              accessibilityRole="button"
              key={anky.id}
              onPress={() => onPressAnky?.(anky)}
              style={({ pressed }) => [
                styles.ankyCard,
                anky.kind === "fragment" && styles.fragmentCard,
                pressed && styles.ankyCardPressed,
              ]}
            >
              <Image
                accessibilityIgnoresInvertColors
                source={anky.avatar}
                style={[styles.ankyAvatar, anky.kind === "fragment" && styles.fragmentAvatar]}
              />
              <View style={styles.ankyTextBlock}>
                <Text numberOfLines={1} style={styles.ankyTitle}>
                  {anky.title}
                </Text>
                <Text numberOfLines={1} style={styles.ankyMeta}>
                  {anky.durationLabel}
                </Text>
                <Text numberOfLines={1} style={styles.ankyPreview}>
                  {anky.firstLine}
                </Text>
              </View>
              <Text style={styles.openGlyph}>›</Text>
            </Pressable>
          ))}
        </ScrollView>
      )}
    </View>
  );
}

function formatPanelCount(day: SojournMapDay): string {
  const fragmentCount = day.fragmentCount ?? 0;
  const parts = [
    `${day.ankyCount} ${day.ankyCount === 1 ? "anky" : "ankys"}`,
    fragmentCount > 0
      ? `${fragmentCount} ${fragmentCount === 1 ? "fragment" : "fragments"}`
      : null,
  ].filter((part): part is string => part != null);

  return parts.join(" · ");
}

function withAlpha(hex: string, alpha: string) {
  return `${hex}${alpha}`;
}

function romanize(value: number) {
  const pairs: Array<[number, string]> = [
    [10, "X"],
    [9, "IX"],
    [5, "V"],
    [4, "IV"],
    [1, "I"],
  ];
  let n = value;
  let out = "";

  for (const [amount, numeral] of pairs) {
    while (n >= amount) {
      out += numeral;
      n -= amount;
    }
  }

  return out || String(value);
}

const styles = StyleSheet.create({
  ankyAvatar: {
    borderColor: "rgba(216,176,107,0.55)",
    borderRadius: sojournMapTokens.card.avatar / 2,
    borderWidth: 1,
    height: sojournMapTokens.card.avatar,
    marginRight: sojournMapTokens.spacing.md,
    width: sojournMapTokens.card.avatar,
  },
  ankyCard: {
    alignItems: "center",
    backgroundColor: sojournMapTokens.colors.panelSoft,
    borderColor: sojournMapTokens.colors.panelBorder,
    borderRadius: sojournMapTokens.radius.md,
    borderWidth: 1,
    flexDirection: "row",
    marginBottom: sojournMapTokens.spacing.sm,
    minHeight: sojournMapTokens.card.minHeight,
    padding: sojournMapTokens.spacing.md,
  },
  ankyCardPressed: {
    opacity: 0.76,
  },
  ankyCount: {
    fontFamily: SERIF,
    fontSize: 17,
    fontWeight: "700",
    lineHeight: 21,
    textShadowColor: "rgba(0,0,0,0.45)",
    textShadowRadius: 3,
  },
  fragmentAvatar: {
    height: sojournMapTokens.card.avatar - 10,
    opacity: 0.58,
    width: sojournMapTokens.card.avatar - 10,
  },
  fragmentCard: {
    backgroundColor: "rgba(18, 16, 34, 0.42)",
    borderColor: "rgba(216,176,107,0.22)",
    minHeight: sojournMapTokens.card.minHeight - 12,
    opacity: 0.78,
  },
  fragmentCount: {
    fontFamily: SERIF,
    fontSize: 22,
    fontWeight: "700",
    lineHeight: 22,
    opacity: 0.58,
  },
  ankyList: {
    maxHeight: 288,
  },
  ankyMeta: {
    color: sojournMapTokens.colors.textSecondary,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.caption,
    marginTop: 2,
  },
  ankyPreview: {
    color: sojournMapTokens.colors.textMuted,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.body,
    marginTop: 4,
  },
  ankyTextBlock: {
    flex: 1,
    minWidth: 0,
  },
  ankyTitle: {
    color: sojournMapTokens.colors.textPrimary,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.cardTitle,
    lineHeight: 23,
  },
  background: {
    flex: 1,
  },
  centerLine: {
    backgroundColor: sojournMapTokens.colors.track,
    bottom: 0,
    left: "50%",
    marginLeft: -1,
    position: "absolute",
    top: 0,
    width: 2,
  },
  container: {
    backgroundColor: "rgba(3, 3, 10, 0.18)",
    flex: 1,
  },
  dayNode: {
    alignItems: "center",
    borderRadius: sojournMapTokens.radius.pill,
    borderWidth: 2,
    elevation: 7,
    justifyContent: "center",
    shadowOffset: { height: 0, width: 0 },
  },
  dayNodeText: {
    color: sojournMapTokens.colors.textPrimary,
    fontFamily: SERIF,
    fontSize: 22,
    lineHeight: 27,
  },
  dayNodeTextSelected: {
    color: sojournMapTokens.colors.goldBright,
    fontSize: 28,
    lineHeight: 34,
  },
  dayRow: {
    alignItems: "center",
    flexDirection: "row",
    height: sojournMapTokens.dayNode.rowHeight,
    justifyContent: "center",
    width: "100%",
  },
  dayRowRightSpacer: {
    width: 94,
  },
  dotRail: {
    alignItems: "center",
    flexDirection: "row",
    justifyContent: "flex-end",
    paddingRight: sojournMapTokens.spacing.lg,
    width: 94,
  },
  emptyCopy: {
    color: sojournMapTokens.colors.textMuted,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.body,
    lineHeight: 20,
    marginTop: sojournMapTokens.spacing.xs,
    textTransform: "lowercase",
  },
  emptyState: {
    backgroundColor: sojournMapTokens.colors.panelSoft,
    borderColor: sojournMapTokens.colors.panelBorder,
    borderRadius: sojournMapTokens.radius.md,
    borderWidth: 1,
    padding: sojournMapTokens.spacing.lg,
  },
  emptyTitle: {
    color: sojournMapTokens.colors.textPrimary,
    fontFamily: SERIF,
    fontSize: 16,
    lineHeight: 21,
    textTransform: "lowercase",
  },
  futureDay: {
    opacity: 0.48,
  },
  header: {
    alignItems: "center",
    paddingBottom: sojournMapTokens.spacing.md,
    paddingHorizontal: sojournMapTokens.spacing.xl,
    paddingTop: sojournMapTokens.spacing.xxxl,
  },
  headerDiamond: {
    borderColor: sojournMapTokens.colors.gold,
    borderWidth: 1,
    height: 9,
    marginHorizontal: 10,
    transform: [{ rotate: "45deg" }],
    width: 9,
  },
  headerLine: {
    backgroundColor: sojournMapTokens.colors.trackGold,
    height: 1,
    width: 78,
  },
  headerOrnament: {
    alignItems: "center",
    flexDirection: "row",
    marginTop: sojournMapTokens.spacing.sm,
  },
  mapArea: {
    flex: 1,
  },
  openGlyph: {
    color: sojournMapTokens.colors.gold,
    fontSize: 26,
    marginLeft: sojournMapTokens.spacing.md,
  },
  panel: {
    backgroundColor: sojournMapTokens.colors.panel,
    borderColor: sojournMapTokens.colors.panelBorder,
    borderTopLeftRadius: sojournMapTokens.radius.lg,
    borderTopRightRadius: sojournMapTokens.radius.lg,
    borderWidth: 1,
    paddingHorizontal: sojournMapTokens.spacing.lg,
    paddingTop: sojournMapTokens.spacing.lg,
  },
  panelSubtitle: {
    color: sojournMapTokens.colors.textSecondary,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.subtitle,
    marginBottom: sojournMapTokens.spacing.md,
    marginTop: 2,
    textAlign: "center",
    textTransform: "lowercase",
  },
  panelTitle: {
    color: sojournMapTokens.colors.textPrimary,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.sheetTitle,
    lineHeight: 36,
    textAlign: "center",
  },
  panelTopDiamond: {
    alignSelf: "center",
    backgroundColor: sojournMapTokens.colors.backgroundDeep,
    borderColor: sojournMapTokens.colors.gold,
    borderWidth: 1,
    height: 12,
    marginBottom: -1,
    marginTop: -23,
    transform: [{ rotate: "45deg" }],
    width: 12,
  },
  pressed: {
    opacity: 0.72,
  },
  scrollContent: {
    alignItems: "center",
    paddingBottom: 190,
    paddingTop: sojournMapTokens.spacing.sm,
  },
  subtitle: {
    color: sojournMapTokens.colors.textSecondary,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.subtitle,
    lineHeight: 22,
    marginTop: 2,
  },
  title: {
    color: sojournMapTokens.colors.textPrimary,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.title,
    lineHeight: 42,
    textShadowColor: "rgba(216,176,107,0.25)",
    textShadowOffset: { height: 0, width: 0 },
    textShadowRadius: 12,
  },
});
