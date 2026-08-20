import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { text } from "./i18n";
import "./App.css";

/** Mirror of the Rust `ProgramInfo` shape (src-tauri/src/programs.rs). Every
 *  string here originates in the registry and is untrusted display data —
 *  rendered exclusively as text, never as markup. */
type UninstallSummary = "msi" | "executable" | "manualOnly" | "none" | "invalid";

interface ProgramInfo {
  id: string;
  source: "machine64" | "machine32" | "user";
  name: string;
  version: string | null;
  publisher: string | null;
  installDate: string | null;
  estimatedSizeKb: number | null;
  installLocation: string | null;
  uninstall: UninstallSummary;
}

/** Mirror of `UninstallPlan` (src-tauri/src/uninstall_exec.rs). Display only:
 *  the backend re-derives everything at execution time. */
interface UninstallPlan {
  programName: string;
  kind: "msi" | "executable";
  command: string[];
  needsElevation: boolean;
  willAttemptRestorePoint: boolean;
  warnings: string[];
}

type RestorePointOutcome =
  { kind: "created" } | { kind: "skipped"; reason: string } | { kind: "failed"; reason: string };

interface UninstallReport {
  programName: string;
  command: string[];
  restorePoint: RestorePointOutcome;
  exitCode: number | null;
  success: boolean;
  rebootRequired: boolean;
  message: string;
  durationMs: number;
}

type LoadState =
  | { phase: "loading" }
  | { phase: "error"; message: string }
  | { phase: "ready"; programs: ProgramInfo[] };

/** The uninstall flow, one program at a time (the backend enforces the same
 *  invariant with a lock). */
type FlowState =
  | { step: "idle" }
  | { step: "planning"; program: ProgramInfo }
  | { step: "planError"; program: ProgramInfo; message: string }
  | { step: "confirm"; program: ProgramInfo; plan: UninstallPlan }
  | { step: "running"; program: ProgramInfo }
  | { step: "report"; program: ProgramInfo; report: UninstallReport }
  | { step: "execError"; program: ProgramInfo; message: string };

function formatSize(kb: number | null): string {
  if (kb === null || kb <= 0) return "—";
  if (kb < 1024) return `${String(kb)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(mb < 10 ? 1 : 0)} MB`;
  return `${(mb / 1024).toFixed(2)} GB`;
}

/** Deterministic hue from the program name, so each monogram keeps a stable
 *  color across launches without storing anything. */
function hueOf(name: string): number {
  let hash = 0;
  for (const ch of name) hash = (hash * 31 + (ch.codePointAt(0) ?? 0)) | 0;
  return Math.abs(hash) % 360;
}

function monogram(name: string): string {
  const codePoint = name.trim().codePointAt(0);
  return codePoint === undefined ? "?" : String.fromCodePoint(codePoint).toUpperCase();
}

/** Renders an argv for humans: tokens with spaces get quotes back. */
function displayCommand(argv: string[]): string {
  return argv.map((token) => (token.includes(" ") ? `"${token}"` : token)).join(" ");
}

function restorePointLine(outcome: RestorePointOutcome): string {
  switch (outcome.kind) {
    case "created":
      return text.uninstall.restorePointCreated;
    case "skipped":
      return text.uninstall.restorePointSkipped(outcome.reason);
    case "failed":
      return text.uninstall.restorePointFailed(outcome.reason);
  }
}

const BADGE_LABEL: Record<UninstallSummary, string> = {
  msi: text.programs.badgeMsi,
  executable: text.programs.badgeExecutable,
  manualOnly: text.programs.badgeManualOnly,
  none: text.programs.badgeNone,
  invalid: text.programs.badgeInvalid,
};

const BADGE_HINT: Partial<Record<UninstallSummary, string>> = {
  manualOnly: text.programs.badgeManualOnlyHint,
  none: text.programs.badgeNoneHint,
  invalid: text.programs.badgeInvalidHint,
};

