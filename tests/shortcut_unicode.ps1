$ErrorActionPreference = 'Stop'
$project = Split-Path $PSScriptRoot -Parent
# An ASCII temp path also lets WScript demonstrate the target encoding bug.
$root = Join-Path $env:TEMP ('XiaoDaJiShortcutTest-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$compiler = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
$manifest = Join-Path $root 'english.manifest'
$xml = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="XiaoDaJi.ShortcutUnicodeTest" version="1.0.0.0" />
  <application xmlns="urn:schemas-microsoft-com:asm.v3"><windowsSettings>
    <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">en-US</activeCodePage>
  </windowsSettings></application>
</assembly>'
[IO.File]::WriteAllText($manifest, $xml, [Text.UTF8Encoding]::new($false))
# The locale manifest is process-local (Windows 11+); never change system locale.
# Older Windows ignores it, while English CI already has the affected code page.
foreach ($mode in @('system', 'english')) {
    $exe = Join-Path $root ('shortcut-' + $mode + '.exe')
    $options = @('/nologo', '/target:exe', "/out:$exe")
    if ($mode -eq 'english') { $options += "/win32manifest:$manifest" }
    & $compiler @options (Join-Path $project 'installer\windows-shortcut.cs') (Join-Path $PSScriptRoot 'shortcut_unicode.cs')
    if ($LASTEXITCODE -ne 0) { throw "Shortcut regression compilation failed: $mode" }
    $fixtureRoot = Join-Path $root $mode
    New-Item -ItemType Directory -Path $fixtureRoot | Out-Null
    & $exe $fixtureRoot
    if ($LASTEXITCODE -ne 0) { throw "Shortcut Unicode regression failed: $mode" }
}
Write-Output "PASS: shortcut regression fixtures: $root"
