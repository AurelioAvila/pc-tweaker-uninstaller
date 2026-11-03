$ErrorActionPreference = 'Stop'

# Exact match: this must never select the separate PC Tweaker application.
[array]$registryEntries = Get-UninstallRegistryKey -SoftwareName 'PC Tweaker Uninstaller'
if ($registryEntries.Count -eq 1) {
  $packageArgs = @{
    packageName    = $env:ChocolateyPackageName
    fileType       = 'exe'
    silentArgs     = '/S'
    validExitCodes = @(0)
    file           = $registryEntries[0].UninstallString.Trim('"')
  }
  Uninstall-ChocolateyPackage @packageArgs
} elseif ($registryEntries.Count -eq 0) {
  Write-Warning 'PC Tweaker Uninstaller has already been uninstalled by other means.'
} else {
  Write-Warning 'Multiple exact PC Tweaker Uninstaller entries were found; no uninstaller was run. Review Installed apps.'
}
