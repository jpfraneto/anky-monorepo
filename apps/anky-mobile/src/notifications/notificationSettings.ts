import { Platform } from "react-native";
import * as Notifications from "expo-notifications";
import * as SecureStore from "expo-secure-store";

const NOTIFICATION_SETTINGS_KEY = "anky.notifications.settings.v1";
const ANKY_REMINDER_CHANNEL = "anky-writing-reminder";

export type NotificationPreset = "afternoon" | "custom" | "evening" | "morning";

export type AnkyNotificationSettings = {
  enabled: boolean;
  hour: number;
  minute: number;
  preset: NotificationPreset;
  scheduledId?: string;
  version: 1;
};

export const DEFAULT_NOTIFICATION_SETTINGS: AnkyNotificationSettings = {
  enabled: false,
  hour: 20,
  minute: 0,
  preset: "evening",
  version: 1,
};

Notifications.setNotificationHandler({
  handleNotification: async () => ({
    shouldPlaySound: false,
    shouldSetBadge: false,
    shouldShowBanner: true,
    shouldShowList: true,
  }),
});

export async function loadNotificationSettings(): Promise<AnkyNotificationSettings> {
  const raw = await SecureStore.getItemAsync(NOTIFICATION_SETTINGS_KEY);

  if (raw == null) {
    return DEFAULT_NOTIFICATION_SETTINGS;
  }

  try {
    const parsed = JSON.parse(raw) as unknown;

    return isNotificationSettings(parsed) ? parsed : DEFAULT_NOTIFICATION_SETTINGS;
  } catch {
    return DEFAULT_NOTIFICATION_SETTINGS;
  }
}

export async function saveNotificationSettings(
  settings: AnkyNotificationSettings,
): Promise<void> {
  await SecureStore.setItemAsync(NOTIFICATION_SETTINGS_KEY, JSON.stringify(settings));
}

export async function enableDailyWritingReminder({
  hour,
  minute,
  preset,
}: {
  hour: number;
  minute: number;
  preset: NotificationPreset;
}): Promise<AnkyNotificationSettings> {
  const current = await loadNotificationSettings();

  if (current.scheduledId != null) {
    await Notifications.cancelScheduledNotificationAsync(current.scheduledId).catch(() => {});
  }

  const permission = await Notifications.getPermissionsAsync();
  const finalPermission =
    permission.granted || permission.status === "granted"
      ? permission
      : await Notifications.requestPermissionsAsync();

  if (!finalPermission.granted && finalPermission.status !== "granted") {
    throw new Error("notifications are not enabled for anky.");
  }

  if (Platform.OS === "android") {
    await Notifications.setNotificationChannelAsync(ANKY_REMINDER_CHANNEL, {
      importance: Notifications.AndroidImportance.DEFAULT,
      name: "writing reminder",
      sound: undefined,
    });
  }

  const scheduledId = await Notifications.scheduleNotificationAsync({
    content: {
      body: "write what is alive.",
      sound: false,
      title: "anky",
    },
    trigger: {
      channelId: Platform.OS === "android" ? ANKY_REMINDER_CHANNEL : undefined,
      hour,
      minute,
      type: Notifications.SchedulableTriggerInputTypes.DAILY,
    },
  });
  const nextSettings: AnkyNotificationSettings = {
    enabled: true,
    hour,
    minute,
    preset,
    scheduledId,
    version: 1,
  };

  await saveNotificationSettings(nextSettings);

  return nextSettings;
}

export async function disableWritingReminder(): Promise<AnkyNotificationSettings> {
  const current = await loadNotificationSettings();

  if (current.scheduledId != null) {
    await Notifications.cancelScheduledNotificationAsync(current.scheduledId).catch(() => {});
  }

  const nextSettings: AnkyNotificationSettings = {
    ...current,
    enabled: false,
    scheduledId: undefined,
    version: 1,
  };

  await saveNotificationSettings(nextSettings);

  return nextSettings;
}

export function getPresetTime(preset: NotificationPreset): { hour: number; minute: number } {
  switch (preset) {
    case "morning":
      return { hour: 8, minute: 0 };
    case "afternoon":
      return { hour: 14, minute: 0 };
    case "evening":
      return { hour: 20, minute: 0 };
    case "custom":
      return { hour: DEFAULT_NOTIFICATION_SETTINGS.hour, minute: 0 };
  }
}

export function formatReminderTime(hour: number, minute: number): string {
  const suffix = hour >= 12 ? "pm" : "am";
  const displayHour = hour % 12 === 0 ? 12 : hour % 12;

  return `${displayHour}:${String(minute).padStart(2, "0")} ${suffix}`;
}

function isNotificationSettings(value: unknown): value is AnkyNotificationSettings {
  if (typeof value !== "object" || value == null) {
    return false;
  }

  const settings = value as Partial<AnkyNotificationSettings>;

  return (
    settings.version === 1 &&
    typeof settings.enabled === "boolean" &&
    typeof settings.hour === "number" &&
    settings.hour >= 0 &&
    settings.hour <= 23 &&
    typeof settings.minute === "number" &&
    settings.minute >= 0 &&
    settings.minute <= 59 &&
    isNotificationPreset(settings.preset) &&
    (settings.scheduledId == null || typeof settings.scheduledId === "string")
  );
}

function isNotificationPreset(value: unknown): value is NotificationPreset {
  return value === "morning" || value === "afternoon" || value === "evening" || value === "custom";
}
