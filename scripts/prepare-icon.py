#!/usr/bin/env python3
"""Prepare XiaoDaJi PNG/ICO from the supplied logo. Requires Pillow (build tool only)."""
from pathlib import Path
import argparse
import shutil
from PIL import Image, ImageChops, ImageDraw


def prepare(source: Path, root: Path) -> None:
    assets = root / 'assets'
    assets.mkdir(parents=True, exist_ok=True)
    original = assets / 'xiaodaji-original.png'
    if source.resolve() != original.resolve():
        shutil.copyfile(source, original)
    image = Image.open(original).convert('RGBA')
    # Remove only near-black pixels connected to the border. Dark eyes/lettering
    # inside the fox remain opaque. Alpha input is kept unchanged.
    if image.getextrema()[3] == (255, 255):
        r, g, b, _ = image.split()
        brightest = ImageChops.lighter(ImageChops.lighter(r, g), b)
        mask = brightest.point(lambda value: 255 if value <= 48 else 0)
        w, h = image.size
        for seed in [(0, 0), (w-1, 0), (0, h-1), (w-1, h-1)]:
            if mask.getpixel(seed) == 255:
                ImageDraw.floodfill(mask, seed, 128)
        image.putalpha(mask.point(lambda value: 0 if value == 128 else 255))
    bbox = image.getbbox()
    if not bbox:
        raise ValueError('Logo has no visible pixels')
    image = image.crop(bbox)
    image.thumbnail((860, 860), Image.Resampling.LANCZOS)
    canvas = Image.new('RGBA', (1024, 1024), (0, 0, 0, 0))
    canvas.alpha_composite(image, ((1024-image.width)//2, (1024-image.height)//2))
    canvas.save(assets / 'xiaodaji.png', optimize=True)
    # BMP-backed ICO entries work with both rc.exe and the Win32 fallback loader.
    canvas.save(assets / 'xiaodaji.ico', format='ICO', bitmap_format='bmp',
                sizes=[(n, n) for n in (16, 20, 24, 32, 40, 48, 64, 128, 256)])
    preview = Image.new('RGB', (780, 320), '#202124')
    draw = ImageDraw.Draw(preview)
    for row, background in enumerate(('#202124', '#f2f2f2')):
        y = row * 160
        draw.rectangle((0, y, 780, y+159), fill=background)
        x = 16
        for size in (16, 20, 24, 32, 48, 64, 128):
            icon = canvas.resize((size, size), Image.Resampling.LANCZOS)
            preview.paste(icon, (x, y+12), icon)
            draw.text((x, y+142), str(size), fill='#9b9b9b')
            x += size+28
    dest = root / 'target' / 'branding-preview.png'
    dest.parent.mkdir(exist_ok=True)
    preview.save(dest)
    print(f'PNG: {assets / "xiaodaji.png"}; ICO: {assets / "xiaodaji.ico"}')
    print(f'Preview: {dest}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    args = parser.parse_args()
    prepare(args.source, Path(__file__).resolve().parent.parent)
