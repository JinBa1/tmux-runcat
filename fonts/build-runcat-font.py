#!/usr/bin/env python3
"""Add cat poses to static monospaced TrueType fonts. Requires fontTools and PyPNG."""

import argparse
import json
from pathlib import Path
import re
import shutil
import unicodedata

from fontTools.pens.basePen import BasePen
from fontTools.colorLib.builder import buildCOLR, buildCPAL, populateCOLRv0
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.svgLib.path import SVGPath
from fontTools.ttLib import TTFont, TTLibError
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
from fontTools.ttLib.tables.C_P_A_L_ import Color

import pet_cats
import cat_coats


class OutlinePen(BasePen):
    """Flatten curves before clipping the silhouette into terminal cells."""

    def __init__(self, glyphs):
        super().__init__(glyphs)
        self.contours = []

    def _moveTo(self, point):
        self.points = [point]

    def _lineTo(self, point):
        self.points.append(point)

    def _qCurveToOne(self, control, end):
        start = self._getCurrentPoint()
        for step in range(1, 33):
            t = step / 32
            self.points.append(tuple((1-t)**2 * start[i] + 2*(1-t)*t * control[i] + t*t * end[i] for i in (0, 1)))

    def _curveToOne(self, first, second, end):
        start = self._getCurrentPoint()
        for step in range(1, 33):
            t = step / 32
            self.points.append(tuple((1-t)**3 * start[i] + 3*(1-t)**2*t * first[i] + 3*(1-t)*t*t * second[i] + t**3 * end[i] for i in (0, 1)))

    def _closePath(self):
        self.contours.append(self.points)

    _endPath = _closePath


def clip_polygon(points, boundary, keep_right):
    output = []
    if not points:
        return output
    previous = points[-1]
    for current in points:
        inside = (current[0] >= boundary) if keep_right else (current[0] <= boundary)
        was_inside = (previous[0] >= boundary) if keep_right else (previous[0] <= boundary)
        if inside != was_inside:
            t = (boundary - previous[0]) / (current[0] - previous[0])
            output.append((boundary, previous[1] + t * (current[1] - previous[1])))
        if inside:
            output.append(current)
        previous = current
    return output


def contour_area(points):
    return sum(points[i-1][0]*points[i][1] - points[i][0]*points[i-1][1]
               for i in range(len(points)))


def cat_poses():
    cat = TTFont(Path(__file__).with_name("runcat-tmux.ttf"))
    cat_glyphs = cat.getGlyphSet()
    outlines = []
    for code in range(0xEFF0, 0xEFF5):
        pen = OutlinePen(cat_glyphs)
        cat_glyphs[cat.getBestCmap()[code]].draw(pen)
        outlines.append(pen.contours)
    points = [point for contours in outlines for contour in contours for point in contour]
    xmin, xmax = min(p[0] for p in points), max(p[0] for p in points)
    ymin, ymax = min(p[1] for p in points), max(p[1] for p in points)
    pen = OutlinePen(None)
    SVGPath(str(Path(__file__).with_name("sleeping-cat.svg"))).draw(pen)
    # The SVG uses even-odd filling; TrueType uses winding direction. Its
    # interior details therefore need the opposite winding to the silhouette.
    orientation = contour_area(pen.contours[0])
    sleep_outline = []
    for index, contour in enumerate(pen.contours):
        if index and contour_area(contour) * orientation > 0:
            contour = list(reversed(contour))
        sleep_outline.append([(x, 1024-y) for x, y in contour])
    sleep_floor = min(y for contour in sleep_outline for _, y in contour)
    poses = outlines + [
        [[(x, sleep_floor + (y-sleep_floor)*(1+0.04*breath)) for x, y in contour]
         for contour in sleep_outline]
        for breath in range(3)
    ]
    # The two-cell mappings remain compatible with previous font versions.
    layout = [(0xEFF0+frame+half*5, 2, half, frame)
              for frame in range(5) for half in range(2)]
    layout += [(0xEFFA+frame*2+half, 2, half, 5+frame)
               for frame in range(3) for half in range(2)]
    layout += [(0xF600+frame, 1, 0, frame) for frame in range(8)]
    layout += [(0xF620+frame*3+half, 3, half, frame)
               for frame in range(8) for half in range(3)]
    layout += [(0xF640+frame*4+half, 4, half, frame)
               for frame in range(8) for half in range(4)]
    return poses, sorted(layout), (xmin, xmax, ymin, ymax)


