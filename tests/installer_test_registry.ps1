# Shared by installer test fixtures only; never used by the production installer.
function Set-InstallerTestAutoStart {
    param(
        [Parameter(Mandatory=$true)][string]$RegistryPath,
        [Parameter(Mandatory=$true)][ValidatePattern('^DaozaiSetupTest[0-9a-f]{32}$')][string]$AppId,
        [Parameter(Mandatory=$true)][string]$Executable
    )
    # Fresh CI profiles can have no Run key. -Force on a value cannot create its key.
    if (-not (Test-Path -LiteralPath $RegistryPath)) {
        New-Item -Path $RegistryPath -Force | Out-Null
    }
    New-ItemProperty -LiteralPath $RegistryPath -Name $AppId -Value ('"' + $Executable + '"') -PropertyType String -Force | Out-Null
}
