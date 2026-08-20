<p align="center">
  <img src="src-tauri/icons/128x128.png" width="112" alt="PC Tweaker Uninstaller logo">
</p>

<h1 align="center">PC Tweaker Uninstaller</h1>

<p align="center">
  <strong>Removal Intelligence for Windows.</strong><br>
  Remove software with clarity, not guesswork — part of the PC Tweaker suite.
</p>

<p align="center">
  <a href="https://github.com/AurelioAvila/pc-tweaker-uninstaller/blob/master/LICENSE"><img src="https://img.shields.io/badge/License-Proprietary-6B7280?style=for-the-badge" alt="Proprietary License"></a>
  <a href="https://github.com/AurelioAvila"><img src="https://img.shields.io/badge/%C2%A9%20Aurelio%20Avila-PC%20Tweaker%20suite-7C3AED?style=for-the-badge" alt="Copyright Aurelio Avila - PC Tweaker suite"></a>
  <a href="https://pctweaker.app"><img src="https://img.shields.io/badge/pctweaker.app-Official%20site-0078D4?style=for-the-badge&logo=windows&logoColor=white" alt="Official site"></a>
</p>


Uninstall Windows programs cleanly — with a safety net. Part of the
PC Tweaker ecosystem: one account, shared entitlements, loyalty pricing for
existing subscribers.

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
