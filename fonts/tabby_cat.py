"""Brown mackerel tabby materials, shared by source pixels and custom poses.

The CC0 pack's Cat-6 supplies the authored markings for ordinary motions.
Custom geometry samples these same materials in body/limb coordinates, before
posing or mirroring, so the stripes move with the fur instead of the screen.
"""

import math
from pathlib import Path

CLEAR = (0, 0, 0, 0)
STRIPE = (80, 68, 58, 255)
SHADOW = (112, 94, 74, 255)
SHADE = (145, 120, 91, 255)
MID = (170, 142, 105, 255)
FUR = (185, 157, 119, 255)
LIGHT = (209, 186, 151, 255)
CREAM = (230, 212, 182, 255)
EYE = (192, 201, 111, 255)
NOSE = (182, 124, 112, 255)
PALETTE = (STRIPE, SHADOW, SHADE, MID, FUR, LIGHT, CREAM, EYE, NOSE)
SOURCE = dict(zip(((79, 79, 79), (100, 100, 100), (132, 132, 132),
                   (160, 160, 160), (168, 169, 169), (192, 192, 192),
                   (214, 214, 214), (232, 216, 42), (101, 57, 27)), PALETTE))
SOURCE[(168, 62, 62)] = NOSE  # Tongue in the grooming frames.


TAILS = {}
for line in (Path(__file__).parent / "pet-cats/tabby-tails.tsv").read_text().splitlines():
    if line and not line.startswith("#"):
        action, frame, points = line.split("\t")
        TAILS[action, int(frame)] = [tuple(map(int, point.split(","))) for point in points.split()]


def recolor(pixels, action=None, frame=0):
    # Remove faint brush residue, including two blue Run-2 pixels at alpha 24.
    # The actual artwork has hard pixel boundaries (occasionally alpha 249).
    result = [[SOURCE[pixel[:3]] if pixel[3] >= 128 else CLEAR for pixel in row] for row in pixels]
    return paint_tail(result, action, frame, limb)


def paint_tail(result, action, frame, material):
    points = TAILS.get((action, frame), [])
    lengths = [math.dist(a, b) for a, b in zip(points, points[1:])]
    total = sum(lengths)
    # Only recolor existing pixels close to the authored tail centerline.
    # The overlay cannot add silhouette pixels or wander into the torso.
    for y, row in enumerate(result):
        for x, pixel in enumerate(row):
            if not pixel[3]:
                continue
            nearest, progress, completed = math.inf, 0, 0
            for a, b, length in zip(points, points[1:], lengths):
                t = min(1, max(0, ((x-a[0])*(b[0]-a[0]) + (y-a[1])*(b[1]-a[1])) / length**2))
                distance = math.dist((x, y), (a[0]+t*(b[0]-a[0]), a[1]+t*(b[1]-a[1])))
                if distance < nearest:
                    nearest, progress = distance, (completed+t*length) / total
                completed += length
            if nearest <= .8:
                row[x] = material("tail", progress)
    return result


def limb(part, progress):
    """Progress runs from the body to the tip, along the articulated limb."""
    if part == "tail":
        return STRIPE if progress >= .84 or .20 <= progress < .31 or .53 <= progress < .64 else MID
    if part == "effect":
        return LIGHT
    if progress >= .91:
        return LIGHT if part == "far-leg" else CREAM
    if .52 <= progress < .68:
        return STRIPE
    return SHADOW if part == "far-leg" else SHADE


def body(x, y, left, right, top, bottom, facing=1.0):
    # Each row is a cross-section of the torso; the bars curve with its width.
    u = (x - left) / max(1, right - left)
    v = (y - top) / max(1, bottom - top)
    # At the front of the turn the chest is visible; lateral stripes recede
    # to the flanks rather than sliding straight across the chest.
    side = abs(facing)
    if side < .72:
        if abs(u-.5) < .12 * (1-side) and v > .25:
            return LIGHT
        if u < .10 or u > .90:
            return STRIPE
        if side < .4:
            return FUR if v < .65 else MID
    if v > .8:
        return SHADE
    if v > .65:
        return MID
    # Three broad, curved bars stay readable in a 16-pixel-high silhouette.
    anatomical_u = u if facing > 0 else 1-u
    stripe = (anatomical_u + .08 * math.sin(v * math.pi)) * 3.0
    if stripe % 1 < .24:
        return STRIPE
    return FUR if v < .42 else MID


def head(pattern, x, y):
    """Five-pixel head: forehead bars, eye, pale muzzle, and a tiny nose."""
    if pattern[y][x] == ".":
        return EYE
    front = pattern[3] == "#.#.#"
    right = pattern[3] == "###.#"
    if not front and not right:
        return head(tuple(row[::-1] for row in pattern), 4-x, y)
    if front:
        if y == 4 and x == 2:
            return NOSE
        if y >= 4 and 1 <= x <= 3:
            return CREAM
        if (y == 2 and x in (1, 3)) or (y == 1 and x == 2) or (y == 3 and x in (0, 4)):
            return STRIPE
    else:
        if y == 4 and x == 4:
            return NOSE
        if y >= 4 and x >= 2:
            return CREAM
        if (y == 2 and x in (1, 3)) or (y == 3 and x == 1):
            return STRIPE
    return SHADE if y == 0 else FUR


def surface(part, point, normal):
    """Sample the rotating cat in its own 3D coordinates, never screen XY."""
    x, y, z = point
    if part == "eye":
        return EYE
    if part == "tail":
        return limb("tail", min(1, max(0, (-z - 10) / 3)))
    if part == "leg":
        if y < 1.1:
            return CREAM
        return STRIPE if 2.1 <= y < 3 else SHADE
    if part == "ear":
        return SHADE if y > 14.5 else FUR
    if part == "head":
        if z > 8.3 and y < 9.8:
            return NOSE if abs(x) < .65 and y > 9.1 else CREAM
        if y > 11 and ((.7 < abs(x) < 1.5) or (abs(x) < .5 and y > 12)):
            return STRIPE
        if z > 7.6 and y < 10.4 and abs(x) > 2:
            return STRIPE
        return FUR if normal[1] > -.25 else LIGHT
    if normal[1] < -.4:
        return LIGHT
    if normal[1] > .65 and abs(x) < .8:
        return STRIPE
    # Rings around the long body axis become the narrow flank bars in profile.
    bar = (z + .35 * (y-6) + 9) / 3.6
    if bar % 1 < .23:
        return STRIPE
    return FUR if normal[1] > .1 else MID
