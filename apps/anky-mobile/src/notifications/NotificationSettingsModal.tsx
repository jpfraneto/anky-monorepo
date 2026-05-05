import { useEffect, useMemo, useState } from "react";
import {
  Modal,
  Pressable,
  StyleSheet,
  Switch,
  Text,
  TextInput,
  View,
} from "react-native";

import { ankyColors, fontSize, spacing } from "../theme/tokens";
import {
  DEFAULT_NOTIFICATION_SETTINGS,
  disableWritingReminder,
  enableDailyWritingReminder,
  formatReminderTime,
  getPresetTime,
  loadNotificationSettings,
  type AnkyNotificationSettings,
  type NotificationPreset,
} from "./notificationSettings";

const PRESETS: Array<{ label: string; value: NotificationPreset }> = [
  { label: "morning", value: "morning" },
  { label: "afternoon", value: "afternoon" },
  { label: "evening", value: "evening" },
  { label: "custom", value: "custom" },
];

export function NotificationSettingsModal({
  onClose,
  onSaved,
  visible,
}: {
  onClose: () => void;
  onSaved?: (settings: AnkyNotificationSettings) => void;
  visible: boolean;
}) {
  const [busy, setBusy] = useState(false);
  const [hourText, setHourText] = useState(String(DEFAULT_NOTIFICATION_SETTINGS.hour));
  const [message, setMessage] = useState("");
  const [minuteText, setMinuteText] = useState("00");
  const [preset, setPreset] = useState<NotificationPreset>("evening");
  const [settings, setSettings] = useState<AnkyNotificationSettings>(
    DEFAULT_NOTIFICATION_SETTINGS,
  );
  const selectedTime = useMemo(() => parseTime(hourText, minuteText), [hourText, minuteText]);

  useEffect(() => {
    if (!visible) {
      return;
    }

    let mounted = true;

    async function load() {
      const nextSettings = await loadNotificationSettings();

      if (!mounted) {
        return;
      }

      setSettings(nextSettings);
      setPreset(nextSettings.preset);
      setHourText(String(nextSettings.hour));
      setMinuteText(String(nextSettings.minute).padStart(2, "0"));
      setMessage("");
    }

    void load().catch((error) => {
      console.error(error);
      setMessage("could not load notification settings.");
    });

    return () => {
      mounted = false;
    };
  }, [visible]);

  function selectPreset(nextPreset: NotificationPreset) {
    setPreset(nextPreset);

    if (nextPreset !== "custom") {
      const time = getPresetTime(nextPreset);

      setHourText(String(time.hour));
      setMinuteText(String(time.minute).padStart(2, "0"));
    }
  }

  async function setEnabled(nextEnabled: boolean) {
    if (busy) {
      return;
    }

    setBusy(true);
    setMessage("");

    try {
      const nextSettings = nextEnabled
        ? await enableDailyWritingReminder({
            hour: selectedTime.hour,
            minute: selectedTime.minute,
            preset,
          })
        : await disableWritingReminder();

      setSettings(nextSettings);
      onSaved?.(nextSettings);
      setMessage(
        nextSettings.enabled
          ? `reminder set for ${formatReminderTime(nextSettings.hour, nextSettings.minute)}.`
          : "reminder turned off.",
      );
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "could not update reminders.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal animationType="fade" onRequestClose={onClose} transparent visible={visible}>
      <View style={styles.root}>
        <Pressable accessibilityRole="button" onPress={onClose} style={styles.scrim} />
        <View style={styles.sheet}>
          <View style={styles.header}>
            <Text style={styles.title}>notifications</Text>
            <Text style={styles.subtitle}>a quiet daily invitation to write.</Text>
          </View>

          <View style={styles.enableRow}>
            <View style={styles.enableCopy}>
              <Text style={styles.rowTitle}>daily reminder</Text>
              <Text style={styles.rowSubtitle}>
                {settings.enabled
                  ? formatReminderTime(settings.hour, settings.minute)
                  : "off"}
              </Text>
            </View>
            <Switch
              disabled={busy}
              ios_backgroundColor="rgba(244, 241, 234, 0.14)"
              onValueChange={(nextValue) => void setEnabled(nextValue)}
              thumbColor={settings.enabled ? ankyColors.gold : "rgba(244, 241, 234, 0.72)"}
              trackColor={{ false: "rgba(244, 241, 234, 0.14)", true: "rgba(214, 147, 68, 0.58)" }}
              value={settings.enabled}
            />
          </View>

          <View style={styles.presets}>
            {PRESETS.map((item) => (
              <Pressable
                accessibilityRole="button"
                key={item.value}
                onPress={() => selectPreset(item.value)}
                style={({ pressed }) => [
                  styles.presetButton,
                  preset === item.value && styles.presetButtonSelected,
                  pressed && styles.pressed,
                ]}
              >
                <Text
                  style={[
                    styles.presetText,
                    preset === item.value && styles.presetTextSelected,
                  ]}
                >
                  {item.label}
                </Text>
              </Pressable>
            ))}
          </View>

          {preset === "custom" ? (
            <View style={styles.timeRow}>
              <TextInput
                keyboardType="number-pad"
                maxLength={2}
                onChangeText={setHourText}
                placeholder="20"
                placeholderTextColor="rgba(255, 240, 201, 0.42)"
                style={styles.timeInput}
                value={hourText}
              />
              <Text style={styles.timeColon}>:</Text>
              <TextInput
                keyboardType="number-pad"
                maxLength={2}
                onChangeText={setMinuteText}
                placeholder="00"
                placeholderTextColor="rgba(255, 240, 201, 0.42)"
                style={styles.timeInput}
                value={minuteText}
              />
            </View>
          ) : null}

          {settings.enabled ? (
            <Pressable
              accessibilityRole="button"
              disabled={busy}
              onPress={() => void setEnabled(true)}
              style={({ pressed }) => [styles.saveButton, pressed && styles.pressed]}
            >
              <Text style={styles.saveText}>{busy ? "saving" : "save time"}</Text>
            </Pressable>
          ) : null}

          {message.length === 0 ? null : <Text style={styles.message}>{message}</Text>}

          <Pressable accessibilityRole="button" onPress={onClose} style={styles.closeButton}>
            <Text style={styles.closeText}>done</Text>
          </Pressable>
        </View>
      </View>
    </Modal>
  );
}

function parseTime(hourText: string, minuteText: string): { hour: number; minute: number } {
  const hour = Number(hourText);
  const minute = Number(minuteText);

  return {
    hour: Number.isInteger(hour) ? Math.max(0, Math.min(23, hour)) : 20,
    minute: Number.isInteger(minute) ? Math.max(0, Math.min(59, minute)) : 0,
  };
}

const styles = StyleSheet.create({
  closeButton: {
    alignItems: "center",
    paddingVertical: spacing.sm,
  },
  closeText: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  enableCopy: {
    flex: 1,
    minWidth: 0,
  },
  enableRow: {
    alignItems: "center",
    backgroundColor: "rgba(255,255,255,0.04)",
    borderColor: "rgba(232, 200, 121, 0.2)",
    borderRadius: 8,
    borderWidth: 1,
    flexDirection: "row",
    marginTop: spacing.lg,
    padding: spacing.md,
  },
  header: {
    alignItems: "center",
  },
  message: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.md,
    textAlign: "center",
    textTransform: "lowercase",
  },
  presetButton: {
    alignItems: "center",
    borderColor: "rgba(232, 200, 121, 0.2)",
    borderRadius: 8,
    borderWidth: 1,
    flex: 1,
    minHeight: 42,
    justifyContent: "center",
    paddingHorizontal: 8,
  },
  presetButtonSelected: {
    backgroundColor: "rgba(232, 200, 121, 0.14)",
    borderColor: "rgba(232, 200, 121, 0.54)",
  },
  presets: {
    flexDirection: "row",
    gap: spacing.xs,
    marginTop: spacing.md,
  },
  presetText: {
    color: ankyColors.textMuted,
    fontSize: 12,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  presetTextSelected: {
    color: ankyColors.gold,
  },
  pressed: {
    opacity: 0.72,
  },
  root: {
    flex: 1,
    justifyContent: "flex-end",
  },
  rowSubtitle: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 19,
    marginTop: 2,
    textTransform: "lowercase",
  },
  rowTitle: {
    color: ankyColors.text,
    fontSize: fontSize.md,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  saveButton: {
    alignItems: "center",
    backgroundColor: "rgba(232, 200, 121, 0.16)",
    borderColor: "rgba(232, 200, 121, 0.46)",
    borderRadius: 8,
    borderWidth: 1,
    marginTop: spacing.md,
    minHeight: 46,
    justifyContent: "center",
  },
  saveText: {
    color: ankyColors.gold,
    fontSize: fontSize.md,
    fontWeight: "700",
    textTransform: "lowercase",
  },
  scrim: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(0, 0, 0, 0.62)",
  },
  sheet: {
    backgroundColor: "rgba(8, 7, 19, 0.98)",
    borderColor: "rgba(232, 200, 121, 0.28)",
    borderTopLeftRadius: 20,
    borderTopRightRadius: 20,
    borderWidth: 1,
    padding: spacing.xl,
  },
  subtitle: {
    color: ankyColors.textMuted,
    fontSize: fontSize.sm,
    lineHeight: 20,
    marginTop: spacing.xs,
    textAlign: "center",
    textTransform: "lowercase",
  },
  timeColon: {
    color: ankyColors.gold,
    fontSize: 22,
    fontWeight: "700",
  },
  timeInput: {
    backgroundColor: "rgba(255,255,255,0.045)",
    borderColor: "rgba(232, 200, 121, 0.24)",
    borderRadius: 8,
    borderWidth: 1,
    color: ankyColors.text,
    fontSize: fontSize.lg,
    minWidth: 64,
    paddingHorizontal: spacing.md,
    paddingVertical: 10,
    textAlign: "center",
  },
  timeRow: {
    alignItems: "center",
    flexDirection: "row",
    gap: spacing.sm,
    justifyContent: "center",
    marginTop: spacing.md,
  },
  title: {
    color: ankyColors.gold,
    fontSize: fontSize.xl,
    fontWeight: "700",
    letterSpacing: 0,
    textTransform: "lowercase",
  },
});
