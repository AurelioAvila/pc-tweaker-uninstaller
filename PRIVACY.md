# Privacy Policy

Last updated: 2026-08-20

PC Tweaker Uninstaller is a desktop app. Everything it does to your system — listing installed programs, running uninstallers, scanning for leftovers — happens entirely on your own machine and is never sent anywhere. This policy covers the only two things that ever touch the network: the **optional** account system used for the Pro upgrade, and update checks.

**Pre-release notice**: this policy is published ahead of the first public release so the privacy model is on record before anyone installs anything.

## What never leaves your machine

- The list of programs installed on your PC.
- The results of leftover scans, and anything about the files and registry keys they find.
- Uninstall history, snapshots, and the quarantine's contents.

The app contains no analytics and no ad or tracking SDKs of any kind.

## What we collect, and why

If you create an account (only needed to unlock Pro):

- **Email address and password** — the password is stored only as a bcrypt hash; we cannot read it. The account is shared across the PC Tweaker ecosystem, so one email works in every product.
- **Name and date of birth** — collected at registration for account identification and age verification.
- **Subscription status** — which plan you're on and when it expires, kept in sync by Stripe webhooks so the app knows whether Pro features are unlocked.

Payments are handled entirely by Stripe Checkout in your browser; card details never pass through our servers and we never see them.

## Where data lives

Account data is stored on the PC Tweaker ecosystem backend (hosted on Railway) in a PostgreSQL database. The signed license the app caches locally contains your account id, subscription status, product name, and a timestamp — nothing else — and can be cleared at any time by signing out.

## Data retention and deletion

Account data is kept while the account exists. To delete your account and its data, contact us through the support form at [pctweaker.app/support](https://pctweaker.app/support) and we will remove it.

## Update checks

Checking for app updates requests a version manifest from GitHub Releases. Like any web request, GitHub receives your IP address as part of serving it; no account information is included.

## Changes to this policy

Material changes will be reflected in the "Last updated" date above and, where appropriate, called out in release notes.

## Contact

Privacy questions: open an issue at [github.com/AurelioAvila/pc-tweaker-uninstaller/issues](https://github.com/AurelioAvila/pc-tweaker-uninstaller/issues) or use the support form at [pctweaker.app/support](https://pctweaker.app/support).
