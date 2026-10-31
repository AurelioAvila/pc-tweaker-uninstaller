// The checks job, in one place: CI (.github/workflows/checks.yml) runs this
// after `npm ci` and the production audit, and so do the local gate
// (tools/github-indipendenza/ci-local.ps1, for when Actions cannot run) and
// `npm run check` before a release. Stops at the first failing step.
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const rust = fileURLToPath(new URL("../src-tauri/", import.meta.url));
const steps = [
  ["TypeScript", "npx tsc --noEmit"],
  ["Release verification regression tests", "node --test scripts/release-verification.test.mjs"],
  ["Release tag and cleanup safety tests", "pwsh -NoProfile -File scripts/test-release-safety.ps1"],
  ["Inventory sorting and date regression tests", "node --test scripts/inventory.test.mjs"],
  ["Plans page price regression tests", "node --test scripts/plans-price.test.mjs"],
  ["Free trial state and copy tests", "node --test scripts/trial.test.mjs"],
  ["Default language regression tests", "node --test scripts/i18n-default.test.mjs"],
  ["Translation table tests", "node --test scripts/i18n-tables.test.mjs"],
  ["Account request regression tests", "node --test scripts/account-registration.test.mjs"],
  ["Tauri JavaScript packages on their crates' versions", "node --test scripts/tauri-versions.test.mjs"],
  ["Accent contrast", "npm run check:contrast"],
  ["ESLint", "npm run lint"],
  ["Prettier", "npm run format:check"],
  ["Frontend build", "npm run build"],
  ["Rust formatting", "cargo fmt --check", rust],
  ["Clippy (warnings are errors)", "cargo clippy -- -D warnings", rust],
  ["Rust tests", "cargo test", rust],
];
for (const [name, command, cwd = root] of steps) {
  console.log(`\n==> ${name}: ${command}`);
  const run = spawnSync(command, { cwd, shell: true, stdio: "inherit" });
  if (run.status !== 0) {
    console.error(`\nFAILED: ${name} (exit ${String(run.status ?? run.signal)})`);
    process.exit(run.status || 1);
  }
}
console.log(`\nAll ${String(steps.length)} checks passed.`);
