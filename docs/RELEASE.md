# 小妲己 v0.0.4 推送、发布与下载

最后更新：2026-11-06。

保留 v0.0.1 / v0.0.2 / v0.0.3 的标签和附件；本次新建 v0.0.4，不覆盖历史版本。AI 不代为执行 `git commit`、`git push`，也不代为发布 Release。

## 1. 手动提交并推送源码

确认差异只包含要发布的内容，由开发者本人执行：

```powershell
cd 'D:\桌面灵动岛\岛仔源\XiaoDaJi'
git remote -v
git status --short
git diff --check
git add Cargo.toml Cargo.lock app.rc CHANGELOG.md README.md docs/BRANDING.md docs/USAGE.md docs/RELEASE.md docs/releases/v0.0.4.md
git add .github/workflows/ci.yml installer/setup.nsi installer/cleanup-legacy-links.ps1 installer/windows-shortcut.cs tests/native_installer.ps1 tests/branding.rs tests/shortcut_unicode.ps1 tests/shortcut_unicode.cs src/updates.rs
git diff --cached --stat
git commit -m "发布小妲己 v0.0.4：修复中文快捷方式校验"
git -c http.sslBackend=schannel -c http.version=HTTP/1.1 push -u origin main
```

远程应为 `https://github.com/Oliver-Lee-1027/XiaoDaJi.git`。不要强制推送、关闭证书校验或覆盖旧标签。`target/`、`dist/` 不作为源码提交，成品通过 Release 附件上传。

推送后进入 GitHub **Actions → Windows 检查**，打开本次提交的运行，确认 `test` 与 `installer` 两个任务均通过，再发布 Release。不要仅对旧提交点 Re-run：旧提交不包含这次修复。

## 2. 新建 v0.0.4 Release

1. 确认 `main` 已包含本次新提交，且对应 Actions 全部通过。
2. 进入仓库 **Releases → Draft a new release / Create a new release**。
3. **Choose a tag** 输入 `v0.0.4` 并创建新标签，**Target** 选择本次已检查的 `main` 提交。如果该标签已存在，先核对，不要覆盖。
4. **Release title** 填 `小妲己 v0.0.4`。
5. 发布说明复制 `docs/releases/v0.0.4.md`。
6. 从 `dist\v0.0.4` 上传以下三个文件，等待上传全部完成：
   - `小妲己-安装包-v0.0.4-x64.exe`
   - `小妲己-v0.0.4-windows-x64.zip`
   - `SHA256SUMS.txt`
7. 不勾选 **Set as a pre-release**，设为 **Latest / Set as the latest release**（如显示），然后 **Publish release**。
8. 发布后核对标签、Latest 标识及三个附件，再按下一节验收。

应用内更新请求本仓库的 `releases/latest`。v0.0.4 支持完整中文 ZIP 名和 GitHub 可能生成的 `-v版本号-windows-x64.zip`，仍严格校验同仓库、版本 URL、大小和 SHA-256。上传本地原始文件即可，不要随意重命名；网页上 EXE 也可能显示为 `-.-v0.0.4-x64.exe`。只上传 EXE、只推送源码、只创建标签、草稿或预发布都不能替代正式更新发布。GitHub 自动生成的 Source code ZIP/TAR.GZ 不是 Windows 安装包。

## 3. 发布后的真实更新验收

1. **v0.0.3 正式包：** 在「设置 → 关于 → 检查更新」可发现 `0.0.4`；但旧版不识别 GitHub 移除中文前缀后的 ZIP 名，本次请点更新窗口的「浏览器下载」下载安装包，退出后保留原位置覆盖升级。v0.0.4 已兼容这种文件名，用于后续更新；GitHub 仍须返回有效 ZIP SHA-256 digest，缺失时不要绕过检查。
2. **v0.0.1 / v0.0.2：** 这些版本没有更新源，先从托盘退出，运行 v0.0.4 安装包，保留原安装目录覆盖安装，不先卸载。
3. 更新完成后确认关于页为 `0.0.4`，设置、待办、置物架及相关记录保留。
4. 检查桌面/开始菜单快捷方式可启动程序，设置快捷方式可打开设置；不要同时运行旧版本。
5. v0.0.4 与远端 v0.0.4 相同，不应提示再次升级。网络失败与版本比较结果要分开判断。
6. 便携用户可退出旧版，把完整 ZIP 解压到新的版本目录再运行 `小妲己.exe`；数据仍使用 `%APPDATA%\Daozai`。

本地构建和隔离安装测试不能替代发布后的真实 GitHub 下载、覆盖更新和重启验收。本轮没有替换用户日常使用的安装。

## 4. 重新打包

在仓库根目录打开 PowerShell：

```powershell
$env:XIAODAJI_RELEASE_REPOSITORY = 'Oliver-Lee-1027/XiaoDaJi'
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\package.ps1 -OutputDirectory .\dist\v0.0.4
```

需要 Rust/MSVC、Windows SDK 和 NSIS 3，可用 `-NsisCompiler` 指定 `makensis.exe`。更新源是编译时配置，每次正式构建前都要设置；不要加 `-SkipBuild` 复用旧版或未配置更新源的程序。`dist\v0.0.4` 已存在时，应另选全新的输出目录，不覆盖旧产物。

本轮复用本机已有工具与构建缓存的命令如下（其他电脑需按实际路径修改）：

```powershell
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_TARGET_DIR = 'D:\桌面灵动岛\岛仔源\xiaotang-daozai\target'
$env:XIAODAJI_RELEASE_REPOSITORY = 'Oliver-Lee-1027/XiaoDaJi'
.\scripts\package.ps1 -NsisCompiler 'D:\桌面灵动岛\岛仔源\xiaotang-daozai\target\packaging-tools\nsis-3.13\makensis.exe' -OutputDirectory .\dist\v0.0.4
```

## 5. 验证记录与边界

- 快捷方式回归：`tests/shortcut_unicode.ps1` 在独立进程测试系统代码页和英文代码页，不修改系统语言；旧版 Windows 可能忽略进程语言清单，以实际输出的 `ACP` 为准。
- 安装/更新测试记录：`target/v0.0.4-installer-tests.log`。
- Rust 回归记录：`target/v0.0.4-rust-tests.log`。
- 构建记录：`target/v0.0.4-build.log`；成品核验：`target/v0.0.4-artifact-verification.json`。
- 本地记录不是 GitHub Actions 的运行结果；本次提交的云端检查须在推送后确认。

本轮本地结果：全目标回归 483 通过、0 失败、42 忽略；随后新增附件名兼容逻辑的更新模块回归 11 通过、0 失败、2 忽略；最终品牌/版本回归 5 通过。系统 ACP 936 与英文进程 ACP 1252 的 Unicode 快捷方式回归均通过，旧 WScript 在 ACP 1252 的乱码已复现；隔离安装、升级、卸载及 8 个更新脚本场景通过。成品版本、更新源、ZIP 程序一致性及 SHA-256 已核对。汇总见 `target/v0.0.4-validation-summary.json`（本地生成，不提交）。
