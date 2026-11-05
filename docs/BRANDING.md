# 小妲己：品牌、兼容与发布

最后更新：2026-11-05。

## 名称与资源

- 源码根目录：`XiaoDaJi`；应用显示名：**小妲己**。
- 原图：`assets/xiaodaji-original.png`，由用户提供，保留原样。
- 窗口/文档图标：`assets/xiaodaji.png`，透明背景，1024 × 1024。
- Windows 图标：`assets/xiaodaji.ico`，包含 16、20、24、32、40、48、64、128、256 像素。
- 保留狐狸图案和额头文字，仅将与画布边缘连通的近黑背景转为透明。小尺寸下不保证文字可读。
- 重新生成：安装 Pillow 后运行 `python scripts/prepare-icon.py assets/xiaodaji-original.png`。Pillow 只是图标制作工具，不是应用运行依赖。

## 旧数据兼容

显示品牌与内部身份分开管理，避免改名导致历史记录、密码、自启或安装识别丢失：

| 保留项 | 原因 |
| --- | --- |
| Rust crate / 可执行文件 `daozai` / `daozai.exe` | 保留现有源码调用、启动及更新契约 |
| `%APPDATA%\Daozai` 和 `DAOZAI_*` 环境变量 | 继续读取原设置、上下班、饮水与置物架记录 |
| `Daozai.*` Windows 凭据服务名 | 继续访问用户已保存的密码/凭据 |
| 注册表产品/自启 ID、单实例互斥体、托盘内部类名 | 覆盖安装而非生成冲突副本 |

覆盖安装创建中文新快捷方式，只移除指向本安装目录的旧品牌快捷方式。不迁移或删除用户数据。

## 打包与更新

运行 `scripts/package.ps1` 生成：

- `小妲己-安装包-v版本号-x64.exe`
- `小妲己-v版本号-windows-x64.zip`
- `SHA256SUMS.txt`

在线更新默认关闭，不会连接原作者仓库。新 GitHub 仓库创建后，在**构建前**设置：

```powershell
$env:XIAODAJI_RELEASE_REPOSITORY = '你的GitHub用户名/XiaoDaJi'
cargo build --locked --release --bin daozai
```

需把对应中文 ZIP 上传为同仓库 Release 资源；沿用 `v主.次.补丁` 标签及 GitHub 返回的 SHA-256 digest 校验。不得手工绕过哈希检查。新设置需要重新编译，不是运行时随意切换更新源。

## 重新上传源码

本轮按用户要求移除根 `.git`，不执行 `git init`、`git commit` 或 `git push`。之后请手动在新目录初始化仓库并关联新地址。保留 `.gitignore`、CI 配置和许可证；`target/`、`dist/` 不属于源码提交内容，网页上传/网盘上传时也应主动排除。

原项目地址仅用于开源致谢，不是当前项目远程或更新源。保留 `LICENSE`、`THIRD_PARTY_NOTICES.txt` 和历史 `CHANGELOG.md`。图标来源为用户提供，未进行商标权属或注册可用性审核。
