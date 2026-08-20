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
  hidden: boolean;
}

type SortKey = "name" | "size" | "date";
type FilterChip = "all" | "large" | "recent";

/** 500 MB — the "what is eating my disk" threshold for the Large filter. */
const LARGE_KB = 512000;
/** The Recent filter window, in days. */
const RECENT_DAYS = 30;

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

/** Sibling products get a "Suite" mark: the uninstaller recognizes its own
 *  family so a user never removes a suite piece by accident. */
function isFamilyApp(p: ProgramInfo): boolean {
  const name = p.name.toLowerCase();
  return name.startsWith("pc tweaker") || name.startsWith("promptshield");
}

function isRecent(installDate: string | null, now: Date): boolean {
  if (installDate === null) return false;
  const then = new Date(`${installDate}T00:00:00`);
  if (Number.isNaN(then.getTime())) return false;
  return (now.getTime() - then.getTime()) / 86400000 <= RECENT_DAYS;
}

function compareBy(a: ProgramInfo, b: ProgramInfo, key: SortKey): number {
  switch (key) {
    case "name":
      return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
    case "size":
      return (a.estimatedSizeKb ?? -1) - (b.estimatedSizeKb ?? -1);
    case "date":
      return (a.installDate ?? "").localeCompare(b.installDate ?? "");
  }
}

