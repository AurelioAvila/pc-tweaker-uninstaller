# PC Tweaker Uninstaller 0.3.0 — Residue Intelligence

Uninstallers lie. They finish "successfully" and leave folders, shortcuts and registry keys behind. This release finds them.

- **Scan for leftovers** — after a successful uninstall, one click finds what survived: the install folder itself, AppData/ProgramData folders, Start Menu and Desktop shortcuts, and registry keys.
- **Conservative by design** — a leftover is only proposed on an exact normalized-name match against the program's name or publisher, with a stoplist of shared vendor names. A "Microsoft" folder is never residue.
- **Reversible by design** — everything you clean goes to the Recycle Bin, not into the void. Per-user registry keys are the one flagged exception; machine-wide keys are listed but never touched.
- **Validated twice** — the cleanup engine re-checks every path against the same rules that proposed it. Nothing outside those rules can be removed, by construction.

Delivered automatically to existing installs via the built-in updater.
