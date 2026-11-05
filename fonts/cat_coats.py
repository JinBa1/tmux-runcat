"""Six coats on the same cat, with materials attached before posing.

Tabbies retain the approved Cat-6 markings. Other coats use Cat-1's identical
gait and its authored shading, with explicit anatomical landmarks for patches.
The same coat methods paint custom parkour parts and the rotating spin model.
"""

from dataclasses import dataclass
from pathlib import Path
import math

import tabby_cat

CLEAR = tabby_cat.CLEAR
CHOICES = ("tabby", "orange-tabby", "silver-tabby", "black", "tuxedo", "calico")


@dataclass(frozen=True)
class Region:
    eye: tuple
    facing: int
    hip: tuple
    shoulder: tuple
    radius: float
    paws: tuple

    def coordinates(self, x, y):
        dx, dy = self.shoulder[0]-self.hip[0], self.shoulder[1]-self.hip[1]
        length = math.hypot(dx, dy)
        u = ((x-self.hip[0])*dx+(y-self.hip[1])*dy) / length**2
        v = (-(x-self.hip[0])*dy+(y-self.hip[1])*dx) / (length*self.radius)
        return u, v


REGIONS = {}
for line in (Path(__file__).parent / "pet-cats/coat-regions.tsv").read_text().splitlines():
    if line and not line.startswith("#"):
        action, frame, eye, facing, hip, shoulder, radius, paws = line.split("\t")
        def point(text):
            return tuple(map(float, text.split(",")))
        REGIONS[action, int(frame)] = Region(point(eye), int(facing), point(hip), point(shoulder),
                                             float(radius), tuple(point(p) for p in paws.split()) if paws != "-" else ())


class Tabby:
    source_cat = 6

    def __init__(self, title, suffix, palette):
        self.title, self.suffix, self.palette = title, suffix, palette
        self.colors = dict(zip(tabby_cat.PALETTE, palette))

    def recolor(self, pixels, action, frame):
        return [[self.colors[pixel] if pixel[3] else CLEAR for pixel in row]
                for row in tabby_cat.recolor(pixels, action, frame)]

    def finish(self, pixels):
        return pixels

    def limb(self, part, progress):
        return self.colors[tabby_cat.limb(part, progress)]

    def body(self, *args):
        return self.colors[tabby_cat.body(*args)]

    def head(self, *args):
        return self.colors[tabby_cat.head(*args)]

    def surface(self, *args):
        return self.colors[tabby_cat.surface(*args)]


class Black:
    source_cat = 1
    title, suffix = "Black", "Black"
    dark = (45, 49, 59, 255)
    shadow = (53, 58, 68, 255)
    shade = (62, 68, 79, 255)
    mid = (70, 77, 89, 255)
    fur = (80, 87, 100, 255)
    light = (98, 106, 119, 255)
    highlight = (116, 125, 139, 255)
    eye = (194, 186, 99, 255)
    nose = (132, 98, 111, 255)
    palette = (dark, shadow, shade, mid, fur, light, highlight, eye, nose)

    def native(self, pixel):
        color = pixel[:3]
        if color == (80, 81, 33):
            return self.eye
        if color in ((168, 62, 62), (172, 122, 10)):
            return self.nose
        return {(155, 117, 33): self.fur, (136, 103, 28): self.shade,
                (136, 103, 29): self.shade, (181, 152, 88): self.light,
                (214, 194, 149): self.highlight, (107, 80, 18): self.dark,
                (120, 91, 24): self.shadow, (143, 108, 31): self.mid,
                (149, 112, 31): self.mid, (180, 136, 39): self.light,
                (126, 94, 24): self.shade, (161, 120, 30): self.fur}[color]

    def recolor(self, pixels, action, frame):
        return [[self.native(p) if p[3] >= 128 else CLEAR for p in row] for row in pixels]

    def finish(self, pixels):
        return pixels

    def limb(self, part, progress):
        if part == "effect":
            return self.highlight
        if part == "tail":
            return self.mid
        if progress > .9:
            return self.light
        return self.shadow if part == "far-leg" else self.shade

    def body(self, x, y, left, right, top, bottom, facing=1.0):
        v = (y-top)/max(1, bottom-top)
        return self.fur if v < .4 else self.mid if v < .8 else self.shade

    def head(self, pattern, x, y):
        if pattern[y][x] == ".":
            return self.eye
        if y == 4 and x == (2 if pattern[3] == "#.#.#" else 4 if pattern[3] == "###.#" else 0):
            return self.nose
        return self.light if y >= 4 else self.fur if y > 0 else self.shade

    def surface(self, part, point, normal):
        x, y, z = point
        if part == "eye":
            return self.eye
        if part == "tail":
            return self.mid
        if part == "leg":
            return self.light if y < 1.1 else self.shade
        if part == "ear":
            return self.shade
        if part == "head":
            if z > 9.2 and abs(x) < .65 and 9.1 < y < 9.8:
                return self.nose
            return self.light if normal[1] < -.25 else self.fur
        return self.fur if normal[1] > .1 else self.shade


