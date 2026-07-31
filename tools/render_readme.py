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


def cover_art(
    draw: ImageDraw.ImageDraw,
    x: int,
    y: int,
    size: int,
    phase: float = 0.0,
) -> None:
    draw.rounded_rectangle(
        (x, y, x + size, y + size),
        radius=max(8, size // 18),
        fill=(21, 15, 35, 235),
        outline=(255, 255, 255, 35),
        width=2,
    )
    center = (x + size // 2, y + size // 2)
    for index, color in enumerate(
        [(105, 76, 166, 210), (196, 143, 255, 190), (67, 196, 210, 165)]
    ):
        radius = int(size * (0.39 - index * 0.085))
        offset = int(math.sin(phase + index * 1.7) * size * 0.035)
        draw.ellipse(
            (
                center[0] - radius + offset,
                center[1] - radius,
                center[0] + radius + offset,
                center[1] + radius,
            ),
            outline=color,
            width=max(3, size // 45),
        )
    draw.ellipse(
        (
            center[0] - size * 0.07,
            center[1] - size * 0.07,
            center[0] + size * 0.07,
            center[1] + size * 0.07,
        ),
        fill=ACCENT,
    )
    draw.line(
        (
            x + size * 0.12,
            y + size * 0.82,
            x + size * 0.88,
            y + size * 0.18,
        ),
        fill=(255, 255, 255, 36),
        width=max(2, size // 70),
    )


def hero_frame(frame_index: int) -> Image.Image:
    image = background(WIDTH, HEIGHT, frame_index / 22)
    draw = ImageDraw.Draw(image, "RGBA")

    draw.line((24, 34, WIDTH - 24, 34), fill=(255, 255, 255, 35), width=1)
    text(draw, (38, 10), "MPRIS TUI  /  TRANSPARENT DESKTOP CANVAS", "small", MUTED)
    text(draw, (WIDTH - 235, 10), "NO BACKGROUND", "small", ACCENT)

    cover_art(draw, 470, 72, 260, frame_index / 8)

    text(draw, (481, 354), "MPRIS TUI  •  PLAYING", "label", ACCENT)
    text(draw, (332, 392), "Afterglow Circuit", "hero", BRIGHT)
    text(draw, (468, 458), "Nocturne Assembly", "body", ACCENT)
    text(draw, (487, 497), "Signals in the Static", "small", MUTED)

    ratio = 0.42 + frame_index / 24 * 0.14
    text(draw, (260, 541), f"2:{7 + frame_index:02d}", "small", TEXT)
    progress(draw, 330, 552, 540, ratio)
    text(draw, (890, 541), "4:18", "small", MUTED)
    text(draw, (453, 579), "[ ◀◀ ]   [ || ]   [ ▶▶ ]", "small", TEXT)

    text(
        draw,
        (38, 579),
        "LEFT-CLICK ONLY",
        "small",
        MUTED,
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

    if variant == "vertical":
        cover_art(draw, (x0 + x1) // 2 - 62, y0 + 65, 124, 0.4)
        text(draw, (x0 + 190, y0 + 208), "Afterglow Circuit", "body", BRIGHT)
        text(draw, (x0 + 225, y0 + 246), "Nocturne Assembly", "small", ACCENT)
    else:
        cover_art(draw, x0 + 35, y0 + 85, 118, 0.4)
        text(draw, (x0 + 180, y0 + 90), "PLAYING  •  MPRIS TUI", "small", ACCENT)
        title_role = "title" if variant == "wide" else "body"
        text(draw, (x0 + 180, y0 + 130), "Afterglow Circuit", title_role, BRIGHT)
        text(draw, (x0 + 180, y0 + 190), "Nocturne Assembly", "small", ACCENT)
    progress(draw, x0 + 35, y1 - 62, x1 - x0 - 70, 0.49)
    text(draw, (x0 + 35, y1 - 48), "2:07", "small", TEXT)
    text(draw, (x1 - 87, y1 - 48), "4:18", "small", MUTED)
    text(draw, ((x0 + x1) // 2 - 93, y1 - 32), "[ ◀◀ ]  [ || ]  [ ▶▶ ]", "small", TEXT)


def render_layouts() -> None:
    image = background(1400, 820, 0.8)
    panel(image, (30, 30, 685, 390), "Vertical", "vertical")
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
