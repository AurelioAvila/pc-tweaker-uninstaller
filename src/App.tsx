import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { text } from "./i18n";
import "./App.css";

/**
 * Phase 1 shell: header, version badge, and an honest empty state. The
 * program list, scan, and uninstall flows land in later phases — this
 * deliberately ships no placeholder data pretending otherwise.
 */
export default function App() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<string>("app_version")
      .then((v) => {
        if (!cancelled) setVersion(v);
      })
      .catch(() => {
        // Non-fatal: the badge simply stays hidden if IPC is unavailable.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="shell">
      <header className="header">
        <h1>{text.app.title}</h1>
        {version !== null && <span className="version">v{version}</span>}
      </header>
      <p className="tagline">{text.app.tagline}</p>
      <section className="empty" aria-live="polite">
        <h2>{text.programs.emptyTitle}</h2>
        <p>{text.programs.emptyBody}</p>
      </section>
    </main>
  );
}
