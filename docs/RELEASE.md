# 小妲己 v0.0.3 推送、发布与下载

保留 v0.0.1 / v0.0.2 的原标签和附件；本次新建 v0.0.3，不覆盖历史版本。AI 不代为执行 `git commit` 或 `git push`，也未代为在 GitHub 发布。

## 1. 手动提交并推送本次源码

确认本地差异只包含要发布的内容，再由开发者本人逐步执行：

```powershell
cd 'D:\桌面灵动岛\岛仔源\XiaoDaJi'
git remote -v
git status --short
git diff --check
git add Cargo.toml Cargo.lock app.rc CHANGELOG.md README.md docs/BRANDING.md docs/USAGE.md docs/RELEASE.md docs/releases/v0.0.3.md
git commit -m "发布小妲己 v0.0.3：接入独立更新仓库"
git -c http.sslBackend=schannel -c http.version=HTTP/1.1 push -u origin main
```

远程应为 `https://github.com/Oliver-Lee-1027/XiaoDaJi.git`。使用有仓库写权限的账号，不要强制推送或关闭证书校验。`dist/` 不提交为源码，成品使用 Release 附件上传。

## 2. 新建 v0.0.3 Release

1. 打开仓库，确认 `main` 已包含本次新提交。
2. 进入 **Releases → Draft a new release / Create a new release**。
3. **Choose a tag** 输入 `v0.0.3` 并创建新标签，**Target** 选择包含本次新提交的 `main`。如果该标签已存在，先核对其提交和发布内容，不要覆盖。
4. **Release title** 填 `小妲己 v0.0.3`。
5. **Describe this release** 复制 `docs/releases/v0.0.3.md` 的内容。
6. 从 `dist\v0.0.3` 上传以下三个文件并等待完成：
   - `小妲己-安装包-v0.0.3-x64.exe`
   - `小妲己-v0.0.3-windows-x64.zip`
   - `SHA256SUMS.txt`
7. 不勾选 **Set as a pre-release**，选择 **Set as the latest release**（如显示）。
8. 点击 **Publish release**，核对标签 `v0.0.3` 和三个附件。

GitHub 自动生成的 Source code ZIP/TAR.GZ 不是 Windows 安装包。应用内更新严格匹配上面的中文 ZIP 文件名，不要重命名；只上传 EXE 不能提供应用内下载安装。

## 3. 本机安装与更新验证

1. 已安装 v0.0.1 / v0.0.2 的用户先从托盘退出程序，运行新的 EXE 安装包，保留原安装位置覆盖安装；无需先卸载。
2. 安装后「设置 → 关于」应显示 `0.0.3`，检查更新按钮不再因缺少仓库而禁用。
3. v0.0.3 与远端 v0.0.3 相同，不会提示有新版本；后续发布 v0.0.4 或更高版本才会提示升级。
4. GitHub 正式发布前，检查更新可能显示暂未找到正式版本，或当前已是最新版本（远端只有更旧版本时）。网络失败须与版本检查结果区分。
5. 覆盖升级保留 `%APPDATA%\Daozai` 的用户数据，便携用户完整解压到新的版本目录后运行 `小妲己.exe`，不要同时运行旧版。

当前旧版没有更新源，无法靠 GitHub 新建 Release 远程补上，必须手动覆盖安装一次。本轮本地打包不等于已验证 GitHub 发布后的真实下载、覆盖更新和重启闭环。

## 4. 重新打包

```powershell
$env:XIAODAJI_RELEASE_REPOSITORY = 'Oliver-Lee-1027/XiaoDaJi'
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\package.ps1 -OutputDirectory .\dist\v0.0.3
```

需要 Rust/MSVC、Windows SDK 和 NSIS（可使用 `-NsisCompiler` 指定 `makensis.exe`）。不要加 `-SkipBuild` 复用未配置更新源的旧程序。该环境变量是编译时配置，必须在每次正式构建前设置。打包目录已有同名暂存目录时，先选新的输出目录，避免覆盖或混入旧产物。