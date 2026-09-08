# Preparing a Windows release

Version 0.8.2 is still unsigned: its EXE, MSI and stable EXE alias were
downloaded again on 2026-09-08 and returned `NotSigned`. Their hashes match
GitHub's asset digests. Do not redistribute this backlog as a signed release.

Release preparation and signing run locally. The repository has checks,
Discord announcements and a WinGet submission workflow, but no automated
application build/upload workflow or configured Authenticode signing command.
`bundle.publisher` is descriptive metadata, not a digital signature.

## Required order

1. Run the checks documented in the README. Build the application locally.
2. Sign the application EXE and distributed DLLs with the owner's authorized
   code-signing certificate and a trusted timestamp before packaging. If the
   certificate, key or signing session is unavailable, stop distribution.
3. Package the signed application, then sign the final EXE and MSI installers.
   Inspect the packaged payload to ensure packaging did not replace the signed
   application with an unsigned rebuild. Verify the extracted binaries too.
4. Generate Tauri updater signatures over the final signed installer bytes.
   Authenticode signing after generating `.sig` files invalidates those updater
   signatures. Never substitute an updater signature for a publisher signature.
5. Install PowerShell 7 (`pwsh`), Windows SDK SignTool and [minisign](https://jedisct1.github.io/minisign/)
   on the release workstation. Set `SIGNTOOL_PATH` and `MINISIGN_PATH` to their
   executable paths if needed. Run:

   ```powershell
   node scripts/make-latest-json.mjs RELEASE-NOTES-<version>.md
   ```

   This requires one current x64 NSIS installer, one current MSI, the main
   application and any release-directory DLLs. Each must have a valid
   Authenticode signature from Aurelio Avila and a trusted timestamp.
   Both updater signatures must pass minisign verification with the public key
   in `tauri.conf.json`. Missing tools, invalid files and ambiguous bundles
   stop preparation before a manifest or stable alias is written.
6. Inspect the final package and record SHA-256 values and verification output.
   A failed attempt does not delete outputs from an earlier attempt: do not
   upload stale files. Publish only the exact verified package through the
   authorized release process. Any subsequent byte change requires signing
   and verification again. This helper prepares files; it does not upload them.
7. Download the published assets again, compare their hashes with GitHub API
   digests and verify Authenticode, publisher and timestamps before promoting
   them through the website, WinGet or any other download catalog.

`scripts/verify-authenticode.ps1 -Path <file>` performs the Windows checks
using [SignTool](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool)
with `/pa /all /v /tw`; warnings are failures. `verify-published-release.ps1`
downloads and verifies the release installers and aliases without signing,
uploading or changing a release. The WinGet job depends on that check and uses
its resolved release tag, including for a manual run with no tag supplied.
It also refuses to proceed when the WinGet index cannot be checked.

The WinGet action subsequently downloads assets itself. Keep published assets
immutable and verify its proposed manifest hashes before merging a catalog PR.
This gate does not prevent someone from bypassing the helpers with a manual
upload, and it does not inspect installed payloads. The local payload review
and final package verification above remain required. Existing Discord release
announcements remain unchanged.

## Verification tests

```powershell
node --test scripts/release-verification.test.mjs
./scripts/test-authenticode.ps1 -SignedFile <verified-installer> -UnsignedFile <unsigned-installer>
```

The optional `UNINSTALLER_TEST_ASSET` environment variable points to a real
downloaded Uninstaller installer with its `.sig` alongside it. With minisign
available, the Node tests also check that its updater signature verifies and
that changing the installer bytes causes verification to fail. These tests
never sign files, run installers or publish artifacts.
