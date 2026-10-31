PC Tweaker Uninstaller 0.13.10 tells you why sign-in fails when the account service cannot be reached, and makes the administrator step of a removal safer.

- When the account service cannot be reached, the sign-in form now says why: no connection, a slow answer, an address that does not resolve, a wrong Windows clock, a secure connection altered by antivirus HTTPS scanning or a proxy, or a connection blocked by a firewall. Account and checkout requests also stop waiting after 30 seconds instead of spinning.
- The administrator helper that runs each machine-wide removal now hands its result straight back to the app and writes nothing in your user folders. It finds Windows and Program Files through Windows itself, not through settings other programs can change.
- On a standard Windows account where an administrator approves the prompt with their own password, the result of a machine-wide removal now reaches the app instead of being reported as missing.
- When your session ends elsewhere, for example after a password change, the Pro licence saved on this PC is removed too, as when you sign out here.
- French removal messages show their punctuation correctly again.
- Updated components, including the secure connection library used for updates and account requests.
