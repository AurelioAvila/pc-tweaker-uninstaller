# Run-time check of the elevated child's DLL search, without elevation or UAC.
# Copies the exe into a fresh temporary folder, puts placeholder files (plain text,
# not loadable) next to it under the names of DLLs a Windows process can load at
# startup, then starts the child's code path with an invalid request:
#   pc-tweaker-uninstaller.exe --elevated-uninstall <encoded "bogus"> <encoded "x">
# with APPDATA pointed into the temporary folder. A correct build ignores the
# placeholders, refuses the request and writes its refusal report (exit 0). If the
# loader took any placeholder from the exe's folder, the process cannot start.
# Nothing outside the temporary folder is written; it is deleted afterwards.
#   test-dll-planting.ps1 -Exe <path> [-ExpectFailure]   (-ExpectFailure: a build
#   without the protection, to show the check detects the problem)
param([Parameter(Mandatory)][string]$Exe, [switch]$ExpectFailure)
$ErrorActionPreference = 'Stop'
$names = 'bcrypt', 'bcryptprimitives', 'comctl32', 'crypt32', 'cryptbase', 'cryptsp', 'dbghelp', 'dwmapi',
  'iphlpapi', 'kernel.appcore', 'msasn1', 'netapi32', 'ntmarta', 'powrprof', 'profapi', 'propsys', 'secur32',
  'srclient', 'sspicli', 'umpdc', 'userenv', 'uxtheme', 'version', 'wevtapi', 'windows.storage', 'winmm',
  'wintrust', 'wldp', 'dnsapi', 'textinputframework', 'coremessaging', 'wbemcomn', 'msimg32', 'dxgi', 'd3d11'
$root = Join-Path ([IO.Path]::GetTempPath()) "pcu-dll-planting-$PID"
$bin = Join-Path $root 'app'; $data = Join-Path $root 'appdata'
New-Item -ItemType Directory -Force $bin, $data | Out-Null
try {
  Copy-Item -LiteralPath $Exe (Join-Path $bin 'pc-tweaker-uninstaller.exe')
  foreach ($n in $names) { Set-Content -LiteralPath (Join-Path $bin "$n.dll") -Value 'placeholder, not a DLL' -Encoding ascii }
  $enc = { param($v) 'b64:' + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($v)).TrimEnd('=').Replace('+', '-').Replace('/', '_') }
  $psi = [Diagnostics.ProcessStartInfo]::new((Join-Path $bin 'pc-tweaker-uninstaller.exe'))
  $psi.ArgumentList.Add('--elevated-uninstall'); $psi.ArgumentList.Add((& $enc 'bogus')); $psi.ArgumentList.Add((& $enc 'x'))
  $psi.WorkingDirectory = $bin
  $psi.UseShellExecute = $false
  $psi.Environment['APPDATA'] = $data
  $p = [Diagnostics.Process]::Start($psi)
  if (-not $p.WaitForExit(60000)) { $p.Kill(); throw 'the child did not exit within 60 s' }
  $report = Join-Path $data 'com.aurel.pc-tweaker-uninstaller\last-uninstall-report.json'
  $ok = $p.ExitCode -eq 0 -and (Test-Path -LiteralPath $report)
  $detail = "exit 0x{0:X8}, report {1}" -f $p.ExitCode, $(if (Test-Path -LiteralPath $report) { 'written' } else { 'missing' })
  if ($ExpectFailure) {
    if ($ok) { throw "expected the unprotected build to fail with placeholders next to it, but it ran ($detail)" }
    "DLL planting check detects the unprotected build ($detail)"
  } else {
    if (-not $ok) { throw "a placeholder next to the exe was loaded or the child failed ($detail)" }
    "DLL planting check ok: $($names.Count) placeholders next to the exe were ignored ($detail)"
  }
} finally {
  Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