def validate_font(path, layout):
    """Fail before producing output if a base cannot safely accept the poses."""
    font = TTFont(path, recalcTimestamp=False)
    if font.flavor or "fvar" in font or "glyf" not in font:
        raise ValueError(f"{path}: use a static TrueType font with glyf outlines (not variable, CFF, or WOFF)")
    if any(table not in font for table in ("OS/2", "hhea", "hmtx", "head", "cmap", "name")):
        raise ValueError(f"{path}: missing required TrueType metrics or character tables")
    cmap = font.getBestCmap() or {}
    if any(code not in cmap for code in range(32, 127)):
        raise ValueError(f"{path}: base font must contain printable ASCII, including space")
    advance = font["hmtx"][cmap[32]][0]
    if advance <= font["head"].unitsPerEm * 0.08 or any(font["hmtx"][cmap[code]][0] != advance for code in range(33, 127)):
        raise ValueError(f"{path}: base font must be monospaced (printable ASCII widths differ)")
    if font["hhea"].ascent <= font["head"].unitsPerEm * 0.08:
        raise ValueError(f"{path}: no usable ascent for the cat glyphs")
    for code, _, _, _ in layout:
        if any(code in table.cmap for table in font["cmap"].tables if table.isUnicode()):
            raise ValueError(f"{path}: cat codepoint U+{code:04X} collides with an existing glyph")
        if f"runcat{code:04x}" in font.getGlyphOrder():
            raise ValueError(f"{path}: already contains RunCat glyphs")
    return font


def rename_font(font, family, style):
    face = {"BoldItalic": "Bold Italic"}.get(style, style)
    # PostScript names are ASCII and at most 63 characters, even for families
    # whose display names contain spaces or non-ASCII characters.
    ascii_family = unicodedata.normalize("NFKD", family).encode("ascii", "ignore").decode()
    postscript = re.sub(r"[^A-Za-z0-9-]", "", ascii_family)[:48] or "RunCat"
    names = {1: family, 2: face, 3: f"{family}-{style}-runcat-v10",
             4: f"{family} {face}", 6: f"{postscript}-{style}",
             16: family, 17: face, 18: f"{family} {face}", 21: family, 22: face}
    for record in list(font["name"].names):
        if record.nameID in names:
            try:
                record.string = names[record.nameID].encode(record.getEncoding())
            except UnicodeEncodeError:
                font["name"].names.remove(record)
    for name_id, value in names.items():
        font["name"].setName(value, name_id, 3, 1, 0x409)


def outline_glyph(contours, bounds, scale, left, baseline, advance, half):
    xmin, _, ymin, _ = bounds
    pen = TTGlyphPen(None)
    for contour in contours:
        transformed = [(left + (x-xmin)*scale, baseline + (y-ymin)*scale) for x, y in contour]
        clipped = clip_polygon(transformed, half*advance, True)
        clipped = clip_polygon(clipped, (half+1)*advance, False)
        if len(clipped) < 3:
            continue
        local = [(x-half*advance, y) for x, y in clipped]
        pen.moveTo(local[0])
        for point in local[1:]:
            pen.lineTo(point)
        pen.closePath()
    return pen.glyph()


