<p align="center">
  <img src="src-tauri/icons/128x128.png" width="112" alt="PC Tweaker Uninstaller logo">
</p>

<h1 align="center">PC Tweaker Uninstaller</h1>

<p align="center">
  <strong>Removal Intelligence for Windows.</strong><br>
  Remove software with clarity, not guesswork — part of the PC Tweaker suite.
</p>

<p align="center">
  <a href="https://pctweaker.app/uninstaller/"><img src="https://img.shields.io/badge/WEBSITE-pctweaker.app-0078D4?style=for-the-badge&logo=windows&logoColor=white" alt="Official product information and release status"></a>
  <a href="https://github.com/AurelioAvila/pc-tweaker-uninstaller/blob/master/LICENSE"><img src="https://img.shields.io/badge/License-Proprietary-6B7280?style=for-the-badge" alt="Proprietary License"></a>
  <a href="https://github.com/AurelioAvila"><img src="https://img.shields.io/badge/%C2%A9%20Aurelio%20Avila-PC%20Tweaker%20suite-7C3AED?style=for-the-badge" alt="Copyright Aurelio Avila - PC Tweaker suite"></a>
  <a href="https://github.com/AurelioAvila/pc-tweaker-uninstaller/releases/latest"><img src="https://img.shields.io/github/v/release/AurelioAvila/pc-tweaker-uninstaller?display_name=tag&style=for-the-badge&color=7C3AED" alt="Latest release"></a>
</p>

## Download

**[Official product page](https://pctweaker.app/uninstaller/)** — release status
and product information. Uninstaller shares a suite account with PC Tweaker,
but requires its own product entitlement; a PC Tweaker license does not unlock it.

The [release archive](../../releases) contains historical Windows 10/11 x64
installers. Version 0.8.2 is not recommended for distribution while a verified,
publisher-signed replacement is pending.

**Signing status:** the v0.8.2 Windows installers are not Authenticode-signed,
as verified again on 2026-09-08. Their updater `.sig` files do not establish a Windows
publisher signature. PC Tweaker's signed release is a separate product and does
not confer signing status on this application. WinGet does not guarantee the
absence of SmartScreen warnings.

See the [signing inventory and verification guide](https://github.com/AurelioAvila/.github/blob/master/CODE_SIGNING.md).
Use official downloads and investigate security warnings before proceeding.

Uninstall Windows programs cleanly — with a safety net. Part of the
PC Tweaker ecosystem: one account with product-specific entitlements.

**Removal Intelligence**: every program carries an evidence-based Removal
Confidence Score (Safe / Review / Keep) with its reasons spelled out; every
uninstall shows a Removal Brief (method, privileges, size, confidence)
before anything runs; every removal leaves a local, exportable receipt in
the Removal Ledger with verified space reclaimed. 5 languages, 8 themes,
one suite account shared with PC Tweaker.

## Architecture

- **Desktop app**: Tauri 2 (Rust core) + React 19 + TypeScript (strict).
- **Backend**: the shared PC Tweaker ecosystem backend (accounts, signed
  licenses, Stripe). This repo contains no server code and no secrets.
- **License model**: the backend signs `{userId, isPro, plan, product,
  issuedAt}` with an Ed25519 key; this client verifies the exact signed
  bytes with the embedded public key and additionally requires
  `product == "uninstaller"` — a valid PC Tweaker license does not unlock
  this app (see `src-tauri/src/license.rs`).
- **Safety model**: every reversible change is snapshotted before it is
  applied (`src-tauri/src/rollback.rs`, atomic writes + locking, ported
  with its tests from PC Tweaker). Privileged actions run one at a time
  through an explicit UAC consent (`src-tauri/src/elevation.rs`).

## Development

```
npm install
npm run tauri dev
```

Checks (all of these gate CI):

```
npm run build         # tsc --noEmit + vite build
npm run lint          # eslint (strict, type-checked)
npm run format:check  # prettier
cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

Release maintainers: follow the [local signing and verification procedure](RELEASING.md).
Distribution must stop if publisher signing or timestamp verification fails.

## Legal

The source is published for review and transparency only — see
[LICENSE](LICENSE): no rights are granted to use, copy, modify, or
redistribute it. Use of the compiled application is governed by
[TERMS.md](TERMS.md); how data is handled is documented in
[PRIVACY.md](PRIVACY.md).

## Security notes

- No secrets in this repo; the signing private key exists only on the
  backend host. The embedded key here is the public (verify-only) half.
- Release builds never enable DevTools (see `src-tauri/Cargo.toml`).
- CSP allows network access exclusively to the ecosystem backend.
