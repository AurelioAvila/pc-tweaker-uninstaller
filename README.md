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
with product-specific entitlements. A PC Tweaker license token cannot be used as
an Uninstaller license token. PC Tweaker Lifetime owners also have a
[documented 12-month Uninstaller Pro benefit](https://github.com/AurelioAvila/pc-tweaker-app#free-and-pro),
activated on first sign-in to Uninstaller with the same account. This is a
separate, time-limited entitlement, not a perpetual Uninstaller license.

**[Download latest signed version](../../releases/latest)** for Windows 10/11 x64.
The application, EXE and MSI installers carry verified Authenticode signatures
identifying **Aurelio Avila**, with trusted timestamps. Both installer updater
signatures were verified separately. Published downloads were checked again on
2026-09-08; historical version 0.8.2 remains unsigned.

Signing identifies the publisher and protects file integrity; neither code signing
nor WinGet guarantees the absence of SmartScreen warnings.

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