export default function App() {
  const [version, setVersion] = useState<string | null>(null);
  const [state, setState] = useState<LoadState>({ phase: "loading" });
  const [query, setQuery] = useState("");
  const [flow, setFlow] = useState<FlowState>({ step: "idle" });

  const load = useCallback(() => {
    setState({ phase: "loading" });
    invoke<ProgramInfo[]>("list_programs")
      .then((programs) => {
        setState({ phase: "ready", programs });
      })
      .catch((error: unknown) => {
        setState({
          phase: "error",
          message: typeof error === "string" ? error : text.errors.generic,
        });
      });
  }, []);

  useEffect(() => {
    let cancelled = false;
    invoke<string>("app_version")
      .then((v) => {
        if (!cancelled) setVersion(v);
      })
      .catch(() => {
        // Non-fatal: the badge simply stays hidden if IPC is unavailable.
      });
    load();
    return () => {
      cancelled = true;
    };
  }, [load]);

  const filtered = useMemo(() => {
    if (state.phase !== "ready") return [];
    const needle = query.trim().toLowerCase();
    if (!needle) return state.programs;
    return state.programs.filter(
      (p) =>
        p.name.toLowerCase().includes(needle) || (p.publisher ?? "").toLowerCase().includes(needle),
    );
  }, [state, query]);

  const beginUninstall = useCallback((program: ProgramInfo) => {
    setFlow({ step: "planning", program });
    invoke<UninstallPlan>("plan_uninstall", { source: program.source, id: program.id })
      .then((plan) => {
        setFlow({ step: "confirm", program, plan });
      })
      .catch((error: unknown) => {
        setFlow({
          step: "planError",
          program,
          message: typeof error === "string" ? error : text.errors.generic,
        });
      });
  }, []);

  const confirmUninstall = useCallback(
    (program: ProgramInfo) => {
      setFlow({ step: "running", program });
      invoke<UninstallReport>("execute_uninstall", { source: program.source, id: program.id })
        .then((report) => {
          setFlow({ step: "report", program, report });
          load(); // The registry changed (or should have): refresh honestly.
        })
        .catch((error: unknown) => {
          setFlow({
            step: "execError",
            program,
            message: typeof error === "string" ? error : text.errors.generic,
          });
          load();
        });
    },
    [load],
  );

  const closeFlow = useCallback(() => {
    setFlow({ step: "idle" });
  }, []);

  // Esc closes the dialog in every step where closing is meaningful. While
  // an uninstall is actually running there is nothing to cancel from here —
  // the child process owns the action.
  const flowStep = flow.step;
  useEffect(() => {
    if (flowStep === "idle" || flowStep === "planning" || flowStep === "running") return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closeFlow();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [flowStep, closeFlow]);

  const totalSizeKb = useMemo(
    () =>
      state.phase === "ready"
        ? state.programs.reduce((sum, p) => sum + (p.estimatedSizeKb ?? 0), 0)
        : 0,
    [state],
  );

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            PU
          </span>
          <div>
            <h1>{text.app.title}</h1>
            <p className="tagline">{text.app.tagline}</p>
          </div>
        </div>
        {version !== null && <span className="version">v{version}</span>}
      </header>

      <main className="content">
        {state.phase === "loading" && (
          <p className="status" role="status">
            <span className="spinner" aria-hidden="true" />
            {text.programs.loading}
          </p>
        )}

        {state.phase === "error" && (
          <section className="empty" role="alert">
            <h2>{text.programs.errorTitle}</h2>
            <p>{state.message}</p>
            <button type="button" className="button" onClick={load}>
              {text.programs.retry}
            </button>
          </section>
        )}

        {state.phase === "ready" && (
          <>
            <div className="toolbar">
              <input
                type="search"
                className="search"
                placeholder={text.programs.searchPlaceholder}
                aria-label={text.programs.searchLabel}
                value={query}
                onChange={(e) => {
                  setQuery(e.target.value);
                }}
              />
              <span className="chip" role="status">
                {text.programs.countSummary(filtered.length, state.programs.length)}
              </span>
              {totalSizeKb > 0 && (
                <span className="chip chip-accent">
                  {formatSize(totalSizeKb)} {text.programs.statTotalSize}
                </span>
              )}
            </div>

            {state.programs.length === 0 && (
              <section className="empty">
                <h2>{text.programs.emptyTitle}</h2>
                <p>{text.programs.emptyBody}</p>
              </section>
            )}

            {state.programs.length > 0 && filtered.length === 0 && (
              <section className="empty">
                <h2>{text.programs.noMatchesTitle}</h2>
                <p>{text.programs.noMatchesBody}</p>
              </section>
            )}

            {filtered.length > 0 && (
              <div className="list" role="table" aria-label={text.programs.columnProgram}>
                <div className="list-head" role="row">
                  <span role="columnheader">{text.programs.columnProgram}</span>
                  <span role="columnheader">{text.programs.columnVersion}</span>
                  <span role="columnheader" className="num">
                    {text.programs.columnSize}
                  </span>
                  <span role="columnheader">{text.programs.columnInstalled}</span>
                  <span role="columnheader" className="num" />
                  <span role="columnheader" className="num" />
                </div>
                {filtered.map((p) => (
                  <div className="row" role="row" key={`${p.source}:${p.id}`}>
                    <span role="cell" className="cell-main">
                      <span
                        className="avatar"
                        aria-hidden="true"
                        style={{ background: `hsl(${String(hueOf(p.name))} 45% 26%)` }}
                      >
                        {monogram(p.name)}
                      </span>
                      <span className="titles">
                        <span className="name">{p.name}</span>
                        <span className="publisher">{p.publisher ?? " "}</span>
                      </span>
                    </span>
                    <span role="cell" className="dim">
                      {p.version ?? "—"}
                    </span>
                    <span role="cell" className="dim num">
                      {formatSize(p.estimatedSizeKb)}
                    </span>
                    <span role="cell" className="dim">
                      {p.installDate ?? "—"}
                    </span>
                    <span role="cell" className="cell-badges num">
                      {p.source === "user" && (
                        <span className="badge badge-user" title={text.programs.badgeUserHint}>
                          {text.programs.badgeUser}
                        </span>
                      )}
                      <span
                        className={`badge badge-${p.uninstall}`}
                        title={BADGE_HINT[p.uninstall]}
                      >
                        {BADGE_LABEL[p.uninstall]}
                      </span>
                    </span>
                    <span role="cell" className="cell-action num">
                      {(p.uninstall === "msi" || p.uninstall === "executable") && (
                        <button
                          type="button"
                          className="row-action"
                          disabled={flow.step !== "idle"}
                          onClick={() => {
                            beginUninstall(p);
                          }}
                        >
                          {text.uninstall.action}
                        </button>
                      )}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </main>

      {flow.step !== "idle" && (
        <div
          className="overlay"
          role="presentation"
          onClick={flow.step === "planning" || flow.step === "running" ? undefined : closeFlow}
        >
          <div
            className="dialog"
            role="dialog"
            aria-modal="true"
            onClick={(e) => {
              e.stopPropagation();
            }}
          >
            {flow.step === "planning" && (
              <p className="status" role="status">
                <span className="spinner" aria-hidden="true" />
                {text.uninstall.planning}
              </p>
            )}

            {flow.step === "planError" && (
              <>
                <h2>{text.uninstall.planFailedTitle}</h2>
                <p className="dialog-body">{flow.message}</p>
                <div className="dialog-actions">
                  <button type="button" className="button" onClick={closeFlow}>
                    {text.uninstall.close}
                  </button>
                </div>
              </>
            )}

            {flow.step === "confirm" && (
              <>
                <h2>{text.uninstall.confirmTitle(flow.plan.programName)}</h2>
                <p className="dialog-body">{text.uninstall.confirmBody}</p>
                <p className="command-label">{text.uninstall.commandLabel}</p>
                <code className="command">{displayCommand(flow.plan.command)}</code>
                <ul className="dialog-notes">
                  {flow.plan.needsElevation && <li>{text.uninstall.elevationNote}</li>}
                  {flow.plan.willAttemptRestorePoint && <li>{text.uninstall.restorePointNote}</li>}
                  {flow.plan.warnings.map((warning) => (
                    <li key={warning}>{warning}</li>
                  ))}
                </ul>
                <div className="dialog-actions">
                  <button type="button" className="button-ghost" onClick={closeFlow}>
                    {text.uninstall.cancel}
                  </button>
                  <button
                    type="button"
                    className="button button-danger"
                    onClick={() => {
                      confirmUninstall(flow.program);
                    }}
                  >
                    {text.uninstall.confirm}
                  </button>
                </div>
              </>
            )}

            {flow.step === "running" && (
              <>
                <p className="status" role="status">
                  <span className="spinner" aria-hidden="true" />
                  {text.uninstall.running(flow.program.name)}
                </p>
                <p className="dialog-body">{text.uninstall.runningNote}</p>
              </>
            )}

            {flow.step === "report" && (
              <>
                <h2>
                  {flow.report.success
                    ? text.uninstall.reportSuccessTitle
                    : text.uninstall.reportFailureTitle}
                </h2>
                <p className="dialog-body">{flow.report.message}</p>
                {flow.report.rebootRequired && (
                  <p className="dialog-body reboot">{text.uninstall.rebootNote}</p>
                )}
                <ul className="dialog-notes">
                  <li>{restorePointLine(flow.report.restorePoint)}</li>
                  {flow.report.exitCode !== null && (
                    <li>
                      {text.uninstall.exitCodeLabel}: {String(flow.report.exitCode)}
                    </li>
                  )}
                </ul>
                <div className="dialog-actions">
                  <button type="button" className="button" onClick={closeFlow}>
                    {text.uninstall.close}
                  </button>
                </div>
              </>
            )}

            {flow.step === "execError" && (
              <>
                <h2>{text.uninstall.reportFailureTitle}</h2>
                <p className="dialog-body">{flow.message}</p>
                <div className="dialog-actions">
                  <button type="button" className="button" onClick={closeFlow}>
                    {text.uninstall.close}
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
