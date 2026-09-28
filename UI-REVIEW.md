# Uninstaller UI review — 28 September 2026

Branch: `feature/uninstaller-ui`.
Baseline: `c173e5e` (official 0.11.3). The original checkout at `C:/Users/aurel/Desktop/pc-tweaker-uninstaller` was not changed.

## Changes
- Inventory workspace with large/recent shortcuts, refresh time, clearer rows, visible uninstall actions and explicit details affordance.
- Native application icons retained; vector fallback and restrained hover motion. Official application icon unchanged.
- Profile uses expandable account, plan, language and theme sections. Named theme choices, current preferences, Escape/focus return and manual update check.
- Signed Tauri updater: startup, hourly and focus/online checks, manual retry, visible offer, actual/indeterminate download progress, duplicate-install guard. Removal actions are inaccessible while an update installs; the update button is disabled during removal flows.
- Copy localized in all five supported languages. Prices, licensing and removal safeguards unchanged.

## Validation
- TypeScript/Vite build and ESLint pass.
- Existing inventory and account-registration checks pass; accent contrast passes all eight themes.
- `scripts/workspace-ui.test.mjs` passes with mocked IPC: 940/1120/1920 widths, column bounds, search, shortcuts, keyboard expansion, menu/locales, all eight themes, updater offer/dismiss/recheck/offline/no-update/unknown and known progress/failure/success/duplicate-click handling. No real uninstall or update install was executed by the tests.
- Native dev build succeeds with CARGO_PROFILE_DEV_DEBUG=0 and CARGO_INCREMENTAL=0. The initial symbol-heavy build ran out of disk; only its newly generated debug cache was removed before rebuilding.
- Actual native window inspected; 188 installed programs displayed with local Windows icons. Native screenshot retained in ignored `ui-evidence/native-inventory.png`.
- New-release notification exercised with a simulated release; no fake public release or live installation was created.

## Preview and rollback
Vite is running on port 1421, backed by this checkout. Native development app is open; its launcher uses a hidden terminal. Local launch files/logs are in ignored `.local-preview/`.

To return to the baseline after stopping the preview, use `git switch --detach c173e5e` in this checkout, then rebuild/restart. Return to these changes using `git switch feature/uninstaller-ui`. This assumes the working tree is clean; preserve any subsequent edits before switching.

No release, signing, publishing, billing changes or announcements were performed.
