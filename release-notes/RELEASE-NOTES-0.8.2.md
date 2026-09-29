# PC Tweaker Uninstaller 0.8.2 — Redaxa is one of ours again

The Uninstaller stopped recognising Redaxa as part of the suite, and started
offering to remove it in bulk.

The suite guard matches on product name, and it knew "promptshield". That
product was renamed to Redaxa and its installer now registers the new name,
so the guard quietly stopped matching. From then on the shipped Uninstaller
treated a sibling product as an ordinary program: eligible for Safe Batch
bulk removal, and listed without the Suite mark that exists to stop somebody
removing it by accident.

## What changed

- Both names are recognised now. A rename only changes what *new* installers
  register: the machines that installed it as PromptShield still report that,
  and dropping the old string would have stopped protecting the users who
  have had it longest.
- Residue path validation handles Windows separators correctly, with the
  tests to match.
- Releases submit themselves to winget from now on. This is the first version
  where that runs, so the first submission still waits on a moderator — new
  packages always do.

## Signing

This is the first release signed **only** with the Uninstaller's own key,
as 0.8.1 said it would be. If you are on 0.8.1 or later the auto-update
works as usual. If you are still on 0.8.0 or earlier, install this one by
hand — your app is still trusting the old shared key and will not accept
this update on its own.
