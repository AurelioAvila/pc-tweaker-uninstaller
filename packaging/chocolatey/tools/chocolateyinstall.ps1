$ErrorActionPreference = 'Stop'

if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64' -and $env:PROCESSOR_ARCHITEW6432 -ne 'AMD64') {
  throw 'PC Tweaker Uninstaller requires Windows x64.'
}

$packageArgs = @{
  packageName    = $env:ChocolateyPackageName
  fileType       = 'exe'
  url64bit       = 'https://github.com/AurelioAvila/pc-tweaker-uninstaller/releases/download/v0.14.0/PC.Tweaker.Uninstaller_0.14.0_x64-setup.exe'
  checksum64     = '65c8efbd661bec137141fc2cbe90432434b734fecbaf2d0b4dd14281a84c118f'
  checksumType64 = 'sha256'
  silentArgs     = '/S'
  validExitCodes = @(0)
  softwareName   = 'PC Tweaker Uninstaller'
}

Install-ChocolateyPackage @packageArgs
