"""Convert the bundled CC0 Pet Cats pixels to foreground-colored outlines."""

from pathlib import Path
import zipfile

import png
import spinning_cat

BASE = 0xF700
XLARGE_BASE = 0xE400
SPIN_BASE = 0x100000
MIRROR_BASE = 0x101000
MANIFEST = Path(__file__).with_name("pet-cats") / "animations.tsv"


def animations():
    return [line.split("\t") for line in MANIFEST.read_text().splitlines()
            if line and not line.startswith("#")]


def rectangles(pixels, accept):
    """Join adjacent pixels into horizontal runs, preserving exact boundaries."""
    contours = []
    for y, row in enumerate(pixels):
        x = 0
        while x < len(row):
            if not accept(row[x]):
                x += 1
                continue
            left = x
            while x < len(row) and accept(row[x]):
                x += 1
            contours.append([(left, 50-y-1), (left, 50-y), (x, 50-y), (x, 50-y-1)])
    return contours


def poses():
    frames = []
    with zipfile.ZipFile(Path(__file__).with_name("vendor") / "Pet-Cats-Pack.zip") as archive:
        for name, filename, count, mode in animations():
            count = int(count)
            if mode == "spin":
                for frame in range(count):
                    mono = rectangles(spinning_cat.pixels(frame, count), lambda pixel: pixel[3])
                    frames.append({"mono": mono, "action": name})
                continue
            path = f"Pet Cats Pack/Cat-1/Cat-1-{filename}.png"
            width, height, rows, _ = png.Reader(bytes=archive.read(path)).asRGBA8()
            rows = [[tuple(row[x:x+4]) for x in range(0, len(row), 4)] for row in rows]
            source_count = 1 if mode == "breathing" else count
            if (width, height) != (source_count * 50, 50):
                raise ValueError(f"Unexpected Pet Cats dimensions for {path}: {width}x{height}")
            for frame in range(count):
                source = 0 if mode == "breathing" else frame
                pixels = [row[source*50:(source+1)*50] for row in rows]
                # The first cat has green eyes against orange fur. Keep these
                # as cutouts so the cat can inherit the status foreground.
                mono = rectangles(pixels, lambda pixel: pixel[3] and pixel[1] < pixel[0]*0.95)
                if mode == "breathing":
                    floor = min(y for contour in mono for _, y in contour)
                    def breathe(contours):
                        return [[(x, floor + (y-floor)*(1+0.04*frame)) for x, y in contour]
                                for contour in contours]
                    mono = breathe(mono)
                frames.append({"mono": mono, "action": name})
    # One common coordinate system for every action prevents accidental
    # resizing or baseline shifts between walking, sitting, and sleeping.
    points = [point for frame in frames for contour in frame["mono"] for point in contour]
    bounds = (min(x for x, _ in points), max(x for x, _ in points),
              min(y for _, y in points), max(y for _, y in points))
    original = next(index for index, frame in enumerate(frames) if frame["action"] == "spin")
    layout = [(BASE + index*6 + offset + half, cells, half, index)
              for index in range(original)
              for cells, offset in ((1, 0), (2, 1), (3, 3)) for half in range(cells)]
    layout += [(XLARGE_BASE + index*4 + half, 4, half, index)
               for index in range(original) for half in range(4)]
    sizes = ((1, 0), (2, 1), (3, 3), (4, 6))
    layout += [(SPIN_BASE + (index-original)*10 + offset + half, cells, half, index)
               for index in range(original, len(frames))
               for cells, offset in sizes for half in range(cells)]
    normal_count = len(frames)
    frames += [{"mono": [[(50-x, y) for x, y in reversed(contour)]
                          for contour in frame["mono"]], "action": frame["action"]}
               for frame in frames[:]]
    layout += [(MIRROR_BASE + index*10 + offset + half, cells, half, normal_count+index)
               for index in range(normal_count)
               for cells, offset in sizes for half in range(cells)]
    return frames, layout, bounds
