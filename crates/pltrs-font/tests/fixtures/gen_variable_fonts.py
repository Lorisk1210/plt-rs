"""Build two small variable fonts, one with TrueType outlines (`gvar`,
`HVAR`, `MVAR`, `avar`) and one with CFF2 outlines (blends in its
charstrings and its Private DICT, two Font DICTs, hint masks), and record
what FreeType 2.14.3 makes of them at their default instance, their named
instances and other coordinates.

Run from the repository root with the project venv and FreeType built as
a shared library (see `ftref.py`):

    FREETYPE_LIB=.../libfreetype.so \\
        .venv/bin/python -I crates/pltrs-font/tests/fixtures/gen_variable_fonts.py

Writes `synthetic-var.ttf`, `synthetic-var.otf` and `synthetic-var.json`
next to this script.
"""

import json
import pathlib
import sys
import tempfile

from fontTools.designspaceLib import (AxisDescriptor, DesignSpaceDocument,
                                      InstanceDescriptor, SourceDescriptor)
from fontTools.fontBuilder import FontBuilder
from fontTools.misc.psCharStrings import T2CharString
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib.tables._g_l_y_f import Glyph, GlyphComponent
from fontTools import varLib

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ftref  # noqa: E402

NAMES = [".notdef", "space", "A", "O", "H", "acute", "Aacute"]
CMAP = {0x20: "space", 0x41: "A", 0x4F: "O", 0x48: "H", 0xB4: "acute", 0xC1: "Aacute"}
# Masters: (weight, width) -> stem thickness and glyph width factor.
MASTERS = [(100, 100), (400, 100), (900, 100), (400, 75)]
INSTANCES = [("Thin", 100, 100), ("Bold", 700, 100), ("Condensed", 400, 75)]
COORDS = [[650.0, 82.0], [250.0, 100.0], [1000.0, 50.0]]


def shape(weight, width):
    stem = 20 + weight // 10
    scale = width / 100
    return stem, scale


