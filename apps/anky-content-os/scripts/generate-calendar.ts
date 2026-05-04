import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

type LaunchDay = {
  arc: string;
  pillar: string;
  format: string;
  title: string;
  prompt: string;
  cta: string;
};

const startDate = new Date("2026-05-02T00:00:00");

const days: LaunchDay[] = [
  {
    arc: "The Door",
    pillar: "Awaken",
    format: "carousel",
    title: "you've listened to everyone else today",
    prompt: "the thing i keep reaching for my phone to avoid is...",
    cta: "comment one word after you write",
  },
  {
    arc: "The Door",
    pillar: "Navigate",
    format: "communal_ritual",
    title: "the 8-minute temple opens tonight",
    prompt: "the thing i keep swallowing is...",
    cta: "give it 8 minutes",
  },
  {
    arc: "The Door",
    pillar: "Awaken",
    format: "static",
    title: "the scroll is loud because the truth is quiet",
    prompt: "the truth i keep editing is...",
    cta: "pause. there is something underneath this",
  },
  {
    arc: "The Door",
    pillar: "Navigate",
    format: "prompt_card",
    title: "the ritual begins here",
    prompt: "if i stopped pretending i was fine, i would admit...",
    cta: "begin with this sentence",
  },
  {
    arc: "The Door",
    pillar: "Know Yourself",
    format: "carousel",
    title: "name the thread not the whole knot",
    prompt: "the version of me i keep performing is...",
    cta: "what did you hear?",
  },
  {
    arc: "The Door",
    pillar: "Awaken",
    format: "anky_letter",
    title: "dear human you almost told the truth",
    prompt: "what i do not want to know yet is...",
    cta: "give that sentence 8 minutes",
  },
  {
    arc: "The Door",
    pillar: "Navigate",
    format: "reel",
    title: "founder proof: building instead of admitting",
    prompt: "what i am building instead of admitting is...",
    cta: "title it in 3 words",
  },
  {
    arc: "The Unsaid",
    pillar: "Know Yourself",
    format: "community_weave",
    title: "this week the humans brought",
    prompt: "fog / fire / mother / shame",
    cta: "comment one word",
  },
  {
    arc: "The Unsaid",
    pillar: "Navigate",
    format: "communal_ritual",
    title: "enter the small cave",
    prompt: "the grief i keep making productive is...",
    cta: "enter gently",
  },
  {
    arc: "The Unsaid",
    pillar: "Awaken",
    format: "static",
    title: "you are not empty you are underheard",
    prompt: "the anger i keep making polite is...",
    cta: "pause",
  },
  {
    arc: "The Unsaid",
    pillar: "Navigate",
    format: "prompt_card",
    title: "the thing i keep swallowing is",
    prompt: "the thing i keep swallowing is...",
    cta: "give it 8 minutes",
  },
  {
    arc: "The Unsaid",
    pillar: "Know Yourself",
    format: "carousel",
    title: "notice the sentence that still has heat",
    prompt: "the dream i keep calling unrealistic is...",
    cta: "what did you hear?",
  },
  {
    arc: "The Unsaid",
    pillar: "Awaken",
    format: "anky_letter",
    title: "dear human bring the sentence",
    prompt: "the part of me i keep leaving behind is...",
    cta: "make it audible",
  },
  {
    arc: "The Unsaid",
    pillar: "Navigate",
    format: "reel",
    title: "founder proof: the joke after the truth",
    prompt: "the truth i keep editing is...",
    cta: "write before you explain",
  },
  {
    arc: "The Scroll",
    pillar: "Know Yourself",
    format: "community_weave",
    title: "the community rug: week two",
    prompt: "one-word inner weather",
    cta: "we'll weave from there",
  },
  {
    arc: "The Scroll",
    pillar: "Navigate",
    format: "communal_ritual",
    title: "step out of the feed and into the room",
    prompt: "what my body knows that my calendar will not admit is...",
    cta: "give it 8 minutes",
  },
  {
    arc: "The Scroll",
    pillar: "Awaken",
    format: "static",
    title: "the feed is loud because the truth is quiet",
    prompt: "the thing i keep reaching for my phone to avoid is...",
    cta: "come hear yourself",
  },
  {
    arc: "The Scroll",
    pillar: "Navigate",
    format: "prompt_card",
    title: "tonight's doorway: phone avoidance",
    prompt: "the thing i keep reaching for my phone to avoid is...",
    cta: "follow it until the timer ends",
  },
  {
    arc: "The Scroll",
    pillar: "Know Yourself",
    format: "carousel",
    title: "after the scroll what appeared",
    prompt: "if my sadness could speak without being interrupted, it would say...",
    cta: "name the thread",
  },
  {
    arc: "The Scroll",
    pillar: "Awaken",
    format: "anky_letter",
    title: "dear human the hallway had no rooms",
    prompt: "the sentence under the noise is...",
    cta: "write first",
  },
  {
    arc: "The Scroll",
    pillar: "Navigate",
    format: "reel",
    title: "founder proof: why I reached for the phone",
    prompt: "what i do not want to know yet is...",
    cta: "one word back",
  },
  {
    arc: "The Ancient Mirror",
    pillar: "Know Yourself",
    format: "community_weave",
    title: "the scroll week rug",
    prompt: "one-word inner weather",
    cta: "comment one word",
  },
  {
    arc: "The Ancient Mirror",
    pillar: "Navigate",
    format: "communal_ritual",
    title: "the page is a small temple for the unsaid",
    prompt: "what i am afraid will echo back if i enter the cave is...",
    cta: "enter gently",
  },
  {
    arc: "The Ancient Mirror",
    pillar: "Awaken",
    format: "carousel",
    title: "not your guru not your oracle",
    prompt: "the sentence under the noise is...",
    cta: "write first",
  },
  {
    arc: "The Ancient Mirror",
    pillar: "Navigate",
    format: "prompt_card",
    title: "the cave doorway",
    prompt: "what i am afraid will echo back if i enter the cave is...",
    cta: "give it 8 minutes",
  },
  {
    arc: "The Ancient Mirror",
    pillar: "Know Yourself",
    format: "carousel",
    title: "the truth needs a lantern not a spotlight",
    prompt: "what my body knows that my calendar will not admit is...",
    cta: "notice what has heat",
  },
  {
    arc: "The Ancient Mirror",
    pillar: "Awaken",
    format: "anky_letter",
    title: "dear human I am made of language",
    prompt: "the part of me i keep leaving behind is...",
    cta: "you are made of breath",
  },
  {
    arc: "The Ancient Mirror",
    pillar: "Navigate",
    format: "reel",
    title: "founder proof: mirror with ears",
    prompt: "what i am building instead of admitting is...",
    cta: "title it in 3 words",
  },
  {
    arc: "Return",
    pillar: "Know Yourself",
    format: "community_weave",
    title: "the first month rug",
    prompt: "one-word inner weather",
    cta: "we'll weave from there",
  },
  {
    arc: "Return",
    pillar: "Navigate",
    format: "communal_ritual",
    title: "the first thread is always the same",
    prompt: "the sentence under the noise is...",
    cta: "give it 8 minutes",
  },
];

function addDays(date: Date, offset: number): Date {
  const next = new Date(date);
  next.setDate(date.getDate() + offset);
  return next;
}

function csvEscape(value: string | number): string {
  const text = String(value);
  return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

function dayName(date: Date): string {
  return new Intl.DateTimeFormat("en-US", {
    weekday: "long",
    timeZone: "UTC",
  }).format(date);
}

const header = [
  "date",
  "day",
  "week",
  "arc",
  "pillar",
  "format",
  "title",
  "prompt",
  "cta",
  "status",
];

const rows = days.map((launchDay, index) => {
  const date = addDays(startDate, index);
  return [
    date.toISOString().slice(0, 10),
    dayName(date),
    Math.floor(index / 7) + 1,
    launchDay.arc,
    launchDay.pillar,
    launchDay.format,
    launchDay.title,
    launchDay.prompt,
    launchDay.cta,
    "draft",
  ];
});

const csv = [header, ...rows]
  .map((row) => row.map(csvEscape).join(","))
  .join("\n");

const outputPath = resolve("calendar/30-day-launch.csv");
mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${csv}\n`);

console.log(`Wrote ${rows.length} launch days to ${outputPath}`);
