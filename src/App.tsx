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

type LoadState =
  | { phase: "loading" }
  | { phase: "error"; message: string }
  | { phase: "ready"; programs: ProgramInfo[] };

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
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </main>
    </div>
  );
}
