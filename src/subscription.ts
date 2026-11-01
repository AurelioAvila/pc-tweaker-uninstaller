/**
 * Manage subscription: the family billing API on api.pctweaker.app
 * (GET /api/billing/subscription, POST …/cancel and …/resume), parsed and
 * turned into what the panel shows. Pure, so scripts/subscription.test.mjs
 * covers every state without a network.
 */

export type FamilyProduct = "uninstaller" | "pctweaker" | "tweaky";

export type Subscription = {
  product: FamilyProduct;
  plan: string | null;
  interval: "month" | "year" | null;
  /** Minor units per interval, before tax. */
  amount: number | null;
  currency: string | null;
  status:
    "active" | "trialing" | "past_due" | "unpaid" | "paused" | "lifetime" | "included" | "none";
  trialEnd: string | null;
  periodEnd: string | null;
  cancelAtPeriodEnd: boolean;
  cancelAt: string | null;
  card: { brand: string; last4: string } | null;
  canCancel: boolean;
  canResume: boolean;
};

const PRODUCTS: FamilyProduct[] = ["uninstaller", "pctweaker", "tweaky"];
const STATUSES: Subscription["status"][] = [
  "active",
  "trialing",
  "past_due",
  "unpaid",
  "paused",
  "lifetime",
  "included",
  "none",
];
/** Stripe states with a live subscription behind them: the only ones that
 *  can be cancelled or resumed. */
const LIVE: Subscription["status"][] = ["active", "trialing", "past_due", "unpaid", "paused"];

const iso = (v: unknown): string | null =>
  typeof v === "string" && Number.isFinite(Date.parse(v)) ? v : null;

/** Null for anything that is not a subscription answer: the panel then shows
 *  its error state instead of guessing. */
export function parseSubscription(value: unknown): Subscription | null {
  const v = value as Record<string, unknown> | null;
  if (!v || typeof v !== "object" || !PRODUCTS.includes(v.product as FamilyProduct)) return null;
  const lifetime = v.lifetime === true;
  const raw = STATUSES.includes(v.status as Subscription["status"])
    ? (v.status as Subscription["status"])
    : null;
  if (!raw) return null;
  const status = lifetime && raw === "none" ? "lifetime" : raw;
  const live = LIVE.includes(status);
  const ending = live && v.cancel_at_period_end === true;
  const pm = v.payment_method as Record<string, unknown> | null | undefined;
  const amount =
    Number.isInteger(v.amount) && (v.amount as number) > 0 ? (v.amount as number) : null;
  return {
    product: v.product as FamilyProduct,
    plan: typeof v.plan === "string" ? v.plan : null,
    interval: v.interval === "month" || v.interval === "year" ? v.interval : null,
    amount,
    currency: typeof v.currency === "string" && amount !== null ? v.currency : null,
    status,
    trialEnd: status === "trialing" ? iso(v.trial_end) : null,
    periodEnd: iso(v.current_period_end),
    cancelAtPeriodEnd: ending,
    cancelAt: ending ? iso(v.cancel_at) : null,
    card:
      pm && typeof pm.brand === "string" && typeof pm.last4 === "string" && /^\d{4}$/.test(pm.last4)
        ? { brand: pm.brand, last4: pm.last4 }
        : null,
    // The server's word when it gives one; otherwise the same rule it uses.
    canCancel: typeof v.can_cancel === "boolean" ? v.can_cancel && live : live && !ending,
    canResume: typeof v.can_resume === "boolean" ? v.can_resume && live : ending,
  };
}

/** What the panel says about one subscription. */
export type SubscriptionView =
  | { kind: "none" }
  | { kind: "lifetime" }
  | { kind: "included"; until: string | null }
  | { kind: "trial"; until: string | null }
  | { kind: "renews"; on: string | null }
  | { kind: "ending"; until: string | null }
  | { kind: "pastDue" }
  | { kind: "paused" };

export function subscriptionView(s: Subscription): SubscriptionView {
  switch (s.status) {
    case "none":
      return { kind: "none" };
    case "lifetime":
      return { kind: "lifetime" };
    case "included":
      return { kind: "included", until: s.periodEnd };
    case "past_due":
    case "unpaid":
      return { kind: "pastDue" };
    case "paused":
      return { kind: "paused" };
    default:
      // Cancelled during a trial: access to the trial's end, no charge.
      if (s.cancelAtPeriodEnd)
        return {
          kind: "ending",
          until: s.status === "trialing" ? (s.trialEnd ?? endsOn(s)) : endsOn(s),
        };
      if (s.status === "trialing") return { kind: "trial", until: s.trialEnd };
      return { kind: "renews", on: s.periodEnd };
  }
}

/** The last day of access once cancelled: the API's rule, cancel_at first. */
export function endsOn(s: Subscription): string | null {
  return s.cancelAt ?? s.periodEnd;
}

/** "€9.99" in the app's language; whole amounts without decimals. */
export function formatAmount(minor: number, currency: string, locale: string): string {
  try {
    return new Intl.NumberFormat(locale, {
      style: "currency",
      currency: currency.toUpperCase(),
      minimumFractionDigits: minor % 100 === 0 ? 0 : 2,
    }).format(minor / 100);
  } catch {
    return `${(minor / 100).toFixed(2)} ${currency.toUpperCase()}`;
  }
}

/** "Visa", "Mastercard", "American Express" from Stripe's brand ids. */
export function cardBrand(brand: string): string {
  const names: Record<string, string> = {
    visa: "Visa",
    mastercard: "Mastercard",
    amex: "American Express",
    discover: "Discover",
    diners: "Diners Club",
    jcb: "JCB",
    unionpay: "UnionPay",
    cartes_bancaires: "Cartes Bancaires",
  };
  return names[brand.toLowerCase()] ?? brand.charAt(0).toUpperCase() + brand.slice(1);
}

/** Why a request failed, as a key of the panel's copy. */
export type SubscriptionErrorKey = "offline" | "auth" | "rate" | "billing";
