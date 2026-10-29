"""Build small synthetic fonts exercising CFF charstrings and TrueType
collections, and record what FreeType/matplotlib make of them.

Run from the repository root with the project venv:

    MPL_IGNORE_SYSTEM_FONTS=1 MPLCONFIGDIR=$(mktemp -d) \
        .venv/bin/python -I crates/pltrs-font/tests/fixtures/gen_synthetic_fonts.py

Writes `synthetic-cff.otf`, `synthetic.ttc` and `synthetic.json` next to
this script.
"""

import json
import os
import pathlib

from fontTools.cffLib import SubrsIndex
from fontTools.fontBuilder import FontBuilder
from fontTools.misc.psCharStrings import T2CharString
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib.ttCollection import TTCollection
from matplotlib import font_manager as fm
from matplotlib.ft2font import FT2Font, LoadFlags

HERE = pathlib.Path(__file__).resolve().parent
NO_HINTING = LoadFlags.NO_HINTING


def cff_font(path):
    """A name-keyed CFF font with subroutines, hints, flex and seac."""
    names = [".notdef", "space", "A", "O", "B", "flex", "acute", "Aacute", "hinted", "curves"]
    fb = FontBuilder(1000, isTTF=False)
    fb.setupGlyphOrder(names)
    fb.setupCharacterMap({0x20: "space", 0x41: "A", 0x4F: "O", 0x42: "B", 0x46: "flex",
                          0xB4: "acute", 0xC1: "Aacute", 0x48: "hinted", 0x43: "curves"})
    # Local subr 0 (biased -107) draws a box; global subr 0 a triangle.
    local_subrs = [T2CharString(program=[0, 100, "rlineto", 100, 0, "rlineto", 0, -100, "rlineto", "return"])]
    global_subrs = [T2CharString(program=[50, 80, "rlineto", 50, -80, "rlineto", "return"])]
    programs = {
        ".notdef": [500, 50, 0, "rmoveto", 400, 0, "rlineto", 0, 700, "rlineto", -400, 0, "rlineto", "endchar"],
        "space": [250, "endchar"],
        "A": [600, 20, 0, "rmoveto", 280, 700, "rlineto", 280, -700, "rlineto", -100, "hlineto",
              -60, 150, "rlineto", -240, "hlineto", -60, -150, "rlineto", "endchar"],
        "O": [700, 350, -10, "rmoveto", 180, 0, 140, 160, 0, 200, "rrcurveto",
              200, -140, 160, -180, -180, "vhcurveto", -180, -140, -160, -200, -200, 140, -160, 180, "hvcurveto",
              "endchar"],
        "B": [650, 100, 0, "rmoveto", -107, "callsubr", -100, 300, "rmoveto", -107, "callgsubr", "endchar"],
        "flex": [600, 50, 300, "rmoveto", 100, 20, 100, 0, 100, -20, 100, -20, 100, 0, 100, 20, 50, "flex",
                 0, -200, "rlineto", -600, "hlineto",
                 -100, 50, -50, 50, -100, -100, -50, -50, -100, "hflex1",
                 "endchar"],
        "acute": [300, 100, 600, "rmoveto", 80, 120, "rlineto", 40, -30, "rlineto", "endchar"],
        "Aacute": [600, 150, 120, 65, 194, "endchar"],
        "hinted": [600, 0, 50, 650, 50, "hstemhm", 100, 60, "hintmask", bytes([0b11000000]), 40, 0, "rmoveto",
                   500, 0, "rlineto", 0, 50, "rlineto", -500, 0, "rlineto", "endchar"],
        "curves": [600, 50, 50, "rmoveto", 100, 50, 150, 200, "hhcurveto", 20, 100, 50, 150, 100, "vvcurveto",
                   300, 0, 100, 100, "hvcurveto", 10, 20, 30, 40, 50, 60, 70, 80, "rcurveline",
                   10, 0, 0, 10, 50, 0, 50, 60, 0, 70, "rlinecurve", "endchar"],
    }
    charstrings = {name: T2CharString(program=program) for name, program in programs.items()}
    subrs = SubrsIndex()
    for subr in local_subrs:
        subrs.append(subr)
    fb.setupCFF("PltSyntheticCFF-Regular", {"FullName": "Plt Synthetic CFF"}, charstrings, {"Subrs": subrs})
    for subr in global_subrs:
        fb.font["CFF "].cff.GlobalSubrs.append(subr)
    fb.setupHorizontalMetrics({name: (int(prog[0]), 0) for name, prog in programs.items()})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({"familyName": "Plt Synthetic CFF", "styleName": "Regular"})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    fb.save(str(path))