class Tuxedo(Black):
    title, suffix = "Tuxedo", "Tuxedo"
    base_patch = "black"
    white = (236, 239, 233, 255)
    white_shade = (202, 211, 210, 255)
    white_shadow = (160, 174, 180, 255)
    nose = (195, 132, 142, 255)
    palette = (*Black.palette[:-1], nose, white, white_shade, white_shadow)

    def patch(self, u, v, facing):
        if facing == 0:
            return "white" if u > .4 and abs(v) < .48 else "black"
        return "white" if u > .78 and v > -.15 else "black"

    def material(self, patch, shade=False):
        return (self.white_shade if shade else self.white) if patch == "white" else self.shade if shade else self.fur

    def recolor(self, pixels, action, frame):
        result = super().recolor(pixels, action, frame)
        region = REGIONS[action, frame]
        ex, ey = region.eye
        hx = ex-2 if region.facing == 0 else ex-3
        pattern = ("#   #", "#####", "#####", "#.#.#", "#####", " ### ") if region.facing == 0 else (
            "  #  ", " ### ", "#####", "###.#", "#####", "#### ")
        for y, row in enumerate(result):
            for x, pixel in enumerate(row):
                if not pixel[3]:
                    continue
                native = pixels[y][x][:3]
                if native in ((80, 81, 33), (168, 62, 62)):
                    continue
                u, v = region.coordinates(x, y)
                if -.15 <= u <= 1.2 and abs(v) <= 1.35:
                    patch = self.patch(u, v, region.facing)
                    if patch != self.base_patch:
                        row[x] = self.material(patch, native in ((136, 103, 28), (136, 103, 29), (107, 80, 18)))
                if hx <= x <= hx+4 and ey-3 <= y <= ey+2:
                    color = self.head(pattern, round(x-hx), round(y-(ey-3)))
                    row[x] = self.fur if color == self.eye else color
                if any(abs(x-px) <= .9 and py-.6 <= y <= py+1 for px, py in region.paws):
                    row[x] = self.white_shadow if native == (107, 80, 18) else self.white
        return result

    def finish(self, pixels):
        # Shade exposed white pixels, after all posing and strokes. This keeps
        # muzzle, socks and pale backs readable on both light and dark rows.
        result = [row[:] for row in pixels]
        for y, row in enumerate(pixels):
            for x, pixel in enumerate(row):
                if pixel == self.white and any(not (0 <= nx < 50 and 0 <= ny < 50) or not pixels[ny][nx][3]
                                               for nx, ny in ((x-1, y), (x+1, y), (x, y-1), (x, y+1))):
                    result[y][x] = self.white_shade
        return result

    def limb(self, part, progress):
        if part == "effect":
            return self.white
        if part != "tail" and progress >= .83:
            return self.white_shadow if part == "far-leg" else self.white
        return super().limb(part, progress)

    def body(self, x, y, left, right, top, bottom, facing=1.0):
        u = (x-left)/max(1, right-left)
        v = (y-top)/max(1, bottom-top)
        if abs(facing) < .4:
            if abs(u-.5) < .16 and .2 < v < .85:
                return self.white
        elif (u if facing > 0 else 1-u) > .8 and v > .35:
            return self.white_shade
        return super().body(x, y, left, right, top, bottom, facing)

    def head(self, pattern, x, y):
        front, right = pattern[3] == "#.#.#", pattern[3] == "###.#"
        if not front and not right:
            return self.head(tuple(row[::-1] for row in pattern), 4-x, y)
        if pattern[y][x] == ".":
            return self.eye
        if y == 4 and x == (2 if front else 4):
            return self.nose
        if (front and x == 2 and y >= 2) or (y >= 4 and (1 <= x <= 3 if front else x >= 2)):
            return self.white
        return self.shade if y == 0 else self.fur

    def surface(self, part, point, normal):
        x, y, z = point
        if part == "head":
            if z > 8.3 and y < 9.8:
                return self.nose if abs(x) < .65 and y > 9.1 else self.white
            if z > 8.4 and abs(x) < .6 and y < 12.4:
                return self.white
        if part == "leg" and y < 1.2:
            return self.white
        if part == "body" and z > 3.8 and normal[1] < .25:
            return self.white_shade
        return super().surface(part, point, normal)


