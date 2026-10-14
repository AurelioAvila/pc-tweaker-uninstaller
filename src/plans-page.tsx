import { useEffect, useRef, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { startBillingPortal, type AccountState, type UninstallerEntitlement } from "./account";
import { text, type Locale } from "./i18n";
import { HalloweenOfferBanner } from "./halloween-offer";
import { plansCopy } from "./plans-copy";
import { LOYALTY_CENTS, money as formatMoney, proPrice } from "./plans-price";
import { promoPercent } from "./promo";
import { UiIcon } from "./ui-icon";
import { usePromo } from "./use-promo";
import "./plans.css";

/** Six capabilities are free; the last three in `rows` are Pro only. */
const FREE_ROWS = 6;

export type CheckoutControls = {
  busy: boolean;
  started: boolean;
  failed: boolean;
  begin: () => void;
  refresh: () => void;
};

export function PlansPage({
  lang,
  account,
  entitlement,
  checkout,
  onBack,
  onOpenAccount,
  onRefreshAccount,
  onOpenPrivacy,
}: {
  lang: Locale;
  account: AccountState;
  entitlement: UninstallerEntitlement | null;
  checkout: CheckoutControls;
  onBack: () => void;
  onOpenAccount: (mode: "signIn" | "register") => void;
  onRefreshAccount: () => void;
  onOpenPrivacy: () => void;
}) {
  const copy = plansCopy[lang];
  const locale = lang === "pt" ? "pt-PT" : lang;
  const money = (cents: number) => formatMoney(cents, locale);
  const day = (iso: string) =>
    new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(new Date(iso));

  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, []);
  const [portal, setPortal] = useState<"idle" | "busy" | "error">("idle");
  const promo = usePromo();

  const signedIn = account.status === "signed-in";
  const loyalty = signedIn && account.isPro;
  const proActive = entitlement?.active === true;
  const bonus = proActive && entitlement.plan === "lifetime_bonus";
  const offer = promo.offer("uninstaller", "annual");
  const { shown: shownOffer, price } = proPrice(loyalty, proActive, offer);
  const reservePromo = !proActive && !loyalty && shownOffer === null && promo.reserve;
  const promoEnd = promo.promo?.endsAt;

  const manage = () => {
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

  let proAction: ReactNode;
  if (bonus) {
    proAction = <p className="plans-status">{copy.bonusStatus}</p>;
  } else if (proActive) {
    proAction =
      signedIn && account.hasBilling ? (
        <button
          type="button"
          className="button plans-cta"
          disabled={portal === "busy"}
          aria-busy={portal === "busy"}
          onClick={manage}
        >
          {copy.manage}
        </button>
      ) : (
        <p className="plans-status">{copy.proActiveStatus}</p>
      );
  } else if (!signedIn) {
    proAction = (
      <>
        <button
          type="button"
          className="button plans-cta"
          disabled={account.status === "checking"}
          onClick={() => {
            onOpenAccount("signIn");
          }}
        >
          {copy.goPro}
        </button>
        {account.status === "anonymous" && <p className="plans-hint">{copy.signInFirst}</p>}
      </>
    );
  } else {
    proAction = (
      <>
        <button
          type="button"
          className="button plans-cta"
          disabled={checkout.busy}
          aria-busy={checkout.busy}
          onClick={checkout.begin}
        >
          {copy.goPro}
        </button>
        {!account.emailVerified && <p className="plans-hint">{copy.unverified}</p>}
      </>
    );
  }

  const mark = (yes: boolean) =>
    yes ? (
      <span className="plans-yes">
        <UiIcon name="check" />
        <span className="sr-only">{copy.included}</span>
      </span>
    ) : (
      <span className="plans-no">
        <span aria-hidden="true">—</span>
        <span className="sr-only">{copy.notIncluded}</span>
      </span>
    );

  return (
    <section className="plans-page" aria-labelledby="plans-title">
      <div className="plans-inner">
        <button type="button" className="button-ghost small plans-back" onClick={onBack}>
          <UiIcon name="arrowLeft" />
          {copy.back}
        </button>
        <header className="plans-heading">
          <p className="plans-eyebrow">{copy.nav}</p>
          <h2 id="plans-title" ref={heading} tabIndex={-1}>
            {copy.title}
          </h2>
          <p>{copy.intro}</p>
        </header>

        <section className="plans-account" aria-labelledby="plans-account-title">
          <span className="plans-account-icon" aria-hidden="true">
            <UiIcon name={proActive ? "crown" : "user"} />
          </span>
          <div className="plans-account-body" role="status">
            <h3 id="plans-account-title">{copy.accountTitle}</h3>
            {account.status === "anonymous" && (
              <>
                <p className="plans-account-main">{copy.signedOut}</p>
                <p>{copy.signedOutBody}</p>
              </>
            )}
            {account.status === "checking" && (
              <p className="plans-account-main">
                <span className="spinner" aria-hidden="true" />
                {copy.checking}
              </p>
            )}
            {account.status === "error" && (
              <>
                <p className="plans-account-main">{account.email}</p>
                <p>{copy.offline}</p>
              </>
            )}
            {account.status === "signed-in" && (
              <>
                <p className="plans-account-main">
                  <span className="plans-email">{account.email}</span>
                  <span className={`plans-pill${proActive ? " plans-pill-pro" : ""}`}>
                    {proActive ? copy.proPlan : copy.freePlan}
                  </span>
                </p>
                {proActive && (
                  <p>
                    {bonus && entitlement.expiresAt
                      ? copy.bonusUntil(day(entitlement.expiresAt))
                      : entitlement.expiresAt
                        ? copy.activeUntil(day(entitlement.expiresAt))
                        : copy.active}
                  </p>
                )}
                {loyalty && !proActive && <p>{copy.loyaltyNote(money(LOYALTY_CENTS))}</p>}
                {!account.emailVerified && !proActive && <p>{copy.unverified}</p>}
              </>
            )}
          </div>
          <div className="plans-account-actions">
            {account.status === "anonymous" && (
              <>
                <button
                  type="button"
                  className="button"
                  onClick={() => {
                    onOpenAccount("signIn");
                  }}
                >
                  {copy.signIn}
                </button>
                <button
                  type="button"
                  className="button-ghost"
                  onClick={() => {
                    onOpenAccount("register");
                  }}
                >
                  {copy.createAccount}
                </button>
              </>
            )}
            {(account.status === "signed-in" || account.status === "error") && (
              <button type="button" className="button-ghost" onClick={onRefreshAccount}>
                <UiIcon name="refresh" />
                {account.status === "error" ? copy.retry : copy.refresh}
              </button>
            )}
          </div>
        </section>

        {(shownOffer !== null || reservePromo) && (
          <HalloweenOfferBanner
            kicker={copy.promoTitle}
            badge={promo.preview && <span className="plans-preview">{copy.preview}</span>}
            heading={
              <>
                {copy.ends}:{" "}
                <time dateTime={promoEnd}>
                  {promoEnd
                    ? new Intl.DateTimeFormat(locale, {
                        dateStyle: "long",
                        timeStyle: "short",
                      }).format(new Date(Date.parse(promoEnd) - 60_000))
                    : ""}
                </time>
              </>
            }
            fine={copy.promoReference}
            endsIn={copy.endsIn}
            units={copy.units}
            remaining={shownOffer !== null ? promo.remaining : 0}
            reserved={shownOffer === null}
            headingId="plans-promo-title"
            headingLevel={3}
          />
        )}

        <div className="plans-cards">
          <article className="plans-card" aria-labelledby="plans-free-name">
            <div className="plans-card-name">
              <h3 id="plans-free-name">{copy.freeName}</h3>
              {!proActive && <span className="plans-current">{copy.current}</span>}
            </div>
            <p className="plans-tagline">{copy.freeTagline}</p>
            <div className="plans-price">
              <strong>{money(0)}</strong>
            </div>
            <p className="plans-price-note">{copy.freeNote}</p>
            <div className="plans-card-action">
              <p className="plans-status">{proActive ? copy.includedInPro : copy.onFree}</p>
            </div>
            <ul className="plans-highlights">
              {copy.freeHighlights.map((line) => (
                <li key={line}>
                  <UiIcon name="check" />
                  {line}
                </li>
              ))}
            </ul>
          </article>

          <article
            className="plans-card plans-card-pro"
            aria-labelledby="plans-pro-name"
            aria-describedby="plans-badge"
          >
            <span className="plans-ribbon" id="plans-badge">
              {copy.recommended}
            </span>
            <div className="plans-card-name">
              <h3 id="plans-pro-name">
                <UiIcon name="crown" />
                {copy.proName}
              </h3>
              {proActive && <span className="plans-current">{copy.current}</span>}
            </div>
            <p className="plans-tagline">{copy.proTagline}</p>
            <div className="plans-price">
              {shownOffer !== null && (
                <s className="plans-was">
                  <span className="sr-only">{copy.was} </span>
                  {money(shownOffer.reference)}
                </s>
              )}
              <strong>
                {shownOffer !== null && <span className="sr-only">{copy.now} </span>}
                {money(price)}
              </strong>
              <span className="plans-period">{copy.perYear}</span>
              {shownOffer !== null && (
                <span className="plans-discount">-{promoPercent(shownOffer)}%</span>
              )}
            </div>
            <p className="plans-price-note">
              {loyalty
                ? copy.loyaltyPrice
                : shownOffer !== null
                  ? copy.firstYear(money(shownOffer.regular))
                  : copy.regularNote}
            </p>
            <div className="plans-card-action">
              {proAction}
              {signedIn && !proActive && checkout.started && (
                <>
                  <p className="plans-hint" role="status">
                    {copy.checkoutOpened}
                  </p>
                  <button type="button" className="button-ghost small" onClick={checkout.refresh}>
                    {copy.checkoutDone}
                  </button>
                </>
              )}
              {signedIn && !proActive && checkout.failed && (
                <p className="plans-hint plans-error" role="alert">
                  {copy.checkoutError}
                </p>
              )}
              {portal === "error" && (
                <p className="plans-hint plans-error" role="alert">
                  {copy.portalError}
                </p>
              )}
            </div>
            <ul className="plans-highlights">
              {copy.proHighlights.map((line) => (
                <li key={line}>
                  <UiIcon name="check" />
                  {line}
                </li>
              ))}
            </ul>
          </article>
        </div>
        <p className="plans-checkout-note">{copy.checkoutNote}</p>

        <section className="plans-section" aria-labelledby="plans-compare-title">
          <h3 id="plans-compare-title">{copy.compareTitle}</h3>
          <table className="plans-table">
            <thead>
              <tr>
                <th scope="col">{copy.capability}</th>
                <th scope="col">{copy.freeName}</th>
                <th scope="col">{copy.proName}</th>
              </tr>
            </thead>
            <tbody>
              {copy.rows.map((label, index) => (
                <tr key={label}>
                  <th scope="row">{label}</th>
                  <td>{mark(index < FREE_ROWS)}</td>
                  <td>{mark(true)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <div className="plans-columns">
          <section className="plans-section" aria-labelledby="plans-faq-title">
            <h3 id="plans-faq-title">{copy.faqTitle}</h3>
            <details className="plans-faq">
              <summary>
                {copy.renewQ}
                <UiIcon name="chevron" />
              </summary>
              <p>{copy.renewA}</p>
              {offer !== null && <p>{copy.promoTerms}</p>}
            </details>
            <details className="plans-faq">
              <summary>
                {copy.cancelQ}
                <UiIcon name="chevron" />
              </summary>
              <p>{copy.cancelA}</p>
            </details>
            <details className="plans-faq">
              <summary>
                {copy.lifetimeQ}
                <UiIcon name="chevron" />
              </summary>
              <p>{copy.lifetimeA}</p>
            </details>
            <details className="plans-faq">
              <summary>
                {copy.loyaltyQ}
                <UiIcon name="chevron" />
              </summary>
              <p>{copy.loyaltyA(money(LOYALTY_CENTS))}</p>
            </details>
          </section>

          <section className="plans-section plans-data" aria-labelledby="plans-data-title">
            <h3 id="plans-data-title">
              <UiIcon name="shield" />
              {copy.dataTitle}
            </h3>
            <ul>
              {copy.data.map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
            <button type="button" className="button-ghost small" onClick={onOpenPrivacy}>
              {copy.privacy}
            </button>
          </section>
        </div>
      </div>
    </section>
  );
}

/** The account menu's one-line price, from the same offer and rules as the
 *  page, so the menu never shows €9.99 while the page shows the offer. Its
 *  own component, so the countdown hook re-renders this line, not the app. */
export function ProPriceSummary({
  lang,
  loyalty,
  proActive,
}: {
  lang: Locale;
  loyalty: boolean;
  proActive: boolean;
}) {
  const copy = plansCopy[lang];
  const locale = lang === "pt" ? "pt-PT" : lang;
  const promo = usePromo();
  const { shown, price } = proPrice(loyalty, proActive, promo.offer("uninstaller", "annual"));
  return (
    <>
      <div className={loyalty ? "plan-row plan-loyalty" : "plan-row"}>
        <span>
          {loyalty && <strong>{text.menu.loyaltyTitle}</strong>}
          <em className="menu-price">
            {shown !== null && (
              <s className="plans-was">
                <span className="sr-only">{copy.was} </span>
                {formatMoney(shown.reference, locale)}
              </s>
            )}{" "}
            {formatMoney(price, locale)} {copy.perYear}
          </em>
        </span>
        {shown !== null && <span className="plans-discount">-{promoPercent(shown)}%</span>}
      </div>
      {shown !== null && (
        <p className="menu-hint">
          {copy.promoTitle} · {copy.firstYear(formatMoney(shown.regular, locale))}
        </p>
      )}
    </>
  );
}
