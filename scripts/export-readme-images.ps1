#Requires -Version 5.1
<#
.SYNOPSIS
Export current egui previews and compose the README figures with synthetic data.
.DESCRIPTION
Requires the Windows Rust/MSVC toolchain, Python 3.11+, Pillow and Microsoft YaHei.
No production app, account connection, player control or system notification runs.
An optional TestBinary must be a lib test executable built from the CURRENT source.
#>
param([string]$TestBinary = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$preview = Join-Path $root 'target\readme-previews'
$exports = @{
    DAOZAI_UI_PREVIEW_DIR = (Join-Path $preview 'panels')
    DAOZAI_CALENDAR_PREVIEW_DIR = (Join-Path $preview 'calendar')
    DAOZAI_WORK_PREVIEW_DIR = (Join-Path $preview 'work')
    DAOZAI_MAIL_PREVIEW = (Join-Path $preview 'mail')
}
$tests = @(
    'ui_alignment_layout_gallery', 'calendar_layout_gallery',
    'work_shelf_ui_preview', 'mailbox_readability_gallery',
    'api_balance_sv_visual_preview', 'api_model_settings_visual_preview'
)
$previous = @{}
Push-Location $root
try {
    foreach ($name in $exports.Keys) {
        $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
        [Environment]::SetEnvironmentVariable($name, $exports[$name], 'Process')
    }
    if ($TestBinary) {
        $TestBinary = (Resolve-Path -LiteralPath $TestBinary).Path
        $listed = & $TestBinary --list
        if ($LASTEXITCODE -ne 0) { throw 'Unable to list preview tests.' }
        foreach ($test in $tests) {
            if ($listed -notcontains ('ui_tests::{0}: test' -f $test)) {
                throw "Missing preview test in binary: $test"
            }
        }
    }
    foreach ($test in $tests) {
        if ($TestBinary) {
            & $TestBinary --ignored --exact "ui_tests::$test" --test-threads=1
        } else {
            & cargo test --locked --release --lib "ui_tests::$test" -- --ignored --exact --test-threads=1
        }
        if ($LASTEXITCODE -ne 0) { throw "Preview export failed: $test" }
    }
    & python (Join-Path $PSScriptRoot 'render-readme-images.py')
    if ($LASTEXITCODE -ne 0) { throw 'README image composition failed.' }
} finally {
    foreach ($name in $previous.Keys) {
        [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process')
    }
    Pop-Location
}
