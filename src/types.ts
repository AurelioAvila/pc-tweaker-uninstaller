export type UninstallSummary = "msi" | "executable" | "manualOnly" | "none" | "invalid" | "store";

export type ConfidenceLevel = "safe" | "review" | "keep";

import type { ConfidenceReason } from "./i18n";

export interface Confidence {
  level: ConfidenceLevel;
  reasons: ConfidenceReason[];
}

export interface ProgramInfo {
  id: string;
  source: "machine64" | "machine32" | "user" | "store";
  name: string;
  version: string | null;
  publisher: string | null;
  installDate: string | null;
  estimatedSizeKb: number | null;
  installLocation: string | null;
  uninstall: UninstallSummary;
  hidden: boolean;
  confidence: Confidence;
  relations: ProgramRelations;
}

export interface ProgramRelations {
  dependents: string[];
  installedVia: string | null;
  publisherSiblings: number;
}

export interface StoreApp {
  id: string;
  name: string;
  publisher: string | null;
  version: string;
  installLocation: string | null;
  installDate: string | null;
  confidence: Confidence;
  isFramework: boolean;
  isSystem: boolean;
  hidden: boolean;
}

export type SortKey = "name" | "size" | "date";
export type FilterChip = "all" | "large" | "recent";

export interface UninstallPlan {
  programName: string;
  kind: "msi" | "executable";
  command: string[];
  needsElevation: boolean;
  willAttemptRestorePoint: boolean;
  warnings: string[];
  confidence: Confidence;
  estimatedSizeKb: number | null;
}

export interface RemovalReceipt {
  ts: number;
  programName: string;
  source: string;
  method: string;
  success: boolean;
  exitCode: number | null;
  rebootRequired: boolean;
  restorePoint: string;
  estimatedSizeKb: number | null;
  verifiedFreedKb: number | null;
  message: string;
}

export type RestorePointOutcome =
  { kind: "created" } | { kind: "skipped"; reason: string } | { kind: "failed"; reason: string };

export interface UninstallReport {
  programName: string;
  command: string[];
  restorePoint: RestorePointOutcome;
  exitCode: number | null;
  success: boolean;
  rebootRequired: boolean;
  message: string;
  durationMs: number;
}

export type LoadState =
  | { phase: "loading" }
  | { phase: "error"; message: string }
  | { phase: "ready"; programs: ProgramInfo[] };

export type FlowState =
  | { step: "idle" }
  | { step: "planning"; program: ProgramInfo }
  | { step: "planError"; program: ProgramInfo; message: string }
  | { step: "confirm"; program: ProgramInfo; plan: UninstallPlan }
  | { step: "running"; program: ProgramInfo }
  | { step: "report"; program: ProgramInfo; report: UninstallReport }
  | { step: "execError"; program: ProgramInfo; message: string }
  | { step: "residueScanning"; program: ProgramInfo }
  | { step: "residue"; program: ProgramInfo; residue: ResidueReport; selected: readonly string[] }
  | { step: "residueDone"; program: ProgramInfo; result: CleanResult }
  | { step: "storeConfirm"; program: ProgramInfo }
  | { step: "storeRunning"; program: ProgramInfo }
  | { step: "storeDone"; program: ProgramInfo }
  | { step: "batchConfirm"; programs: ProgramInfo[] }
  | { step: "batchRunning"; programs: ProgramInfo[]; index: number; results: BatchItemResult[] }
  | { step: "batchDone"; results: BatchItemResult[] };

export interface BatchItemResult {
  name: string;
  success: boolean;
  message: string;
}

export interface ResidueItem {
  kind: string;
  path: string;
  sizeKb: number | null;
  deletable: boolean;
}

export interface ResidueReport {
  items: ResidueItem[];
  totalKb: number;
}

export interface CleanResult {
  removed: string[];
  failed: string[];
  freedKb: number;
}
