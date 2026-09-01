#!/usr/bin/env python3
"""Rasterize custom/icon.png and custom/logo.png into the app icon slots."""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
CUSTOM = ROOT / "custom"
ICON_SRC = CUSTOM / "icon.png"
LOGO_SRC = CUSTOM / "logo.png"


def load_rgba(path: Path) -> Image.Image:
    return Image.open(path).convert("RGBA")


def near_black(r: int, g: int, b: int, threshold: int = 28) -> bool:
    return r < threshold and g < threshold and b < threshold


def make_transparent_bg(im: Image.Image, threshold: int = 28) -> Image.Image:
    im = im.copy()
    pixels = im.load()
    w, h = im.size
    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            if near_black(r, g, b, threshold):
                pixels[x, y] = (r, g, b, 0)
    return im


def crop_content(im: Image.Image, pad: int = 24) -> Image.Image:
    pixels = im.load()
    w, h = im.size
    minx, miny, maxx, maxy = w, h, 0, 0
    found = False
    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            if a > 16 and r + g + b > 40:
                found = True
                minx = min(minx, x)
                miny = min(miny, y)
                maxx = max(maxx, x)
                maxy = max(maxy, y)
    if not found:
        return im
    minx = max(0, minx - pad)
    miny = max(0, miny - pad)
    maxx = min(w - 1, maxx + pad)
    maxy = min(h - 1, maxy + pad)
    return im.crop((minx, miny, maxx + 1, maxy + 1))


def recolor_white_to_dark(im: Image.Image) -> Image.Image:
    im = im.copy()
    pixels = im.load()
    w, h = im.size
    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            if a > 16 and r > 200 and g > 200 and b > 200:
                pixels[x, y] = (11, 18, 32, a)
    return im


def resize_square(im: Image.Image, size: int) -> Image.Image:
    return im.resize((size, size), Image.Resampling.LANCZOS)


def save_png(im: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    im.save(path, "PNG")


def write_ico(im: Image.Image, path: Path, sizes: list[int]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    # Pillow builds the ICO from this image; 256px is the largest official ICO frame.
    resize_square(im, max(sizes)).save(
        path,
        format="ICO",
        sizes=[(s, s) for s in sizes],
    )


def write_icns(im: Image.Image, path: Path) -> None:
    try:
        import icnsutil
    except ImportError:
        print("icnsutil not installed; skipping AppIcon.icns", file=sys.stderr)
        return
    tmp = path.with_suffix(".png")
    square = resize_square(im, 1024)
    square.save(tmp)
    img = icnsutil.IcnsFile()
    img.add_media(file=str(tmp))
    img.write(str(path))
    tmp.unlink(missing_ok=True)


def main() -> int:
    if not ICON_SRC.is_file():
        print(f"missing {ICON_SRC}", file=sys.stderr)
        return 1
    icon = load_rgba(ICON_SRC)

    save_png(icon, ROOT / "res" / "icon.png")
    save_png(resize_square(icon, 32), ROOT / "res" / "32x32.png")
    save_png(resize_square(icon, 64), ROOT / "res" / "64x64.png")
    save_png(resize_square(icon, 128), ROOT / "res" / "128x128.png")
    save_png(resize_square(icon, 256), ROOT / "res" / "128x128@2x.png")
    save_png(resize_square(icon, 256), ROOT / "res" / "mac-icon.png")
    save_png(icon, ROOT / "flutter" / "assets" / "icon.png")

    ico_sizes = [16, 24, 32, 48, 64, 128, 256]
    write_ico(icon, ROOT / "res" / "icon.ico", ico_sizes)
    write_ico(icon, ROOT / "res" / "tray-icon.ico", ico_sizes)
    write_ico(
        icon,
        ROOT / "flutter" / "windows" / "runner" / "resources" / "app_icon.ico",
        ico_sizes,
    )
    write_ico(icon, ROOT / "flutter" / "assets" / "icon.ico", ico_sizes)

    android_sizes = {
        "mdpi": 48,
        "hdpi": 72,
        "xhdpi": 96,
        "xxhdpi": 144,
        "xxxhdpi": 192,
    }
    android_res = ROOT / "flutter" / "android" / "app" / "src" / "main" / "res"
    for density, size in android_sizes.items():
        scaled = resize_square(icon, size)
        folder = android_res / f"mipmap-{density}"
        for name in (
            "ic_launcher.png",
            "ic_launcher_round.png",
            "ic_launcher_foreground.png",
            "ic_stat_logo.png",
        ):
            save_png(scaled, folder / name)

    ios_dir = ROOT / "flutter" / "ios" / "Runner" / "Assets.xcassets" / "AppIcon.appiconset"
    ios_sizes = {
        "Icon-App-20x20@1x.png": 20,
        "Icon-App-20x20@2x.png": 40,
        "Icon-App-20x20@3x.png": 60,
        "Icon-App-29x29@1x.png": 29,
        "Icon-App-29x29@2x.png": 58,
        "Icon-App-29x29@3x.png": 87,
        "Icon-App-40x40@1x.png": 40,
        "Icon-App-40x40@2x.png": 80,
        "Icon-App-40x40@3x.png": 120,
        "Icon-App-60x60@2x.png": 120,
        "Icon-App-60x60@3x.png": 180,
        "Icon-App-76x76@1x.png": 76,
        "Icon-App-76x76@2x.png": 152,
        "Icon-App-83.5x83.5@2x.png": 167,
        "Icon-App-1024x1024@1x.png": 1024,
    }
    for name, size in ios_sizes.items():
        save_png(resize_square(icon, size), ios_dir / name)

    write_icns(icon, ROOT / "flutter" / "macos" / "Runner" / "AppIcon.icns")

    if LOGO_SRC.is_file():
        logo = crop_content(make_transparent_bg(load_rgba(LOGO_SRC)))
        save_png(logo, ROOT / "flutter" / "assets" / "logo.png")
        save_png(logo, ROOT / "flutter" / "assets" / "logo_dark.png")
        save_png(recolor_white_to_dark(logo), ROOT / "flutter" / "assets" / "logo_light.png")

    print("branding icons written")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
