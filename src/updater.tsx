import { useEffect, useState } from "react";
import { check as checkForUpdate, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { text } from "./i18n";

/**
 * Checks GitHub once at startup for a newer signed build and, when one
 * exists, offers it in a small card. Silent on failure on purpose: dev
 * builds have no update endpoint and an offline start is not actionable.
 * Installation goes through the updater plugin's signature verification —
 * a manifest pointing at an unsigned or tampered binary is rejected before
 * anything runs. Ported from PC Tweaker's UpdateBanner.
 */
export function UpdateBanner() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [phase, setPhase] = useState<"offer" | "downloading" | "installing">("offer");
  const [percent, setPercent] = useState(0);
  const [dismissed, setDismissed] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    checkForUpdate()
      .then((u) => {
        if (alive && u) setUpdate(u);
      })
      .catch(() => {
        // Silent: see the component doc comment.
      });
    return () => {
      alive = false;
    };
  }, []);

  if (!update || dismissed) return null;

  async function install() {
    if (!update) return;
    setError(null);
    try {
      setPhase("downloading");
      let total = 0;
      let received = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? 0;
            break;
          case "Progress":
            received += event.data.chunkLength;
            if (total > 0) setPercent(Math.min(100, Math.round((received / total) * 100)));
            break;
          case "Finished":
            setPhase("installing");
            break;
        }
      });
      await relaunch();
    } catch (err) {
      setPhase("offer");
      setError(text.updater.error(String(err)));
    }
  }

  return (
    <div className="update-card" role="status">
      <div className="update-head">
        <span className="update-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none">
            <path
              d="M12 4v10m0 0 4-4m-4 4-4-4M5 19h14"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </span>
        <div>
          <p className="update-title">{text.updater.title(update.version)}</p>
          <p className="update-body">{text.updater.body}</p>
        </div>
      </div>
      {phase === "offer" ? (
        <div className="update-actions">
          <button type="button" className="primary small-pad" onClick={() => void install()}>
            {text.updater.install}
          </button>
          <button
            type="button"
            className="button-ghost small"
            onClick={() => {
              setDismissed(true);
            }}
          >
            {text.updater.later}
          </button>
        </div>
      ) : (
        <div className="update-progress">
          <p>
            {phase === "downloading" ? text.updater.downloading(percent) : text.updater.installing}
          </p>
          <div className="update-track">
            <div
              className="update-fill"
              style={{ width: phase === "installing" ? "100%" : `${String(percent)}%` }}
            />
          </div>
        </div>
      )}
      {error !== null && <p className="detail-notice">{error}</p>}
    </div>
  );
}
