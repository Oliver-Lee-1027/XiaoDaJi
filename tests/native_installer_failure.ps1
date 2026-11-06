# Ensure a missing uninstaller in finally cannot hide the original migration error.
param([Parameter(Mandatory=$true)][string]$NsisCompiler)
$ErrorActionPreference = 'Stop'
$project = Split-Path $PSScriptRoot -Parent
$root = Join-Path $project ('target\installer-failure-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$stdout = Join-Path $root 'stdout.log'
$stderr = Join-Path $root 'stderr.log'
$shell = (Get-Process -Id $PID).Path
$script = Join-Path $PSScriptRoot 'native_installer.ps1'
# Use a child process: the injected failure MUST exit nonzero without failing this verifier.
$arguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "{0}" -NsisCompiler "{1}" -SimulateMissingUninstaller' -f $script, $NsisCompiler
$process = Start-Process -FilePath $shell -ArgumentList $arguments -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
try {
    if (-not $process.WaitForExit(180000)) {
        $process.Kill()
        $null = $process.WaitForExit(5000)
        throw '清理故障回归超时；已停止本测试子进程'
    }
    $exitCode = $process.ExitCode
} finally { $process.Dispose() }
$errorOutput = Get-Content -LiteralPath $stderr -Raw
$output = (Get-Content -LiteralPath $stdout -Raw) + "`n" + $errorOutput
if ($null -eq $exitCode -or $exitCode -eq 0) { throw "注入的原始失败被吞掉；日志：$root" }
if ($output -notmatch 'INJECTED_MISSING_UNINSTALLER: (DaozaiSetupTest[0-9a-f]{32})') {
    throw "未运行到故障注入点；日志：$root`n$output"
}
$id = $Matches[1]
if ($errorOutput -notmatch ('INJECTED_MISSING_UNINSTALLER: ' + [regex]::Escape($id))) {
    throw "向外抛出的不是原始异常；日志：$root`n$output"
}
if ($output -notmatch 'TEST_CLEANUP_MISSING_UNINSTALLER:') { throw "缺失卸载器分支未覆盖；日志：$root" }
if ($output -match 'TEST_CLEANUP_SECONDARY_FAILURE:|Start-Process\s*:') {
    throw "清理阶段仍有附加异常；日志：$root`n$output"
}
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\' + $id
if (Test-Path -LiteralPath $key) { throw '故障后仍残留测试卸载注册项' }
if ((Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue).$id) { throw '故障后仍残留测试自启项' }
$shellObject = New-Object -ComObject WScript.Shell
$name = '小妲己安装测试-' + $id.Substring(15,8)
$legacyName = '旧品牌测试-' + $id.Substring(15,8)
foreach ($linkName in @($name, $legacyName)) {
    foreach ($path in @((Join-Path $shellObject.SpecialFolders.Item('Desktop') "$linkName.lnk"), (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$linkName.lnk"), (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\${linkName}设置.lnk"))) {
        if (Test-Path -LiteralPath $path) { throw "故障后仍残留测试快捷方式：$path" }
    }
}
if (Test-Path -LiteralPath (Join-Path $env:APPDATA $id)) { throw '故障后仍残留测试用户数据目录' }
Write-Output "通过：卸载器缺失时保留原始非零失败，清理独立测试注册项／快捷方式／自启项；诊断日志：$root"
