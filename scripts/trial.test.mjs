import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import ts from "typescript";

const load = async (file) => {
  const code = ts.transpileModule(fs.readFileSync(new URL(file, import.meta.url), "utf8"), {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  }).outputText;
  return import("data:text/javascript;base64," + Buffer.from(code).toString("base64"));
};
const { parseBillingStatus, parsePriceList, trialOffer, trialUsed, trialLeft, billingSnapshot, billingChanges } =
  await load("../src/trial.ts");

const row = (over = {}) => ({
  product: "uninstaller",
  state: "none",
  plan: null,
  trial_ends_at: null,
  current_period_end: null,
  cancel_at_period_end: false,
  can_start_trial: true,
  blocked_reason: "none",
  ...over,
});
const status = (rows, over = {}) =>
  parseBillingStatus({ trial_enabled: true, trial_days: 3, portal_available: true, products: rows, ...over });
const prices = parsePriceList({
  trial_enabled: true,
  trial_days: 3,
  currency: "eur",
  prices: [{ product: "uninstaller", plan: "annual", amount: 999, interval: "year" }],
});

test("the trial button needs the server switch, eligibility and a server price", () => {
  assert.deepEqual(trialOffer("uninstaller", "annual", prices, null), { days: 3, amount: 999 });
  assert.deepEqual(trialOffer("uninstaller", "annual", prices, status([row()])), { days: 3, amount: 999 });
  assert.equal(trialOffer("uninstaller", "annual", null, null), null);
  assert.equal(trialOffer("uninstaller", "annual", prices, status([row({ can_start_trial: false })])), null);
  assert.equal(trialOffer("uninstaller", "annual", prices, status([row()], { trial_enabled: false })), null);
  // A loyalty account's own renewal price wins over the public list.
  assert.deepEqual(trialOffer("uninstaller", "annual", prices, status([row({ renew_amount: 499 })])), {
    days: 3,
    amount: 499,
  });
});

test("unknown states and reasons follow the contract; trial use is reported", () => {
  const odd = status([row({ state: "paused" })]);
  assert.equal(odd.products[0].state, "none");
  assert.equal(status([row({ state: "blocked", blocked_reason: "x" })]).products[0].blockedReason, "canceled");
  assert.equal(trialUsed("uninstaller", status([row({ can_start_trial: false })])), true);
  assert.equal(trialUsed("uninstaller", status([row({ state: "active", can_start_trial: false })])), false);
});

test("countdown and the one notice on return", () => {
  const end = Date.parse("2026-10-17T10:00:00Z");
  const left = trialLeft(end, 3, end - 26 * 3_600_000);
  assert.deepEqual([left.days, left.hours], [1, 2]);
  const seen = billingSnapshot(status([row({ state: "trialing", trial_ends_at: "2026-10-17T10:00:00Z" })]));
  const now = (state, blocked_reason = "none") => status([row({ state, blocked_reason })]);
  assert.deepEqual(billingChanges(seen, now("blocked", "payment_failed")), [{ product: "uninstaller", kind: "paused" }]);
  assert.deepEqual(billingChanges(seen, now("active")), [{ product: "uninstaller", kind: "converted" }]);
  assert.deepEqual(billingChanges(null, now("blocked", "payment_failed")), []);
});

test("every trial string exists in every language with the same placeholders", async () => {
  const { trialCopy } = await load("../src/trial-copy.ts");
  const holes = (text) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join();
  for (const [lang, copy] of Object.entries(trialCopy)) {
    for (const [key, value] of Object.entries(trialCopy.en)) {
      assert.equal(typeof copy[key], "string", `${lang}.${key}`);
      assert.equal(holes(copy[key]), holes(value), `${lang}.${key}`);
    }
  }
  assert.deepEqual(Object.keys(trialCopy).sort(), ["de", "en", "es", "fr", "it", "pt"]);
  assert.doesNotMatch(JSON.stringify(trialCopy.pt), /\b(arquivo|usuário|tela|baixar)\b/i, "pt-PT, not pt-BR");
});
