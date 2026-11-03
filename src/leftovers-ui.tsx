import { useEffect, useRef, useState, type ReactNode } from "react";
import type { Locale } from "./i18n";
import { leftoversCopy } from "./leftovers-copy";
import {
  groupItems,
  isProOnly,
  isSelectable,
  reviewIds,
  summarize,
  type LeftoverItem,
  type CleanPlan,
  type LeftoverReport,
  type RemovedProgramRow,
} from "./leftovers";
import { UiIcon } from "./ui-icon";

type Copy = (typeof leftoversCopy)[Locale];

function proofLine(c: Copy, item: LeftoverItem, subject: string): string | null {
  if (item.tier === "keep") return null;
  if (item.evidence === "dangling" && item.missing !== null) return c.proofDangling(item.missing);
  if (item.evidence === "name") return c.proofName;
  if (item.evidence === "path") return c.proofPath(subject);
  return null;
}

function Row({
  c,
  item,
  subject,
  checked,
  pro,
  size,
  onToggle,
}: {
  c: Copy;
  item: LeftoverItem;
  subject: string;
  checked: boolean;
  pro: boolean;
  size: string;
  onToggle: () => void;
}) {
  const selectable = isSelectable(item, pro);
  const proof = proofLine(c, item, subject);
  const nameId = `lo-name-${item.id}`;
  const noteId = `lo-note-${item.id}`;
  const note = item.blocker !== null ? `${c.blockers[item.blocker]}. ${c.keptHint}` : proof;
  return (
    <li className={`leftover leftover-${item.tier}${selectable ? "" : " leftover-fixed"}`}>
      <label className="leftover-main">
        <input
          type="checkbox"
          checked={checked}
          disabled={!selectable}
          aria-labelledby={nameId}
          aria-describedby={noteId}
          onChange={onToggle}
        />
        <span className="leftover-body">
          <span className="leftover-head">
            <span className="leftover-name" id={nameId}>
              {item.label || item.path}
            </span>
            <span className={`badge leftover-chip leftover-chip-${item.tier}`}>
              {c.tiers[item.tier]}
            </span>
            <span className="badge leftover-chip">{c.kinds[item.kind]}</span>
            <span className="badge leftover-chip">{c.scope[item.scope]}</span>
            {isProOnly(item.kind) && item.tier !== "keep" && (
              <span className="badge leftover-chip leftover-chip-pro">Pro</span>
            )}
          </span>
          <span className="leftover-path">{item.command ?? item.path}</span>
          <span className="leftover-foot">
            {note !== null && (
              <span className="leftover-note" id={noteId}>
                {note}
              </span>
            )}
            {note === null && <span id={noteId} hidden />}
            <span className="leftover-meta">
              {c.restore[item.restore]}
              {size !== "" && ` · ${size}`}
            </span>
          </span>
        </span>
      </label>
    </li>
  );
}

