#!/usr/bin/env python3
"""Turn a frame manifest into an optimised GIF (Pillow only).

usage: assemble.py MANIFEST.json OUT.gif [--width PX] [--colors N]
The manifest is [{"file": "0001.png", "ms": 120}, ...] (paths relative to it).
Identical neighbouring frames were already merged by the capture script.
"""
import json
import sys
from pathlib import Path

from PIL import Image

args = sys.argv[1:]
manifest, out = Path(args[0]), Path(args[1])
width = int(args[args.index("--width") + 1]) if "--width" in args else 0
colors = int(args[args.index("--colors") + 1]) if "--colors" in args else 96

frames = json.loads(manifest.read_text())
imgs, durations = [], []
for f in frames:
    im = Image.open(manifest.parent / f["file"]).convert("RGB")
    if width and im.width != width:
        im = im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)
    imgs.append(im)
    durations.append(max(20, int(round(f["ms"] / 10.0)) * 10))

# One shared palette from a sample of frames keeps colours steady (no flicker)
# and lets Pillow store only the changed rectangle of each frame.
step = max(1, len(imgs) // 12)
sample = imgs[::step]
sheet = Image.new("RGB", (imgs[0].width, imgs[0].height * len(sample)))
for i, im in enumerate(sample):
    sheet.paste(im, (0, i * imgs[0].height))
pal = sheet.quantize(colors=colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
quant = [im.quantize(palette=pal, dither=Image.Dither.NONE) for im in imgs]

quant[0].save(
    out,
    save_all=True,
    append_images=quant[1:],
    duration=durations,
    loop=0,
    optimize=False,
    disposal=1,
)
print(f"{out.name}: {len(imgs)} frames, {out.stat().st_size / 1024:.0f} KB")
