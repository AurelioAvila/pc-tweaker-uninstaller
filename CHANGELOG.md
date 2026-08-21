# Changelog

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
