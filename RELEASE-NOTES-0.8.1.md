# PC Tweaker Uninstaller 0.8.1 — its own update-signing key

No feature changes. This release exists to fix one thing: the auto-updater
trusted the same signing key as PC Tweaker and Redaxa — three separate
products verifying updates against one shared private key. A compromise of
that one key, together with push access to any one of the three
repositories, could have signed a malicious update for all three install
bases at once, not just the product actually breached.

## What changed

- A new, Uninstaller-only Ed25519 keypair now signs its updates. This
  release is still signed with the *old* shared key, because that is the
  only key existing 0.8.0 installs trust — it is what lets this update
  reach them at all. From the release after this one onward, the
  Uninstaller signs only with its own key.
- Nothing else moved: same binary, same behaviour, same detection logic.

## If you're on 0.8.0 already

You update to this release exactly as you would any other — nothing to do
by hand. If you skip checking for updates for a long stretch and land
directly on a future release without passing through this one first, the
auto-update check for that one jump may fail silently, since your app would
still be trusting the old key. A normal manual install fixes it, the same
as installing 0.8.0 the first time did.