class Calico(Tuxedo):
    title, suffix = "Calico", "Calico"
    base_patch = "white"
    orange = (218, 143, 76, 255)
    orange_shade = (178, 110, 60, 255)
    orange_light = (237, 178, 113, 255)
    palette = (*Tuxedo.palette, orange, orange_shade, orange_light)

    def patch(self, u, v, facing):
        if facing == 0:
            return "orange" if v < -.45 else "black" if v > .5 and u > .30 else "white"
        if ((u-.23)/.30)**2+((v+.2)/.85)**2 < 1:
            return "orange"
        if ((u-.81)/.22)**2+((v+.12)/.72)**2 < 1:
            return "black"
        return "white"

    def material(self, patch, shade=False):
        if patch == "orange":
            return self.orange_shade if shade else self.orange
        return super().material(patch, shade)

    def recolor(self, pixels, action, frame):
        # Start with a shaded white coat; the parent projects the authored
        # anatomical patch regions, head pattern, and paw landmarks onto it.
        result = super().recolor(pixels, action, frame)
        return tabby_cat.paint_tail(result, action, frame, self.limb)

    def native(self, pixel):
        color = super().native(pixel)
        if color in (self.eye, self.nose):
            return color
        if color in (self.dark, self.shadow):
            return self.white_shadow
        return self.white_shade if color in (self.shade, self.mid) else self.white

    def limb(self, part, progress):
        if part == "tail":
            return self.orange_shade if progress < .38 else self.mid
        if part == "hind-leg" and progress < .75:
            return self.orange_shade
        return super().limb(part, progress)

    def body(self, x, y, left, right, top, bottom, facing=1.0):
        u = (x-left)/max(1, right-left)
        v = (y-top)/max(1, bottom-top)
        if abs(facing) < .4:
            patch = "orange" if u < .28 else "black" if u > .75 else "white"
        else:
            patch = self.patch(u if facing > 0 else 1-u, 1.7*v-.85, 1)
        return self.material(patch, v > .65)

    def head(self, pattern, x, y):
        front, right = pattern[3] == "#.#.#", pattern[3] == "###.#"
        if not front and not right:
            return self.head(tuple(row[::-1] for row in pattern), 4-x, y)
        if pattern[y][x] == ".":
            return self.eye
        if y == 4 and x == (2 if front else 4):
            return self.nose
        if y >= 4 or x == 2:
            return self.white
        if x < 2:
            return self.orange if y > 0 else self.orange_shade
        return self.fur

    def surface(self, part, point, normal):
        x, y, z = point
        if part == "body":
            return self.material(self.patch((z+6)/12, -normal[1], 1), normal[1] < -.4)
        if part == "tail":
            return self.limb("tail", min(1, max(0, (-z-10)/3)))
        if part == "leg":
            if y < 1.2:
                return self.white
            return self.orange_shade if z < 0 else self.shade
        if part == "ear":
            return self.orange_shade if x < 0 else self.shade
        if part == "head":
            if z > 8.3 and y < 9.8:
                return self.nose if abs(x) < .65 and y > 9.1 else self.white
            if abs(x) < .6 or y < 9.2:
                return self.white
            return self.orange if x < 0 else self.fur
        return super().surface(part, point, normal)


COATS = {
    "tabby": Tabby("Brown tabby", "Tabby", tabby_cat.PALETTE),
    "orange-tabby": Tabby("Orange tabby", "Orange Tabby", (
        (145, 79, 38, 255), (168, 99, 45, 255), (192, 121, 61, 255),
        (216, 145, 77, 255), (235, 171, 101, 255), (244, 199, 145, 255),
        (250, 226, 191, 255), (171, 189, 97, 255), (210, 131, 123, 255))),
    "silver-tabby": Tabby("Silver tabby", "Silver Tabby", (
        (62, 69, 78, 255), (94, 103, 113, 255), (126, 137, 147, 255),
        (161, 172, 181, 255), (187, 198, 204, 255), (217, 225, 226, 255),
        (238, 239, 230, 255), (161, 192, 115, 255), (176, 124, 131, 255))),
    "black": Black(), "tuxedo": Tuxedo(), "calico": Calico(),
}


def get(name):
    return None if name is None or name == "mono" else COATS[name]


def material(name, part):
    coat = get(name)
    return (lambda progress: coat.limb(part, progress)) if coat else None
