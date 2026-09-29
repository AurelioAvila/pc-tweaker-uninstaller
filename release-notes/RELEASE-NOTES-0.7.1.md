# PC Tweaker Uninstaller 0.7.1

Bug fix: a program could stay listed after being uninstalled.

## Fixed

Many third-party uninstallers — NSIS or Inno Setup based, common for
Electron apps — copy themselves to a temp file, relaunch that copy, and
exit almost immediately. The original process reported success while the
real removal work (deleting the registry key, the install folder) was
still running in the detached child, so refreshing the list right after
ran too early and the program appeared not to have been removed.

Executable-kind uninstalls now wait (up to 5 seconds, checking every
250ms) for the registry entry to actually disappear before the report is
returned. MSI uninstalls are unaffected — `msiexec.exe` performs the
removal synchronously and already reported correctly.
