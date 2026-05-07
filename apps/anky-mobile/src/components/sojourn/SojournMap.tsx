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

  function scrollToDay(dayNumber: number, animated: boolean) {
    const index = orderedDays.findIndex((day) => day.day === dayNumber);

    if (index < 0) {
      return;
    }

    scrollRef.current?.scrollTo({
      animated,
      y: Math.max(0, index * sojournMapTokens.dayNode.rowHeight - height * 0.23),
    });
  }

  function selectDay(day: SojournMapDay) {
    setSelectedDayNumber(day.day);
    onSelectDay?.(day);
  }

  function travelToToday() {
    const today = days.find((day) => day.day === currentDay);

    if (today == null) {
      return;
    }

    selectDay(today);
    setTimeout(() => scrollToDay(currentDay, true), 40);
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
            currentDay={currentDay}
            day={selectedDay}
            onPressAnky={onPressAnky}
            onTravelToToday={travelToToday}
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
  currentDay,
  day,
  onPressAnky,
  onTravelToToday,
}: {
  bottomInset: number;
  currentDay: number;
  day: SojournMapDay;
  onPressAnky?: (anky: SojournMapAnky) => void;
  onTravelToToday: () => void;
}) {
  const completeAnkys = day.ankys.filter((anky) => anky.kind === "anky");
  const fragments = day.ankys.filter((anky) => anky.kind === "fragment");
  const hasWriting = completeAnkys.length > 0 || fragments.length > 0;
  const isPastDay = day.day < currentDay;

  return (
    <View style={[styles.panel, { paddingBottom: sojournMapTokens.spacing.lg + bottomInset }]}>
      <View style={styles.panelTopDiamond} />
      <Text style={styles.panelTitle}>Day {day.day}</Text>

      {!hasWriting ? (
        <View style={styles.emptyState}>
          <Text style={styles.emptyTitle}>{getEmptyDayTitle(day, currentDay)}</Text>
          <Text style={styles.emptyCopy}>{getEmptyDayCopy(day, currentDay)}</Text>
        </View>
      ) : (
        <ScrollView
          contentContainerStyle={styles.dayListContent}
          nestedScrollEnabled
          showsVerticalScrollIndicator={false}
          style={styles.dayList}
        >
          {completeAnkys.length > 0 ? (
            <View style={styles.ankySection}>
              <SectionDivider
                label={`${completeAnkys.length} ${completeAnkys.length === 1 ? "anky" : "ankys"}`}
              />
              {completeAnkys.map((anky) => (
                <AnkyPanelRow anky={anky} key={anky.id} onPress={() => onPressAnky?.(anky)} />
              ))}
            </View>
          ) : null}

          {fragments.length > 0 ? (
            <View style={styles.fragmentSection}>
              <SectionDivider
                label={`${fragments.length} ${fragments.length === 1 ? "fragment" : "fragments"}`}
              />
              {fragments.map((fragment) => (
                <FragmentPanelRow
                  fragment={fragment}
                  key={fragment.id}
                  onPress={() => onPressAnky?.(fragment)}
                />
              ))}
            </View>
          ) : null}
        </ScrollView>
      )}

      {isPastDay ? (
        <Pressable
          accessibilityRole="button"
          onPress={onTravelToToday}
          style={({ pressed }) => [styles.todayTravelButton, pressed && styles.pressed]}
        >
          <View style={styles.todayTravelLine} />
          <Text style={styles.todayTravelText}>return to today</Text>
          <View style={styles.todayTravelDiamond} />
          <View style={styles.todayTravelLine} />
        </Pressable>
      ) : null}
    </View>
  );
}

function AnkyPanelRow({
  anky,
  onPress,
}: {
  anky: SojournMapAnky;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      onPress={onPress}
      style={({ pressed }) => [styles.ankyCard, pressed && styles.ankyCardPressed]}
    >
      <Image
        accessibilityIgnoresInvertColors
        source={anky.avatar}
        style={styles.ankyAvatar}
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
  );
}

function FragmentPanelRow({
  fragment,
  onPress,
}: {
  fragment: SojournMapAnky;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      onPress={onPress}
      style={({ pressed }) => [styles.fragmentRow, pressed && styles.ankyCardPressed]}
    >
      <View style={styles.fragmentTextBlock}>
        <Text numberOfLines={1} style={styles.fragmentMeta}>
          {formatFragmentMeta(fragment)}
        </Text>
        <Text numberOfLines={1} style={styles.fragmentPreview}>
          {fragment.firstLine}
        </Text>
      </View>
      <Text style={styles.fragmentOpenGlyph}>›</Text>
    </Pressable>
  );
}