def quad_glyphs(weight, width):
    stem, scale = shape(weight, width)
    w = round(600 * scale)
    glyphs = {}
    pen = TTGlyphPen(None)
    pen.moveTo((50, 0)); pen.lineTo((50, 700)); pen.lineTo((450, 700)); pen.lineTo((450, 0))
    pen.closePath()
    glyphs[".notdef"] = pen.glyph()
    glyphs["space"] = TTGlyphPen(None).glyph()
    pen = TTGlyphPen(None)
    pen.moveTo((20, 0)); pen.lineTo((w // 2 - stem // 2, 700)); pen.lineTo((w // 2 + stem // 2, 700))
    pen.lineTo((w - 20, 0)); pen.lineTo((w - 20 - stem, 0)); pen.lineTo((w // 2, 600 - stem))
    pen.lineTo((20 + stem, 0)); pen.closePath()
    glyphs["A"] = pen.glyph()
    pen = TTGlyphPen(None)
    pen.moveTo((w // 2, -10)); pen.qCurveTo((w - 30, -10), (w - 30, 350)); pen.qCurveTo((w - 30, 710), (w // 2, 710))
    pen.qCurveTo((30, 710), (30, 350)); pen.qCurveTo((30, -10), (w // 2, -10)); pen.closePath()
    pen.moveTo((w // 2, stem - 10)); pen.qCurveTo((30 + stem, stem - 10), (30 + stem, 350))
    pen.qCurveTo((30 + stem, 710 - stem), (w // 2, 710 - stem)); pen.qCurveTo((w - 30 - stem, 710 - stem), (w - 30 - stem, 350))
    pen.qCurveTo((w - 30 - stem, stem - 10), (w // 2, stem - 10)); pen.closePath()
    glyphs["O"] = pen.glyph()
    pen = TTGlyphPen(None)
    pen.moveTo((60, 0)); pen.lineTo((60, 700)); pen.lineTo((60 + stem, 700)); pen.lineTo((60 + stem, 380))
    pen.lineTo((w - 60 - stem, 380)); pen.lineTo((w - 60 - stem, 700)); pen.lineTo((w - 60, 700))
    pen.lineTo((w - 60, 0)); pen.lineTo((w - 60 - stem, 0)); pen.lineTo((w - 60 - stem, 320 - stem // 4))
    pen.lineTo((60 + stem, 320 - stem // 4)); pen.lineTo((60 + stem, 0)); pen.closePath()
    glyphs["H"] = pen.glyph()
    pen = TTGlyphPen(None)
    pen.moveTo((0, 750)); pen.lineTo((stem, 750)); pen.lineTo((120, 880)); pen.lineTo((120 - stem, 880))
    pen.closePath()
    glyphs["acute"] = pen.glyph()
    composite = Glyph()
    composite.numberOfContours = -1
    base = GlyphComponent()
    base.glyphName, base.x, base.y, base.flags = "A", 0, 0, 0x0200
    accent = GlyphComponent()
    accent.glyphName, accent.x, accent.y, accent.flags = "acute", w // 2 - 60 + stem // 4, 0, 0
    composite.components = [base, accent]
    glyphs["Aacute"] = composite
    return glyphs, w


def cubic_programs(weight, width):
    """Type 2 programs of the same structure for every master, with hints
    and a hint mask, so that they merge into blends."""
    stem, scale = shape(weight, width)
    w = round(600 * scale)
    a = w // 2
    return {
        ".notdef": [50, 0, "rmoveto", 400, 0, "rlineto", 0, 700, "rlineto", -400, 0, "rlineto", "endchar"],
        "space": ["endchar"],
        "A": [0, stem, 600 - stem, stem, "hstemhm", 20, stem, w - 40 - 2 * stem, stem, "vstemhm",
              "hintmask", bytes([0b11110000]),
              20, 0, "rmoveto", a - 20 - stem // 2, 700, "rlineto", stem, 0, "rlineto",
              a - 20 - stem // 2, -700, "rlineto", -stem, 0, "rlineto", -(a - 20 - stem), 600 - stem, "rlineto",
              -(a - 20 - stem), -(600 - stem), "rlineto", "endchar"],
        "O": [-10, stem, 720 - 2 * stem, stem, "hstem",
              a, -10, "rmoveto", a - 30, 0, 0, 360, 0, 360, "rrcurveto",
              0, 360 - 20, -(a - 30), 0, -(a - 30), 0, "rrcurveto",
              -(a - 30), 0, 0, -360, 0, -360, "rrcurveto", 0, -(360 - 20), a - 30, 0, a - 30, 0, "rrcurveto",
              0, stem, "rmoveto", -(a - 30 - stem), 0, 0, 340 - stem, 0, 340 - stem, "rrcurveto",
              0, 340 - stem, a - 30 - stem, 0, a - 30 - stem, 0, "rrcurveto",
              a - 30 - stem, 0, 0, -(340 - stem), 0, -(340 - stem), "rrcurveto",
              0, -(340 - stem), -(a - 30 - stem), 0, -(a - 30 - stem), 0, "rrcurveto", "endchar"],
        "H": [0, 700, "hstem", 60, stem, w - 120 - 2 * stem, stem, "vstem",
              60, 0, "rmoveto", 0, 700, "rlineto", stem, 0, "rlineto", 0, -320, "rlineto",
              w - 120 - 2 * stem, 0, "rlineto", 0, 320, "rlineto", stem, 0, "rlineto", 0, -700, "rlineto",
              -stem, 0, "rlineto", 0, 320 - stem, "rlineto", -(w - 120 - 2 * stem), 0, "rlineto",
              0, -(320 - stem), "rlineto", "endchar"],
        "acute": [0, 750, "rmoveto", stem, 0, "rlineto", 120 - stem, 130, "rlineto", -stem, 0, "rlineto", "endchar"],
        "Aacute": [20, 0, "rmoveto", a - 20 - stem // 2, 700, "rlineto", stem, 0, "rlineto",
                   a - 20 - stem // 2, -700, "rlineto", "endchar"],
    }, w


def master(path, weight, width, cff):
    fb = FontBuilder(1000, isTTF=not cff)
    fb.setupGlyphOrder(NAMES)
    fb.setupCharacterMap(CMAP)
    if cff:
        programs, w = cubic_programs(weight, width)
        charstrings = {name: T2CharString(program=program) for name, program in programs.items()}
        stem = shape(weight, width)[0]
        private = {"BlueValues": [-10, 0, 700, 700 + stem // 10], "StdHW": stem, "StdVW": stem}
        fb.setupCFF("PltVarCFF", {"FullName": "Plt Var CFF"}, charstrings, private)
    else:
        glyphs, w = quad_glyphs(weight, width)
        fb.setupGlyf(glyphs)
    advances = {".notdef": 500, "space": 250 + weight // 10, "A": w, "O": w, "H": w,
                "acute": 140, "Aacute": w}
    fb.setupHorizontalMetrics({name: (advance, 0) for name, advance in advances.items()})
    ascent = 900 + weight // 20
    fb.setupHorizontalHeader(ascent=ascent, descent=-200)
    family = "Plt Var CFF" if cff else "Plt Var"
    fb.setupNameTable({"familyName": family, "styleName": "Regular"})
    fb.setupOS2(sTypoAscender=ascent, sTypoDescender=-200, usWinAscent=ascent, usWinDescent=200,
                sxHeight=500 + weight // 20)
    fb.setupPost()
    fb.save(str(path))


def build(cff, out):
    with tempfile.TemporaryDirectory() as tmp:
        doc = DesignSpaceDocument()
        for tag, name, minimum, default, maximum in [("wght", "Weight", 100, 400, 900),
                                                     ("wdth", "Width", 75, 100, 100)]:
            axis = AxisDescriptor()
            axis.tag, axis.name = tag, name
            axis.minimum, axis.default, axis.maximum = minimum, default, maximum
            if tag == "wght":
                axis.map = [(100, 100), (250, 330), (400, 400), (700, 760), (900, 900)]
            doc.addAxis(axis)
        for weight, width in MASTERS:
            path = pathlib.Path(tmp) / f"m{weight}-{width}.{'otf' if cff else 'ttf'}"
            master(path, weight, width, cff)
            source = SourceDescriptor()
            source.path = str(path)
            source.location = {"Weight": weight, "Width": width}
            doc.addSource(source)
        for style, weight, width in INSTANCES:
            instance = InstanceDescriptor()
            instance.styleName = style
            instance.familyName = "Plt Var CFF" if cff else "Plt Var"
            instance.location = {"Weight": weight, "Width": width}
            doc.addInstance(instance)
        font, _, _ = varLib.build(doc)
        font.save(str(out))


def record(path):
    """FreeType's view of each instance, each set on a face of its own (a
    face's state depends on the instances set before)."""
    instances = [("default", None)] + [(f"named {i}", i) for i in range(1, len(INSTANCES) + 1)]
    instances += [(f"design {coords}", coords) for coords in COORDS]
    out = []
    for label, which in instances:
        face = ftref.Face(path)
        if isinstance(which, int):
            face.set_named_instance(which)
        elif which is not None:
            face.set_design(which)
        face.set_size(15, 96)
        entry = {"label": label, "instance": which, "normalized": face.normalized(2),
                 "metrics": face.metrics(), "glyphs": []}
        for gid in range(len(NAMES)):
            entry["glyphs"].append({
                "gid": gid,
                "unscaled": face.load(gid, ftref.NO_SCALE),
                "scaled": face.load(gid, ftref.NO_HINTING),
                "hinted": face.load(gid, 0),
            })
        out.append(entry)
    return out


def main():
    ttf = HERE / "synthetic-var.ttf"
    otf = HERE / "synthetic-var.otf"
    build(False, ttf)
    build(True, otf)
    data = {"ttf": record(ttf), "otf": record(otf)}
    (HERE / "synthetic-var.json").write_text(json.dumps(data, separators=(",", ":")) + "\n")
    print("wrote", ttf, otf, HERE / "synthetic-var.json")


if __name__ == "__main__":
    main()