def ttf_face(family, style, weight, scale):
    """A tiny TrueType face; glyph shapes differ by `scale`."""
    names = [".notdef", "space", "A", "Abar"]
    fb = FontBuilder(1024, isTTF=True)
    fb.setupGlyphOrder(names)
    fb.setupCharacterMap({0x20: "space", 0x41: "A", 0x100: "Abar"})
    pen = TTGlyphPen(None)
    pen.moveTo((0, 0)); pen.lineTo((0, 700)); pen.lineTo((500, 700)); pen.lineTo((500, 0)); pen.closePath()
    notdef = pen.glyph()
    pen = TTGlyphPen(None)
    pen.moveTo((10, 0))
    pen.qCurveTo((300 * scale, 900), (600 * scale, 0))
    pen.closePath()
    a = pen.glyph()
    glyphs = {".notdef": notdef, "space": TTGlyphPen(None).glyph(), "A": a, "Abar": a}
    fb.setupGlyf(glyphs)
    # Abar: composite of A plus a scaled, offset copy of .notdef.
    from fontTools.ttLib.tables._g_l_y_f import Glyph, GlyphComponent
    composite = Glyph()
    composite.numberOfContours = -1
    first = GlyphComponent()
    first.glyphName, first.x, first.y, first.flags = "A", 0, 0, 0x0200
    second = GlyphComponent()
    second.glyphName, second.x, second.y, second.flags = ".notdef", 100, 750, 0
    second.transform = [[0.5, 0], [0, 0.125]]
    composite.components = [first, second]
    fb.font["glyf"]["Abar"] = composite
    fb.setupHorizontalMetrics({".notdef": (500, 0), "space": (250, 0), "A": (int(620 * scale), 10), "Abar": (int(620 * scale), 10)})
    fb.setupHorizontalHeader(ascent=900, descent=-200)
    fb.setupNameTable({"familyName": family, "styleName": style})
    fb.setupOS2(usWeightClass=weight, fsSelection=0x20 if weight >= 700 else 0x40,
                sTypoAscender=900, sTypoDescender=-200, usWinAscent=900, usWinDescent=200)
    fb.setupPost()
    return fb.font


def path_of(font):
    vertices, codes = font.get_path()
    flat = []
    for (x, y), code in zip(vertices, codes):
        flat += [int(code), round(x * 64), round(y * 64)]
    return flat


def describe(path, face_index):
    font = FT2Font(str(path), face_index=face_index)
    out = {
        "family_name": font.family_name,
        "style_name": font.style_name,
        "postscript_name": font.postscript_name,
        "num_faces": font.num_faces,
        "num_glyphs": font.num_glyphs,
        "entry": None,
        "glyphs": [],
    }
    prop = fm.ttfFontProperty(font)
    out["entry"] = {"name": prop.name, "style": prop.style, "weight": int(prop.weight), "stretch": prop.stretch}
    upem = font.units_per_EM
    for gid in range(font.num_glyphs):
        name = font.get_glyph_name(gid)
        font.set_size(upem, 72)
        font.load_glyph(gid, flags=NO_HINTING)
        units_x64 = path_of(font)
        font.set_size(12.0, 72.0)
        glyph = font.load_glyph(gid, flags=NO_HINTING)
        out["glyphs"].append({"gid": gid, "name": name, "units_x64": units_x64,
                              "bbox_12": [int(v) for v in glyph.bbox]})
    return out


def main():
    otf = HERE / "synthetic-cff.otf"
    cff_font(otf)
    ttc = HERE / "synthetic.ttc"
    collection = TTCollection()
    collection.fonts = [ttf_face("Plt Collection", "Regular", 400, 1.0),
                        ttf_face("Plt Collection", "Bold", 700, 1.25)]
    collection.save(str(ttc))
    manager = fm.FontManager()
    manager.addfont(str(ttc))
    prop = fm.FontProperties(family="Plt Collection", weight="bold")
    found = manager.findfont(prop, directory=str(HERE), fallback_to_default=False)
    data = {
        "cff": describe(otf, 0),
        "ttc": [describe(ttc, 0), describe(ttc, 1)],
        "ttc_bold_face_index": found.face_index,
        "ttc_bold_file": os.path.basename(found),
    }
    (HERE / "synthetic.json").write_text(json.dumps(data, separators=(",", ":")) + "\n")
    print("wrote", otf, ttc, HERE / "synthetic.json")


if __name__ == "__main__":
    main()
