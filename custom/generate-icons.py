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


def crop_alpha(im: Image.Image, pad: int = 0) -> Image.Image:
    bbox = im.getbbox()
    if not bbox:
        return im
    left, top, right, bottom = bbox
    left = max(0, left - pad)
    top = max(0, top - pad)
    right = min(im.width, right + pad)
    bottom = min(im.height, bottom + pad)
    return im.crop((left, top, right, bottom))


def fit_square(im: Image.Image, size: int, pad_ratio: float = 0.08) -> Image.Image:
    """Center the artwork on a transparent square, keeping a margin for OS masks."""
    src = crop_alpha(im)
    inner = max(1, int(size * (1.0 - 2.0 * pad_ratio)))
    scale = min(inner / src.width, inner / src.height)
    new_w = max(1, int(round(src.width * scale)))
    new_h = max(1, int(round(src.height * scale)))
    scaled = src.resize((new_w, new_h), Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    canvas.paste(scaled, ((size - new_w) // 2, (size - new_h) // 2), scaled)
    return canvas


def flatten(im: Image.Image, bg: tuple[int, int, int, int] = (0, 0, 0, 255)) -> Image.Image:
    base = Image.new("RGBA", im.size, bg)
    return Image.alpha_composite(base, im)


def save_png(im: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    im.save(path, "PNG")


def write_ico(im: Image.Image, path: Path, sizes: list[int]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fit_square(im, max(sizes)).save(
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
    fit_square(im, 1024).save(tmp)
    img = icnsutil.IcnsFile()
    img.add_media(file=str(tmp))
    img.write(str(path))
    tmp.unlink(missing_ok=True)


def main() -> int:
    if not ICON_SRC.is_file():
        print(f"missing {ICON_SRC}", file=sys.stderr)
        return 1
    icon = load_rgba(ICON_SRC)
    icon_1024 = fit_square(icon, 1024)

    save_png(icon_1024, ROOT / "res" / "icon.png")
    save_png(fit_square(icon, 32), ROOT / "res" / "32x32.png")
    save_png(fit_square(icon, 64), ROOT / "res" / "64x64.png")
    save_png(fit_square(icon, 128), ROOT / "res" / "128x128.png")
    save_png(fit_square(icon, 256), ROOT / "res" / "128x128@2x.png")
    save_png(fit_square(icon, 256), ROOT / "res" / "mac-icon.png")
    save_png(icon_1024, ROOT / "flutter" / "assets" / "icon.png")

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
        scaled = fit_square(icon, size)
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
        # App Store rejects alpha on the 1024px marketing icon.
        framed = flatten(fit_square(icon, size)).convert("RGB")
        save_png(framed, ios_dir / name)

    write_icns(icon, ROOT / "flutter" / "macos" / "Runner" / "AppIcon.icns")

    logo_src = LOGO_SRC if LOGO_SRC.is_file() else ICON_SRC
    logo = crop_alpha(load_rgba(logo_src), pad=8)
    save_png(logo, ROOT / "flutter" / "assets" / "logo.png")
    save_png(logo, ROOT / "flutter" / "assets" / "logo_dark.png")
    save_png(logo, ROOT / "flutter" / "assets" / "logo_light.png")

    print("branding icons written")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
