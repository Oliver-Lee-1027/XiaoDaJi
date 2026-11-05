# 小妲己 v0.0.1 发布步骤

本轮已统一源码与安装包版本，但不会代为执行 `git commit` 或 `git push`。

## 1. 手动提交并推送源码

在 PowerShell 中执行，每步成功后再执行下一步：

```powershell
cd 'D:\桌面灵动岛\岛仔源\XiaoDaJi'
git remote -v
git status --short
git add .
git commit -m "发布小妲己 v0.0.1"
git -c http.sslBackend=schannel -c http.version=HTTP/1.1 push -u origin main
```

远程应为 `https://github.com/Oliver-Lee-1027/XiaoDaJi.git`。如果 GitHub 要求认证，请使用有本仓库写入权限的账号。不要使用强制推送，不要关闭证书校验。

本轮 `dist/` 仍被 `.gitignore` 排除，安装包不需要作为源码提交。

## 2. 在 GitHub 创建 Release

1. 打开 `Oliver-Lee-1027/XiaoDaJi` 仓库，确认首页显示刚推送的版本提交。
2. 进入 **Releases → Create a new release / Draft a new release**。
3. **Choose a tag** 填 `v0.0.1`，选择创建新标签；**Target** 选择包含本轮版本变更的 `main`。
4. **Release title** 填 `小妲己 v0.0.1`。
5. **Describe this release** 填 `小妲己第一版`。
6. 从 `dist\v0.0.1` 上传以下三个文件，等待全部上传完成：
   - `小妲己-安装包-v0.0.1-x64.exe`
   - `小妲己-v0.0.1-windows-x64.zip`
   - `SHA256SUMS.txt`
7. 不勾选 **Set as a pre-release**（除非你想标记为测试版）；如果出现 **Set as the latest release**，可勾选。
8. 点击 **Publish release**。

可以由发布页创建标签，不必提前执行 `git tag` 或另推标签。发布后检查标签是 `v0.0.1`，且 Assets 中含上述三个文件。GitHub 自动提供的源码压缩包不能代替 Windows 安装包。

## 注意

- 使用本次新生成的 `v0.0.1` 文件，不要给旧 `v0.6.49` 文件改名后上传。
- 在线更新仍默认关闭；本次发布不会自动让旧安装切换更新源。
- 版本调整保留原有数据和安装标识，不删除用户设置或上下班记录。
