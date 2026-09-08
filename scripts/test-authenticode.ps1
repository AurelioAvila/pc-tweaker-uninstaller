param([Parameter(Mandatory)][string]$SignedFile, [Parameter(Mandatory)][string]$UnsignedFile)
$ErrorActionPreference = 'Stop'
$verifier = Join-Path $PSScriptRoot 'verify-authenticode.ps1'
function Assert-Rejected([string]$Name, [scriptblock]$Action, [string]$Pattern) {
    try { & $Action | Out-Null }
    catch {
        if ($_.Exception.Message -notmatch $Pattern) { throw }
        Write-Output "PASS: $Name"
        return
    }
    throw "Expected rejection: $Name"
}
$result = & $verifier -Path $SignedFile
if ($result.Status -ne 'Valid') { throw 'Signed fixture was not accepted.' }
Write-Output 'PASS: trusted signed installer accepted'
Assert-Rejected 'unsigned installer' { & $verifier -Path $UnsignedFile } 'Authenticode verification failed'
Assert-Rejected 'wrong publisher' { & $verifier -Path $SignedFile -ExpectedPublisher 'Wrong Publisher' } 'Unexpected publisher'
Assert-Rejected 'missing tool' { & $verifier -Path $SignedFile -SignToolPath "$PSScriptRoot/missing-signtool.exe" } 'SignTool is required'
Assert-Rejected 'missing file' { & $verifier -Path "$PSScriptRoot/missing-installer.exe" } 'does not exist|Cannot find'
Assert-Rejected 'unsupported file' { & $verifier -Path $PSCommandPath } 'Expected an EXE'
