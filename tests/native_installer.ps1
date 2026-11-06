param([Parameter(Mandatory=$true)][string]$NsisCompiler)
$ErrorActionPreference = 'Stop'
$project = Split-Path $PSScriptRoot -Parent
if (-not ('XiaoDaJi.WindowsShortcut' -as [type])) { Add-Type -Path (Join-Path $project 'installer\windows-shortcut.cs') }
$id = 'DaozaiSetupTest' + [Guid]::NewGuid().ToString('N')
$name = '小妲己安装测试-' + $id.Substring(15,8)
$legacyName = '旧品牌测试-' + $id.Substring(15,8)
$root = Join-Path $project ('target\' + $id)
$installed = Join-Path $root 'installed 中文 files'
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\' + $id
$data = Join-Path $env:APPDATA $id
New-Item -ItemType Directory -Path $root -Force | Out-Null
$sentinel = Join-Path $data 'settings.txt'
$source = Join-Path $root 'fixture.cs'
'[assembly:System.Reflection.AssemblyFileVersion("9.8.7")] class Fixture { static void Main() { } }' | Set-Content -LiteralPath $source
$fixture = Join-Path $root 'fixture.exe'
& (Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe') /nologo /target:winexe "/win32icon:$(Join-Path $project 'assets\xiaodaji.ico')" "/out:$fixture" $source
if ($LASTEXITCODE -ne 0) { throw '测试程序编译失败' }
$setup = Join-Path $root 'Daozai-TestInstaller.exe'
& $NsisCompiler /V2 /WX "/DAPP_ID=$id" "/DAPP_NAME=$name" "/DLEGACY_APP_NAME=$legacyName" /DAPP_VERSION=9.8.7 "/DPROJECT_DIR=$project" "/DSOURCE_EXE=$fixture" "/DOUTPUT_FILE=$setup" (Join-Path $project 'installer\setup.nsi')
if ($LASTEXITCODE -ne 0) { throw '测试安装器编译失败' }
New-Item -ItemType Directory -Path $data -Force | Out-Null
Set-Content -LiteralPath $sentinel -Value 'keep-user-data'
function Run-Setup {
    param([string[]]$Options = @(), [switch]$UseRegisteredPath)
    $arguments = @('/S') + $Options
    if (-not $UseRegisteredPath) { $arguments += "/D=$installed" }
    $process = Start-Process -FilePath $setup -ArgumentList $arguments -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(30000) -or $process.ExitCode -ne 0) { throw '安装失败或超时' }
}
function Assert-Installed {
    param([bool]$Desktop = $true, [bool]$AutoStart = $false)
    $entry = Get-ItemProperty -LiteralPath $key
    if ($entry.DisplayName -ne $name) { throw '中文应用显示名不正确' }
    if ($entry.InstallLocation -ne $installed -or $entry.DisplayVersion -ne '9.8.7') { throw '安装注册信息不正确' }
    if ($entry.UninstallString -ne ('"' + (Join-Path $installed '卸载小妲己.exe') + '"')) { throw '卸载命令不正确' }
    if ((Get-FileHash (Join-Path $installed '小妲己.exe')).Hash -ne (Get-FileHash $fixture).Hash) { throw '程序复制不正确' }
    $shell = New-Object -ComObject WScript.Shell
    $desktopLink = Join-Path $shell.SpecialFolders.Item('Desktop') "$name.lnk"
    if ((Test-Path -LiteralPath $desktopLink) -ne $Desktop) { throw '桌面快捷方式选项未生效' }
    $paths = @((Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$name.lnk"), (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\${name}设置.lnk"))
    if ($Desktop) { $paths += $desktopLink }
    foreach ($path in $paths) {
        if (-not (Test-Path -LiteralPath $path)) { throw "快捷方式不存在：$path" }
        # Read the original link via IShellLinkW; WScript can replace Chinese with '?'.
        $shortcut = [XiaoDaJi.WindowsShortcut]::Read($path)
        $actual = $shortcut.TargetPath
        $expected = Join-Path $installed '小妲己.exe'
        if (-not (Test-Path -LiteralPath $actual) -or (Get-Item -LiteralPath $actual).FullName -ne (Get-Item -LiteralPath $expected).FullName) { throw "快捷方式目标不正确：$actual；预期：$expected" }
        $arguments = if ([IO.Path]::GetFileName($path) -eq "${name}设置.lnk") { '--settings' } else { '' }
        if ($shortcut.Arguments -ne $arguments) { throw "快捷方式启动参数不正确：$path" }
        if ($shortcut.WorkingDirectory -ne $installed) { throw "快捷方式工作目录不正确：$path" }
    }
    foreach ($legacyFile in @('daozai.exe','Uninstall.exe')) {
        if (Test-Path -LiteralPath (Join-Path $installed $legacyFile)) { throw "旧文件未清理：$legacyFile" }
    }
    if ($entry.DisplayIcon -ne ((Join-Path $installed '小妲己.exe') + ',0')) { throw '应用列表图标路径不正确' }
    $run = (Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue).$id
    if ($AutoStart) {
        if ($run -ne ('"' + (Join-Path $installed '小妲己.exe') + '"')) { throw '开机自启选项或路径不正确' }
    } elseif ($run) { throw '未勾选自启却设置了开机启动' }
    if ((Get-Content -LiteralPath $sentinel -Raw).Trim() -ne 'keep-user-data') { throw '用户数据被修改' }
}
try {
    Run-Setup
    Assert-Installed
    & (Join-Path $project 'scripts\check-icons.ps1') -Executable (Join-Path $installed '小妲己.exe')
    & (Join-Path $project 'scripts\check-icons.ps1') -Executable (Join-Path $installed '卸载小妲己.exe')
    Write-Output '通过：首次安装、中文含空格路径、中文文件名、快捷方式和卸载注册、程序与卸载器狐狸图标'
    # Exercise the same legacy-link cleanup branch as the real product, but with
    # unique test-only names, never the user's existing Island shortcuts.
    $shell = New-Object -ComObject WScript.Shell
    $legacyDesktop = Join-Path $shell.SpecialFolders.Item('Desktop') "$legacyName.lnk"
    $legacyMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$legacyName.lnk"
    $legacySettings = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\${legacyName}设置.lnk"
    foreach ($path in @($legacyDesktop, $legacyMenu)) {
        [XiaoDaJi.WindowsShortcut]::Write($path, (Join-Path $installed '小妲己.exe'), '')
    }
    [XiaoDaJi.WindowsShortcut]::Write($legacySettings, $fixture, '')
    $currentDesktop = Join-Path $shell.SpecialFolders.Item('Desktop') "$name.lnk"
    Remove-Item -LiteralPath $currentDesktop
    Run-Setup -UseRegisteredPath
    Assert-Installed
    if ((Test-Path -LiteralPath $legacyDesktop) -or (Test-Path -LiteralPath $legacyMenu)) { throw '同目录旧品牌快捷方式未移除' }
    if (-not (Test-Path -LiteralPath $legacySettings)) { throw '误删其他目录的快捷方式' }
    Remove-Item -LiteralPath $legacySettings
    Write-Output '通过：旧品牌快捷方式替换、保留桌面选择、不删除其他目标的链接'
    # Reproduce v0.0.1 filenames and enabled autostart without touching the real install.
    Copy-Item -LiteralPath (Join-Path $installed '小妲己.exe') -Destination (Join-Path $installed 'daozai.exe')
    Copy-Item -LiteralPath (Join-Path $installed '卸载小妲己.exe') -Destination (Join-Path $installed 'Uninstall.exe')
    Remove-Item -LiteralPath (Join-Path $installed '小妲己.exe'), (Join-Path $installed '卸载小妲己.exe')
    foreach ($path in @($currentDesktop, (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$name.lnk"), (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\${name}设置.lnk"))) {
        [XiaoDaJi.WindowsShortcut]::Write($path, (Join-Path $installed 'daozai.exe'), '')
    }
    New-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name $id -Value ('"'+(Join-Path $installed 'daozai.exe')+'"') -PropertyType String -Force | Out-Null
    Set-ItemProperty -LiteralPath $key -Name UninstallString -Value ('"'+(Join-Path $installed 'Uninstall.exe')+'"')
    Run-Setup -UseRegisteredPath
    Assert-Installed -AutoStart $true
    Write-Output '通过：v0.0.1旧文件名覆盖迁移、保留自启选择并修正路径、无旧程序残留'
    Set-Content -LiteralPath (Join-Path $installed '小妲己.exe') -Value 'previous-version'
    Set-Content -LiteralPath (Join-Path $installed 'uninstall.ps1') -Value 'previous-uninstaller'
    Set-ItemProperty -LiteralPath $key -Name UninstallString -Value 'powershell.exe legacy-uninstall.ps1'
    Run-Setup
    Assert-Installed -AutoStart $true
    if (Test-Path -LiteralPath (Join-Path $installed 'uninstall.ps1')) { throw '旧卸载脚本未移除' }
    Write-Output '通过：覆盖升级、旧脚本安装迁移、保留数据'
    Run-Setup -Options @('/DESKTOP=0','/AUTOSTART=1')
    Assert-Installed -Desktop $false -AutoStart $true
    Write-Output '通过：取消桌面快捷方式、开启自启'
    Run-Setup -UseRegisteredPath
    Assert-Installed -Desktop $false -AutoStart $true
    Write-Output '通过：升级保留现有路径和选项'
    Run-Setup -Options @('/DESKTOP=1','/AUTOSTART=0')
    Assert-Installed
    Write-Output '通过：恢复桌面快捷方式、关闭自启'
    $mutex = [Threading.Mutex]::new($false, "Local\$id.SingleInstance")
    try {
        $process = Start-Process -FilePath $setup -ArgumentList @('/S',"/D=$installed") -WindowStyle Hidden -PassThru
        if (-not $process.WaitForExit(30000) -or $process.ExitCode -eq 0) { throw '运行中的程序未阻止覆盖安装' }
        Assert-Installed
        Write-Output '通过：运行中拒绝覆盖程序'
    } finally { $mutex.Dispose() }
    Run-Setup -Options @('/AUTOSTART=1')
    Assert-Installed -AutoStart $true
    Set-Content -LiteralPath (Join-Path $installed 'user-file.txt') -Value 'keep-extra-file'
    # 使用原安装路径运行原生卸载器，只操作本测试的独立产品标识。
    $process = Start-Process -FilePath (Join-Path $installed '卸载小妲己.exe') -ArgumentList @('/S',"_?=$installed") -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(30000) -or $process.ExitCode -ne 0) { throw '卸载失败或超时' }
    if ((Test-Path -LiteralPath $key) -or (Test-Path -LiteralPath (Join-Path $installed '小妲己.exe'))) { throw '卸载残留程序或注册信息' }
    if ((Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue).$id) { throw '卸载残留自启项' }
    if (-not (Test-Path -LiteralPath (Join-Path $installed 'user-file.txt'))) { throw '卸载误删额外文件' }
    if ((Get-Content -LiteralPath $sentinel -Raw).Trim() -ne 'keep-user-data') { throw '卸载误删用户数据' }
    Write-Output '通过：原生卸载、保留用户数据和额外文件'
} finally {
    # Remove only explicitly named test links in case a preceding assertion fails.
    foreach ($path in @($legacyDesktop, $legacyMenu, $legacySettings)) {
        if ($path -and (Test-Path -LiteralPath $path)) { Remove-Item -LiteralPath $path }
    }
    # 清理本测试的独立安装，避免失败时残留桌面快捷方式和自启项。
    if (Test-Path -LiteralPath $key) {
        $cleanupDirectory = (Get-ItemProperty -LiteralPath $key).InstallLocation
        $defaultDirectory = Join-Path $env:LOCALAPPDATA "Programs\$id"
        if ($cleanupDirectory -ne $installed -and $cleanupDirectory -ne $defaultDirectory) { throw '测试清理目录超出预期范围' }
        $cleanup = Start-Process -FilePath (Join-Path $cleanupDirectory '卸载小妲己.exe') -ArgumentList @('/S',"_?=$cleanupDirectory") -WindowStyle Hidden -PassThru
        $null = $cleanup.WaitForExit(30000)
    }
    if (-not (Test-Path -LiteralPath $key)) {
        Remove-Item -LiteralPath $sentinel
        Remove-Item -LiteralPath $data
    }
}
