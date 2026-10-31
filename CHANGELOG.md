# Changelog

## 0.13.11

PC Tweaker Uninstaller 0.13.11 makes the administrator step and leftover cleanup safer, and says how long to wait after too many sign-in attempts.

- The uninstall command shown before a removal is now the only one that can run. If the program's entry changes after you confirm, even while the Windows administrator prompt is open, nothing runs and the app asks you to start the removal again to review the new command.
- Leftover cleanup now works only from what Windows recorded for the program just before this app removed it: its name and its install folder. A folder reached through a link to another location is not offered.
- A program installed for your account only is started with administrator rights only when its uninstaller is in a Program Files folder that accounts without administrator rights cannot change. Otherwise the app says why and suggests Windows Settings.
- The space freed by a removal now counts only what actually left the program's folder.
- After too many sign-in attempts, the message says how long to wait when the account service gives a time.
- The sign-in error no longer sits right under the Sign in button.

## 0.13.10

PC Tweaker Uninstaller 0.13.10 tells you why sign-in fails when the account service cannot be reached, and makes the administrator step of a removal safer.

- When the account service cannot be reached, the sign-in form now says why: no connection, a slow answer, an address that does not resolve, a wrong Windows clock, a secure connection altered by antivirus HTTPS scanning or a proxy, or a connection blocked by a firewall. Account and checkout requests also stop waiting after 30 seconds instead of spinning.
- The administrator helper that runs each machine-wide removal now hands its result straight back to the app and writes nothing in your user folders. It finds Windows and Program Files through Windows itself, not through settings other programs can change.
- On a standard Windows account where an administrator approves the prompt with their own password, the result of a machine-wide removal now reaches the app instead of being reported as missing.
- When your session ends elsewhere, for example after a password change, the Pro licence saved on this PC is removed too, as when you sign out here.
- French removal messages show their punctuation correctly again.
- Updated components, including the secure connection library used for updates and account requests.

## 0.13.9

Clearer, more accurate translations in every language.

## 0.13.8

PC Tweaker Uninstaller 0.13.8 tells you exactly what Windows did about the System Restore point before a removal.

- The report and the Removal Ledger now say whether Windows created a new System Restore point or reused an existing one, with its date and time. Windows makes at most one point a day: when it already made one, it accepts the request and creates nothing.
- Before a machine-wide removal, the confirmation says what to expect: a new point, the point Windows already made that day, or none because System Restore is turned off.
- When no point is made, the report says why: System Restore is off, there is not enough free disk space, or Windows refused the request.
- Ledger entries from earlier versions no longer read as a created restore point: those versions did not check, so they are shown as requested but not verified.
- The administrator helper that runs each machine-wide removal loads Windows libraries from the system folder only, starts in the system folder, and is not started while the app runs from a temporary or Downloads folder.

## 0.13.7

PC Tweaker Uninstaller 0.13.7 is a maintenance and security hardening update.

## 0.13.6

PC Tweaker Uninstaller 0.13.6 is a routine maintenance update.

## 0.13.5

PC Tweaker Uninstaller 0.13.5 offers the 3-day free trial of Pro where Pro is needed, and keeps your licence tied to your own PC.

- Cleaning leftovers, removing several programs at once and removing broken entries now offer the 3-day free trial of Uninstaller Pro when your account can start one. The button opens Plans & pricing, which shows the price after the trial, the date of the first charge and how to cancel.
- The main window shows Start 3-day free trial next to Plans & pricing while you can start one, and the time left while it runs.
- Your Pro licence is checked against this PC, so a licence file copied to another computer no longer unlocks Pro there. Signing in on your other PCs works as before.
- Moving the Windows clock back while the app is open no longer stretches the time Pro keeps working offline.
- Plans & pricing marks the plan you are on as Current plan, and the Free plan as Included in your plan while Pro or its trial is active.

## 0.13.4

PC Tweaker Uninstaller 0.13.4 stays in the notification area when you close its window, the same way PC Tweaker does.

- Closing the window keeps PC Tweaker Uninstaller in the notification area. Click its icon to reopen it, or right-click it for Open PC Tweaker Uninstaller and Exit PC Tweaker Uninstaller.
- Turn off Keep running in the background from the icon's menu if you prefer closing the window to quit. Your choice is saved.
- Opening the Uninstaller while it is already running brings its window back instead of starting a second copy.
- The icon's menu and the notice shown the first time you close the window are in the language you chose in the app.

