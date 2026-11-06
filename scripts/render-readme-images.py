"""Compose README figures from real egui preview exports; never invent UI pixels.

Requires Python 3.11+ and Pillow. Run via export-readme-images.ps1.
"""
from __future__ import annotations

import argparse
from datetime import date
from pathlib import Path
import tomllib

from PIL import Image, ImageDraw, ImageFont, ImageOps

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/images"
RAW = ROOT / "target/readme-previews"
API = ROOT / "target/api-previews"
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
BG, CARD, EDGE = "#101620", "#192230", "#2c384a"
TEXT, MUTED, BLUE = "#f2f6fc", "#a8b8cd", "#79bfff"
FONTS = Path("C:/Windows/Fonts")


def font(size: int, bold: bool = False):
    return ImageFont.truetype(str(FONTS / ("msyhbd.ttc" if bold else "msyh.ttc")), size)


def panel(folder: Path, name: str, crop=None):
    with Image.open(folder / name) as source:
        image = source.convert("RGB")
    return image.crop(crop) if crop else image


def save(image: Image.Image, name: str):
    image.save(OUT / name, optimize=True)


def board(title: str, subtitle: str, height: int, captured: str):
    image = Image.new("RGB", (1280, height), BG)
    draw = ImageDraw.Draw(image)
    draw.text((48, 30), title, font=font(38, True), fill=TEXT)
    draw.text((48, 91), subtitle, font=font(20), fill=MUTED)
    draw.line((48, height - 54, 1232, height - 54), fill=EDGE, width=1)
    draw.text((48, height - 39), f"小妲己 v{VERSION}  /  当前界面渲染 · 演示数据  /  {captured}", font=font(16), fill=MUTED)
    return image


def card(image, box, title, subtitle=""):
    draw = ImageDraw.Draw(image)
    x, y, width, height = box
    draw.rounded_rectangle((x, y, x + width, y + height), radius=18, fill=CARD, outline=EDGE)
    draw.text((x + 24, y + 20), title, font=font(25, True), fill=TEXT)
    if subtitle:
        draw.text((x + 24, y + 60), subtitle, font=font(17), fill=MUTED)


