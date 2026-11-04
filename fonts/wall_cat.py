"""Wall reactions drawn from the bundled Cat-1 pixels, in their 50 px cell."""

import io
import zipfile

import png
import parkour_cat


INK = (255, 160, 60, 255)
CLEAR = (0, 0, 0, 0)


def source(archive: zipfile.ZipFile, action: str, frame: int):
    path = f"Pet Cats Pack/Cat-1/Cat-1-{action}.png"
    width, height, rows, _ = png.Reader(file=io.BytesIO(archive.read(path))).asRGBA8()
    rows = list(rows)
    if height != 50 or width < (frame + 1) * 50:
        raise ValueError(f"Unexpected Cat-1 sprite sheet: {path}")
    return [[tuple(row[4 * (frame * 50 + x):4 * (frame * 50 + x + 1)])
             for x in range(50)] for row in rows]


def line(canvas, start, end, width=1):
    steps = max(abs(end[0] - start[0]), abs(end[1] - start[1]))
    for step in range(steps + 1):
        x = round(start[0] + (end[0] - start[0]) * step / max(steps, 1))
        y = round(start[1] + (end[1] - start[1]) * step / max(steps, 1))
        for dx in range(width):
            if 0 <= x + dx < 50 and 0 <= y < 50:
                canvas[y][x + dx] = INK


def pixels(mode: str, frame: int, count: int, archive: zipfile.ZipFile):
    if mode == "wall-parkour":
        return parkour_cat.pixels(frame, lambda action, index: source(archive, action, index))
    if mode == "wall-bump":
        if frame < 3:
            base, index, shift = "Run", frame % 3, (0, 2, 4)[frame]
        elif frame < 5:
            base, index, shift = "Run", 1, 11
        elif frame < 8:
            base, index, shift = "Walk", frame - 5, (3, 2, 0)[frame - 5]
        elif frame < 11:
            base, index, shift = "Laying", frame - 8, 0
        elif frame < 18:
            base, index, shift = "Sitting", 0, 1
        else:
            base, index, shift = "Walk", frame - 18, 0
        rise = 0.0
    elif mode == "wall-scratch":
        if frame < 2:
            base, index, shift, rise, climb = "Walk", frame, frame, 0.0, 0
        elif frame < 14:
            base, index = "Stretching", frame - 2
            shift = 3 if frame == 2 else (4 if frame == 13 else 5)
            rise = (0.12, 0.3, 0.36, 0.42, 0.48, 0.42, 0.36,
                    0.46, 0.52, 0.44, 0.36, 0.25)[frame - 2]
            climb = (0, 0, 1, 2, 3, 2, 1, 3, 4, 2, 1, 0)[frame - 2]
        else:
            base, index, shift, rise, climb = "Walk", 15 - frame, 2 if frame == 14 else 0, 0.0, 0
    else:
        raise ValueError(f"Unknown wall animation: {mode}")
    canvas = [[CLEAR for _ in range(50)] for _ in range(50)]
    for y, row in enumerate(source(archive, base, index)):
        for x, pixel in enumerate(row):
            if not pixel[3]:
                continue
            target_x = x + shift
            target_y = y - round(max(0, x - 24) * rise)
            if mode == "wall-scratch":
                target_y -= round(climb * min(1.0, max(0, x - 26) / 7))
            if mode == "wall-bump" and 11 <= frame < 18 and x <= 21 and y >= 24:
                target_y -= (0, 0, 1, 2, 1, 0, 1)[frame - 11]
            target_y = min(31, max(16, target_y))
            if mode == "wall-bump" and 3 <= frame < 5 and x >= 23:
                target_x = min(target_x, 39)
                if y < 23:
                    target_y += 2
            if 0 <= target_x < 50 and 0 <= target_y < 50:
                canvas[target_y][target_x] = pixel
    if mode == "wall-bump" and 3 <= frame < 5:
        # A compressed nose and two tiny impact ticks, then a backward step.
        line(canvas, (38, 18), (39, 16))
        line(canvas, (38, 22), (39, 23))
    if mode == "wall-scratch" and 3 <= frame < 13:
        # The hind paws stay planted as the torso lengthens upward. Forepaws
        # climb in alternating strokes, with small claw tips at the edge.
        stroke = (frame - 3) % 4
        near_y = (21, 18, 16, 19)[stroke]
        far_y = (17, 20, 19, 16)[stroke]
        line(canvas, (32, 25 - climb // 2), (38, near_y), 2)
        line(canvas, (30, 26 - climb // 2), (37, far_y))
        line(canvas, (38, near_y), (39, near_y + 1))
        line(canvas, (37, far_y), (38, far_y + 1))
    return canvas
