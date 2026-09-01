import { text, LOCALES, type Locale } from "../i18n";
import { THEMES, type ThemeCode } from "../themes";
import type { AccountState, UninstallerEntitlement } from "../account";

interface MenuPanelProps {
  setMenuOpen: React.Dispatch<React.SetStateAction<boolean>>;
  account: AccountState;
  uninstallerPro: UninstallerEntitlement | null;
  checkoutBusy: boolean;
  beginProCheckout: () => void;
  checkoutStarted: boolean;
  refreshEntitlement: () => void;
  checkoutError: boolean;
  signOut: () => void;
  submitLogin: () => void;
  loginEmail: string;
  setLoginEmail: React.Dispatch<React.SetStateAction<string>>;
  loginPassword: string;
  setLoginPassword: React.Dispatch<React.SetStateAction<string>>;
  loginError: string | null;
  openLink: (target: string) => void;
  suiteDetected: boolean;
  openPcTweaker: () => void;
  isProConfirmed: boolean;
  lang: Locale;
  chooseLang: (l: Locale) => void;
  theme: ThemeCode;
  setTheme: React.Dispatch<React.SetStateAction<ThemeCode>>;
}

export function MenuPanel({
  setMenuOpen,
  account,
  uninstallerPro,
  checkoutBusy,
  beginProCheckout,
  checkoutStarted,
  refreshEntitlement,
  checkoutError,
  signOut,
  submitLogin,
  loginEmail,
  setLoginEmail,
  loginPassword,
  setLoginPassword,
  loginError,
  openLink,
  suiteDetected,
  openPcTweaker,
  isProConfirmed,
  lang,
  chooseLang,
  theme,
  setTheme,
}: MenuPanelProps) {
  return (
    <>
      <div
        className="menu-scrim"
        role="presentation"
        onClick={() => {
          setMenuOpen(false);
        }}
      />
      <div className="menu-panel" role="dialog" aria-label={text.menu.open}>
        <section className="menu-section">
          <h3>{text.menu.account}</h3>

          {account.status === "signed-in" && (
            <>
              <div className="plan-row">
                <span>{account.email}</span>
              </div>
              {/* The ONLY place Pro status is asserted: read straight from
                  the account just verified against the backend, never
                  from local PC Tweaker detection. */}
              <p className={account.isPro ? "menu-hint menu-hint-ok" : "menu-hint"}>
                {account.isPro ? text.menu.proActive : text.menu.proInactive}
              </p>
              {/* Uninstaller Pro: read from /api/entitlements, purchased
                  via Stripe Checkout in the system browser. The backend
                  picks the loyalty price server-side; the label here only
                  mirrors what it will charge. */}
              <p className={uninstallerPro?.active ? "menu-hint menu-hint-ok" : "menu-hint"}>
                {uninstallerPro?.active ? text.menu.upsActive : text.menu.upsInactive}
              </p>
              {!uninstallerPro?.active && (
                <>
                  <button
                    type="button"
                    className="button-ghost small full"
                    disabled={checkoutBusy}
                    onClick={beginProCheckout}
                  >
                    {account.isPro ? text.menu.upsGoProLoyalty : text.menu.upsGoPro}
                  </button>
                  {checkoutStarted && (
                    <>
                      <p className="menu-hint">{text.menu.upsCheckoutHint}</p>
                      <button
                        type="button"
                        className="button-ghost small full"
                        onClick={refreshEntitlement}
                      >
                        {text.menu.upsRefresh}
                      </button>
                    </>
                  )}
                  {checkoutError && <p className="menu-hint">{text.menu.upsError}</p>}
                </>
              )}
              <button
                type="button"
                className="button-ghost small full"
                onClick={() => {
                  signOut();
                }}
              >
                {text.menu.signOut}
              </button>
            </>
          )}

          {account.status !== "signed-in" && (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                submitLogin();
              }}
            >
              <input
                type="email"
                className="search"
                placeholder={text.menu.emailLabel}
                value={loginEmail}
                autoComplete="email"
                onChange={(e) => {
                  setLoginEmail(e.target.value);
                }}
              />
              <input
                type="password"
                className="search"
                style={{ marginTop: 6 }}
                placeholder={text.menu.passwordLabel}
                value={loginPassword}
                autoComplete="current-password"
                onChange={(e) => {
                  setLoginPassword(e.target.value);
                }}
              />
              <button
                type="submit"
                className="button small full"
                style={{ marginTop: 8 }}
                disabled={account.status === "checking"}
              >
                {account.status === "checking" ? text.menu.signingIn : text.menu.signInButton}
              </button>
              {account.status === "error" && <p className="detail-notice">{account.message}</p>}
              {loginError !== null && <p className="detail-notice">{loginError}</p>}
              <p className="menu-hint">{text.menu.registerHint}</p>
              <button
                type="button"
                className="button-ghost small full"
                onClick={() => {
                  openLink("account");
                }}
              >
                {text.menu.signIn}
              </button>
            </form>
          )}

          {suiteDetected && (
            <button
              type="button"
              className="button-ghost small full"
              style={{ marginTop: 8 }}
              onClick={openPcTweaker}
            >
              {text.menu.openPcTweaker}
            </button>
          )}
        </section>

        <section className="menu-section">
          <h3>{text.menu.plans}</h3>
          {/* Loyalty pricing requires a VERIFIED Pro account, not merely
              PC Tweaker being present on this PC. isProConfirmed is the
              one gate every price/perk in this section reads. */}
          {isProConfirmed ? (
            <div className="plan-row plan-loyalty">
              <span>
                <strong>{text.menu.loyaltyTitle}</strong>
                <em>{text.menu.loyaltyPrice}</em>
              </span>
            </div>
          ) : (
            account.status !== "signed-in" && <p className="menu-hint">{text.menu.loyaltyLocked}</p>
          )}
          <div className="plan-row">
            <span>{text.menu.planMonthly}</span>
          </div>
          <div className="plan-row">
            <span>{text.menu.planAnnual}</span>
          </div>
          <p className="menu-hint">{text.menu.loyaltyHint}</p>
          <button
            type="button"
            className="button-ghost small full"
            onClick={() => {
              openLink("pricing");
            }}
          >
            {text.menu.choosePlans}
          </button>
        </section>

        <section className="menu-section">
          <h3>{text.menu.language}</h3>
          <div className="menu-chips">
            {LOCALES.map((l) => (
              <button
                key={l.code}
                type="button"
                className={`chip chip-button${lang === l.code ? " chip-active" : ""}`}
                aria-pressed={lang === l.code}
                onClick={() => {
                  chooseLang(l.code);
                }}
              >
                {l.native}
              </button>
            ))}
          </div>
        </section>

        <section className="menu-section">
          <h3>{text.menu.theme}</h3>
          <div className="menu-swatches">
            {THEMES.map((t) => (
              <button
                key={t.code}
                type="button"
                className={`swatch${theme === t.code ? " swatch-active" : ""}`}
                title={t.label}
                aria-label={t.label}
                aria-pressed={theme === t.code}
                style={{ background: t.vars.accent }}
                onClick={() => {
                  setTheme(t.code);
                }}
              />
            ))}
          </div>
        </section>
      </div>
    </>
  );
}
