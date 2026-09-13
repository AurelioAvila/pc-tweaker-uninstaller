# Changelog

## v0.11.0 — 2026-09-13

Leftover cleanup could send a whole well-known folder to the Recycle Bin.

The only guard on an install directory was a path-component count, and a
count cannot say what it looks like it says: `C:\Program Files` is three
components and `C:\Windows\System32` is four, the same shapes a real
per-application install directory has. A program whose recorded
InstallLocation points at its containing folder rather than its own — which
installers write badly often enough that a safety tool has to assume it —
would have had that folder offered as a leftover. Cleanup is the one
destructive step here and it is the Pro feature, so the exposure pointed at
paying users.

- Every Windows well-known root, the user profile and its document folders,
  and any bare drive root are now refused as install locations. The
  application's own directory underneath them is still cleanable, and a test
  asserts both directions so the guard cannot quietly go back to a count.
- The window no longer freezes. Six commands ran on the main thread, and two
  of them fire together at startup: reading every installed program out of
  the registry and every MSIX package out of the WinRT package manager. They
  now run off it, as do Store removal, residue scanning and the dry run.
- The expandable row is reachable by keyboard. It holds the confidence
  reasons, the install folder, the registry entry and the exact uninstall
  command, and none of that could be opened without a mouse. Enter and Space
  now work, with a visible focus ring.
- An uninstall that takes more than twenty seconds says what is probably
  happening: the program's own uninstaller has opened a window behind ours.
  Translated in all five languages.
- A local diagnostic log. The app wrote nothing, anywhere, and is built for
  the GUI subsystem, so a crash was completely silent. It now records the
  decisions it takes and any panic, in a file beside the removal ledger, and
  "Open log folder" in the ledger dialog is how you reach it. Nothing is
  uploaded and there is no crash-reporting service; the privacy policy says
  so, and says the file contains program names and paths.

Windows binaries and installers are digitally signed by Aurelio Avila and
timestamped; automatic updates carry a separate updater signature.

## v0.8.2 — 2026-09-02

The Uninstaller stopped recognising Redaxa as one of ours, and started
offering to remove it in bulk.

The suite guard matched on product name, and it knew "promptshield". That
product was renamed to Redaxa and its installer now registers the new name,
so the guard quietly stopped matching. From then on the shipped Uninstaller
treated a sibling product as an ordinary program: eligible for Safe Batch
bulk removal, and listed without the Suite mark that exists to stop somebody
removing it by accident.

- Both names are recognised now. A rename only changes what *new* installers
  register: the machines that installed it as PromptShield still report that,
  and dropping the old string would have stopped protecting the users who
  have had it longest.
- Releases submit themselves to winget from now on. This is the first version
  where that runs, so the first submission still waits on a moderator — new
  packages always do.

## v0.8.1 — 2026-09-01

Housekeeping, no user-visible change. Written down after the fact: this
release shipped without an entry here.

- The Uninstaller has its own update-signing key instead of borrowing PC
  Tweaker's, so the two products can be released independently.
- Releases are announced on Discord, through the same workflow PC Tweaker
  uses.
- The package metadata states the proprietary license.

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
