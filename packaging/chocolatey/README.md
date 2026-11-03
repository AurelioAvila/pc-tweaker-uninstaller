# Chocolatey package source

This package downloads the official signed Windows x64 installer with a pinned
SHA-256. It contains no application binaries and performs no program removal,
leftover scan or cleanup. The application installs for the current Windows user.

The package is submitted to community moderation separately from GitHub releases.
Source availability here does not establish approval or availability in the feed.

For a release update, verify the final artifact's hash, publisher signature and
trusted timestamp, then update the version-specific URL, checksum and nuspec
version together. Preserve accurate Free/Pro and proprietary-license wording.

`choco pack` builds the package without running the setup. The initial 0.14.0
package was checked for PowerShell syntax, metadata, archive contents and agreement
with the signed official release. Installation and uninstallation were not tested
on the publisher's working PC; the community verifier result remains separate.
