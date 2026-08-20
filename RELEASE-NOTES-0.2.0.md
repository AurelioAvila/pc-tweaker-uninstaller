# PC Tweaker Uninstaller 0.2.0 — "Removal Intelligence"

**The first public release.** Most uninstallers show you a list and a button. This one shows you *evidence* — because the scariest part of removing software isn't clicking Uninstall, it's not knowing what you're about to break.

## Removal Intelligence for Windows

- **Removal Confidence Score.** Every program is banded **Safe to remove**, **Review before removing**, or **Keep — system-related**, from visible evidence in the Windows registry: shared runtimes other apps depend on (Visual C++, .NET, WebView2, Java), driver and chipset packages, game launchers, missing publishers, broken uninstall entries. Every verdict explains itself line by line — and every verdict is labeled for what it is: a guide, never a certainty.
- **The Removal Brief.** Before anything runs, you see the method, the privileges required, the estimated space you'll get back, the confidence band with its reasons, and the *exact command* that will execute. You also see what will **not** be removed automatically — no pretend deep-cleaning.
- **The Clean Removal Ledger.** Every removal leaves a local receipt — successes and failures alike — with the restore-point outcome and disk space **verified by measuring the install folder before and after**, not just repeating the registry's claim. Export it as JSON anytime. It lives on your PC and never leaves it.

## Built on a paranoid core

The command that runs is rebuilt from the registry at execution time and re-validated — the preview you confirmed is display-only, so nothing can swap a different command underneath your click. MSI removals are reconstructed as `msiexec /x {GUID} /qn /norestart` with the GUID as the only registry-derived byte. Script-based uninstallers are refused outright. One UAC consent per machine-wide removal, with a System Restore point attempted first. No silent deletions. No shell. Ever.

## Also in this release

- **Automatic updates** — signed builds verify their signature and update in place.
- **Five languages** (EN, IT, FR, ES, DE) and **eight themes**, both remembered.
- **One suite account** with [PC Tweaker](https://github.com/AurelioAvila/pc-tweaker-app): sign in with the same credentials, and a verified PC Tweaker Pro subscription unlocks **Uninstaller Pro at €4.99/year**. Plans and registration live on [pctweaker.app](https://pctweaker.app).

**[Download for Windows 10/11](https://github.com/AurelioAvila/pc-tweaker-uninstaller/releases/latest)** — and if your PC deserves more than a cleanup, PC Tweaker 1.0 "Control Room" shipped today too.
