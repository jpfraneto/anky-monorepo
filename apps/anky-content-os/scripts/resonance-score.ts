import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

type MetricRow = Record<string, string>;

const inputPath = resolve(process.argv[2] ?? "metrics/post-metrics.csv");

function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let inQuotes = false;

  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    const next = text[index + 1];

    if (char === '"' && inQuotes && next === '"') {
      field += '"';
      index += 1;
      continue;
    }

    if (char === '"') {
      inQuotes = !inQuotes;
      continue;
    }

    if (char === "," && !inQuotes) {
      row.push(field);
      field = "";
      continue;
    }

    if ((char === "\n" || char === "\r") && !inQuotes) {
      if (char === "\r" && next === "\n") {
        index += 1;
      }
      row.push(field);
      if (row.some((value) => value.length > 0)) {
        rows.push(row);
      }
      row = [];
      field = "";
      continue;
    }

    field += char;
  }

  if (field.length > 0 || row.length > 0) {
    row.push(field);
    rows.push(row);
  }

  return rows;
}

function csvEscape(value: string | number): string {
  const text = String(value);
  return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

function toNumber(value: string | undefined): number {
  const parsed = Number(value ?? "0");
  return Number.isFinite(parsed) ? parsed : 0;
}

function score(row: MetricRow): number {
  return (
    toNumber(row.saves) * 3 +
    toNumber(row.shares) * 3 +
    toNumber(row.one_word_comments) * 2 +
    toNumber(row.dms_with_stories) * 5 +
    toNumber(row.ritual_completions) * 8
  );
}

const csv = readFileSync(inputPath, "utf8");
const parsed = parseCsv(csv);
const [header, ...records] = parsed;

if (!header) {
  throw new Error(`No CSV header found in ${inputPath}`);
}

const rows = records.map((record) =>
  Object.fromEntries(header.map((column, index) => [column, record[index] ?? ""]))
);

const outputRows = rows.map((row) => ({
  ...row,
  resonance_score: String(score(row)),
}));

const outputHeader = header.includes("resonance_score")
  ? header
  : [...header, "resonance_score"];

const output = [
  outputHeader,
  ...outputRows.map((row) => outputHeader.map((column) => row[column] ?? "")),
]
  .map((row) => row.map(csvEscape).join(","))
  .join("\n");

writeFileSync(inputPath, `${output}\n`);

const ranked = [...outputRows]
  .sort((a, b) => toNumber(b.resonance_score) - toNumber(a.resonance_score))
  .slice(0, 5);

console.log(`Updated resonance scores in ${inputPath}`);
console.log("Top posts:");
for (const row of ranked) {
  console.log(`${row.resonance_score.padStart(4, " ")}  ${row.date}  ${row.post_id}  ${row.title}`);
}
