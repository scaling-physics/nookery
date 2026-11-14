# Renders the SYNTHETIC rewe.txt into rewe-clean.png and rewe-photo.jpg (Pillow).
# rewe-text.pdf: cupsfilter -i text/plain -m application/pdf rewe.txt (comment line removed)
# rewe-scan.pdf:  sips -s format pdf rewe-photo.jpg --out rewe-scan.pdf (image only, no text layer)
# Usage: python3 make.py ../../../receipt/tests/fixtures/rewe.txt .
import sys
from PIL import Image, ImageDraw, ImageFont, ImageFilter
src, out = sys.argv[1], sys.argv[2]
lines = [l for l in open(src, encoding='utf-8').read().splitlines() if not l.startswith('#')]
font = ImageFont.truetype('/System/Library/Fonts/Menlo.ttc', 26)
w = 40 + max(int(font.getlength(l)) for l in lines) + 40
h = 40 + 34 * len(lines) + 40
img = Image.new('L', (w, h), 250)
d = ImageDraw.Draw(img)
for i, l in enumerate(lines):
    d.text((40, 40 + 34 * i), l, fill=30, font=font)
img.save(f'{out}/rewe-clean.png')
# a "phone photo": grey table, slight rotation, blur, uneven light, JPEG
bg = Image.new('L', (w + 200, h + 200), 110)
rot = img.rotate(2.5, expand=True, fillcolor=110, resample=Image.BICUBIC)
bg.paste(rot, (100, 100))
bg = bg.filter(ImageFilter.GaussianBlur(1.1))
grad = Image.linear_gradient('L').resize(bg.size).point(lambda v: 200 + v * 55 // 255)
from PIL import ImageChops
bg = ImageChops.multiply(bg, grad)
bg.convert('RGB').save(f'{out}/rewe-photo.jpg', quality=70)
print(img.size, bg.size)