/** The leftovers dialog body: grouped, with proof lines and tier chips. */
export function LeftoversView({
  lang,
  subject,
  report,
  selected,
  onSelected,
  pro,
  proNotice,
  proAction,
  plan,
  adminNotes,
  onPreview,
  onBack,
  onClean,
  onClose,
  formatSize,
  words,
}: {
  lang: Locale;
  subject: string;
  report: LeftoverReport;
  selected: readonly string[];
  onSelected: (ids: string[]) => void;
  pro: boolean;
  /** The sentence about Pro, with the sign-in hint when needed. */
  proNotice: ReactNode;
  /** The trial or checkout button (empty when Pro is active). */
  proAction: ReactNode;
  plan: CleanPlan | null;
  /** Shown with a plan that needs the administrator step: the restore
   *  point forecast. */
  adminNotes: ReactNode;
  onPreview: () => void;
  onBack: () => void;
  onClean: () => void;
  onClose: () => void;
  formatSize: (kb: number | null) => string;
  words: { empty: string; emptyTitle: string; close: string };
}) {
  const c = leftoversCopy[lang];
  const { items } = report;
  const summary = summarize(items);
  const toggle = (id: string) => {
    onSelected(selected.includes(id) ? selected.filter((s) => s !== id) : [...selected, id]);
  };
  if (items.length === 0) {
    return (
      <>
        <h2>{words.emptyTitle}</h2>
        <p className="dialog-body">{words.empty}</p>
        <div className="dialog-actions">
          <button type="button" className="button" onClick={onClose}>
            {words.close}
          </button>
        </div>
      </>
    );
  }
  const needsPro = !pro && items.some((item) => item.tier !== "keep" && isProOnly(item.kind));
  return (
    <>
      <h2>{report.mode === "sweep" ? c.titleSweep : c.titleFound}</h2>
      {subject !== "" && <p className="leftovers-subject">{subject}</p>}
      <p className="leftovers-teaser">
        {c.teaser(summary.count, formatSize(summary.sizeKb))}
        {summary.startup > 0 && report.mode !== "sweep" && ` · ${c.teaserStartup(summary.startup)}`}
      </p>
      {report.mode === "manual" && <p className="dialog-body subtle">{c.noteManual}</p>}
      {report.mode === "sweep" && <p className="dialog-body subtle">{c.noteSweep}</p>}

      {plan === null ? (
        <div className="leftovers-groups">
          {groupItems(items).map((group) => {
            const review = reviewIds(group.items, pro);
            return (
              <details key={group.id} className="leftovers-group" open={group.id !== "kept"}>
                <summary>
                  <UiIcon name="chevron" />
                  <span className="leftovers-group-name">{c.groups[group.id]}</span>
                  <span className="leftovers-group-count">{group.items.length}</span>
                </summary>
                {review.length > 0 && (
                  <button
                    type="button"
                    className="button-ghost small leftovers-select"
                    onClick={() => {
                      onSelected([...new Set([...selected, ...review])]);
                    }}
                  >
                    {c.selectReview}
                  </button>
                )}
                <ul className="leftover-list">
                  {group.items.map((item) => (
                    <Row
                      key={item.id}
                      c={c}
                      item={item}
                      subject={subject}
                      checked={selected.includes(item.id)}
                      pro={pro}
                      size={item.sizeKb !== null && item.sizeKb > 0 ? formatSize(item.sizeKb) : ""}
                      onToggle={() => {
                        toggle(item.id);
                      }}
                    />
                  ))}
                </ul>
              </details>
            );
          })}
        </div>
      ) : (
        <section className="leftovers-preview" aria-label={c.preview}>
          <h3>{c.preview}</h3>
          {plan.steps.length === 0 ? (
            <p className="dialog-body">{c.previewNone}</p>
          ) : (
            <ul>
              {plan.steps.map((step) => {
                const item = items.find((candidate) => candidate.id === step.id);
                return (
                  <li key={step.id} className={step.locked || step.refused ? "skipped" : ""}>
                    <span className="leftover-name">{item?.label ?? step.id}</span>
                    {item && <span className="leftover-path">{item.command ?? item.path}</span>}
                    <span className="leftover-note">
                      {step.refused
                        ? c.previewRefused
                        : step.locked
                          ? c.previewLocked
                          : step.step === "recycle"
                            ? c.previewRecycle
                            : c.previewBackup}
                      {step.admin && !step.locked && !step.refused && ` · ${c.scope.machine}`}
                    </span>
                  </li>
                );
              })}
            </ul>
          )}
          {plan.needsAdmin && (
            <>
              <p className="dialog-body">{c.previewAdmin}</p>
              {adminNotes}
            </>
          )}
          <p className="dialog-body subtle">{c.previewUnchanged}</p>
        </section>
      )}

      {needsPro && (
        <div className="leftovers-pro">
          <p className="dialog-body reboot">{proNotice}</p>
          {proAction}
        </div>
      )}
      <div className="dialog-actions leftovers-actions">
        <span className="leftovers-count" role="status">
          {c.selected(selected.length)}
        </span>
        {plan === null ? (
          <button
            type="button"
            className="button-ghost"
            disabled={selected.length === 0}
            onClick={onPreview}
          >
            {c.preview}
          </button>
        ) : (
          <button type="button" className="button-ghost" onClick={onBack}>
            {c.previewBack}
          </button>
        )}
        <button
          type="button"
          className="button primary"
          disabled={selected.length === 0}
          onClick={onClean}
        >
          {c.clean}
        </button>
        <button type="button" className="button-ghost" onClick={onClose}>
          {words.close}
        </button>
      </div>
    </>
  );
}

