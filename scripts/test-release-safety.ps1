$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/release-safety.ps1"
function Assert-Rejected([scriptblock]$Action) {
    try { & $Action }
    catch { return }
    throw 'Expected rejection.'
}
Assert-ReleaseTag 'v0.8.2'
Assert-ReleaseTag 'v10.20.30'
foreach ($tag in @('', 'v01.2.3', '1.2.3', 'v1.2.3-beta', "v1.2.3`n", "v1.2.3`r`ninjected=true")) {
    Assert-Rejected { Assert-ReleaseTag $tag }
}
$root = (Resolve-Path -LiteralPath ([IO.Path]::GetTempPath())).ProviderPath
$directory = [IO.Path]::GetFullPath((Join-Path $root ('uninstaller-release-' + [guid]::NewGuid())))
New-Item -ItemType Directory -Path $directory | Out-Null
try {
    $resolved = (Resolve-Path -LiteralPath $directory).ProviderPath
    Assert-TemporaryDirectory $resolved $directory $root
    Assert-Rejected { Assert-TemporaryDirectory $root $root $root }
    Assert-Rejected { Assert-TemporaryDirectory $resolved ($directory + '-other') $root }
    Assert-Rejected { Assert-TemporaryDirectory $resolved $directory ($root + '-sibling') }
    Assert-Rejected { Assert-TemporaryDirectory 'relative/path' $directory $root }
    $child = Join-Path $directory 'child'
    Assert-Rejected { Assert-TemporaryDirectory $child $child $root }
} finally {
    $resolved = (Resolve-Path -LiteralPath $directory).ProviderPath
    Assert-TemporaryDirectory $resolved $directory $root
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
Write-Output 'PASS: release tag and temporary directory containment checks'
