import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SubscriptionRequestError, startBillingPortal, subscriptionRequest } from "./account";
import { accountCopy } from "./account-copy";
import { text, type Locale } from "./i18n";
import {
  CANCEL_REASONS,
  cardBrand,
  parseSubscription,
  endsOn,
  formatAmount,
  subscriptionView,
  type CancelFeedback,
  type CancelReason,
  type FamilyProduct,
  type Subscription,
  type SubscriptionErrorKey,
} from "./subscription";
import { fill } from "./trial-copy";
import { UiIcon } from "./ui-icon";

/** The last day of access of a cancelled subscription (the trial's end
 *  when cancelled during the trial). */
function endingDate(s: Subscription): string | null {
  const v = subscriptionView(s);
  return v.kind === "ending" ? v.until : endsOn(s);
}

/** A modal over the app: Escape and the scrim close it, focus starts on the
 *  title and goes back to whatever opened it. */
function Dialog({
  label,
  onClose,
  onEscape,
  children,
}: {
  label: string;
  onClose: () => void;
  /** Escape steps back inside the dialog first (the cancel confirmation). */
  onEscape?: () => boolean;
  children: (titleRef: React.RefObject<HTMLHeadingElement | null>) => ReactNode;
}) {
  const title = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    title.current?.focus();
    return () => {
      if (opener?.isConnected) opener.focus();
    };
  }, []);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.stopPropagation();
      if (!onEscape?.()) onClose();
    };
    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("keydown", onKey, true);
    };
  }, [onClose, onEscape]);
  return (
    <div className="overlay" role="presentation" onClick={onClose}>
      <div
        className="dialog account-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={label}
        onClick={(e) => {
          e.stopPropagation();
        }}
      >
        {children(title)}
      </div>
    </div>
  );
}

/** One request, parsed: an answer that is not a subscription is a billing
 *  error, never a guessed state. */
async function subscription(
  product: FamilyProduct,
  change?: "cancel" | "resume",
  feedback?: CancelFeedback,
): Promise<Subscription> {
  const parsed = parseSubscription(await subscriptionRequest(product, change, feedback));
  if (!parsed) throw new SubscriptionRequestError("billing");
  return parsed;
}

type Load =
  | { phase: "loading" }
  | { phase: "error"; key: SubscriptionErrorKey }
  | { phase: "ready"; mine: Subscription; others: Subscription[] };

/**
 * Manage subscription, from the account menu: what is active on this account
 * for the Uninstaller (and the family's other products), with Cancel at the
 * end of the paid period, Resume, and the Stripe portal for the card and
 * invoices. Never an immediate cancellation, never a refund.
 */
