import { useCallback, useEffect, useRef, useState } from "react";
import { check as checkForUpdate, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { text } from "./i18n";
import { UiIcon } from "./ui-icon";
import { workspaceCopy } from "./workspace-copy";

export function useAppUpdater() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [checking, setChecking] = useState(false);
  const [checkStatus, setCheckStatus] = useState<"idle" | "current" | "error">("idle");
  const [phase, setPhase] = useState<"offer" | "downloading" | "installing">("offer");
  const [percent, setPercent] = useState<number | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const alive = useRef(false);
  const checkingRef = useRef(false);
  const installing = useRef(false);
  const lastCheck = useRef(0);
  const updateResource = useRef<Update | null>(null);
  const check = useCallback(async (manual = false) => {
    if (checkingRef.current || installing.current) return;
    if (!manual && Date.now() - lastCheck.current < 60 * 60 * 1000) return;
    checkingRef.current = true;
    setChecking(true);
    setCheckStatus("idle");
    try {
      const result = await checkForUpdate();
      if (!alive.current) {
        await result?.close();
        return;
      }
      void updateResource.current?.close().catch(() => {});
      updateResource.current = result;
      lastCheck.current = Date.now();
      setUpdate(result);
      if (result) {
        setDismissed(false);
        setError(null);
      } else if (manual) setCheckStatus("current");
    } catch {
      if (alive.current && manual) setCheckStatus("error");
    } finally {
      checkingRef.current = false;
      if (alive.current) setChecking(false);
    }
  }, []);
  useEffect(() => {
    alive.current = true;
    void check();
    const refresh = () => {
      void check();
    };
    window.addEventListener("focus", refresh);
    window.addEventListener("online", refresh);
    const timer = window.setInterval(refresh, 60 * 60 * 1000);
    return () => {
      alive.current = false;
      window.removeEventListener("focus", refresh);
      window.removeEventListener("online", refresh);
      window.clearInterval(timer);
      void updateResource.current?.close().catch(() => {});
      updateResource.current = null;
    };
  }, [check]);
  async function install() {
    if (!update || installing.current) return;
    installing.current = true;
    setError(null);
    setPercent(null);
    setPhase("downloading");
    try {
      let total = 0;
      let received = 0;
      // Tauri verifies the configured publisher update signature before installation.
      await update.downloadAndInstall((event) => {
        if (!alive.current) return;
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          received += event.data.chunkLength;
          if (total > 0) setPercent(Math.min(100, Math.round((received / total) * 100)));
        }
        if (event.event === "Finished") setPhase("installing");
      });
      await relaunch();
    } catch (err) {
      if (alive.current) {
        setPhase("offer");
        setError(text.updater.error(String(err)));
      }
    } finally {
      installing.current = false;
    }
  }
  return {
    update,
    checking,
    checkStatus,
    phase,
    percent,
    dismissed,
    error,
    check,
    install,
    dismiss: () => {
      setDismissed(true);
    },
  };
}

export function UpdateBanner({
  updater,
  busy,
  words,
}: {
  updater: ReturnType<typeof useAppUpdater>;
  busy: boolean;
  words: typeof workspaceCopy.en;
}) {
  const { update, phase, percent, error } = updater;
  if (!update || updater.dismissed) return null;
  return (
    <aside className="update-card" aria-label={text.updater.title(update.version)}>
      <div className="update-head" role="status">
        <span className="update-icon">
          <UiIcon name="download" />
        </span>
        <div>
          <p className="update-title">{text.updater.title(update.version)}</p>
          <p className="update-body">{text.updater.body}</p>
        </div>
      </div>
      {phase === "offer" ? (
        <>
          <div className="update-actions">
            <button
              className="primary small-pad"
              disabled={busy}
              onClick={() => {
                if (!busy) void updater.install();
              }}
            >
              <UiIcon name="download" />
              {text.updater.install}
            </button>
            <button className="button-ghost small" onClick={updater.dismiss}>
              {text.updater.later}
            </button>
          </div>
          {busy && <p className="update-body">{words.busy}</p>}
        </>
      ) : (
        <div className="update-progress" role="status">
          <p>
            {phase === "installing"
              ? text.updater.installing
              : percent === null
                ? words.unknownProgress
                : text.updater.downloading(percent)}
          </p>
          <progress
            max={100}
            value={phase === "installing" ? undefined : (percent ?? undefined)}
            aria-label={words.unknownProgress}
          />
        </div>
      )}
      {error && (
        <p className="detail-notice" role="alert">
          {error}
        </p>
      )}
    </aside>
  );
}
