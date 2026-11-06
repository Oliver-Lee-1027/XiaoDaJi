param(
    [Parameter(Mandatory=$true)][string]$InstallDirectory,
    [Parameter(Mandatory=$true)][string]$LegacyName,
    [Parameter(Mandatory=$true)][string]$DesktopDirectory,
    [Parameter(Mandatory=$true)][string]$ProgramsDirectory
)
$ErrorActionPreference = 'Stop'
# Validate the name before joining paths; never interpret it as a path itself.
if ([string]::IsNullOrWhiteSpace($LegacyName) -or $LegacyName.IndexOfAny([IO.Path]::GetInvalidFileNameChars()) -ge 0 -or $LegacyName -in @('.', '..')) { throw '旧品牌名称无效。' }
$install = [IO.Path]::GetFullPath($InstallDirectory)
if ([IO.Path]::IsPathRooted($InstallDirectory) -eq $false -or [IO.Path]::IsPathRooted($DesktopDirectory) -eq $false -or [IO.Path]::IsPathRooted($ProgramsDirectory) -eq $false) { throw '快捷方式清理只接受绝对目录。' }
$expected = @((Join-Path $install 'daozai.exe'), (Join-Path $install '小妲己.exe'))
if (-not ('XiaoDaJi.WindowsShortcut' -as [type])) { Add-Type -Path (Join-Path $PSScriptRoot 'windows-shortcut.cs') }
$paths = @((Join-Path $DesktopDirectory ($LegacyName+'.lnk')), (Join-Path $ProgramsDirectory ($LegacyName+'.lnk')), (Join-Path $ProgramsDirectory ($LegacyName+'设置.lnk')))
foreach ($path in $paths) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }
    $target = [XiaoDaJi.WindowsShortcut]::Read($path).TargetPath
    if ($target -and $expected -contains [IO.Path]::GetFullPath($target)) {
        Remove-Item -LiteralPath $path -Force
    }
}
