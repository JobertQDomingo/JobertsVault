"""T24: the contrast of every text run t24i.mjs found, at its worst spot in the running app.

For each run, the background is the screenshot taken with all text transparent; the worst spot
is the 2x2-pixel block inside the run's boxes with the lowest WCAG ratio against the run's color
(alpha and opacity composited over that block). Large text (24 px, or 18.66 px at weight 700)
needs 3:1, other text 4.5:1. Disabled controls are exempt (WCAG 1.4.3) and listed apart.

    python .claude/skills/task/tools/passes/t24text.py <folder> [--all]
"""

import json
import os
import re
import sys
from collections import defaultdict

from PIL import Image

DIR = sys.argv[1]
meta = json.load(open(os.path.join(DIR, "runs.json"), encoding="utf-8"))
dpr = meta["dpr"]


def parse(css):
    nums = [float(x) for x in re.findall(r"[\d.]+", css)]
    if css.startswith("oklab") or css.startswith("color("):
        return None
    a = nums[3] if len(nums) > 3 else 1.0
    return nums[0], nums[1], nums[2], a


def lum(c):
    def ch(v):
        v /= 255
        return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (ch(v) for v in c)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


fails = []
worst_by_color = defaultdict(lambda: (99, None))
odd = set()
for name, p in meta["passes"].items():
    im = Image.open(os.path.join(DIR, name + ".png")).convert("RGB")
    small = im.resize((im.size[0] // 2, im.size[1] // 2), Image.BOX)
    for run in p["runs"]:
        c = parse(run["color"])
        if c is None:
            odd.add(run["color"])
            continue
        alpha = c[3] * run["opacity"]
        worst = 99
        for x, y, w, h in run["rects"]:
            x0, y0 = int(x * dpr / 2), int(y * dpr / 2)
            x1, y1 = int((x + w) * dpr / 2), int((y + h) * dpr / 2)
            for yy in range(max(0, y0), min(small.size[1], y1)):
                for xx in range(max(0, x0), min(small.size[0], x1)):
                    bg = small.getpixel((xx, yy))
                    fg = tuple(alpha * c[i] + (1 - alpha) * bg[i] for i in range(3))
                    worst = min(worst, ratio(fg, bg))
        large = run["size"] >= 24 or (run["size"] >= 18.66 and run["weight"] >= 700)
        need = 3.0 if large else 4.5
        if not run["disabled"]:
            k = (p["key"], run["color"] + (f" a{alpha:.2f}" if alpha < 1 else ""))
            if worst < worst_by_color[k][0]:
                worst_by_color[k] = (worst, f'{p["screen"]} "{run["text"]}"')
        if worst < need:
            fails.append((p["key"], p["screen"], run["text"], run["color"], alpha, run["size"], run["weight"], round(worst, 2), need, run["disabled"], run["rects"][0]))

print("BELOW TARGET (key, screen, text, color, alpha, size, weight, measured, needed, disabled, first box)")
for f in sorted(fails, key=lambda f: (f[9], f[0], f[1])):
    print(" ", f)
print("\nlowest per color (key: color -> ratio, where)")
for (key, color), (r, where) in sorted(worst_by_color.items()):
    print(f"  {key}: {color} -> {r:.2f} {where}")
if odd:
    print("unparsed colors:", odd)