export function SubscriptionDialog({
  lang,
  email,
  hasBilling,
  onClose,
  onSeePlans,
  onChanged,
}: {
  lang: Locale;
  email: string;
  /** A Stripe customer exists, so the portal has something to open. */
  hasBilling: boolean;
  onClose: () => void;
  onSeePlans: () => void;
  /** After a cancel or resume, so the rest of the app re-reads Pro. */
  onChanged: () => void;
}) {
  const c = accountCopy[lang];
  const locale = lang === "pt" ? "pt-PT" : lang;
  const day = (iso: string | null) =>
    iso ? new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(new Date(iso)) : "";

  const [load, setLoad] = useState<Load>({ phase: "loading" });
  // Cancelling is two steps: 1 = the optional reason, 2 = the confirmation.
  const [confirming, setConfirming] = useState<0 | 1 | 2>(0);
  const [reason, setReason] = useState<CancelReason | null>(null);
  const [comment, setComment] = useState("");
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<SubscriptionErrorKey | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [portal, setPortal] = useState<"idle" | "busy" | "error">("idle");

  const fetchAll = useCallback(() => {
    setLoad({ phase: "loading" });
    const others: FamilyProduct[] = ["pctweaker"];
    // The family's other products are a courtesy line: their failure never
    // hides this app's own subscription.
    Promise.all([
      subscription("uninstaller"),
      Promise.all(others.map((p) => subscription(p).catch(() => null))),
    ])
      .then(([mine, rest]) => {
        setLoad({
          phase: "ready",
          mine,
          others: rest.filter(
            (s): s is Subscription =>
              s !== null && s.product !== "uninstaller" && s.status !== "none",
          ),
        });
      })
      .catch((error: unknown) => {
        setLoad({
          phase: "error",
          key: error instanceof SubscriptionRequestError ? error.key : "billing",
        });
      });
  }, []);
  useEffect(() => {
    fetchAll();
  }, [fetchAll]);

  const change = (kind: "cancel" | "resume") => {
    if (load.phase !== "ready") return;
    setBusy(true);
    setActionError(null);
    setDone(null);
    const feedback =
      kind === "cancel" && reason
        ? { reason, ...(comment.trim() ? { comment: comment.trim() } : {}) }
        : undefined;
    subscription("uninstaller", kind, feedback)
      .then((mine) => {
        setLoad({ ...load, mine });
        setConfirming(0);
        setDone(
          kind === "cancel"
            ? fill(c.cancelled, { date: day(endingDate(mine)) })
            : fill(c.resumed, { date: day(mine.periodEnd) }),
        );
        onChanged();
      })
      .catch((error: unknown) => {
        setActionError(error instanceof SubscriptionRequestError ? error.key : "billing");
      })
      .finally(() => {
        setBusy(false);
      });
  };

  const openPortal = () => {
    setPortal("busy");
    startBillingPortal()
      .then((url) => invoke("open_checkout_url", { url }))
      .then(() => {
        setPortal("idle");
      })
      .catch(() => {
        setPortal("error");
      });
  };

  const price = (s: Subscription) =>
    s.amount !== null && s.currency
      ? `${formatAmount(s.amount, s.currency, locale)}${s.interval === "month" ? c.perMonth : s.interval === "year" ? c.perYear : ""}`
      : null;
  const intervalName = (s: Subscription) =>
    s.interval === "month" ? c.monthly : s.interval === "year" ? c.yearly : null;

  /** The status pill and the one sentence under the plan. */
  const describe = (s: Subscription): { pill: string; tone: string; line: string } => {
    const v = subscriptionView(s);
    switch (v.kind) {
      case "none":
        return { pill: c.pills.none, tone: "muted", line: c.none };
      case "lifetime":
        return { pill: c.pills.lifetime, tone: "ok", line: c.lifetime };
      case "included":
        return {
          pill: c.pills.included,
          tone: "ok",
          line: v.until ? fill(c.includedUntil, { date: day(v.until) }) : c.included,
        };
      case "trial":
        return {
          pill: c.pills.trial,
          tone: "accent",
          line: fill(c.trialUntil, { date: day(v.until) }),
        };
      case "renews":
        return { pill: c.pills.active, tone: "ok", line: fill(c.renews, { date: day(v.on) }) };
      case "ending":
        return {
          pill: c.pills.ending,
          tone: "warn",
          line: fill(s.status === "trialing" ? c.trialEnding : c.endingBody, {
            date: day(v.until),
          }),
        };
      case "pastDue":
        return { pill: c.pills.pastDue, tone: "danger", line: c.pastDue };
      case "paused":
        return { pill: c.pills.paused, tone: "warn", line: c.paused };
    }
  };

  const mine = load.phase === "ready" ? load.mine : null;
  const view = mine ? subscriptionView(mine) : null;
  const needsCard = view?.kind === "pastDue" || view?.kind === "paused";
  const confirmText = mine
    ? mine.status === "trialing" && mine.trialEnd
      ? fill(c.confirmTrial, { date: day(mine.trialEnd) })
      : mine.periodEnd
        ? fill(c.confirmBody, { date: day(endsOn(mine)) })
        : c.confirmNoDate
    : "";

  return (
    <Dialog
      label={c.manage}
      onClose={onClose}
      onEscape={() => {
        if (!confirming) return false;
        setConfirming(0);
        return true;
      }}
    >
      {(title) => (
        <>
          <div className="account-dialog-head">
            <div>
              {confirming > 0 && (
                <span className="sub-step">{fill(c.step, { n: String(confirming) })}</span>
              )}
              <h2 ref={title} id="sub-dialog-title" tabIndex={-1}>
                {confirming === 1 ? c.reasonTitle : confirming === 2 ? c.confirmTitle : c.manage}
              </h2>
              <p className="account-dialog-sub">{email}</p>
            </div>
            <button type="button" className="icon-button" aria-label={c.close} onClick={onClose}>
              <UiIcon name="close" />
            </button>
          </div>

          {load.phase === "loading" && (
            <p className="sub-loading" role="status">
              <span className="spinner" aria-hidden="true" />
              {c.loading}
            </p>
          )}

          {load.phase === "error" && (
            <div className="sub-error" role="alert">
              <p>{c.errors[load.key]}</p>
              <div className="dialog-actions">
                {hasBilling && load.key === "billing" && (
                  <button
                    type="button"
                    className="button-ghost"
                    disabled={portal === "busy"}
                    onClick={openPortal}
                  >
                    {c.portal}
                    <UiIcon name="external" />
                  </button>
                )}
                {load.key !== "auth" && (
                  <button type="button" className="button" onClick={fetchAll}>
                    <UiIcon name="refresh" />
                    {c.retry}
                  </button>
                )}
              </div>
            </div>
          )}

          {mine && view && !confirming && (
            <>
              <section
                className={`sub-card sub-${describe(mine).tone}`}
                aria-label={c.products.uninstaller}
              >
                <div className="sub-card-head">
                  <span className="sub-card-icon" aria-hidden="true">
                    <UiIcon name="card" />
                  </span>
                  <div className="sub-card-title">
                    <strong>{c.products.uninstaller}</strong>
                    {intervalName(mine) && <span>{intervalName(mine)}</span>}
                  </div>
                  <span className={`sub-pill sub-pill-${describe(mine).tone}`}>
                    {describe(mine).pill}
                  </span>
                </div>
                {price(mine) && <p className="sub-price">{price(mine)}</p>}
                <p className="sub-line">{describe(mine).line}</p>
                {view.kind === "none" && <p className="sub-hint">{c.noneHint}</p>}
                {mine.card && (
                  <p className="sub-meta">
                    <span>{c.payment}</span>
                    {fill(c.card, { brand: cardBrand(mine.card.brand), last4: mine.card.last4 })}
                  </p>
                )}
              </section>

              {done && (
                <p className="sub-done" role="status">
                  <UiIcon name="check" />
                  {done}
                </p>
              )}
              {actionError && (
                <p className="sub-inline-error" role="alert">
                  {c.errors[actionError]}
                </p>
              )}
              {portal === "error" && (
                <p className="sub-inline-error" role="alert">
                  {c.portalError}
                </p>
              )}

              {load.phase === "ready" && load.others.length > 0 && (
                <section className="sub-others" aria-labelledby="sub-others-title">
                  <h3 id="sub-others-title">{c.alsoOn}</h3>
                  <ul>
                    {load.others.map((s) => {
                      const d = describe(s);
                      return (
                        <li key={s.product}>
                          <span className="sub-others-name">
                            <strong>{c.products[s.product]}</strong>
                            <span>{[intervalName(s), price(s)].filter(Boolean).join(" · ")}</span>
                          </span>
                          <span className={`sub-pill sub-pill-${d.tone}`}>{d.pill}</span>
                        </li>
                      );
                    })}
                  </ul>
                </section>
              )}

              <div className="sub-actions">
                {view.kind === "none" && (
                  <button type="button" className="button" onClick={onSeePlans}>
                    <UiIcon name="tag" />
                    {c.seePlans}
                  </button>
                )}
                {needsCard && hasBilling && (
                  <button
                    type="button"
                    className="button"
                    disabled={portal === "busy"}
                    aria-busy={portal === "busy"}
                    onClick={openPortal}
                  >
                    {c.portal}
                    <UiIcon name="external" />
                  </button>
                )}
                {mine.canResume && (
                  <button
                    type="button"
                    className="button"
                    data-test="sub-resume"
                    disabled={busy}
                    aria-busy={busy}
                    onClick={() => {
                      change("resume");
                    }}
                  >
                    {c.resume}
                  </button>
                )}
                {hasBilling && !needsCard && view.kind !== "lifetime" && view.kind !== "none" && (
                  <button
                    type="button"
                    className="button-ghost sub-portal"
                    disabled={portal === "busy"}
                    aria-busy={portal === "busy"}
                    onClick={openPortal}
                  >
                    {c.portal}
                    <UiIcon name="external" />
                  </button>
                )}
                {mine.canCancel && (
                  <button
                    type="button"
                    className="button-ghost sub-cancel"
                    data-test="sub-cancel"
                    onClick={() => {
                      setDone(null);
                      setActionError(null);
                      setReason(null);
                      setComment("");
                      setConfirming(1);
                    }}
                  >
                    {c.cancel}
                  </button>
                )}
              </div>
              {mine.canCancel && <p className="sub-fine">{c.cancelNote}</p>}
            </>
          )}

          {mine && confirming === 1 && (
            <fieldset
              className="sub-reasons"
              aria-labelledby="sub-dialog-title"
              aria-describedby="sub-reasons-hint"
            >
              <p id="sub-reasons-hint" className="sub-reasons-hint">
                {c.reasonHint}
              </p>
              <div className="sub-reason-list">
                {CANCEL_REASONS.map((code, i) => (
                  <label key={code} className="sub-reason">
                    <input
                      type="radio"
                      name="sub-reason"
                      value={code}
                      checked={reason === code}
                      autoFocus={i === 0}
                      onChange={() => {
                        setReason(code);
                      }}
                    />
                    <span>{c.reasons[code]}</span>
                  </label>
                ))}
              </div>
              {reason && (
                <label className="sub-comment">
                  <span>{reason === "other" ? c.otherLabel : c.commentLabel}</span>
                  <textarea
                    rows={2}
                    maxLength={500}
                    value={comment}
                    placeholder={c.commentPlaceholder}
                    onChange={(e) => {
                      setComment(e.target.value);
                    }}
                  />
                  <small aria-hidden="true">{comment.length}/500</small>
                </label>
              )}
              <div className="dialog-actions">
                <button
                  type="button"
                  className="button-ghost sub-push"
                  onClick={() => {
                    setConfirming(0);
                  }}
                >
                  {c.keep}
                </button>
                <button
                  type="button"
                  className="button-ghost sub-skip"
                  onClick={() => {
                    setReason(null);
                    setComment("");
                    setConfirming(2);
                  }}
                >
                  {c.skip}
                </button>
                <button
                  type="button"
                  className="button"
                  onClick={() => {
                    setConfirming(2);
                  }}
                >
                  {c.next}
                </button>
              </div>
            </fieldset>
          )}

          {mine && confirming === 2 && (
            <div className="sub-confirm">
              <p className="dialog-body">{confirmText}</p>
              {actionError && (
                <p className="sub-inline-error" role="alert">
                  {c.errors[actionError]}
                </p>
              )}
              <div className="dialog-actions">
                <button
                  type="button"
                  className="button-ghost sub-back sub-push"
                  disabled={busy}
                  onClick={() => {
                    setConfirming(1);
                  }}
                >
                  <span aria-hidden="true">←</span> {c.back}
                </button>
                <button
                  type="button"
                  className="button-ghost"
                  autoFocus
                  disabled={busy}
                  onClick={() => {
                    setConfirming(0);
                  }}
                >
                  {c.keep}
                </button>
                <button
                  type="button"
                  className="button button-danger"
                  data-test="sub-cancel-confirm"
                  disabled={busy}
                  aria-busy={busy}
                  onClick={() => {
                    change("cancel");
                  }}
                >
                  {c.cancel}
                </button>
              </div>
            </div>
          )}
        </>
      )}
    </Dialog>
  );
}

