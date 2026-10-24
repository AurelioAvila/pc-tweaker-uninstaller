PC Tweaker Uninstaller 0.13.8 tells you exactly what Windows did about the System Restore point before a removal.

- Windows makes at most one System Restore point a day. When it already made one, it accepts the request and creates nothing. The report and the Removal Ledger now say whether a new point was created, or which existing point Windows reused, with its date and time.
- Before a machine-wide removal, the confirmation says what to expect: a new point, the point Windows already made that day, or none because System Restore is turned off.
- When no point is made, the report says why: System Restore is off, there is not enough free disk space, or Windows refused the request.
- Ledger entries from earlier versions no longer read as a created restore point: those versions did not check, so they are shown as requested but not verified.
- The administrator helper that runs each machine-wide removal loads Windows libraries from the system folder only, starts in the system folder, and is not started while the app runs from a temporary or Downloads folder.