const SOURCE_LABEL: Record<ProgramInfo["source"], string> = {
  machine64: text.programs.sourceMachine64,
  machine32: text.programs.sourceMachine32,
  user: text.programs.sourceUser,
};

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
  const [chip, setChip] = useState<FilterChip>("all");
  const [showHidden, setShowHidden] = useState(false);
  const [sortKey, setSortKey] = useState<SortKey>("name");
  const [sortAsc, setSortAsc] = useState(true);
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  const [detailNotice, setDetailNotice] = useState<string | null>(null);

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

  const visible = useMemo(
    () => (state.phase === "ready" ? state.programs.filter((p) => showHidden || !p.hidden) : []),
    [state, showHidden],
  );

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const now = new Date();
    const matched = visible.filter((p) => {
      if (
        needle &&
        !p.name.toLowerCase().includes(needle) &&
        !(p.publisher ?? "").toLowerCase().includes(needle)
      ) {
        return false;
      }
      if (chip === "large") return (p.estimatedSizeKb ?? 0) >= LARGE_KB;
      if (chip === "recent") return isRecent(p.installDate, now);
      return true;
    });
    const dir = sortAsc ? 1 : -1;
    return [...matched].sort((a, b) => dir * compareBy(a, b, sortKey));
  }, [visible, query, chip, sortKey, sortAsc]);

  const toggleSort = useCallback(
    (key: SortKey) => {
      if (key === sortKey) {
        setSortAsc((asc) => !asc);
      } else {
        setSortKey(key);
        // Size and date read most naturally biggest/newest first.
        setSortAsc(key === "name");
      }
    },
    [sortKey],
  );

  const toggleExpanded = useCallback((key: string) => {
    setDetailNotice(null);
    setExpandedKey((current) => (current === key ? null : key));
  }, []);

  const openFolder = useCallback((program: ProgramInfo) => {
    setDetailNotice(null);
    invoke("open_install_folder", { source: program.source, id: program.id }).catch(
      (error: unknown) => {
        setDetailNotice(typeof error === "string" ? error : text.errors.generic);
      },
    );
  }, []);

  const openLink = useCallback((target: string) => {
    invoke("open_ecosystem_link", { target }).catch(() => {
      // Non-fatal: the site link simply not opening is visible on its own.
    });
  }, []);

  const openRestoreUi = useCallback(() => {
    invoke("open_system_restore").catch(() => {
      // Non-fatal, same reasoning as openLink.
    });
  }, []);

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
    () => visible.reduce((sum, p) => sum + (p.estimatedSizeKb ?? 0), 0),
    [visible],
  );

  // The strategic cross-link: when the flagship is installed on this PC, the
  // uninstaller greets its owner as a suite member. Detection is local (the
  // program list we already read) — no network, no account required.
  const suiteDetected = useMemo(
    () =>
      state.phase === "ready" &&
      state.programs.some((p) => {
        const name = p.name.toLowerCase();
        return name.startsWith("pc tweaker") && !name.includes("uninstaller");
      }),
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
        <div className="topbar-right">
          {suiteDetected && (
            <button
              type="button"
              className="suite-pill"
              title={text.app.suiteDetectedHint}
              onClick={() => {
                openLink("pctweaker");
              }}
            >
              <span className="suite-dot" aria-hidden="true" />
              {text.app.suiteDetected}
            </button>
          )}
          {version !== null && <span className="version">v{version}</span>}
        </div>
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
              <div className="chip-group" role="group" aria-label={text.programs.filterAll}>
                {(
                  [
                    ["all", text.programs.filterAll],
                    ["large", text.programs.filterLarge],
                    ["recent", text.programs.filterRecent],
                  ] as const
                ).map(([value, label]) => (
                  <button
                    key={value}
                    type="button"
                    className={`chip chip-button${chip === value ? " chip-active" : ""}`}
                    aria-pressed={chip === value}
                    onClick={() => {
                      setChip(value);
                    }}
                  >
                    {label}
                  </button>
                ))}
                <button
                  type="button"
                  className={`chip chip-button${showHidden ? " chip-active" : ""}`}
                  aria-pressed={showHidden}
                  title={text.programs.badgeHiddenHint}
                  onClick={() => {
                    setShowHidden((v) => !v);
                  }}
                >
                  {text.programs.showHidden}
                </button>
              </div>
              <span className="chip" role="status">
                {text.programs.countSummary(filtered.length, visible.length)}
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
                  <span
                    role="columnheader"
                    aria-sort={sortKey === "name" ? (sortAsc ? "ascending" : "descending") : "none"}
                  >
                    <button
                      type="button"
                      className="sort-button"
                      onClick={() => {
                        toggleSort("name");
                      }}
                    >
                      {text.programs.columnProgram}
                      {sortKey === "name" && <span aria-hidden="true">{sortAsc ? "▲" : "▼"}</span>}
                    </button>
                  </span>
                  <span role="columnheader">{text.programs.columnVersion}</span>
                  <span
                    role="columnheader"
                    className="num"
                    aria-sort={sortKey === "size" ? (sortAsc ? "ascending" : "descending") : "none"}
                  >
                    <button
                      type="button"
                      className="sort-button"
                      onClick={() => {
                        toggleSort("size");
                      }}
                    >
                      {text.programs.columnSize}
                      {sortKey === "size" && <span aria-hidden="true">{sortAsc ? "▲" : "▼"}</span>}
                    </button>
                  </span>
                  <span
                    role="columnheader"
                    aria-sort={sortKey === "date" ? (sortAsc ? "ascending" : "descending") : "none"}
                  >
                    <button
                      type="button"
                      className="sort-button"
                      onClick={() => {
                        toggleSort("date");
                      }}
                    >
                      {text.programs.columnInstalled}
                      {sortKey === "date" && <span aria-hidden="true">{sortAsc ? "▲" : "▼"}</span>}
                    </button>
                  </span>
                  <span role="columnheader" className="num" />
                  <span role="columnheader" className="num" />
                </div>
                {filtered.map((p) => {
                  const rowKey = `${p.source}:${p.id}`;
                  const expanded = expandedKey === rowKey;
                  return (
                    <div key={rowKey} className={expanded ? "row-group expanded" : "row-group"}>
                      <div
                        className="row"
                        role="row"
                        aria-expanded={expanded}
                        onClick={() => {
                          toggleExpanded(rowKey);
                        }}
                      >
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
                          {isFamilyApp(p) && (
                            <span
                              className="badge badge-suite"
                              title={text.programs.badgeSuiteHint}
                            >
                              {text.programs.badgeSuite}
                            </span>
                          )}
                          {p.hidden && (
                            <span
                              className="badge badge-hidden"
                              title={text.programs.badgeHiddenHint}
                            >
                              {text.programs.badgeHidden}
                            </span>
                          )}
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
                              onClick={(e) => {
                                e.stopPropagation();
                                beginUninstall(p);
                              }}
                            >
                              {text.uninstall.action}
                            </button>
                          )}
                        </span>
                      </div>
                      {expanded && (
                        <div className="row-details" role="row">
                          <div role="cell" className="details-grid">
                            <div>
                              <span className="detail-label">{text.programs.detailSource}</span>
                              <span className="detail-value">{SOURCE_LABEL[p.source]}</span>
                            </div>
                            <div>
                              <span className="detail-label">{text.programs.detailKey}</span>
                              <span className="detail-value mono">{p.id}</span>
                            </div>
                            <div className="detail-wide">
                              <span className="detail-label">{text.programs.detailLocation}</span>
                              <span className="detail-value mono">
                                {p.installLocation ?? text.programs.detailNoLocation}
                              </span>
                            </div>
                            <div className="detail-actions">
                              {p.installLocation !== null && (
                                <button
                                  type="button"
                                  className="button-ghost small"
                                  onClick={() => {
                                    openFolder(p);
                                  }}
                                >
                                  {text.programs.openFolder}
                                </button>
                              )}
                            </div>
                            {detailNotice !== null && (
                              <p className="detail-notice" role="alert">
                                {detailNotice}
                              </p>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </>
        )}
      </main>

      <footer className="footbar">
        <div className="footbar-left">
          <span className="footbar-family">{text.footer.family}</span>
          <button
            type="button"
            className="footbar-link"
            onClick={() => {
              openLink("pctweaker");
            }}
          >
            {text.footer.pcTweaker}
          </button>
          <span className="footbar-sep" aria-hidden="true">
            ·
          </span>
          <button
            type="button"
            className="footbar-link"
            onClick={() => {
              openLink("promptshield");
            }}
          >
            {text.footer.promptShield}
          </button>
          <span className="footbar-sep" aria-hidden="true">
            ·
          </span>
          <button
            type="button"
            className="footbar-link"
            onClick={() => {
              openLink("privacy");
            }}
          >
            {text.footer.privacy}
          </button>
        </div>
        <div className="footbar-right">
          <button
            type="button"
            className="footbar-link accent"
            title={text.footer.restoreInfo}
            onClick={openRestoreUi}
          >
            {text.footer.openRestore}
          </button>
        </div>
      </footer>

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
                  {isFamilyApp(flow.program) && (
                    <li className="note-suite">{text.uninstall.familyNote}</li>
                  )}
                  {flow.program.hidden && (
                    <li className="note-warn">{text.uninstall.hiddenNote}</li>
                  )}
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