def add_coat_colors(font, layers, colors):
    """Append our palette and v0 layers without replacing base-font artwork."""
    offset = 0
    if "CPAL" in font:
        palette = font["CPAL"]
        offset = palette.numPaletteEntries
        for entries in palette.palettes:
            entries.extend(Color.fromRGBA(*color) for color in colors)
        palette.numPaletteEntries += len(colors)
        if palette.version == 1 and palette.paletteEntryLabels:
            palette.paletteEntryLabels.extend([0xFFFF] * len(colors))
    else:
        font["CPAL"] = buildCPAL([[tuple(component/255 for component in color)
                                  for color in colors]])
    layers = {name: [(glyph, index + offset) for glyph, index in entries]
              for name, entries in layers.items()}
    if "COLR" not in font:
        font["COLR"] = buildCOLR(layers, version=0, glyphMap=font.getReverseGlyphMap())
    elif font["COLR"].version == 0:
        font["COLR"].ColorLayers.update(buildCOLR(layers, version=0).ColorLayers)
    else:
        table = font["COLR"].table
        existing = {}
        if table.BaseGlyphRecordArray:
            records = table.LayerRecordArray.LayerRecord
            for base in table.BaseGlyphRecordArray.BaseGlyphRecord:
                existing[base.BaseGlyph] = [(layer.LayerGlyph, layer.PaletteIndex)
                                            for layer in records[base.FirstLayerIndex:
                                                                 base.FirstLayerIndex + base.NumLayers]]
        populateCOLRv0(table, {**existing, **layers}, font.getReverseGlyphMap())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="extracted default JetBrainsMono Nerd Font release")
    parser.add_argument("output", type=Path, help="directory for installer font files")
    parser.add_argument("--cat-only", action="store_true", help="omit clean default NF family files")
    parser.add_argument("--base-font", type=Path, help="custom regular monospaced static TTF")
    parser.add_argument("--bold-font", type=Path)
    parser.add_argument("--italic-font", type=Path)
    parser.add_argument("--bold-italic-font", type=Path)
    parser.add_argument("--family", help="display name of the patched family")
    parser.add_argument("--fur", choices=("mono", *cat_coats.CHOICES), default="mono",
                        help="colored coat, or normal foreground outlines")
    args = parser.parse_args()
    if not args.base_font and any((args.bold_font, args.italic_font, args.bold_italic_font)):
        parser.error("optional font faces require --base-font")
    sources = {"Regular": args.base_font} if args.base_font else {
        style: args.source / f"JetBrainsMonoNerdFont-{style}.ttf"
        for style in ("Regular", "Bold", "Italic", "BoldItalic")
    }
    for style, path in (("Bold", args.bold_font), ("Italic", args.italic_font),
                        ("BoldItalic", args.bold_italic_font)):
        if path:
            sources[style] = path
    poses, layout, classic_bounds = cat_poses()
    coat = cat_coats.get(args.fur)
    pet_frames, pet_layout, pet_bounds = pet_cats.poses(args.fur if coat else None)
    pet_codes = {code for code, _, _, _ in pet_layout}
    layout += pet_layout
    fonts = {}
    try:
        for style, path in sources.items():
            fonts[style] = validate_font(path, layout)
        if args.family is not None:
            family = args.family.strip()
        elif args.base_font:
            names = fonts["Regular"]["name"]
            base_family = names.getDebugName(16) or names.getDebugName(1) or args.base_font.stem
            family = f"{base_family} RunCat {coat.suffix} v1" if coat else f"{base_family} RunCat v10"
        elif coat:
            family = f"JetBrainsMono RunCat {coat.suffix} v1"
        else:
            family = Path(__file__).with_name("font-family.txt").read_text().strip()
        if not family or any(ord(char) < 32 or ord(char) == 127 for char in family):
            raise ValueError("font family must be nonempty and contain no control characters")
        # Mixing unrelated faces gives a font family inconsistent spacing and
        # style selection. Users can supply only Regular when no other faces exist.
        regular = fonts["Regular"]
        base_name = regular["name"].getDebugName(16) or regular["name"].getDebugName(1)
        for style, font in fonts.items():
            name = font["name"].getDebugName(16) or font["name"].getDebugName(1)
            if name != base_name:
                raise ValueError(f"{sources[style]}: all faces must belong to the same base font family")
    except (OSError, ValueError, TTLibError) as error:
        parser.error(str(error))

    args.output.mkdir(parents=True, exist_ok=True)
    generated = []
    if not args.base_font and not args.cat_only:
        for path in sorted(args.source.glob("*.ttf")):
            destination = args.output / ("NF-3.5.1-" + path.name)
            shutil.copyfile(path, destination)
            generated.append(destination)
    for style, font in fonts.items():
        original_cmap = font.getBestCmap().copy()
        if not any(table.isUnicode() and table.format == 12 for table in font["cmap"].tables):
            table = CmapSubtable.newSubtable(12)
            table.platformID, table.platEncID, table.language = 3, 10, 0
            table.cmap = original_cmap.copy()
            font["cmap"].tables.append(table)
        advance = font["hmtx"][original_cmap[32]][0]
        units = font["head"].unitsPerEm
        baseline, margin = units * 0.02, units * 0.04
        row_height = font["hhea"].ascent - 2 * baseline
        cap_height = getattr(font["OS/2"], "sCapHeight", 0)
        if cap_height <= 0:
            # Older TrueType fonts need not declare sCapHeight.
            cap_height = getattr(font["glyf"][original_cmap[ord("H")]], "yMax", units * 0.7)
        cap_height = min(cap_height or units * 0.7, row_height)
        order = list(font.getGlyphOrder())
        color_layers = {}

        def store_glyph(name, glyph):
            font["glyf"][name] = glyph
            glyph.recalcBounds(font["glyf"])
            font["hmtx"][name] = (advance, getattr(glyph, "xMin", 0))
            order.append(name)

        for code, cells, half, frame in layout:
            name = f"runcat{code:04x}"
            is_pet = code in pet_codes
            bounds = pet_bounds if is_pet else classic_bounds
            xmin, xmax, ymin, ymax = bounds
            height = row_height if cells == 3 else cap_height
            floor = baseline
            if cells == 4:
                # Use the complete existing line box, including its descent.
                # Ordinary text keeps its original metrics and font size.
                floor = min(0, font["hhea"].descent) + baseline
                height = font["hhea"].ascent - baseline - floor
            scale = min((cells*advance-2*margin)/(xmax-xmin), height/(ymax-ymin))
            left = (cells*advance - (xmax-xmin)*scale)/2
            geometry = (bounds, scale, left, floor, advance, half)
            contours = pet_frames[frame]["mono"] if is_pet else poses[frame]
            store_glyph(name, outline_glyph(contours, *geometry))
            if is_pet and coat:
                layers = []
                for index, contours in pet_frames[frame]["layers"]:
                    glyph = outline_glyph(contours, *geometry)
                    if glyph.numberOfContours:
                        layer_name = f"{name}.fur{index}"
                        store_glyph(layer_name, glyph)
                        layers.append((layer_name, index))
                if layers:
                    color_layers[name] = layers
            for table in font["cmap"].tables:
                if table.isUnicode() and table.format != 14 and (code <= 0xFFFF or table.format in (12, 13)):
                    table.cmap[code] = name
        font.setGlyphOrder(order)
        if color_layers:
            add_coat_colors(font, color_layers, coat.palette)
        rename_font(font, family, style)
        destination = args.output / f"RunCat-{style}.ttf"
        font.save(destination)
        generated.append(destination)
        # Existing characters, glyphs, metrics and font features are preserved.
        saved = TTFont(destination)
        assert all(saved.getBestCmap()[code] == glyph for code, glyph in original_cmap.items())
        assert saved["hmtx"][original_cmap[32]][0] == advance
        print(destination)

    manifest = {}
    # Only this build's files are installed. Old files from another selection
    # may still be present in the generated directory.
    for path in generated:
        font = TTFont(path)
        display_family = font["name"].getDebugName(1)
        display_face = font["name"].getDebugName(2)
        manifest[path.name] = f"{display_family} {display_face} (TrueType)"
    (args.output / "font-names.json").write_text(json.dumps(manifest), encoding="utf-8")
    (args.output / "font-family.txt").write_text(family + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
