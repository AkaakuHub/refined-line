#!/usr/bin/env python3
"""Batch-generate numbered badge icons.

Examples:
  python scirpts/number.py --font-ttf C:/Windows/Fonts/segoeui.ttf
  python scirpts/number.py --badge-only --sizes 16,20,24,32,40,48,64 --make-ico
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Iterable

try:
  from PIL import Image, ImageDraw, ImageFont
except ImportError as exc:
  raise SystemExit(
    "Pillow is required. Install with: python -m pip install pillow"
  ) from exc


def parse_csv_ints(raw: str) -> list[int]:
  values: list[int] = []
  for part in raw.split(","):
    token = part.strip()
    if not token:
      continue
    try:
      value = int(token)
    except ValueError as exc:
      raise argparse.ArgumentTypeError(f"Invalid integer: {token}") from exc
    if value <= 0:
      raise argparse.ArgumentTypeError(f"Size must be > 0: {value}")
    values.append(value)
  dedup = sorted(set(values))
  if not dedup:
    raise argparse.ArgumentTypeError("At least one size is required.")
  return dedup


def parse_csv_labels(raw: str) -> list[str]:
  labels = [item.strip() for item in raw.split(",") if item.strip()]
  if not labels:
    raise argparse.ArgumentTypeError("At least one label is required.")
  return labels


def parse_hex_color(value: str) -> tuple[int, int, int, int]:
  raw = value.strip().lstrip("#")
  if len(raw) == 6:
    raw = f"{raw}FF"
  if len(raw) != 8:
    raise argparse.ArgumentTypeError(f"Color must be #RRGGBB or #RRGGBBAA: {value}")
  try:
    return tuple(int(raw[i : i + 2], 16) for i in range(0, 8, 2))  # type: ignore[return-value]
  except ValueError as exc:
    raise argparse.ArgumentTypeError(f"Invalid color value: {value}") from exc


def normalize_name(label: str) -> str:
  out = label.replace("+", "plus")
  out = "".join(ch for ch in out if ch.isalnum() or ch in {"-", "_"})
  return out or "label"


def pick_default_font() -> Path | None:
  candidates = [
    Path("C:/Windows/Fonts/segoeui.ttf"),
    Path("C:/Windows/Fonts/seguiemj.ttf"),
    Path("C:/Windows/Fonts/meiryo.ttc"),
    Path("C:/Windows/Fonts/msgothic.ttc"),
  ]
  for candidate in candidates:
    if candidate.exists():
      return candidate
  return None


def load_font(font_ttf: Path | None, size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
  if font_ttf is not None:
    return ImageFont.truetype(str(font_ttf), size=size)
  fallback = pick_default_font()
  if fallback is not None:
    return ImageFont.truetype(str(fallback), size=size)
  return ImageFont.load_default()


def ensure_dir(path: Path) -> None:
  path.mkdir(parents=True, exist_ok=True)


def draw_badge_icon(
  base_icon: Image.Image,
  size: int,
  label: str,
  *,
  badge_only: bool,
  badge_color: tuple[int, int, int, int],
  text_color: tuple[int, int, int, int],
  stroke_color: tuple[int, int, int, int],
  badge_scale: float,
  badge_margin: float,
  badge_offset_x: float,
  badge_offset_y: float,
  text_offset_x: float,
  text_offset_y: float,
  font_scale: float,
  plus_font_scale: float,
  stroke_ratio: float,
  font_ttf: Path | None,
) -> Image.Image:
  if badge_only:
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
  else:
    canvas = base_icon.resize((size, size), Image.Resampling.LANCZOS)

  draw = ImageDraw.Draw(canvas)

  radius = max(1, int(round(size * badge_scale / 2.0)))
  margin = int(round(size * badge_margin))
  cx = size - margin - radius + int(round(size * badge_offset_x))
  cy = margin + radius + int(round(size * badge_offset_y))

  left = cx - radius
  top = cy - radius
  right = cx + radius
  bottom = cy + radius
  draw.ellipse((left, top, right, bottom), fill=badge_color)

  scale = plus_font_scale if "+" in label else font_scale
  font_px = max(1, int(round(size * scale)))
  font = load_font(font_ttf, font_px)
  stroke_px = max(0, int(round(size * stroke_ratio)))

  bbox = draw.textbbox((0, 0), label, font=font, stroke_width=stroke_px)
  text_w = bbox[2] - bbox[0]
  text_h = bbox[3] - bbox[1]
  tx = cx - (text_w / 2.0) - bbox[0] + (size * text_offset_x)
  ty = cy - (text_h / 2.0) - bbox[1] + (size * text_offset_y)

  draw.text(
    (tx, ty),
    label,
    font=font,
    fill=text_color,
    stroke_width=stroke_px,
    stroke_fill=stroke_color if stroke_px > 0 else None,
  )
  return canvas


def positive_float(value: str) -> float:
  out = float(value)
  if out <= 0:
    raise argparse.ArgumentTypeError(f"Value must be > 0: {value}")
  return out


def non_negative_float(value: str) -> float:
  out = float(value)
  if out < 0:
    raise argparse.ArgumentTypeError(f"Value must be >= 0: {value}")
  return out


def build_parser() -> argparse.ArgumentParser:
  parser = argparse.ArgumentParser(
    description="Generate numbered badge icon images from a base icon.",
    formatter_class=argparse.ArgumentDefaultsHelpFormatter,
  )
  parser.add_argument("--base-icon", type=Path, default=Path("src-tauri/icons/icon.png"))
  parser.add_argument("--output-dir", type=Path, default=Path("src-tauri/icons/notify-badge"))
  parser.add_argument("--font-ttf", type=Path, default=None)
  parser.add_argument("--labels", type=parse_csv_labels, default="1,2,3,4,5,6,7,8,9,9+")
  parser.add_argument("--sizes", type=parse_csv_ints, default="16,20,24,32,40,48,64")
  parser.add_argument("--prefix", type=str, default="badge")
  parser.add_argument("--badge-only", action="store_true")
  parser.add_argument("--make-ico", action="store_true")
  parser.add_argument("--ico-sizes", type=str, default=None)

  parser.add_argument("--badge-color", type=parse_hex_color, default="#E02424")
  parser.add_argument("--text-color", type=parse_hex_color, default="#FFFFFF")
  parser.add_argument("--stroke-color", type=parse_hex_color, default="#C81E1E")

  parser.add_argument("--badge-scale", type=positive_float, default=0.72)
  parser.add_argument("--badge-margin", type=non_negative_float, default=0.03)
  parser.add_argument("--badge-offset-x", type=float, default=0.0)
  parser.add_argument("--badge-offset-y", type=float, default=0.0)
  parser.add_argument("--text-offset-x", type=float, default=0.0)
  parser.add_argument("--text-offset-y", type=float, default=0.0)
  parser.add_argument("--font-scale", type=positive_float, default=0.40)
  parser.add_argument("--plus-font-scale", type=positive_float, default=0.32)
  parser.add_argument("--stroke-ratio", type=non_negative_float, default=0.04)
  return parser


def to_list(value: Iterable[int] | str) -> list[int]:
  if isinstance(value, str):
    if not value.strip():
      return []
    return parse_csv_ints(value)
  return list(value)


def main() -> int:
  parser = build_parser()
  args = parser.parse_args()

  labels = parse_csv_labels(args.labels) if isinstance(args.labels, str) else args.labels
  sizes = to_list(args.sizes)
  ico_sizes = parse_csv_ints(args.ico_sizes) if args.ico_sizes else sizes

  if not args.badge_only and not args.base_icon.exists():
    print(f"Base icon not found: {args.base_icon}", file=sys.stderr)
    return 1

  if args.font_ttf is not None and not args.font_ttf.exists():
    print(f"Font not found: {args.font_ttf}", file=sys.stderr)
    return 1

  base_icon = (
    Image.open(args.base_icon).convert("RGBA")
    if not args.badge_only
    else Image.new("RGBA", (max(sizes), max(sizes)), (0, 0, 0, 0))
  )

  ensure_dir(args.output_dir)
  generated = 0

  print(f"Output dir: {args.output_dir}")
  if args.font_ttf:
    print(f"Font: {args.font_ttf}")
  else:
    fallback = pick_default_font()
    print(f"Font: {fallback if fallback else 'Pillow default'}")

  for label in labels:
    key = normalize_name(label)
    rendered_by_size: dict[int, Image.Image] = {}
    for size in sizes:
      img = draw_badge_icon(
        base_icon,
        size,
        label,
        badge_only=args.badge_only,
        badge_color=args.badge_color,
        text_color=args.text_color,
        stroke_color=args.stroke_color,
        badge_scale=args.badge_scale,
        badge_margin=args.badge_margin,
        badge_offset_x=args.badge_offset_x,
        badge_offset_y=args.badge_offset_y,
        text_offset_x=args.text_offset_x,
        text_offset_y=args.text_offset_y,
        font_scale=args.font_scale,
        plus_font_scale=args.plus_font_scale,
        stroke_ratio=args.stroke_ratio,
        font_ttf=args.font_ttf,
      )
      rendered_by_size[size] = img
      out_dir_size = args.output_dir / str(size)
      ensure_dir(out_dir_size)
      out_path = out_dir_size / f"{args.prefix}_{key}_{size}.png"
      img.save(out_path)
      generated += 1

    if args.make_ico:
      max_size = max(ico_sizes)
      source = rendered_by_size.get(max_size)
      if source is None:
        source = draw_badge_icon(
          base_icon,
          max_size,
          label,
          badge_only=args.badge_only,
          badge_color=args.badge_color,
          text_color=args.text_color,
          stroke_color=args.stroke_color,
          badge_scale=args.badge_scale,
          badge_margin=args.badge_margin,
          badge_offset_x=args.badge_offset_x,
          badge_offset_y=args.badge_offset_y,
          text_offset_x=args.text_offset_x,
          text_offset_y=args.text_offset_y,
          font_scale=args.font_scale,
          plus_font_scale=args.plus_font_scale,
          stroke_ratio=args.stroke_ratio,
          font_ttf=args.font_ttf,
        )
      ico_path = args.output_dir / f"{args.prefix}_{key}.ico"
      source.save(
        ico_path,
        format="ICO",
        sizes=[(s, s) for s in sorted(set(ico_sizes))],
      )
      generated += 1

  print(f"Generated files: {generated}")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