/** "Foo was removed outside this app" and the first-launch card. */
export function RemovedNotice({
  title,
  body,
  action,
  onAction,
  dismiss,
  onDismiss,
}: {
  title: string;
  body: string;
  action: string;
  onAction: () => void;
  dismiss: string;
  onDismiss: () => void;
}) {
  return (
    <section className="removed-notice" aria-label={title}>
      <UiIcon name="history" />
      <div className="removed-notice-text">
        <strong>{title}</strong>
        <span>{body}</span>
      </div>
      <div className="removed-notice-actions">
        <button type="button" className="button primary" onClick={onAction}>
          {action}
        </button>
        <button type="button" className="button-ghost" onClick={onDismiss}>
          {dismiss}
        </button>
      </div>
    </section>
  );
}

/** The removed-programs list and the manual scan form. */
export function RemovedPanel({
  lang,
  rows,
  busy,
  error,
  onScan,
  onManual,
  onSweep,
  onClose,
  formatDate,
  closeLabel,
}: {
  lang: Locale;
  rows: RemovedProgramRow[] | null;
  busy: boolean;
  error: string | null;
  onScan: (row: RemovedProgramRow) => void;
  onManual: (name: string, folder: string) => void;
  onSweep: () => void;
  onClose: () => void;
  formatDate: (seconds: number) => string;
  closeLabel: string;
}) {
  const c = leftoversCopy[lang];
  const [name, setName] = useState("");
  const [folder, setFolder] = useState("");
  const root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    root.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);
  const canScan = !busy && (name.trim() !== "" || folder.trim() !== "");
  return (
    <div className="overlay" role="presentation" onClick={onClose}>
      <div
        ref={root}
        tabIndex={-1}
        className="dialog dialog-wide removed-panel"
        role="dialog"
        aria-modal="true"
        aria-label={c.removedTitle}
        onClick={(e) => {
          e.stopPropagation();
        }}
      >
        <h2 className="ledger-heading">
          <UiIcon name="history" />
          {c.removedTitle}
        </h2>
        <p className="dialog-body">{c.removedIntro}</p>
        {rows === null && (
          <p className="status" role="status">
            <span className="spinner" aria-hidden="true" />
          </p>
        )}
        {rows !== null && rows.length === 0 && <p className="dialog-body">{c.removedEmpty}</p>}
        {rows !== null && rows.length > 0 && (
          <ul className="removed-list">
            {rows.map((row) => (
              <li key={row.rid} className="removed-row">
                <span className="removed-main">
                  <span className="removed-name">{row.name}</span>
                  <span className="removed-meta">
                    <span className={`badge ${row.how === "app" ? "badge-user" : "badge-hidden"}`}>
                      {row.how === "app" ? c.howApp : c.howOutside}
                    </span>
                    {c.removedOn(formatDate(row.removedAt))}
                    {row.publisher !== null && ` · ${row.publisher}`}
                  </span>
                </span>
                <button
                  type="button"
                  className="button-ghost small"
                  disabled={busy}
                  onClick={() => {
                    onScan(row);
                  }}
                >
                  {c.scanThis}
                </button>
              </li>
            ))}
          </ul>
        )}

        <form
          className="removed-form"
          onSubmit={(e) => {
            e.preventDefault();
            if (canScan) onManual(name, folder);
          }}
        >
          <h3>{c.manualTitle}</h3>
          <p className="dialog-body subtle">{c.manualIntro}</p>
          <label className="field">
            <span>{c.manualName}</span>
            <input
              className="search"
              type="text"
              value={name}
              maxLength={120}
              autoComplete="off"
              spellCheck={false}
              onChange={(e) => {
                setName(e.target.value);
              }}
            />
          </label>
          <label className="field">
            <span>{c.manualFolder}</span>
            <input
              className="search"
              type="text"
              value={folder}
              maxLength={260}
              autoComplete="off"
              spellCheck={false}
              aria-describedby="removed-folder-hint"
              onChange={(e) => {
                setFolder(e.target.value);
              }}
            />
            <small id="removed-folder-hint">{c.manualFolderHint}</small>
          </label>
          {error !== null && (
            <p className="detail-notice" role="alert">
              {error}
            </p>
          )}
          <div className="dialog-actions">
            <button type="button" className="button-ghost" disabled={busy} onClick={onSweep}>
              {c.sweepButton}
            </button>
            <button type="submit" className="button primary" disabled={!canScan}>
              {busy && <span className="spinner" aria-hidden="true" />}
              {c.manualScan}
            </button>
            <button type="button" className="button-ghost" onClick={onClose}>
              {closeLabel}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