## 0.13.3

PC Tweaker Uninstaller 0.13.3 looks for updates on the PC Tweaker download server as well as on GitHub.

- Updates are checked at dl.pctweaker.app first and on GitHub second. Both serve the same signed installer, and the app verifies its signature before installing.

## 0.13.2

PC Tweaker Uninstaller 0.13.2 lets you change your password and email from the account panel and sends account emails in the app's language.

- Change your password (the current one is required; your other PCs are signed out and this one stays signed in) or your email address (the change takes effect when you confirm the link sent to the new address).
- Verification, password reset and other account emails arrive in the language the app is set to.
- If a subscription payment fails, the account panel says so and offers Update payment method, which opens Stripe's billing portal. When Pro stops, the app tells you once, the next time you open it: removals already done stay done, and the Recycle Bin can still restore what it holds.

## 0.13.1

PC Tweaker Uninstaller 0.13.1 keeps you signed in and makes account problems easier to sort out.

- You stay signed in while you keep using the app, instead of being signed out after 30 days.
- Forgot your password? Ask for a reset link from the sign-in panel. A lost verification email can be sent again from your account.
- Sign-in and registration errors now appear in your language, and an address that already has an account from PC Tweaker or Tweaky Driver is explained as such.
- Uninstaller Pro stays active on PCs whose clock runs behind.
- Checkboxes follow your theme, text fields show a single focus ring, and the Recommended badge on the Pro card has a cleaner look.

## 0.13.0

PC Tweaker Uninstaller 0.13.0 adds a Plans & pricing page, one click from the top bar.

- Free and Pro, compared feature by feature: everything you can do for free, and what Pro adds.
- Your account at a glance, with sign-in and account creation on the same page.
- Offers appear only while they are running, with a live countdown.
- Manage your subscription in Stripe's billing portal, straight from the app.
- A short FAQ, and a plain summary of what the app does with your data.

## 0.12.4

PC Tweaker Uninstaller 0.12.4 adds Portuguese and shows what each removal actually freed.

- Portuguese is now available alongside English, Italian, French, Spanish and German.
- A new summary tile adds up the space your removals freed, measured after each one.
- After an uninstall the app checks for leftovers straight away and shows what it found.
- Install dates read in your language, and the window remembers its size and position.

## 0.12.1

PC Tweaker Uninstaller 0.12.1 refines application review and local removal history.

- Clearer, keyboard-accessible details controls and removal guidance.
- Improved local app-icon lookup with recognizable fallback icons.
- Clear removal history with confirmation; apps, restore points and existing exports are preserved.

## 0.12.0

PC Tweaker Uninstaller 0.12.0 is a routine daily maintenance update.

## v0.11.3 - 2026-09-26

PC Tweaker Uninstaller 0.11.3 is a routine maintenance update.

## v0.11.2 - 2026-09-23

- Leftover scans no longer offer protected Windows or user folders as removable install directories. The shared safety check now also recognizes differently cased Windows paths and aliases.

## v0.11.1 - 2026-09-14

Pro stopped working during an outage, and the button you press to buy it was
unreadable.

A cached licence was trusted for three days from the moment it was issued,
and that window applied whether or not the signature said the customer had
paid through next March. Any four days without a reachable backend - the
server down, a company firewall, a laptop somewhere without signal - took
Pro away from people who were paid up, with no way for them to prove it.
Leftover cleaning and batch removal are the two things Pro buys, and both
stopped.

- A licence whose payload names a paid period still ahead now holds for
  thirty days without a refresh, never past the date the backend signed. The
  three-day window stays for licences that state no expiry, and past the
  signed date no window applies at all.
- The primary button carried white text on the accent gradient, which fails
  WCAG AA on every one of the eight themes: 3.46:1 on Violet at best, 1.48:1
  on Slate Mono, against a 4.5:1 requirement. It is dark ink now, which
  clears the requirement on all eight. A check in CI reads the themes out of
  the source and fails the build if a new palette ever breaks it again.

Windows binaries and installers are digitally signed by Aurelio Avila and
timestamped; automatic updates carry a separate updater signature.

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
