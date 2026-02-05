/**
 * Streak Utility Functions
 *
 * The key insight: a "day" doesn't end at midnight for most people.
 * If you go to sleep at 2am, writing at 1am should count as "today".
 *
 * We solve this by using a configurable "day boundary" (default: 4am).
 * A "logical day" runs from 4am to 4am the next calendar day.
 *
 * Example with 4am boundary:
 * - Writing at Jan 5, 3:00am → Logical day = Jan 4
 * - Writing at Jan 5, 5:00am → Logical day = Jan 5
 */

/**
 * Calculate the "logical date" based on the day boundary hour
 *
 * @param timestamp - The actual timestamp of the writing
 * @param dayBoundaryHour - Hour when the "day" resets (0-23), default 4 (4am)
 * @param timezone - User's timezone, default UTC
 * @returns The logical date (start of the logical day)
 */
export function getLogicalDate(
  timestamp: Date = new Date(),
  dayBoundaryHour: number = 4,
  timezone: string = 'UTC'
): Date {
  // Convert to user's timezone
  const formatter = new Intl.DateTimeFormat('en-US', {
    timeZone: timezone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    hour12: false,
  });

  const parts = formatter.formatToParts(timestamp);
  const year = parseInt(parts.find(p => p.type === 'year')?.value || '2024');
  const month = parseInt(parts.find(p => p.type === 'month')?.value || '1') - 1;
  const day = parseInt(parts.find(p => p.type === 'day')?.value || '1');
  const hour = parseInt(parts.find(p => p.type === 'hour')?.value || '0');

  // Create date in user's timezone context
  let logicalDay = new Date(year, month, day);

  // If current hour is before the boundary, it's still "yesterday"
  if (hour < dayBoundaryHour) {
    logicalDay.setDate(logicalDay.getDate() - 1);
  }

  // Return as UTC midnight of that logical day
  return new Date(Date.UTC(logicalDay.getFullYear(), logicalDay.getMonth(), logicalDay.getDate()));
}

/**
 * Check if two dates are consecutive logical days
 */
export function areConsecutiveDays(date1: Date, date2: Date): boolean {
  const d1 = new Date(date1);
  const d2 = new Date(date2);

  // Normalize to start of day
  d1.setUTCHours(0, 0, 0, 0);
  d2.setUTCHours(0, 0, 0, 0);

  const diffMs = Math.abs(d2.getTime() - d1.getTime());
  const diffDays = diffMs / (1000 * 60 * 60 * 24);

  return diffDays === 1;
}

/**
 * Check if two dates are the same logical day
 */
export function isSameDay(date1: Date, date2: Date): boolean {
  const d1 = new Date(date1);
  const d2 = new Date(date2);

  d1.setUTCHours(0, 0, 0, 0);
  d2.setUTCHours(0, 0, 0, 0);

  return d1.getTime() === d2.getTime();
}

/**
 * Calculate days since last activity (for streak checking)
 */
export function daysSince(lastDate: Date, currentDate: Date = new Date()): number {
  const last = new Date(lastDate);
  const current = new Date(currentDate);

  last.setUTCHours(0, 0, 0, 0);
  current.setUTCHours(0, 0, 0, 0);

  const diffMs = current.getTime() - last.getTime();
  return Math.floor(diffMs / (1000 * 60 * 60 * 24));
}

/**
 * Generate a short, URL-safe share ID
 */
export function generateShareId(): string {
  const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  let result = '';
  for (let i = 0; i < 8; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
}
