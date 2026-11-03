/**
 * The leftover scan as the webview sees it: the shapes `residue.rs` sends,
 * and the pure rules for grouping, default selection and what each plan may
 * clean. No path ever goes back to Rust, only the scan's token and item ids.
 */

export type LeftoverKind =
  | "install-dir"
  | "app-data"
  | "shortcut"
  | "registry-user"
  | "registry-machine"
  | "startup-run"
  | "startup-shortcut";
export type Tier = "safe" | "review" | "keep";
export type Evidence = "path" | "dangling" | "name";
export type Blocker =
  | "windowsFolder"
  | "protectedFolder"
  | "otherProgram"
  | "linked"
  | "notRestorable"
  | "wellKnownKey";
export type ScanMode = "registry" | "manual" | "sweep";
export type RestoreKind = "recycle" | "backup" | "none";

/** Mirror of `ResidueItem` (src-tauri/src/residue.rs). */
export interface LeftoverItem {
  id: string;
  kind: LeftoverKind;
  scope: "user" | "machine";
  path: string;
  label: string;
  sizeKb: number | null;
  deletable: boolean;
  tier: Tier;
  evidence: Evidence;
  blocker: Blocker | null;
  command: string | null;
  missing: string | null;
  restore: RestoreKind;
}

export interface LeftoverReport {
  token: string;
  mode: ScanMode;
  items: LeftoverItem[];
  totalKb: number;
}

export interface CleanOutcome {
  removed: string[];
  failed: string[];
  locked: string[];
  freedKb: number;
  /** The backup id Undo takes; null when nothing was cleaned. */
  stamp: string | null;
  /** A `RestorePointOutcome`, requested before items of all users changed. */
  restorePoint: { kind: string } | null;
  /** Why the items of all users were not cleaned (English, backend-copy). */
  adminRefused: string | null;
}

export interface PlanStep {
  id: string;
  /** "backup": backed up byte for byte, then deleted. */
  step: "recycle" | "backup";
  proOnly: boolean;
  locked: boolean;
  refused: boolean;
  /** An item of all users, cleaned in the one administrator step. */
  admin: boolean;
}

/** Mirror of `CleanPlan`: the dry run, and the fingerprint the cleanup
 *  must come back with. */
export interface CleanPlan {
  steps: PlanStep[];
  fingerprint: string;
  needsAdmin: boolean;
  restorePoint: { kind: "off" } | { kind: "reuse"; at: number } | { kind: "new" } | null;
}

export type UndoOutcome = "restored" | "conflict" | "refused" | "missing" | "failed";

/** Mirror of `UndoReport` (src-tauri/src/residue_elevated.rs). */
export interface UndoReport {
  outcomes: { id: string; outcome: UndoOutcome }[];
  refused: string | null;
}

/** Put back, left alone because something is there again, and the rest. */
export function undoSummary(report: UndoReport): {
  restored: number;
  kept: number;
  failed: number;
} {
  const count = (pick: (o: UndoOutcome) => boolean) =>
    report.outcomes.filter((o) => pick(o.outcome)).length;
  return {
    restored: count((o) => o === "restored"),
    kept: count((o) => o === "conflict"),
    failed: count((o) => o !== "restored" && o !== "conflict"),
  };
}

/** Mirror of `RemovedView` (src-tauri/src/removed_programs.rs). */
export interface RemovedProgramRow {
  rid: string;
  name: string;
  publisher: string | null;
  removedAt: number;
  how: "app" | "outside";
  seen: boolean;
}

export interface RemovedOverview {
  records: RemovedProgramRow[];
  startupCard: number | null;
}

export type GroupId = "files" | "startup" | "registry" | "kept";
export const GROUP_ORDER: readonly GroupId[] = ["files", "startup", "registry", "kept"];

/** Free cleans files and folders; everything else is Pro (decided in
 *  `residue_rules.rs`, enforced again in `clean_residue`). */
export function isProOnly(kind: LeftoverKind): boolean {
  return kind !== "install-dir" && kind !== "app-data" && kind !== "shortcut";
}

export function groupOf(item: LeftoverItem): GroupId {
  if (item.tier === "keep") return "kept";
  if (item.kind === "startup-run" || item.kind === "startup-shortcut") return "startup";
  if (item.kind === "registry-user" || item.kind === "registry-machine") return "registry";
  return "files";
}

/** Non-empty groups in display order. */
export function groupItems(
  items: readonly LeftoverItem[],
): { id: GroupId; items: LeftoverItem[] }[] {
  return GROUP_ORDER.map((id) => ({
    id,
    items: items.filter((item) => groupOf(item) === id),
  })).filter((group) => group.items.length > 0);
}

/** Whether the checkbox can be ticked: cleanable, never Keep, and Pro-only
 *  kinds only with Pro. */
export function isSelectable(item: LeftoverItem, pro: boolean): boolean {
  return item.tier !== "keep" && item.deletable && (pro || !isProOnly(item.kind));
}

/** Only items proven to come from the removed program are pre-selected. A
 *  manual or startup-sweep scan, and any name match, starts unselected. */
export function defaultSelection(report: LeftoverReport, pro: boolean): string[] {
  if (report.mode !== "registry") return [];
  return report.items
    .filter((item) => item.tier === "safe" && isSelectable(item, pro))
    .map((item) => item.id);
}

/** "Select all to review" for one group: the Review items in it. */
export function reviewIds(items: readonly LeftoverItem[], pro: boolean): string[] {
  return items.filter((item) => item.tier === "review" && isSelectable(item, pro)).map((i) => i.id);
}

/** The line above the groups: how many, how big, how many still start with
 *  Windows. Keep items are not counted. */
export function summarize(items: readonly LeftoverItem[]): {
  count: number;
  sizeKb: number;
  startup: number;
} {
  const open = items.filter((item) => item.tier !== "keep");
  return {
    count: open.length,
    sizeKb: open.reduce((sum, item) => sum + Math.max(0, item.sizeKb ?? 0), 0),
    startup: open.filter((item) => groupOf(item) === "startup").length,
  };
}
