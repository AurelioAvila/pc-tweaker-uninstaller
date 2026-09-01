import { text } from "../i18n";
import appLogo from "../../src-tauri/icons/128x128.png";

interface HeaderProps {
  suiteDetected: boolean;
  openPcTweaker: () => void;
  openLedger: () => void;
  menuOpen: boolean;
  setMenuOpen: React.Dispatch<React.SetStateAction<boolean>>;
  version: string | null;
}

export function Header({
  suiteDetected,
  openPcTweaker,
  openLedger,
  menuOpen,
  setMenuOpen,
  version,
}: HeaderProps) {
  return (
    <header className="topbar">
      <div className="brand">
        <img className="brand-logo" src={appLogo} alt="" aria-hidden="true" />
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
            onClick={openPcTweaker}
          >
            <span className="suite-dot" aria-hidden="true" />
            {text.app.suiteDetected}
          </button>
        )}
        <button type="button" className="button-ghost small" onClick={openLedger}>
          {text.ledger.open}
        </button>
        <button
          type="button"
          className="menu-trigger"
          aria-label={text.menu.open}
          aria-expanded={menuOpen}
          onClick={() => {
            setMenuOpen((v) => !v);
          }}
        >
          <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <circle cx="12" cy="8.4" r="3.2" stroke="currentColor" strokeWidth="1.7" />
            <path
              d="M4.8 19.5c1.4-3.4 4.2-5.2 7.2-5.2s5.8 1.8 7.2 5.2"
              stroke="currentColor"
              strokeWidth="1.7"
              strokeLinecap="round"
            />
          </svg>
        </button>
        {version !== null && <span className="version">v{version}</span>}
      </div>
    </header>
  );
}
