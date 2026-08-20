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

  return (
    <main className="shell">
      <header className="header">
        <h1>{text.app.title}</h1>
        {version !== null && <span className="version">v{version}</span>}
      </header>
      <p className="tagline">{text.app.tagline}</p>

      {state.phase === "loading" && (
        <p className="status" role="status">
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
            <span className="count" role="status">
              {text.programs.countSummary(filtered.length, state.programs.length)}
            </span>
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
            <table className="programs">
              <thead>
                <tr>
                  <th scope="col">{text.programs.columnProgram}</th>
                  <th scope="col">{text.programs.columnVersion}</th>
                  <th scope="col">{text.programs.columnPublisher}</th>
                  <th scope="col" className="num">
                    {text.programs.columnSize}
                  </th>
                  <th scope="col">{text.programs.columnInstalled}</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((p) => (
                  <tr key={`${p.source}:${p.id}`}>
                    <td>
                      <span className="name">{p.name}</span>
                      <span
                        className={`badge badge-${p.uninstall}`}
                        title={BADGE_HINT[p.uninstall]}
                      >
                        {BADGE_LABEL[p.uninstall]}
                      </span>
                      {p.source === "user" && (
                        <span className="badge badge-user" title={text.programs.badgeUserHint}>
                          {text.programs.badgeUser}
                        </span>
                      )}
                    </td>
                    <td className="dim">{p.version ?? "—"}</td>
                    <td className="dim">{p.publisher ?? "—"}</td>
                    <td className="dim num">{formatSize(p.estimatedSizeKb)}</td>
                    <td className="dim">{p.installDate ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </>
      )}
    </main>
  );
}
