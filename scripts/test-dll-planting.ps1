# Run-time check of the elevated child's DLL search, without elevation or UAC.
# Copies the exe into a fresh temporary folder, puts placeholder files (plain text,
# not loadable) next to it under the names of DLLs a Windows process can load at
# startup, then starts the child's code path with an invalid request:
#   pc-tweaker-uninstaller.exe --elevated-uninstall <encoded "bogus"> <encoded "x"> <encoded fingerprint> <encoded pipe>
# with a report pipe made here, as the app makes one, and APPDATA pointed into the
# temporary folder. A correct build ignores the placeholders, refuses the request,
# sends its refusal report into the pipe (exit 0) and writes nothing to disk. If the
# loader took any placeholder from the exe's folder, the process cannot start.
# Nothing outside the temporary folder is written; it is deleted afterwards.
#   test-dll-planting.ps1 -Exe <path> [-ExpectFailure]   (-ExpectFailure: a build
#   without the protection, to show the check detects the problem)
param([Parameter(Mandatory)][string]$Exe, [switch]$ExpectFailure)
$ErrorActionPreference = 'Stop'
Add-Type -Namespace PcuCheck -Name Pipe -MemberDefinition @'
[DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
public static extern Microsoft.Win32.SafeHandles.SafeFileHandle CreateNamedPipeW(string name, uint openMode, uint pipeMode,
  uint maxInstances, uint outSize, uint inSize, uint timeout, IntPtr security);
[DllImport("kernel32.dll", SetLastError = true)]
public static extern bool GetNamedPipeClientProcessId(Microsoft.Win32.SafeHandles.SafeFileHandle pipe, out uint pid);
'@
$names = 'bcrypt', 'bcryptprimitives', 'comctl32', 'crypt32', 'cryptbase', 'cryptsp', 'dbghelp', 'dwmapi',
  'iphlpapi', 'kernel.appcore', 'msasn1', 'netapi32', 'ntmarta', 'powrprof', 'profapi', 'propsys', 'secur32',
  'srclient', 'sspicli', 'umpdc', 'userenv', 'uxtheme', 'version', 'wevtapi', 'windows.storage', 'winmm',
  'wintrust', 'wldp', 'dnsapi', 'textinputframework', 'coremessaging', 'wbemcomn', 'msimg32', 'dxgi', 'd3d11'
$root = Join-Path ([IO.Path]::GetTempPath()) "pcu-dll-planting-$PID"
$bin = Join-Path $root 'app'; $data = Join-Path $root 'appdata'
New-Item -ItemType Directory -Force $bin, $data | Out-Null
# Same shape as elevation.rs: inbound, first and only instance, local clients only.
$pipeName = '\\.\pipe\pc-tweaker-uninstaller-report-{0:x}-0-{1:x}' -f $PID, [DateTime]::UtcNow.Ticks
$pipe = [PcuCheck.Pipe]::CreateNamedPipeW($pipeName, 0x80001, 0x8, 1, 0, 65536, 0, [IntPtr]::Zero)
if ($pipe.IsInvalid) { throw "could not create the report pipe $pipeName" }
try {
  Copy-Item -LiteralPath $Exe (Join-Path $bin 'pc-tweaker-uninstaller.exe')
  foreach ($n in $names) { Set-Content -LiteralPath (Join-Path $bin "$n.dll") -Value 'placeholder, not a DLL' -Encoding ascii }
  $enc = { param($v) 'b64:' + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($v)).TrimEnd('=').Replace('+', '-').Replace('/', '_') }
  $psi = [Diagnostics.ProcessStartInfo]::new((Join-Path $bin 'pc-tweaker-uninstaller.exe'))
  foreach ($a in '--elevated-uninstall', (& $enc 'bogus'), (& $enc 'x'), (& $enc ('0' * 64)), (& $enc $pipeName)) { $psi.ArgumentList.Add($a) }
  $psi.WorkingDirectory = $bin
  $psi.UseShellExecute = $false
  $psi.Environment['APPDATA'] = $data
  $p = [Diagnostics.Process]::Start($psi)
  if (-not $p.WaitForExit(60000)) { $p.Kill(); throw 'the child did not exit within 60 s' }
  $client = [uint32]0
  $fromChild = [PcuCheck.Pipe]::GetNamedPipeClientProcessId($pipe, [ref]$client) -and $client -eq $p.Id
  $report = ''
  if ($fromChild) {
    $buffer = [IO.MemoryStream]::new()
    try { [IO.FileStream]::new($pipe, [IO.FileAccess]::Read).CopyTo($buffer) } catch { }
    $report = [Text.Encoding]::UTF8.GetString($buffer.ToArray())
  }
  $refused = $report -match '"success":false' -and $report -match 'Unknown program source'
  $written = @(Get-ChildItem -LiteralPath $data -Recurse -Force)
  $ok = $p.ExitCode -eq 0 -and $refused -and $written.Count -eq 0
  $detail = "exit 0x{0:X8}, report {1}, files written {2}" -f $p.ExitCode, $(if ($refused) { 'received' } elseif ($report) { "unexpected: $report" } else { 'missing' }), $written.Count
  if ($ExpectFailure) {
    if ($ok) { throw "expected the unprotected build to fail with placeholders next to it, but it ran ($detail)" }
    "DLL planting check detects the unprotected build ($detail)"
  } else {
    if (-not $ok) { throw "a placeholder next to the exe was loaded or the child failed ($detail)" }
    "DLL planting check ok: $($names.Count) placeholders next to the exe were ignored ($detail)"
  }
} finally {
  $pipe.Dispose()
  Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
