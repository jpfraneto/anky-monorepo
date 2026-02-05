export const ANKY_THRESHOLD = 480;

export function formatDuration(s: number): string {
  return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, "0")}`;
}

export function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML.replace(/\n/g, "<br>");
}

export function isMobile(): boolean {
  return window.innerWidth < 768;
}

export function formatDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

export function formatTime(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });
}
