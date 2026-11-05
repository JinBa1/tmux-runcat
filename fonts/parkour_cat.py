"""Cat-1 wall jump: planted paws, an articulated turn, and a hind-leg push.

The canonical clip approaches the right edge and leaves facing left. Mirroring
the complete clip supplies the left edge; facing never flips the whole sprite
halfway through a planted pose. All artwork fits the existing 30 by 16 bounds.
"""

import math
import cat_coats

INK = (255, 160, 60, 255)
CLEAR = (0, 0, 0, 0)
RIGHT = ("  #  ", " ### ", "#####", "###.#", "#####", "#### ")
QUARTER_RIGHT = (" # # ", "#####", "#####", "###.#", "#####", " ### ")
FRONT = ("#   #", "#####", "#####", "#.#.#", "#####", " ### ")
QUARTER_LEFT = tuple(row[::-1] for row in QUARTER_RIGHT)
LEFT = tuple(row[::-1] for row in RIGHT)


def blank():
    return [[CLEAR for _ in range(50)] for _ in range(50)]


def stroke(canvas, points, width=1, material=None):
    lengths = [math.dist(first, second) for first, second in zip(points, points[1:])]
    total, completed = sum(lengths), 0
    for segment, ((x0, y0), (x1, y1)) in enumerate(zip(points, points[1:])):
        steps = max(abs(x1 - x0), abs(y1 - y0), 1)
        for step in range(steps + 1):
            x = round(x0 + (x1 - x0) * step / steps)
            y = round(y0 + (y1 - y0) * step / steps)
            for dx in range(width):
                if 10 <= x + dx <= 39 and 16 <= y <= 31:
                    progress = (completed + lengths[segment] * step / steps) / max(1, total)
                    canvas[y][x + dx] = material(progress) if material else INK
        completed += lengths[segment]


def head(canvas, pattern, x, y, fur=None):
    coat = cat_coats.get(fur)
    for dy, row in enumerate(pattern):
        for dx, pixel in enumerate(row):
            canvas[y + dy][x + dx] = (coat.head(pattern, dx, dy)
                                      if coat and pixel in "#." else
                                      INK if pixel == "#" else CLEAR)


def torso(canvas, spans, shift=0, fur=None, facing=1.0):
    coat = cat_coats.get(fur)
    top, bottom = min(row[0] for row in spans), max(row[0] for row in spans)
    for y, left, right in spans:
        for x in range(left - shift, right - shift + 1):
            canvas[y][x] = coat.body(x + shift, y, left, right, top, bottom, facing) if coat else INK


def turn(stage, fur=None):
    canvas = blank()
    heads = (
        (33, 20, RIGHT), (33, 20, RIGHT), (32, 19, QUARTER_RIGHT),
        (31, 19, FRONT), (29, 19, FRONT), (27, 19, QUARTER_LEFT),
        (25, 19, LEFT), (23, 19, LEFT), (22, 19, LEFT),
    )
    hips = (23, 24, 25, 27, 29, 31, 33, 34, 34)
    tails = (
        ((23, 24), (19, 22), (13, 21)),
        ((24, 24), (19, 20), (14, 19)),
        ((25, 24), (21, 19), (17, 17)),
        ((27, 24), (24, 18), (21, 16)),
        ((29, 24), (28, 18), (27, 16)),
        ((31, 24), (32, 19), (33, 17)),
        ((33, 24), (36, 20), (38, 19)),
        ((34, 24), (38, 22), (39, 21)),
        ((34, 24), (38, 23), (39, 22)),
    )
    stroke(canvas, tails[stage], material=cat_coats.material(fur, "tail"))

    # The rear leg folds under the belly, then swings into the lower plant.
    hip = hips[stage]
    foot = (
        (21, 29), (22, 29), (24, 29), (27, 29), (32, 29),
        (36, 30), (39, 30), (39, 30), (39, 29),
    )[stage]
    stroke(canvas, [(hip, 26), (hip - 1 if stage < 4 else hip + 1, 28), foot], 3,
           cat_coats.material(fur, "hind-leg"))

    # Keep the forepaws planted until the rear paw can take their weight.
    near = (39, 27) if stage < 7 else ((34, 29) if stage == 7 else (28, 28))
    far = ((38, 30) if stage < 5 else
           (36, 29) if stage == 5 else (31, 29) if stage == 6 else (26, 28))
    shoulder = 34 - round(stage * 1.1)
    stroke(canvas, [(shoulder, 25), (35 if stage < 6 else shoulder, 27), far], 2,
           cat_coats.material(fur, "far-leg"))
    stroke(canvas, [(shoulder, 25), (35 if stage < 7 else shoulder - 1, 26), near], 2,
           cat_coats.material(fur, "leg"))
    torso(canvas, ((22, 25, 31), (23, 23, 34), (24, 22, 35),
                   (25, 22, 35), (26, 23, 34), (27, 24, 33), (28, 26, 31)),
          fur=fur, facing=math.cos(stage * math.pi / 8))
    x, y, pattern = heads[stage]
    head(canvas, pattern, x, y, fur)
    return canvas


