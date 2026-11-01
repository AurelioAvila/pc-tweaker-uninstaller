import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { startTip } from "./account";
import { UiIcon } from "./ui-icon";
import type { workspaceCopy } from "./workspace-copy";

/** The backend clamps to the same number; this only stops the stepper from
 *  offering a count that would be clamped. */
const MAX_COFFEES = 10;

/**
 * The tip entry: the last row of the account menu, the same size and shape
 * as Language and Theme above it, and quieter than Plans & pricing in the top
 * bar. Folded until asked for; open, it is a "how many coffees" stepper and
 * one button that opens Stripe Checkout in the browser (PC Tweaker's
 * CoffeeCard, adapted to this menu).
 *
 * A native <details>, like its sibling rows: closed, its body is not rendered,
 * so no hidden control can be reached by Tab or take a stray click.
 */
export function CoffeeRow({ copy }: { copy: (typeof workspaceCopy)["en"]["coffee"] }) {
  const [count, setCount] = useState(1);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);

  const tip = () => {
    setBusy(true);
    setFailed(false);
    startTip(count)
      .then((url) => invoke("open_checkout_url", { url }))
      .catch(() => {
        setFailed(true);
      })
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <details className="menu-section coffee">
      <summary className="coffee-row">
        <span className="coffee-mark" aria-hidden="true">
          ☕
        </span>
        {copy.nav}
        <UiIcon name="chevron" />
      </summary>
      <div className="menu-section-body coffee-body">
        <div className="coffee-stepper">
          <button
            type="button"
            className="coffee-step coffee-fewer"
            aria-label={copy.fewer}
            disabled={count <= 1}
            onClick={() => {
              setCount((n) => Math.max(1, n - 1));
            }}
          >
            −
          </button>
          {/* The live region stays put; only the inner span remounts (key),
              which replays the pop on every press. */}
          <span aria-live="polite">
            <span key={count} className="coffee-count">
              {count} ☕
            </span>
          </span>
          <button
            type="button"
            className="coffee-step coffee-more"
            aria-label={copy.more}
            disabled={count >= MAX_COFFEES}
            onClick={() => {
              setCount((n) => Math.min(MAX_COFFEES, n + 1));
            }}
          >
            +
          </button>
        </div>
        <button
          type="button"
          className="button-ghost small full coffee-cta"
          disabled={busy}
          aria-busy={busy}
          onClick={tip}
        >
          {copy.cta}
        </button>
        {failed && (
          <p className="menu-hint" role="alert">
            {copy.error}
          </p>
        )}
      </div>
    </details>
  );
}
