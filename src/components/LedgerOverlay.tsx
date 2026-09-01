import { text } from "../i18n";
import type { RemovalReceipt } from "../types";
import { formatSize } from "../utils";

interface LedgerOverlayProps {
  setLedgerOpen: React.Dispatch<React.SetStateAction<boolean>>;
  receipts: RemovalReceipt[] | null;
  exportNote: string | null;
  exportLedger: () => void;
}

export function LedgerOverlay({
  setLedgerOpen,
  receipts,
  exportNote,
  exportLedger,
}: LedgerOverlayProps) {
  return (
    <div
      className="overlay"
      role="presentation"
      onClick={() => {
        setLedgerOpen(false);
      }}
    >
      <div
        className="dialog dialog-wide"
        role="dialog"
        aria-modal="true"
        aria-label={text.ledger.title}
        onClick={(e) => {
          e.stopPropagation();
        }}
      >
        <h2>{text.ledger.title}</h2>
        <p className="dialog-body">{text.ledger.subtitle}</p>

        {receipts === null && (
          <p className="status" role="status">
            <span className="spinner" aria-hidden="true" />
          </p>
        )}
        {receipts !== null && receipts.length === 0 && (
          <p className="dialog-body">{text.ledger.empty}</p>
        )}
        {receipts !== null && receipts.length > 0 && (
          <ul className="ledger-list">
            {receipts.map((r, i) => (
              <li key={`${String(r.ts)}-${String(i)}`} className="ledger-row">
                <span
                  className={`conf-dot ${r.success ? "dot-ok" : "dot-bad"}`}
                  aria-hidden="true"
                />
                <span className="ledger-main">
                  <span className="ledger-name">
                    {r.programName}
                    <span className="badge badge-user">{r.method.toUpperCase()}</span>
                    {r.rebootRequired && (
                      <span className="badge badge-hidden">{text.ledger.rebootFlag}</span>
                    )}
                    {!r.success && (
                      <span className="badge badge-invalid">{text.ledger.failedFlag}</span>
                    )}
                  </span>
                  <span className="ledger-meta">
                    {new Date(r.ts * 1000).toLocaleString()} ·{" "}
                    {r.verifiedFreedKb !== null && r.verifiedFreedKb > 0
                      ? text.ledger.verifiedFreed(formatSize(r.verifiedFreedKb))
                      : r.estimatedSizeKb !== null && r.estimatedSizeKb > 0
                        ? text.ledger.estimatedOnly(formatSize(r.estimatedSizeKb))
                        : "—"}{" "}
                    · {text.ledger.restorePointLabel}: {r.restorePoint}
                  </span>
                  <span className="ledger-message">{r.message}</span>
                </span>
              </li>
            ))}
          </ul>
        )}

        {exportNote !== null && <p className="detail-notice">{exportNote}</p>}
        <div className="dialog-actions">
          {receipts !== null && receipts.length > 0 && (
            <button type="button" className="button-ghost" onClick={exportLedger}>
              {text.ledger.exportButton}
            </button>
          )}
          <button
            type="button"
            className="button"
            onClick={() => {
              setLedgerOpen(false);
            }}
          >
            {text.uninstall.close}
          </button>
        </div>
      </div>
    </div>
  );
}