def put(image, source, box):
    x, y, width, height = box
    fitted = ImageOps.contain(source, (width, height), Image.Resampling.LANCZOS)
    image.paste(fitted, (x + (width - fitted.width) // 2, y + (height - fitted.height) // 2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--date", default=date.today().isoformat(), help="Export date, not a fixture event date")
    args = parser.parse_args()
    captured = date.fromisoformat(args.date).isoformat()
    OUT.mkdir(parents=True, exist_ok=True)
    # Crops only remove the unused preview canvas; all controls remain as rendered.
    island = (170, 0, 1110, 320)
    sources = {
        "compact.png": (RAW / "panels", "collapsed-200.png", (300, 0, 980, 88)),
        "music.png": (RAW / "panels", "lyrics-200.png", island),
        "focus.png": (RAW / "panels", "focus-running-200.png", island),
        "water.png": (RAW / "panels", "water-chart-200.png", island),
        "todo.png": (RAW / "panels", "page-4-200.png", island),
        "reminder.png": (RAW / "panels", "reminder-200.png", island),
        "ai-quota.png": (RAW / "panels", "page-6-200.png", island),
        "calendar.png": (RAW / "calendar", "calendar-mixed.png", None),
        "calendar-editor.png": (RAW / "calendar", "calendar-local-editor.png", None),
        "mailbox.png": (RAW / "mail", "多账号邮箱.png", None),
        "work.png": (RAW / "work", "work-panel.png", (80, 0, 560, 210)),
        "shelf.png": (RAW / "work", "shelf-panel.png", (80, 0, 560, 210)),
        "shelf-settings.png": (RAW / "work", "shelf-settings.png", None),
        "api-balance.png": (API, "api-detail.png", island),
        "model-badge.png": (API, "sv-balance.png", (390, 0, 890, 88)),
        "settings.png": (API, "settings-0-600.png", (0, 0, 600, 650)),
        "api-settings.png": (API, "settings-11-600.png", (0, 0, 600, 850)),
    }
    images = {name: panel(*source) for name, source in sources.items()}
    for name, image in images.items():
        save(image, name)

    image = board("让重要的信息，轻轻出现在屏幕顶部", "收起看状态，靠近展开；需要提醒时，再出现。", 770, captured)
    card(image, (48, 146, 1184, 164), "收起 · 常驻状态", "时间、电量和已选 AI 额度，集中在一条小刘海里")
    put(image, images["compact.png"], (90, 232, 1100, 64))
    card(image, (48, 334, 1184, 360), "展开 · 音乐与歌词", "切换功能页面，在同一处查看和处理桌面小事")
    put(image, images["music.png"], (90, 429, 1100, 246))
    save(image, "showcase-hero.png")

    image = board("工作有节奏，日常也不落下", "专注与休息 · 饮水记录 · 待办提醒 · 上下班", 908, captured)
    cards = [
        ("专注计时", "专注 / 休息切换，按需开始与暂停", "focus.png"),
        ("饮水记录", "目标、提醒时段、快捷记录与历史图表", "water.png"),
        ("待办清单", "随手记录，完成勾选，可单独设置提醒", "todo.png"),
        ("上下班", "记录今日时长，查看倒计时，手动确认下班", "work.png"),
    ]
    for index, (title, subtitle, name) in enumerate(cards):
        x, y = 48 + index % 2 * 604, 146 + index // 2 * 346
        card(image, (x, y, 580, 322), title, subtitle)
        put(image, images[name], (x + 20, y + 99, 540, 203))
    save(image, "showcase-daily.png")

    image = board("临时文件，放在顺手的地方", "置物架只保存路径引用，不移动、复制或删除原文件。", 726, captured)
    card(image, (48, 146, 580, 504), "分区置物架", "按落点投放，在分区之间整理和归类")
    put(image, images["shelf.png"], (70, 250, 536, 234))
    draw = ImageDraw.Draw(image)
    for row, text in enumerate(["打开 / 定位 / 拖出到其他应用", "清空分区只移除引用，原文件保留", "原文件失效时保留条目并提示"]):
        draw.text((76, 513 + 36 * row), text, font=font(20), fill=MUTED)
    card(image, (652, 146, 580, 504), "管理分区", "最多 6 个分区，支持命名、排序与删除")
    put(image, images["shelf-settings.png"], (676, 244, 532, 382))
    save(image, "showcase-work.png")

    image = board("邮件与日程，不用来回切窗口", "验证码一键复制；本机安排、工作日历与邮件邀请集中查看。", 906, captured)
    card(image, (48, 146, 1184, 252), "邮箱 · 多账号与验证码", "点击验证码复制，点击邮件查看正文与链接")
    put(image, images["mailbox.png"], (90, 240, 1100, 140))
    card(image, (48, 422, 580, 408), "合并日程", "月历、全天安排、来源标识与提前提醒")
    put(image, images["calendar.png"], (68, 529, 540, 279))
    card(image, (652, 422, 580, 408), "本机日程", "无需连接账号，可新建、编辑和删除")
    put(image, images["calendar-editor.png"], (672, 529, 540, 279))
    save(image, "showcase-calendar.png")

    image = board("AI 订阅额度与 API 余额，分开看清楚", "订阅圆环不是 API 余额；手动图标也不代表自动检测模型。", 770, captured)
    card(image, (48, 146, 580, 324), "订阅额度", "多账号、额度圆环、重置时间与自选配色")
    put(image, images["ai-quota.png"], (68, 246, 540, 204))
    card(image, (652, 146, 580, 324), "NewAPI 余额", "剩余与已用，独立账户、刷新间隔和换算系数")
    put(image, images["api-balance.png"], (672, 246, 540, 204))
    card(image, (48, 494, 1184, 200), "手动模型标识 + 常驻余额", "可选择预设或自定义图标；不切换模型、不改变请求路由")
    put(image, images["model-badge.png"], (90, 590, 1100, 80))
    save(image, "showcase-ai.png")

    image = board("保留你需要的，藏起暂时不用的", "设置主题 · 手动模型标识 · 按需启用的服务连接", 1036, captured)
    card(image, (48, 146, 580, 814), "常规设置", "跟随系统 / 浅色 / 深色，以及显示行为设置")
    put(image, images["settings.png"], (72, 250, 532, 680))
    card(image, (652, 146, 580, 814), "API 余额设置", "测试查询与保存分开，访问令牌不出现在配图中")
    put(image, images["api-settings.png"], (676, 250, 532, 680))
    save(image, "showcase-settings.png")
    print(f"Generated {len(images)} screenshots and 6 README figures for v{VERSION} ({captured}).")


if __name__ == "__main__":
    main()
