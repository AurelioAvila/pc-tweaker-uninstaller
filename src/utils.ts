import { text } from "./i18n";
import type {
  Confidence,
  ConfidenceLevel,
  ProgramInfo,
  RestorePointOutcome,
  SortKey,
  StoreApp,
  UninstallSummary,
} from "./types";

export const LARGE_KB = 512000;
export const RECENT_DAYS = 30;

export function storeAppAsProgram(app: StoreApp): ProgramInfo {
  return {
    id: app.id,
    source: "store",
    name: app.name,
    version: app.version || null,
    publisher: app.publisher,
    installDate: app.installDate,
    estimatedSizeKb: null,
    installLocation: app.installLocation,
    uninstall: "store",
    hidden: app.hidden,
    confidence: app.confidence,
    relations: { dependents: [], installedVia: null, publisherSiblings: 0 },
  };
}

export function isBatchable(p: ProgramInfo): boolean {
  return (
    (p.uninstall === "msi" || p.uninstall === "executable") &&
    p.confidence.level !== "keep" &&
    !isFamilyApp(p)
  );
}

export function batchOrder(programs: ProgramInfo[]): ProgramInfo[] {
  return [...programs].sort(
    (a, b) => (b.installLocation ?? "").length - (a.installLocation ?? "").length,
  );
}

export function formatSize(kb: number | null): string {
  if (kb === null || kb <= 0) return "—";
  if (kb < 1024) return `${String(kb)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(mb < 10 ? 1 : 0)} MB`;
  return `${(mb / 1024).toFixed(2)} GB`;
}

export function hueOf(name: string): number {
  let hash = 0;
  for (const ch of name) hash = (hash * 31 + (ch.codePointAt(0) ?? 0)) | 0;
  return Math.abs(hash) % 360;
}

export function monogram(name: string): string {
  const codePoint = name.trim().codePointAt(0);
  return codePoint === undefined ? "?" : String.fromCodePoint(codePoint).toUpperCase();
}

export function displayCommand(argv: string[]): string {
  return argv.map((token) => (token.includes(" ") ? `"${token}"` : token)).join(" ");
}

export function isFamilyApp(p: ProgramInfo): boolean {
  const name = p.name.toLowerCase().replace(/[-_]/g, " ");
  return name.startsWith("pc tweaker") || name.startsWith("promptshield");
}

export function isRecent(installDate: string | null, now: Date): boolean {
  if (installDate === null) return false;
  const then = new Date(`${installDate}T00:00:00`);
  if (Number.isNaN(then.getTime())) return false;
  return (now.getTime() - then.getTime()) / 86400000 <= RECENT_DAYS;
}

export function compareBy(a: ProgramInfo, b: ProgramInfo, key: SortKey): number {
  switch (key) {
    case "name":
      return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
    case "size":
      return (a.estimatedSizeKb ?? -1) - (b.estimatedSizeKb ?? -1);
    case "date":
      return (a.installDate ?? "").localeCompare(b.installDate ?? "");
  }
}

export function sourceLabel(source: ProgramInfo["source"]): string {
  switch (source) {
    case "machine64":
      return text.programs.sourceMachine64;
    case "machine32":
      return text.programs.sourceMachine32;
    case "user":
      return text.programs.sourceUser;
    case "store":
      return text.programs.sourceStore;
  }
}

export function confidenceLabel(level: ConfidenceLevel): string {
  switch (level) {
    case "safe":
      return text.confidence.labelSafe;
    case "review":
      return text.confidence.labelReview;
    case "keep":
      return text.confidence.labelKeep;
  }
}

export function confidenceTitle(c: Confidence): string {
  const lines = c.reasons.map((r) => text.confidence.reasons[r]);
  return [...lines, text.confidence.disclaimer].join("\n");
}

export function restorePointLine(outcome: RestorePointOutcome): string {
  switch (outcome.kind) {
    case "created":
      return text.uninstall.restorePointCreated;
    case "skipped":
      return text.uninstall.restorePointSkipped(outcome.reason);
    case "failed":
      return text.uninstall.restorePointFailed(outcome.reason);
  }
}

export function badgeLabel(summary: UninstallSummary): string {
  switch (summary) {
    case "msi":
      return text.programs.badgeMsi;
    case "executable":
      return text.programs.badgeExecutable;
    case "manualOnly":
      return text.programs.badgeManualOnly;
    case "none":
      return text.programs.badgeNone;
    case "invalid":
      return text.programs.badgeInvalid;
    case "store":
      return text.programs.badgeStore;
  }
}

export function badgeHint(summary: UninstallSummary): string | undefined {
  switch (summary) {
    case "manualOnly":
      return text.programs.badgeManualOnlyHint;
    case "none":
      return text.programs.badgeNoneHint;
    case "invalid":
      return text.programs.badgeInvalidHint;
    case "store":
      return text.programs.badgeStoreHint;
    default:
      return undefined;
  }
}
