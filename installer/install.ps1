param([string]$SourceExecutable)
if(-not $SourceExecutable){
    $packaged=Join-Path $PSScriptRoot '..\小妲己.exe'
    $SourceExecutable=if(Test-Path -LiteralPath $packaged){$packaged}else{Join-Path $PSScriptRoot '..\target\release\daozai.exe'}
}
$ErrorActionPreference='Stop'
$sourcePath=[System.IO.Path]::GetFullPath($SourceExecutable)
if(-not(Test-Path -LiteralPath $sourcePath)){throw '找不到小妲己程序，请先完成构建。'}
$installDir=Join-Path $env:LOCALAPPDATA 'Programs\Daozai'
$exePath=Join-Path $installDir '小妲己.exe'
$legacyExe=Join-Path $installDir 'daozai.exe'
if(Test-Path -LiteralPath (Join-Path $installDir '卸载小妲己.exe')){throw '检测到原生安装，请使用新版小妲己安装包覆盖升级。'}
if(Test-Path -LiteralPath (Join-Path $installDir 'Uninstall.exe')){throw '检测到旧版原生安装，请使用新版小妲己安装包覆盖升级。'}
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
$running=Get-Process -Name 'daozai','小妲己' -ErrorAction SilentlyContinue | Where-Object {$_.Path -in @($exePath,$legacyExe)}
if($running){throw '请先从托盘退出小妲己，再重新安装。'}
if(Test-Path -LiteralPath $exePath){Copy-Item -LiteralPath $exePath -Destination (Join-Path $installDir 'daozai-previous.exe') -Force}
Copy-Item -LiteralPath $sourcePath -Destination $exePath -Force
if(Test-Path -LiteralPath (Join-Path $PSScriptRoot '..\THIRD_PARTY_NOTICES.txt')){Copy-Item -LiteralPath (Join-Path $PSScriptRoot '..\THIRD_PARTY_NOTICES.txt') -Destination (Join-Path $installDir 'THIRD_PARTY_NOTICES.txt') -Force}
if(Test-Path -LiteralPath (Join-Path $PSScriptRoot '..\LICENSE')){Copy-Item -LiteralPath (Join-Path $PSScriptRoot '..\LICENSE') -Destination (Join-Path $installDir 'LICENSE') -Force}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'uninstall.ps1') -Destination (Join-Path $installDir 'uninstall.ps1') -Force
# Migrate an existing enabled autostart entry, but do not enable it on first install.
$runKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$oldRun=(Get-ItemProperty -LiteralPath $runKey -ErrorAction SilentlyContinue).Daozai
if($oldRun -eq ('"'+$legacyExe+'"')){Set-ItemProperty -LiteralPath $runKey -Name Daozai -Value ('"'+$exePath+'"')}
if(Test-Path -LiteralPath $legacyExe){Remove-Item -LiteralPath $legacyExe}
# Native installs should always use the native package, not be downgraded to a script uninstaller.
$shell=New-Object -ComObject WScript.Shell
# Remove only old-brand links that point to this installation.
$legacyLinks=@((Join-Path $shell.SpecialFolders.Item('Desktop') '岛仔.lnk'),(Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\岛仔.lnk'),(Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\岛仔设置.lnk'))
foreach($legacy in $legacyLinks){if((Test-Path -LiteralPath $legacy) -and $shell.CreateShortcut($legacy).TargetPath -in @($exePath,$legacyExe)){Remove-Item -LiteralPath $legacy}}
$entries=@(
    @((Join-Path $shell.SpecialFolders.Item('Desktop') '小妲己.lnk'),''),
    @((Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\小妲己.lnk'),''),
    @((Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\小妲己设置.lnk'),'--settings')
)
foreach($entry in $entries){
    $link=$shell.CreateShortcut($entry[0]);$link.TargetPath=$exePath;$link.Arguments=$entry[1]
    $link.WorkingDirectory=$installDir;$link.IconLocation=$exePath+',0';$link.Description='小妲己';$link.Save()
}
$key='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Daozai'
New-Item -Path $key -Force | Out-Null
$version=(Get-Item -LiteralPath $exePath).VersionInfo.FileVersion
$psExe=Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
$values=@{DisplayName='小妲己';DisplayVersion=$version;Publisher='小妲己';InstallLocation=$installDir;DisplayIcon=$exePath+',0';UninstallString=('"'+$psExe+'" -NoProfile -ExecutionPolicy Bypass -File "'+(Join-Path $installDir 'uninstall.ps1')+'"')}
foreach($name in $values.Keys){New-ItemProperty -Path $key -Name $name -Value $values[$name] -PropertyType String -Force | Out-Null}
foreach($name in @('NoModify','NoRepair')){New-ItemProperty -Path $key -Name $name -Value 1 -PropertyType DWord -Force | Out-Null}
Start-Process -FilePath $exePath -WorkingDirectory $installDir -WindowStyle Hidden
Write-Output ('小妲己 '+$version+' 已安装：'+$exePath)
