# PC Tweaker Uninstaller 0.4.0 — Relationship Map

Programs are not islands. This release maps how they relate — from evidence, never guesses — and warns you before a removal breaks something else.

- **Dependents** — a program installed inside another's folder (a Steam game under steamapps, a plugin under its host) is its dependent. The confirmation dialog now names them: "Removing this also takes down: ...".
- **Installed via** — the contained program gets the mirror warning: "Installed via Steam — consider removing it from there instead."
- **Publisher context** — "5 other programs from this publisher are installed" before you remove one of a family.
- **Conservative by design** — bare drives and shallow paths never form trees; identical install locations are not each other's dependents. Every claim traces to a registry value.

Delivered automatically via the built-in updater.