def push(stage, fur=None):
    canvas = blank()
    shift = (1, 3)[stage]
    stroke(canvas, [(34 - shift, 24), (38 - shift, 22), (39, 22)], material=cat_coats.material(fur, "tail"))
    stroke(canvas, [(33 - shift, 26), (36, 28), (39, 29)], 2, cat_coats.material(fur, "hind-leg"))
    stroke(canvas, [(32 - shift, 27), (35, 30), (38, 30)], 2, cat_coats.material(fur, "far-leg"))
    stroke(canvas, [(25 - shift, 25), (21 - shift, 26), (18 - shift, 27)], 2, cat_coats.material(fur, "leg"))
    stroke(canvas, [(27 - shift, 25), (22 - shift, 25), (20 - shift, 26)], 2, cat_coats.material(fur, "far-leg"))
    torso(canvas, ((22, 25, 30), (23, 23, 33), (24, 21, 34),
                   (25, 22, 33), (26, 24, 32), (27, 26, 30)), shift, fur, facing=-1)
    head(canvas, LEFT, 22 - shift, 19 - stage, fur)
    return canvas


def copied(pixels, shift=0, mirror=False, head_drop=0):
    canvas = blank()
    for y, row in enumerate(pixels):
        for x, pixel in enumerate(row):
            if not pixel[3]:
                continue
            target_x = (49 - x if mirror else x) + shift
            target_y = y + (head_drop if x >= 31 and y <= 24 else 0)
            if 10 <= target_x <= 39 and 16 <= target_y <= 31:
                canvas[target_y][target_x] = pixel
    return canvas


def pixels(frame, source, fur=None):
    if frame < 6:
        # Align the nose through gather, reach, and the original paw contact.
        # A walk-to-run substitution used to jerk the head backwards here.
        index, shift = ((1, 5), (2, 1), (2, 2), (2, 3), (3, 0), (4, 0))[frame]
        return copied(source("Run", index), shift)
    if frame < 15:
        return turn(frame - 6, fur)
    if frame < 17:
        return push(frame - 15, fur)
    if frame < 20:
        index, shift = ((2, 1), (2, 0), (3, 1))[frame - 17]
        return copied(source("Run", index), shift, mirror=True)
    if frame == 20:
        canvas = copied(source("Run", 4), mirror=True)
        stroke(canvas, [(13, 28), (11, 30), (11, 31)], 2, cat_coats.material(fur, "leg"))
        return canvas
    if frame < 23:
        return copied(source("Walk", 0), mirror=True, head_drop=22 - frame)
    if frame < 26:
        # Finish on Run-7 so the normal Run-0 handoff continues its cycle.
        return copied(source("Run", frame - 18), mirror=True)
    raise ValueError(f"Unknown parkour frame: {frame}")
