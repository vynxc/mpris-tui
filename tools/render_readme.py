#!/usr/bin/env python3
"""Generate deterministic, fictional README media."""

from __future__ import annotations

import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
WIDTH = 1200
HEIGHT = 620
ACCENT = (196, 143, 255)
BRIGHT = (239, 234, 244)
TEXT = (197, 190, 207)
MUTED = (139, 132, 151)
FAINT = (75, 69, 88)

REGULAR_PATH = Path("/usr/share/fonts/TTF/JetBrainsMonoNLNerdFont-Regular.ttf")
BOLD_PATH = Path("/usr/share/fonts/TTF/JetBrainsMonoNLNerdFont-Bold.ttf")


def font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    preferred = BOLD_PATH if bold else REGULAR_PATH
    if preferred.exists():
        return ImageFont.truetype(str(preferred), size)
    return ImageFont.truetype("DejaVuSansMono.ttf", size)


FONTS = {
    "small": font(17),
    "label": font(18, bold=True),
    "body": font(24),
    "title": font(42, bold=True),
    "hero": font(54, bold=True),
}


def background(width: int, height: int, phase: float = 0.0) -> Image.Image:
    image = Image.new("RGB", (width, height))
    pixels = image.load()
    for y in range(height):
        for x in range(width):
            wave = math.sin(x / 170 + phase) * 9 + math.cos(y / 115 - phase) * 7
            radial = math.hypot(x - width * 0.72, y - height * 0.36) / width
            pixels[x, y] = (
                max(9, int(26 + wave - radial * 12)),
                max(8, int(18 + wave * 0.35 - radial * 8)),
                max(16, int(42 + wave + (1 - radial) * 17)),
            )
    draw = ImageDraw.Draw(image, "RGBA")
    draw.ellipse(
        (width * 0.61, -height * 0.32, width * 1.12, height * 0.67),
        fill=(118, 52, 146, 30),
        outline=(224, 147, 255, 35),
        width=3,
    )
    rng = random.Random(42)
    for _ in range(110):
        x = rng.randrange(width)
        y = rng.randrange(height)
        alpha = rng.randrange(20, 80)
        draw.point((x, y), fill=(230, 220, 255, alpha))
    for x in range(-height, width, 96):
        draw.line((x, 0, x + height, height), fill=(255, 255, 255, 8), width=1)
    return image


def text(
    draw: ImageDraw.ImageDraw,
    position: tuple[int, int],
    value: str,
    role: str,
    fill: tuple[int, int, int],
) -> None:
    draw.text(position, value, font=FONTS[role], fill=fill)


def progress(
    draw: ImageDraw.ImageDraw,
    x: int,
    y: int,
    width: int,
    ratio: float,
) -> None:
    draw.line((x, y, x + width, y), fill=FAINT, width=3)
    draw.line((x, y, x + int(width * ratio), y), fill=ACCENT, width=4)
    knob = x + int(width * ratio)
    draw.ellipse((knob - 5, y - 5, knob + 5, y + 5), fill=ACCENT)


def signal_bars(
    draw: ImageDraw.ImageDraw,
    x: int,
    y: int,
    count: int,
    phase: int,
    scale: float = 1.0,
) -> None:
    for index in range(count):
        height = int((18 + ((index * 17 + phase * 11) % 7) * 11) * scale)
        color = (*ACCENT, 190 + (index % 3) * 20)
        left = x + int(index * 12 * scale)
        draw.rounded_rectangle(
            (left, y - height // 2, left + max(4, int(6 * scale)), y + height // 2),
            radius=3,
            fill=color,
        )


def hero_frame(frame_index: int) -> Image.Image:
    image = background(WIDTH, HEIGHT, frame_index / 22)
    draw = ImageDraw.Draw(image, "RGBA")

    draw.line((24, 34, WIDTH - 24, 34), fill=(255, 255, 255, 35), width=1)
    text(draw, (38, 10), "MPRIS TUI  /  TRANSPARENT DESKTOP CANVAS", "small", MUTED)
    text(draw, (WIDTH - 235, 10), "NO BACKGROUND", "small", ACCENT)

    signal_bars(draw, 95, 315, 28, frame_index, 1.25)
    text(draw, (105, 400), "SIGNAL / 04", "label", MUTED)

    x = 560
    text(draw, (x, 155), "PLAYING  /  MPRIS TUI", "label", ACCENT)
    text(draw, (x, 210), "Afterglow Circuit", "hero", BRIGHT)
    text(draw, (x, 282), "Nocturne Assembly", "body", ACCENT)
    text(draw, (x, 325), "Signals in the Static", "small", MUTED)

    ratio = 0.42 + frame_index / 24 * 0.14
    progress(draw, x, 405, 555, ratio)
    text(draw, (x, 425), f"2:{7 + frame_index:02d}", "small", TEXT)
    text(draw, (1064, 425), "4:18", "small", MUTED)

    text(
        draw,
        (38, HEIGHT - 42),
        "The wallpaper remains visible through every unused terminal cell.",
        "small",
        TEXT,
    )
    return image


def render_hero() -> None:
    frames = [hero_frame(index) for index in range(18)]
    frames[0].save(
        DOCS / "hero.gif",
        save_all=True,
        append_images=frames[1:],
        duration=110,
        loop=0,
        optimize=True,
    )


def panel(
    image: Image.Image,
    box: tuple[int, int, int, int],
    name: str,
    variant: str,
) -> None:
    x0, y0, x1, y1 = box
    draw = ImageDraw.Draw(image, "RGBA")
    draw.rounded_rectangle(box, radius=24, outline=(255, 255, 255, 34), width=2)
    text(draw, (x0 + 24, y0 + 18), name.upper(), "label", MUTED)

    if variant == "minimal":
        text(draw, (x0 + 35, (y0 + y1) // 2), "▶  Afterglow Circuit — Nocturne Assembly   2:07 / 4:18", "small", BRIGHT)
        return

    text(draw, (x0 + 35, y0 + 88), "PLAYING  /  MPRIS TUI", "small", ACCENT)
    title_role = "title" if variant in {"hero", "wide"} else "body"
    text(draw, (x0 + 35, y0 + 130), "Afterglow Circuit", title_role, BRIGHT)
    text(draw, (x0 + 35, y0 + 190), "Nocturne Assembly", "small", ACCENT)
    if variant == "hero":
        signal_bars(draw, x1 - 225, y0 + 242, 12, 3, 0.68)
    progress(draw, x0 + 35, y1 - 62, x1 - x0 - 70, 0.49)
    text(draw, (x0 + 35, y1 - 48), "2:07", "small", TEXT)


def render_layouts() -> None:
    image = background(1400, 820, 0.8)
    panel(image, (30, 30, 685, 390), "Hero", "hero")
    panel(image, (715, 30, 1370, 390), "Wide", "wide")
    panel(image, (30, 420, 685, 790), "Compact", "compact")
    panel(image, (715, 420, 1370, 790), "Minimal", "minimal")
    image.save(DOCS / "layouts.webp", "WEBP", quality=92, method=6)


def main() -> None:
    DOCS.mkdir(parents=True, exist_ok=True)
    render_hero()
    render_layouts()
    print("Generated docs/hero.gif and docs/layouts.webp")


if __name__ == "__main__":
    main()
