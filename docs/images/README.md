# 当前界面图片

- 对应版本：**v0.0.4**。
- 本次导出日期：**2026-11-06**。导出时工作树的应用源码对应提交 `b2e8a7a`。
- 来源：当前应用 `App` 的真实 egui 界面，经仓库已有的软件光栅器渲染；非 AI 概念图，也不是连接真实账号后的桌面实机截图。
- 数据：隔离目录中的合成账号、歌曲、验证码、文件、日程和余额响应。截图内日期为各预览用例固定的演示日期，并非导出当天；不会读取日常用户配置或发送真实通知。
- 处理：单张界面仅裁掉多余画布；展示图额外加入标题、说明、背景与排版，未手工重绘或替换界面控件。Windows 字体、缩放和原生窗口环境可能带来显示差异。
- 品牌素材 `assets/xiaodaji.png`、`SV-icon.png` 与品牌图标预览不是功能截图，本轮保留原图，不改变应用或第三方标识。

## README 展示图

| 图片 | 内容 |
| --- | --- |
| [showcase-hero.png](showcase-hero.png) | 收起状态与展开后的音乐／歌词 |
| [showcase-daily.png](showcase-daily.png) | 专注、饮水、待办、上下班 |
| [showcase-work.png](showcase-work.png) | 分区置物架与分区管理 |
| [showcase-calendar.png](showcase-calendar.png) | 多账号邮箱、合并日程、本机日程编辑 |
| [showcase-ai.png](showcase-ai.png) | 订阅额度、NewAPI 余额、手动标识与常驻余额 |
| [showcase-settings.png](showcase-settings.png) | 常规设置与 API 余额设置 |

## 单张界面

[收起](compact.png) · [音乐与歌词](music.png) · [专注](focus.png) · [饮水](water.png) · [待办](todo.png) · [待办提醒](reminder.png) · [上下班](work.png) · [置物架](shelf.png) · [分区设置](shelf-settings.png) · [邮箱](mailbox.png) · [日程](calendar.png) · [本机日程编辑](calendar-editor.png) · [订阅额度](ai-quota.png) · [API 余额](api-balance.png) · [模型标识](model-badge.png) · [常规设置](settings.png) · [API 设置](api-settings.png)

## 重新生成

在 Windows 上准备 Rust/MSVC 构建环境、Python 3.11+、Pillow 和微软雅黑字体。Pillow 仅供文档配图工具使用，不是应用运行依赖。未安装时可先执行 `python -m pip install Pillow`。

在仓库根目录执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\export-readme-images.ps1
```

脚本只运行以下 6 个按需界面预览测试，使用隔离数据，不启动主程序或真实服务连接：

| 预览测试（`src/ui_tests.rs`） | 输出内容 |
| --- | --- |
| `ui_alignment_layout_gallery` | 常驻栏、音乐、专注、饮水、待办及订阅额度等 |
| `calendar_layout_gallery` | 工作／邮件／本机混合日程及本机编辑器 |
| `work_shelf_ui_preview` | 上下班、分区置物架及设置 |
| `mailbox_readability_gallery` | 演示邮件列表与详情 |
| `api_balance_sv_visual_preview` | 合成 API 余额与 SV 标识 |
| `api_model_settings_visual_preview` | 常规设置与 API 设置 |

中间素材写入 `target/readme-previews` 与 `target/api-previews`，不应提交。随后 `scripts/render-readme-images.py` 生成本目录的 **17 张单图和 6 张展示图**，页脚自动读取 `Cargo.toml` 中的版本与本机导出日期。

如使用已构建的 lib 测试程序，可传 `-TestBinary <完整路径>` 跳过 cargo 编译；必须确认该二进制由当前源码构建，不可复用旧版界面。仅调整排版时可运行 `python .\scripts\render-readme-images.py`，但这不会刷新底层素材，源码变更后必须先完整导出。

导出后逐张检查裁切、文字与隐私内容，并同步本说明顶部的版本、导出日期和源码提交信息。预览通过仅证明界面可以渲染，不代表外部账号连接、Windows 原生交互或所有功能均已验收。
