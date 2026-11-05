# 小妲己 v0.0.2 推送、发布与下载

v0.0.1 是历史版本，保留原标签和下载附件；本次新建 v0.0.2，不覆盖旧标签。本轮不代为执行 `git commit` 或 `git push`。

## 1. 手动提交并推送源码

在 PowerShell 中执行，每步成功后再执行下一步：

```powershell
cd 'D:\桌面灵动岛\岛仔源\XiaoDaJi'
git remote -v
git status --short
git add .
git commit -m "发布小妲己 v0.0.2：中文程序名与图标完善"
git -c http.sslBackend=schannel -c http.version=HTTP/1.1 push -u origin main
```

远程应为 `https://github.com/Oliver-Lee-1027/XiaoDaJi.git`。认证时使用有本仓库写权限的账号。不要强制推送，也不要关闭证书校验。

`dist/` 被 Git 忽略，安装包不作为源码提交，使用下一节上传到 Release。

## 2. 新建 v0.0.2 Release 并添加下载附件

1. 打开 `Oliver-Lee-1027/XiaoDaJi`，确认 `main` 显示本轮新提交。
2. 进入 **Releases → Draft a new release / Create a new release**，不要编辑 v0.0.1 的标签或附件。
3. **Choose a tag** 输入 `v0.0.2` 并创建新标签，**Target** 选包含本轮新提交的 `main`。
4. **Release title** 填 `小妲己 v0.0.2`。
5. **Describe this release** 可复制 `docs/releases/v0.0.2.md` 的内容。
6. 从 `dist\v0.0.2` 上传三个文件并等待上传完成：
   - `小妲己-安装包-v0.0.2-x64.exe`
   - `小妲己-v0.0.2-windows-x64.zip`
   - `SHA256SUMS.txt`
7. 不勾选 **Set as a pre-release**；如出现 **Set as the latest release**，勾选它。
8. 点击 **Publish release**，确认新页标签为 `v0.0.2` 且 Assets 中三个文件齐全。

发布页面可直接创建新标签，不必再执行 `git tag` 或另推标签。只推源码不会自动出现安装包下载；GitHub 自动生成的 Source code ZIP/TAR.GZ 也不是 Windows 安装包。

## 3. 用户下载和更新

1. 在仓库首页点击 **Releases**，打开 **小妲己 v0.0.2**（标签 `v0.0.2`）。
2. 展开 **Assets**，下载安装版 `小妲己-安装包-v0.0.2-x64.exe`。便携用户选名称含 `windows-x64.zip` 的附件，不要选 Source code。
3. 已安装 v0.0.1 的用户：先从托盘退出小妲己，保留原安装位置运行新版安装器直接覆盖，无需先卸载。
4. 安装后主程序为 `小妲己.exe`，卸载器为 `卸载小妲己.exe`；原设置、凭据和记录保留。文件属性的版本应为 `0.0.2`。
5. 便携用户完整解压到新的版本目录后运行 `小妲己.exe`，不要与旧版同时运行；默认仍读取同一用户数据目录。

在线更新仍未启用，v0.0.1 不会因为 GitHub 新建 Release 自动升级，需手动下载安装包。

## 4. 重新打包（开发者）

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\package.ps1 -OutputDirectory .\dist\v0.0.2
```

需要 Rust/MSVC、Windows SDK 和 NSIS。源码构建目标仍为 `daozai.exe`，只在发布包/安装目录中改为中文名。输出目录必须没有同名便携版暂存目录，避免混入旧文件；发布脚本检查版本、运行库依赖及内嵌狐狸图标。
