# Changelog

## v0.8.0 — 2026-08-29

Microsoft Store apps are now listed and removable.

Until now this app read the three registry Uninstall views — the same
ones Add/Remove Programs has read for twenty years. Modern Windows keeps
a second, parallel world of MSIX packages that appear in Settings >
Installed apps but in no registry key at all. On a normal Windows 11
machine that was over a hundred programs an uninstaller was quietly
claiming did not exist.

- Store/MSIX packages appear in the same list, searchable and sortable
  alongside everything else, with their own badge and install date.
- The Confidence Score bands them from evidence Windows itself reports:
  frameworks and shell components are Keep, packages signed as part of
  Windows are Review, ordinary Store installs are Safe.
- Frameworks, resource packages and components Windows gives no display
  name to sit behind the existing "Show hidden" toggle rather than
  cluttering the list — the same treatment registry system components
  already get.
- Removal goes through the WinRT package manager, the same API Settings
  uses. Not PowerShell: this app refuses to run interpreters for
  registry uninstalls, and shelling out to `Remove-AppxPackage` would
  have made that rule cosmetic.
- No residue step for these: MSIX removal is contained by the packaging
  format, and the app says so instead of running a scan that would find
  nothing.

One limit is stated plainly in the code and in the UI copy: without
administrator rights Windows gives no way to tell an app that shipped
with the system from one you installed — Notepad and Spotify report
identical evidence. Rather than hardcode a list of Microsoft package
names and present a guess as evidence, inbox apps can land in Safe, and
Safe for an MSIX package says what it means: removal leaves nothing
behind and the app can be reinstalled from the Store.

23 new tests, including live ones that exercise the real package manager
on the machine running them.

## v0.7.1 — 2026-08-23

Fixed: a program could stay listed after being uninstalled. Executable-
kind uninstallers (NSIS/Inno, common for Electron apps) can report
success before the registry key is actually gone; the app now waits for
it to disappear before refreshing the list. MSI uninstalls were
unaffected.

## v0.7.0 — 2026-08-21

Pro gating: residue cleanup and Safe Batch execution require
Uninstaller Pro; residue scanning, batch preview and every single-
uninstall feature stay free.

## v0.6.0 — 2026-08-21

Uninstaller Pro purchasable in-app: Stripe checkout in the system
browser, server-side loyalty pricing (4.99/yr with PC Tweaker Pro,
13.99/yr otherwise), entitlement shown in the account menu.

## v0.5.0 — 2026-08-21

Safe Batch: multi-select removal with protected entries unselectable,
contained-before-container ordering, per-item ledger receipts.
Removal Intelligence complete.

## v0.4.0 — 2026-08-21

Relationship Map: containment-based dependents ('removing this also
takes down...'), installed-via attribution, publisher siblings — all
derived from install paths and publishers, never guessed.

## v0.3.0 — 2026-08-21

Residue Intelligence: post-uninstall leftover scan (install folder,
AppData/ProgramData, shortcuts, registry) with Recycle Bin cleanup,
exact-match conservative detection, and double path validation.
Localized in five languages. 6 new unit tests.

All notable changes to PC Tweaker Uninstaller are logged here, newest
first.

## v0.2.0 "Removal Intelligence" — 2026-08-21

The first public release, and the reason this app exists: remove software
with clarity, not guesswork.

### Added

- **Removal Confidence Score.** Every installed program is banded
  **Safe to remove**, **Review before removing**, or **Keep —
  system-related**, from visible registry evidence: shared runtimes
  (Visual C++, .NET, WebView2, Java), driver and chipset packages, game
  launchers, missing publishers, broken uninstall commands. Every verdict
  lists its reasons line by line, and every verdict carries the same
  disclaimer: a guide, never a certainty.
- **Removal Brief.** Before anything runs you see the method, the
  privileges required, the estimated space to reclaim, the confidence
  band with its reasons, the exact command that will execute — and what
  will NOT be removed automatically.
- **Clean Removal Ledger.** A local receipt for every removal, successes
  and failures alike: restore-point outcome, exit code, and disk space
  **verified** by measuring the install folder before and after — never
  just the registry's claim. Exportable as JSON. Stored on this PC,
  never uploaded.
- **Automatic updates.** Signed builds now update in place: the app
  checks GitHub at startup, verifies the signature before installing,
  and restarts itself.
- **Five languages** (English, Italiano, Français, Español, Deutsch) and
  **eight themes**, both persisted.
- **One suite account.** Sign in with your PC Tweaker credentials; a
  verified PC Tweaker Pro subscription unlocks the loyalty price
  (€4.99/year) for Uninstaller Pro. Registration lives on pctweaker.app.
- **Open PC Tweaker** in one click when the flagship is installed.

### Security model (unchanged, and non-negotiable)

- The command that runs is rebuilt from the Windows registry at execution
  time and re-validated — the preview is display-only.
- MSI removals run as `msiexec /x {GUID} /qn /norestart` with the GUID as
  the only registry-derived byte; executable uninstallers must exist on
  disk as regular files at an absolute path; script interpreters are
  refused.
- One UAC consent per machine-wide removal, with a System Restore point
  attempted first.
- No silent deletions, no arbitrary commands, no shell anywhere in the
  execution path.
