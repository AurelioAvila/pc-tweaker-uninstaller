# Privacy Policy

Last updated: 2026-11-03

PC Tweaker Uninstaller is a desktop app. Everything it does to your system — listing installed programs, running uninstallers, scanning for leftovers — happens entirely on your own machine and is never sent anywhere. This policy covers the only two things that ever touch the network: the **optional** account system used for the Pro upgrade, and update checks.

PC Tweaker Uninstaller is run by Aurelio Avila, a private individual, who is the controller of the personal data described here. You can reach him at support@pctweaker.app. The complete policy for PC Tweaker, PC Tweaker Uninstaller and Tweaky Driver, which share one account, is at [pctweaker.app/privacy](https://pctweaker.app/privacy/) and describes every record in full.

## What never leaves your machine

- The list of programs installed on your PC.
- The results of leftover scans, and anything about the files and registry keys they find.
- Uninstall history, snapshots, and the quarantine's contents.
- The list of programs removed earlier (`removed-programs.json`) and the last-seen list it is compared with (`inventory-snapshot.json`), both in the app's own data folder: program names, publishers, install folders and removal times. They are never uploaded.
- The diagnostic log. The app writes one to `uninstaller.log`, next to the removal ledger in its own data folder, recording the decisions it took: version and start time, elevation attempts, uninstall exit codes and durations, leftover-cleanup outcomes, and any crash. It therefore contains program names and file paths from your machine. It is never uploaded, never scheduled, and never read by us. "Open log folder" in the Removal Ledger dialog is the only way it is surfaced, and it reaches anyone else only if you attach it to a support message yourself. The file rotates once at about 1 MB and you can delete both generations at any time.

The app contains no analytics and no ad or tracking SDKs of any kind. There is no crash-reporting service.

## What we collect, and why

If you create an account (only needed to unlock Pro):

- **Email address and password** — the password is stored only as a bcrypt hash; we cannot read it. The account is shared across the PC Tweaker ecosystem, so one email works in every product.
- **Name and date of birth** — collected at registration for account identification and age verification.
- **Subscription status** — which plan you're on and when it expires, kept in sync by Stripe webhooks so the app knows whether Pro features are unlocked.
- **Account times** — when you confirmed your email address and when you last signed in with your password.
- **Purchase confirmation** — before the payment page opens you tick a box to accept the Terms and to ask for immediate access, which means you lose the 14-day right of withdrawal once access starts. We store the IP address the confirmation came from, the version of the Terms, the time, the product and plan, the browser family (for example "Chrome", not the full browser identification string) and the reference of the Stripe checkout session that opens next; the Terms version and a reference to this record are also added to that session at Stripe. Purpose: proof of that consent, and defending payment disputes and fraud (Article 6(1)(c) and (f) GDPR).
- **Proof of use** — when a signed-in copy of the app refreshes its licence, a dated entry with your account, the product and a one-way hash of the random installation identifier the app created, at most one per installation per day, plus the country the request came from when our network provider Cloudflare supplies it. No IP address is stored with it. Purpose: showing that a purchase was used, in a payment dispute (Article 6(1)(f) GDPR).

Payments are handled entirely by Stripe Checkout in your browser; card details never pass through our servers and we never see them.

## Where data lives

Account data is stored on the PC Tweaker ecosystem backend (hosted on Railway) in a PostgreSQL database. The signed license the app caches locally contains your account id, subscription status, product name, and a timestamp — nothing else — and can be cleared at any time by signing out.

## Data retention and deletion

Account data is kept while the account exists. Purchase confirmations and proof-of-use entries are deleted after 13 months, and are kept for that period even if the account is deleted sooner, because a payment can be disputed after an account is gone. To delete your account and its data, contact us through the support form at [pctweaker.app/support](https://pctweaker.app/support) and we will remove it.

## Update checks

Checking for app updates requests a version manifest from GitHub Releases. Like any web request, GitHub receives your IP address as part of serving it; no account information is included.

## Changes to this policy

Material changes will be reflected in the "Last updated" date above and, where appropriate, called out in release notes.

## Contact

Privacy questions: open an issue at [github.com/AurelioAvila/pc-tweaker-uninstaller/issues](https://github.com/AurelioAvila/pc-tweaker-uninstaller/issues) or use the support form at [pctweaker.app/support](https://pctweaker.app/support).
