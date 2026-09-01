import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { currentLocale, setLocale, text, type Locale } from "./i18n";
import { applyTheme, initialTheme, type ThemeCode } from "./themes";
import {
  fetchAccount,
  fetchUninstallerEntitlement,
  login,
  logout,
  startUninstallerCheckout,
  type AccountState,
  type UninstallerEntitlement,
} from "./account";
import { UpdateBanner } from "./updater";
import "./App.css";
import { Header } from "./components/Header";
import { Footer } from "./components/Footer";
import { MenuPanel } from "./components/MenuPanel";
import { LedgerOverlay } from "./components/LedgerOverlay";
import { UninstallFlowDialogs } from "./components/UninstallFlowDialogs";
import { ProgramList } from "./components/ProgramList";

import type {
  CleanResult,
  FilterChip,
  FlowState,
  LoadState,
  ProgramInfo,
  RemovalReceipt,
  ResidueReport,
  BatchItemResult,
  SortKey,
  StoreApp,
  UninstallPlan,
  UninstallReport,
} from "./types";
import { LARGE_KB, compareBy, isRecent, storeAppAsProgram } from "./utils";

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
  const [batchSelected, setBatchSelected] = useState<readonly string[]>([]);
  const [detailNotice, setDetailNotice] = useState<string | null>(null);

  // Language and theme: persisted locally, English + Violet by default.
  const [lang, setLang] = useState<Locale>(() => currentLocale());
  const [theme, setTheme] = useState<ThemeCode>(() => initialTheme());
  const [menuOpen, setMenuOpen] = useState(false);
  useEffect(() => {
    applyTheme(theme);
  }, [theme]);
  const chooseLang = useCallback((l: Locale) => {
    setLocale(l); // swaps the module `text` binding before the re-render
    setLang(l);
  }, []);

  // Removal Ledger overlay.
  const [ledgerOpen, setLedgerOpen] = useState(false);
  const [receipts, setReceipts] = useState<RemovalReceipt[] | null>(null);
  const [exportNote, setExportNote] = useState<string | null>(null);
  const openLedger = useCallback(() => {
    setExportNote(null);
    setLedgerOpen(true);
    setReceipts(null);
    invoke<RemovalReceipt[]>("list_removal_ledger")
      .then(setReceipts)
      .catch(() => {
        setReceipts([]);
      });
  }, []);
  const exportLedger = useCallback(() => {
    invoke<string>("export_removal_ledger")
      .then((path) => {
        setExportNote(text.ledger.exportedTo(path));
      })
      .catch((error: unknown) => {
        setExportNote(typeof error === "string" ? error : text.errors.generic);
      });
  }, []);

  // Suite account: the sole source of truth for Pro status and loyalty
  // eligibility. Checked on mount and on window focus (catches a purchase
  // made on pctweaker.app in the system browser), mirroring PC Tweaker's own
  // refreshAccount pattern.
  const [account, setAccount] = useState<AccountState>({ status: "anonymous" });
  const [uninstallerPro, setUninstallerPro] = useState<UninstallerEntitlement | null>(null);
  const [checkoutBusy, setCheckoutBusy] = useState(false);
  const [checkoutStarted, setCheckoutStarted] = useState(false);
  const [checkoutError, setCheckoutError] = useState(false);

  const refreshEntitlement = useCallback(() => {
    fetchUninstallerEntitlement()
      .then(setUninstallerPro)
      .catch(() => {
        setUninstallerPro(null);
      });
  }, []);

  const beginProCheckout = useCallback(() => {
    setCheckoutBusy(true);
    startUninstallerCheckout()
      .then((url) => invoke("open_checkout_url", { url }))
      .then(() => {
        setCheckoutStarted(true);
      })
      .catch(() => {
        setCheckoutStarted(false);
        setCheckoutError(true);
      })
      .finally(() => {
        setCheckoutBusy(false);
      });
  }, []);
  const [loginEmail, setLoginEmail] = useState("");
  const [loginPassword, setLoginPassword] = useState("");
  const [loginError, setLoginError] = useState<string | null>(null);
  const isProConfirmed = account.status === "signed-in" && account.isPro;

  const refreshAccount = useCallback(() => {
    fetchAccount()
      .then((next) => {
        setAccount(next);
        if (next.status === "signed-in") refreshEntitlement();
      })
      .catch(() => {
        // fetchAccount never rejects in practice; this is defense in depth.
      });
  }, [refreshEntitlement]);

  useEffect(() => {
    refreshAccount();
    window.addEventListener("focus", refreshAccount);
    return () => {
      window.removeEventListener("focus", refreshAccount);
    };
  }, [refreshAccount]);

  const submitLogin = useCallback(() => {
    setLoginError(null);
    setAccount({ status: "checking", email: loginEmail });
    login(loginEmail, loginPassword)
      .then((next) => {
        setAccount(next);
        setLoginPassword("");
        if (next.status === "signed-in") refreshEntitlement();
      })
      .catch((error: unknown) => {
        setAccount({ status: "anonymous" });
        setLoginError(typeof error === "string" ? error : (error as Error).message);
      });
  }, [loginEmail, loginPassword, refreshEntitlement]);

  const signOut = useCallback(() => {
    logout();
    setAccount({ status: "anonymous" });
    setLoginEmail("");
  }, []);

  const load = useCallback(() => {
    setState({ phase: "loading" });
    // Two independent sources, one list. The registry read decides whether
    // the screen can render at all; the MSIX read is allowed to fail on its
    // own (an older Windows, a package manager that will not start) without
    // taking the rest of the inventory down with it.
    Promise.all([
      invoke<ProgramInfo[]>("list_programs"),
      invoke<StoreApp[]>("list_store_apps").catch(() => [] as StoreApp[]),
    ])
      .then(([programs, storeApps]) => {
        setState({
          phase: "ready",
          programs: [...programs, ...storeApps.map(storeAppAsProgram)],
        });
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
    // An MSIX package has no uninstall command to plan: Windows owns the
    // removal. Planning it would mean showing a command line that does not
    // exist, so this path goes straight to its own confirmation.
    if (program.source === "store") {
      setFlow({ step: "storeConfirm", program });
      return;
    }
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

  const confirmStoreRemoval = useCallback(
    (program: ProgramInfo) => {
      setFlow({ step: "storeRunning", program });
      invoke("remove_store_app", { packageFullName: program.id })
        .then(() => {
          setFlow({ step: "storeDone", program });
          load();
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

  const beginResidueScan = useCallback((program: ProgramInfo) => {
    setFlow({ step: "residueScanning", program });
    invoke<ResidueReport>("scan_residue", {
      name: program.name,
      publisher: program.publisher,
      installLocation: program.installLocation,
    })
      .then((residue) => {
        setFlow({
          step: "residue",
          program,
          residue,
          // Everything cleanable starts selected; the user deselects, which
          // is the right default for a cleanup the user explicitly asked for.
          selected: residue.items.filter((item) => item.deletable).map((item) => item.path),
        });
      })
      .catch(() => {
        setFlow({ step: "idle" });
      });
  }, []);

  const runResidueClean = useCallback((program: ProgramInfo, selected: readonly string[]) => {
    setFlow({ step: "residueScanning", program });
    invoke<CleanResult>("clean_residue", {
      name: program.name,
      publisher: program.publisher,
      installLocation: program.installLocation,
      paths: [...selected],
    })
      .then((result) => {
        setFlow({ step: "residueDone", program, result });
      })
      .catch(() => {
        setFlow({ step: "idle" });
      });
  }, []);

  // Safe Batch: sequential removals with the same per-item machinery as a
  // single uninstall — the backend's one-at-a-time lock stays authoritative,
  // this loop just feeds it in a safe order (contained before container).
  const runBatch = useCallback(
    async (programs: ProgramInfo[]) => {
      const results: BatchItemResult[] = [];
      for (const [index, program] of programs.entries()) {
        setFlow({ step: "batchRunning", programs, index, results: [...results] });
        try {
          const report = await invoke<UninstallReport>("execute_uninstall", {
            source: program.source,
            id: program.id,
          });
          results.push({ name: program.name, success: report.success, message: report.message });
        } catch (error: unknown) {
          results.push({
            name: program.name,
            success: false,
            message: typeof error === "string" ? error : text.errors.generic,
          });
        }
      }
      setBatchSelected([]);
      setFlow({ step: "batchDone", results });
      load();
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
    if (
      flowStep === "idle" ||
      flowStep === "planning" ||
      flowStep === "running" ||
      flowStep === "residueScanning" ||
      flowStep === "batchRunning"
    )
      return;
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
        // Same hyphen normalization as isFamilyApp: the flagship registers
        // itself as "pc-tweaker-app".
        const name = p.name.toLowerCase().replace(/[-_]/g, " ");
        return name.startsWith("pc tweaker") && !name.includes("uninstaller");
      }),
    [state],
  );

  const openPcTweaker = useCallback(() => {
    invoke("open_pc_tweaker").catch(() => {
      // Non-fatal: the button only appears when detection succeeded, and a
      // race (just uninstalled) simply does nothing visible.
    });
  }, []);

  // `lang` is otherwise only read through the module `text` binding; keeping
  // it referenced here documents that renders depend on it.
  void lang;

  return (
    <div className="shell">
      <Header
        suiteDetected={suiteDetected}
        openPcTweaker={openPcTweaker}
        openLedger={openLedger}
        menuOpen={menuOpen}
        setMenuOpen={setMenuOpen}
        version={version}
      />

      {menuOpen && (
        <MenuPanel
          setMenuOpen={setMenuOpen}
          account={account}
          uninstallerPro={uninstallerPro}
          checkoutBusy={checkoutBusy}
          beginProCheckout={beginProCheckout}
          checkoutStarted={checkoutStarted}
          refreshEntitlement={refreshEntitlement}
          checkoutError={checkoutError}
          signOut={signOut}
          submitLogin={submitLogin}
          loginEmail={loginEmail}
          setLoginEmail={setLoginEmail}
          loginPassword={loginPassword}
          setLoginPassword={setLoginPassword}
          loginError={loginError}
          openLink={openLink}
          suiteDetected={suiteDetected || false}
          openPcTweaker={openPcTweaker}
          isProConfirmed={isProConfirmed}
          lang={lang}
          chooseLang={chooseLang}
          theme={theme}
          setTheme={setTheme}
        />
      )}

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
          <ProgramList
            query={query}
            setQuery={setQuery}
            chip={chip}
            setChip={setChip}
            showHidden={showHidden}
            setShowHidden={setShowHidden}
            filtered={filtered}
            visible={visible}
            totalSizeKb={totalSizeKb}
            batchSelected={batchSelected}
            setBatchSelected={setBatchSelected}
            flow={flow}
            setFlow={setFlow}
            programs={state.programs}
            sortKey={sortKey}
            sortAsc={sortAsc}
            toggleSort={toggleSort}
            expandedKey={expandedKey}
            toggleExpanded={toggleExpanded}
            beginUninstall={beginUninstall}
            openFolder={openFolder}
            detailNotice={detailNotice}
          />
        )}
      </main>

      <Footer openLink={openLink} openRestoreUi={openRestoreUi} />

      <UpdateBanner />

      {ledgerOpen && (
        <LedgerOverlay
          setLedgerOpen={setLedgerOpen}
          receipts={receipts}
          exportNote={exportNote}
          exportLedger={exportLedger}
        />
      )}

      <UninstallFlowDialogs
        flow={flow}
        closeFlow={closeFlow}
        confirmUninstall={confirmUninstall}
        confirmStoreRemoval={confirmStoreRemoval}
        beginResidueScan={beginResidueScan}
        runResidueClean={runResidueClean}
        setFlow={setFlow}
        uninstallerPro={uninstallerPro}
        account={account}
        checkoutBusy={checkoutBusy}
        beginProCheckout={beginProCheckout}
        runBatch={runBatch}
      />
    </div>
  );
}