/**
 * Help & support, from the top bar: the support page, the diagnostic log
 * this app already writes (applog.rs, never uploaded) and a copy of the
 * facts support asks for first. No email or program names in the copy.
 */
export function SupportDialog({
  lang,
  version,
  facts,
  onOpenSupport,
  onClose,
}: {
  lang: Locale;
  version: string | null;
  /** Extra lines for the copy (account state, theme), already in English. */
  facts: string[];
  onOpenSupport: () => void;
  onClose: () => void;
}) {
  const c = accountCopy[lang];
  const [note, setNote] = useState<{ text: string; error: boolean } | null>(null);

  const copyInfo = () => {
    const lines = [
      `PC Tweaker Uninstaller ${version ?? "(version unknown)"}`,
      `Windows: ${/Windows NT [\d.]+/.exec(navigator.userAgent)?.[0] ?? "unknown"}`,
      `Language: ${lang}`,
      ...facts,
      `Time: ${new Date().toISOString()}`,
    ];
    navigator.clipboard
      .writeText(lines.join("\n"))
      .then(() => {
        setNote({ text: c.copied, error: false });
      })
      .catch(() => {
        setNote({ text: c.copyFailed, error: true });
      });
  };
  const openLog = () => {
    invoke<string>("open_log_folder")
      .then((path) => {
        setNote({ text: text.ledger.logAt(path), error: false });
      })
      .catch((error: unknown) => {
        setNote({ text: typeof error === "string" ? error : text.errors.generic, error: true });
      });
  };

  return (
    <Dialog label={c.supportTitle} onClose={onClose}>
      {(title) => (
        <>
          <div className="account-dialog-head">
            <div>
              <h2 ref={title} tabIndex={-1}>
                {c.supportTitle}
              </h2>
              <p className="account-dialog-sub">{c.supportIntro}</p>
            </div>
            <button type="button" className="icon-button" aria-label={c.close} onClick={onClose}>
              <UiIcon name="close" />
            </button>
          </div>
          <div className="support-list">
            <button type="button" className="support-item support-primary" onClick={onOpenSupport}>
              <UiIcon name="lifebuoy" />
              <span>
                <strong>{c.contact}</strong>
                <small>{c.contactHint}</small>
              </span>
              <UiIcon name="external" />
            </button>
            <button type="button" className="support-item" onClick={copyInfo}>
              <UiIcon name="copy" />
              <span>
                <strong>{c.copyInfo}</strong>
                <small>PC Tweaker Uninstaller {version ?? ""} · Windows</small>
              </span>
            </button>
            <button type="button" className="support-item" onClick={openLog}>
              <UiIcon name="folder" />
              <span>
                <strong>{c.openLog}</strong>
                <small>{c.logHint}</small>
              </span>
            </button>
          </div>
          {note && (
            <p className={note.error ? "sub-inline-error" : "sub-done"} role="status">
              {!note.error && <UiIcon name="check" />}
              {note.text}
            </p>
          )}
        </>
      )}
    </Dialog>
  );
}