function SectionDivider({ label }: { label: string }) {
  return (
    <View style={styles.sectionDivider}>
      <View style={styles.sectionLine} />
      <Text style={styles.sectionTitle}>{label}</Text>
      <View style={styles.sectionLine} />
    </View>
  );
}

function formatFragmentMeta(fragment: SojournMapAnky): string {
  const duration = fragment.durationLabel.match(/\d+\s+min/)?.[0] ?? "fragment";
  const words = countWords(fragment.firstLine);

  return `${duration} · ${words} ${words === 1 ? "word" : "words"}`;
}

function countWords(value: string): number {
  const words = value
    .replace(/[^\p{L}\p{N}\s'-]/gu, "")
    .split(/\s+/)
    .filter(Boolean);

  return words.length;
}

function getEmptyDayTitle(day: SojournMapDay, currentDay: number): string {
  if (day.isFuture) {
    return "not open yet";
  }

  if (day.day < currentDay) {
    return "this day passed unwritten";
  }

  return "nothing here yet";
}

function getEmptyDayCopy(day: SojournMapDay, currentDay: number): string {
  if (day.isFuture) {
    return "this day has not arrived.";
  }

  if (day.day < currentDay) {
    return "anky simply witnessed the quiet.";
  }

  return "write when the day calls.";
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
  ankySection: {
    marginTop: sojournMapTokens.spacing.xs,
  },
  fragmentCount: {
    fontFamily: SERIF,
    fontSize: 22,
    fontWeight: "700",
    lineHeight: 22,
    opacity: 0.58,
  },
  fragmentMeta: {
    color: sojournMapTokens.colors.textMuted,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.caption,
    lineHeight: 17,
  },
  fragmentOpenGlyph: {
    color: sojournMapTokens.colors.gold,
    fontSize: 24,
    lineHeight: 28,
    marginLeft: sojournMapTokens.spacing.md,
  },
  fragmentPreview: {
    color: sojournMapTokens.colors.textSecondary,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.body,
    lineHeight: 18,
  },
  fragmentRow: {
    alignItems: "center",
    backgroundColor: "rgba(9, 8, 22, 0.46)",
    borderColor: "rgba(216, 176, 107, 0.18)",
    borderRadius: sojournMapTokens.radius.md,
    borderWidth: 1,
    flexDirection: "row",
    marginBottom: 5,
    minHeight: 54,
    paddingHorizontal: sojournMapTokens.spacing.lg,
    paddingVertical: sojournMapTokens.spacing.xs,
  },
  fragmentSection: {
    marginTop: sojournMapTokens.spacing.sm,
  },
  fragmentTextBlock: {
    flex: 1,
    minWidth: 0,
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
  dayList: {
    maxHeight: 336,
  },
  dayListContent: {
    paddingBottom: sojournMapTokens.spacing.xs,
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
  sectionDivider: {
    alignItems: "center",
    flexDirection: "row",
    gap: sojournMapTokens.spacing.md,
    marginBottom: sojournMapTokens.spacing.sm,
    marginTop: sojournMapTokens.spacing.xs,
  },
  sectionLine: {
    backgroundColor: sojournMapTokens.colors.trackGold,
    flex: 1,
    height: 1,
  },
  sectionTitle: {
    color: sojournMapTokens.colors.goldBright,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.subtitle,
    fontWeight: "700",
    lineHeight: 22,
    textTransform: "lowercase",
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
  todayTravelButton: {
    alignItems: "center",
    flexDirection: "row",
    gap: sojournMapTokens.spacing.sm,
    justifyContent: "center",
    marginTop: sojournMapTokens.spacing.sm,
    minHeight: 36,
    paddingHorizontal: sojournMapTokens.spacing.md,
  },
  todayTravelDiamond: {
    borderColor: sojournMapTokens.colors.gold,
    borderWidth: 1,
    height: 7,
    opacity: 0.74,
    transform: [{ rotate: "45deg" }],
    width: 7,
  },
  todayTravelLine: {
    backgroundColor: sojournMapTokens.colors.trackGold,
    flex: 1,
    height: 1,
    maxWidth: 72,
    opacity: 0.55,
  },
  todayTravelText: {
    color: sojournMapTokens.colors.goldBright,
    fontFamily: SERIF,
    fontSize: sojournMapTokens.typography.caption,
    lineHeight: 17,
    opacity: 0.88,
    textTransform: "lowercase",
  },
});
