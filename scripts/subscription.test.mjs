import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import ts from "typescript";

// Manage subscription: every state the family API can answer, parsed and
// described the way the panel shows it (src/subscription.ts). Mocks only:
// nothing here talks to Stripe or the live API.
const load = async (file) => {
  const code = ts.transpileModule(fs.readFileSync(new URL(file, import.meta.url), "utf8"), {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  }).outputText;
  return import("data:text/javascript;base64," + Buffer.from(code).toString("base64"));
};
const { parseSubscription, subscriptionView, endsOn, formatAmount, cardBrand } =
  await load("../src/subscription.ts");

const answer = (over = {}) => ({
  product: "uninstaller",
  plan: "annual",
  interval: "year",
  amount: 999,
  currency: "eur",
  status: "active",
  trial_end: null,
  current_period_end: "2027-03-12T10:00:00.000Z",
  cancel_at_period_end: false,
  cancel_at: null,
  payment_method: { brand: "visa", last4: "4242" },
  lifetime: false,
  can_cancel: true,
  can_resume: false,
  ...over,
});

test("a renewing subscription can be cancelled, not resumed", () => {
  const s = parseSubscription(answer());
  assert.deepEqual(subscriptionView(s), { kind: "renews", on: "2027-03-12T10:00:00.000Z" });
  assert.equal(s.canCancel, true);
  assert.equal(s.canResume, false);
  assert.deepEqual(s.card, { brand: "visa", last4: "4242" });
});

test("cancel at period end keeps access to the paid date and offers resume", () => {
  const s = parseSubscription(
    answer({ cancel_at_period_end: true, cancel_at: "2027-03-12T10:00:00.000Z", can_cancel: false, can_resume: true }),
  );
  assert.deepEqual(subscriptionView(s), { kind: "ending", until: "2027-03-12T10:00:00.000Z" });
  assert.equal(s.canCancel, false);
  assert.equal(s.canResume, true);
  // cancel_at wins over the period end when both are set (the API's rule).
  assert.equal(endsOn(parseSubscription(answer({ cancel_at_period_end: true, cancel_at: "2027-01-01T00:00:00.000Z" }))), "2027-01-01T00:00:00.000Z");
});

test("a trial shows its end; cancelled during the trial it ends then, with no charge", () => {
  const trial = parseSubscription(answer({ status: "trialing", trial_end: "2026-11-04T10:00:00.000Z" }));
  assert.deepEqual(subscriptionView(trial), { kind: "trial", until: "2026-11-04T10:00:00.000Z" });
  const ending = parseSubscription(
    answer({ status: "trialing", trial_end: "2026-11-04T10:00:00.000Z", cancel_at_period_end: true, can_cancel: false, can_resume: true }),
  );
  assert.deepEqual(subscriptionView(ending), { kind: "ending", until: "2026-11-04T10:00:00.000Z" });
});

test("lifetime, included and none have nothing to cancel or resume", () => {
  const lifetime = parseSubscription(
    answer({ plan: "lifetime", status: "lifetime", lifetime: true, amount: null, currency: null, interval: null, current_period_end: null, payment_method: null, can_cancel: false }),
  );
  assert.deepEqual(subscriptionView(lifetime), { kind: "lifetime" });
  assert.equal(lifetime.canCancel || lifetime.canResume, false);
  const included = parseSubscription(answer({ status: "included", plan: "lifetime_bonus", can_cancel: false }));
  assert.deepEqual(subscriptionView(included), { kind: "included", until: "2027-03-12T10:00:00.000Z" });
  assert.equal(included.canCancel, false);
  const none = parseSubscription(answer({ status: "none", plan: null, amount: null, can_cancel: false }));
  assert.deepEqual(subscriptionView(none), { kind: "none" });
  assert.equal(none.amount, null);
  // Even a server that wrongly says can_cancel on a non-live state is not obeyed.
  assert.equal(parseSubscription(answer({ status: "lifetime", lifetime: true, can_cancel: true })).canCancel, false);
});

test("payment problems point at the card, not at cancelling", () => {
  assert.deepEqual(subscriptionView(parseSubscription(answer({ status: "past_due" }))), { kind: "pastDue" });
  assert.deepEqual(subscriptionView(parseSubscription(answer({ status: "unpaid" }))), { kind: "pastDue" });
  assert.deepEqual(subscriptionView(parseSubscription(answer({ status: "paused" }))), { kind: "paused" });
});

test("the SPEC shape without can_cancel/can_resume falls back to the same rule", () => {
  const { can_cancel, can_resume, ...bare } = answer();
  void can_cancel;
  void can_resume;
  assert.equal(parseSubscription(bare).canCancel, true);
  const ending = parseSubscription({ ...bare, cancel_at_period_end: true });
  assert.equal(ending.canCancel, false);
  assert.equal(ending.canResume, true);
});

test("malformed answers are rejected instead of guessed", () => {
  assert.equal(parseSubscription(null), null);
  assert.equal(parseSubscription({ error: "no_subscription" }), null);
  assert.equal(parseSubscription(answer({ product: "someone-else" })), null);
  assert.equal(parseSubscription(answer({ status: "weird" })), null);
  // A non-card or broken payment method is simply not shown.
  assert.equal(parseSubscription(answer({ payment_method: { brand: "visa", last4: "42x2" } })).card, null);
  assert.equal(parseSubscription(answer({ current_period_end: "not a date" })).periodEnd, null);
});

test("prices and card brands read naturally", () => {
  assert.equal(formatAmount(999, "eur", "en"), "€9.99");
  assert.equal(formatAmount(1000, "eur", "en"), "€10");
  assert.equal(cardBrand("amex"), "American Express");
  assert.equal(cardBrand("link"), "Link");
});
