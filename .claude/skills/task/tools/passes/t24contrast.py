"""T24: worst-case contrast of each token on each surface, from t24h.mjs's screenshots.

For every surface x theme x transparency, the background is the screenshot (the aurora under the
app's own glass). The worst spot is the 4x4-pixel block whose average gives the lowest WCAG
ratio with the token's color, over the whole window.

    python -I .claude/skills/task/tools/passes/t24contrast.py <folder>
"""

import json
import os
import re
import sys

from PIL import Image

DIR = sys.argv[1]
meta = json.load(open(os.path.join(DIR, "tokens.json"), encoding="utf-8"))


def rgb(css):
    return tuple(int(float(x)) for x in re.findall(r"[\d.]+", css)[:3])


def lum(c):
    def ch(v):
        v /= 255
        return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (ch(v) for v in c)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def blocks(path, step=4):
    im = Image.open(path).convert("RGB")
    # The oversized probe can show a scrollbar on the right and bottom edges: never under a panel.
    im = im.crop((0, 0, im.size[0] - 17, im.size[1] - 17))
    w, h = im.size
    small = im.resize((w // step, h // step), Image.BOX)  # 4x4 block averages
    return small, w / small.size[0]


# Which tokens to report on which surface (text 4.5, icons and UI parts 3.0).
TEXT = ["foreground", "muted-foreground", "primary", "income", "expense", "warning", "danger", "success"]
PAIRS = {
    "panel": TEXT,
    "raised": TEXT,
    "panel_wash": TEXT,
    "panel_field": ["foreground", "muted-foreground"],
    "raised_field": ["foreground", "muted-foreground"],
    "panel_wash_wash": ["foreground", "muted-foreground"],
    "danger_banner": ["foreground", "muted-foreground", "danger", "primary"],
    "danger_banner_field": ["foreground"],
    "warning_banner": ["foreground", "muted-foreground", "warning", "primary"],
    "warning_banner_field": ["foreground"],
    "info_banner": ["foreground", "muted-foreground", "primary"],
    "panel_danger_tint": ["foreground", "danger"],
    "panel_warning_tint": ["foreground", "warning"],
    "net_card": ["foreground", "muted-foreground", "income", "expense", "primary"],
    "primary": ["primary-foreground"],
    "danger": ["danger-foreground"],
}

results = {}
for key, toks in meta["tokens"].items():
    for surface, want in PAIRS.items():
        path = os.path.join(DIR, f"{key}__{surface}.png")
        small, scale = blocks(path)
        px = list(small.getdata())
        uniq = set(px)
        for t in want:
            c = rgb(toks[t])
            worst = min(uniq, key=lambda p: ratio(c, p))
            i = px.index(worst)
            x, y = (i % small.size[0]) * scale / meta["dpr"], (i // small.size[0]) * scale / meta["dpr"]
            results[(surface, t, key)] = (ratio(c, worst), worst, (round(x), round(y)))

keys = list(meta["tokens"].keys())
print("surface | token | " + " | ".join(keys))
for surface, want in PAIRS.items():
    for t in want:
        cells = []
        for k in keys:
            r, bg, at = results[(surface, t, k)]
            cells.append(f"{r:.2f} @{at[0]},{at[1]}")
        print(f"{surface} | {t} | " + " | ".join(cells))

# UI parts: a filled --primary mark (step dot, path node, ring arc, trend line) and the
# --muted-foreground outline and lock icon, against the panel and a wash card.
print("\nnon-text (3:1)")
for surface in ["panel", "panel_wash", "raised"]:
    for t in ["primary", "muted-foreground", "success", "danger", "warning"]:
        print(surface, t, " | ".join(f"{results.get((surface, t, k), (0,))[0]:.2f}" for k in keys) if (surface, t, keys[0]) in results else "")
# The empty track: --wash fill against its panel (a 'not reached' step, node or level).
print("\nwash track vs panel (lowest, any spot)")
for k in keys:
    a, sa = blocks(os.path.join(DIR, f"{k}__panel.png"))
    b, _ = blocks(os.path.join(DIR, f"{k}__panel_wash.png"))
    pa, pb = list(a.getdata()), list(b.getdata())
    print(k, f"{min(ratio(p, q) for p, q in zip(pa[::7], pb[::7])):.2f}")
print("\ntokens", json.dumps({k: {t: v[t] for t in ("foreground", "muted-foreground", "primary", "danger", "danger-banner")} for k, v in meta["tokens"].items()}))
