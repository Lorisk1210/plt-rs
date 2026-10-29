"""Build two small Arabic test fonts and record matplotlib's layouts of a
few strings with them (libraqm + HarfBuzz).

* `arabic-fallback.ttf` maps Arabic letters and their presentation forms
  but has no layout tables, so HarfBuzz synthesizes the positional forms
  and ligatures and stacks marks itself.
* `arabic-cursive.ttf` has `init`/`medi`/`fina`/`rlig` substitutions,
  right-to-left cursive attachment, mark attachment and kerning for
  Arabic, plus left-to-right cursive attachment for two Latin letters.

Every glyph is a rectangle with its own advance, so any wrong glyph choice
changes the widths. Run from the repository root with the project venv:

    MPL_IGNORE_SYSTEM_FONTS=1 MPLCONFIGDIR=$(mktemp -d) \\
        .venv/bin/python -I crates/pltrs-font/tests/fixtures/gen_arabic_fonts.py

Writes the fonts and `arabic.json` next to this script.
"""

import json
import pathlib
import unicodedata

from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from matplotlib.ft2font import FT2Font, LoadFlags

HERE = pathlib.Path(__file__).resolve().parent
NO_HINTING = LoadFlags.NO_HINTING
SIZES = [(12.0, 72.0), (10.0, 100.0)]

LETTERS = [0x0627, 0x0628, 0x062A, 0x062D, 0x0631, 0x0644, 0x0645, 0x0647, 0x064A]
MARKS = [0x064E, 0x0650, 0x0651]
# Ligatures HarfBuzz's fallback knows: lam-alef forms, lam-meem-hah, shadda-fatha.
LIGATURES = [0xFEFB, 0xFEFC, 0xFD88, 0xFC60]

STRINGS = [
    "بتب",
    "مرحبا",
    "لا لال",
    "لمح",
    "بَبَّ بِ",
    "يه ههه",
    "ab بب ba",
    "ببب abab",
]


def presentation_forms():
    """Presentation-form characters of LETTERS (any shape)."""
    forms = []
    for code in range(0xFB50, 0xFF00):
        decomposition = unicodedata.decomposition(chr(code)).split()
        if len(decomposition) == 2 and decomposition[0] in (
                "<initial>", "<medial>", "<final>", "<isolated>"):
            if int(decomposition[1], 16) in LETTERS:
                forms.append(code)
    return forms


def box(width, height):
    pen = TTGlyphPen(None)
    pen.moveTo((50, 0))
    pen.lineTo((50, height))
    pen.lineTo((width - 50, height))
    pen.lineTo((width - 50, 0))
    pen.closePath()
    return pen.glyph()


def build(path, codes, extra_names, features):
    """A font mapping `codes` (plus unmapped `extra_names`) to boxes of
    distinct advances; `features` is feature-file source (or None)."""
    names = [".notdef", "space"]
    cmap = {0x20: "space"}
    for code in codes:
        name = "u%04X" % code
        names.append(name)
        cmap[code] = name
    names += extra_names
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(names)
    fb.setupCharacterMap(cmap)
    glyphs, metrics = {}, {}
    for index, name in enumerate(names):
        mark = name in ("u%04X" % m for m in MARKS)
        width = 0 if mark else 300 + 37 * index
        glyphs[name] = box(max(width, 200), 300 if mark else 400 + 11 * index)
        metrics[name] = (width, 50)
    glyphs["space"] = TTGlyphPen(None).glyph()
    metrics["space"] = (250, 0)
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=900, descent=-300)
    fb.setupNameTable({"familyName": path.stem, "styleName": "Regular"})
    fb.setupOS2(sTypoAscender=900, sTypoDescender=-300, usWinAscent=900, usWinDescent=300)
    fb.setupPost()
    if features:
        addOpenTypeFeaturesFromString(fb.font, features)
    fb.save(str(path))


CURSIVE_FEATURES = """
languagesystem DFLT dflt;
languagesystem arab dflt;
languagesystem latn dflt;

@LETTERS = [u0628 u062A u0645 u062D];
@INIT = [beh.init teh.init meem.init hah.init];
@MEDI = [beh.medi teh.medi meem.medi hah.medi];
@FINA = [beh.fina teh.fina meem.fina hah.fina];

feature init { script arab; sub @LETTERS by @INIT; } init;
feature medi { script arab; sub @LETTERS by @MEDI; } medi;
feature fina { script arab; sub @LETTERS by @FINA; } fina;
feature rlig { script arab; sub u0644 u0627 by lamalef; } rlig;

markClass [u064E u0650 u0651] <anchor 100 350> @MARK;

feature curs {
    script arab;
    lookup arabic_cursive {
        lookupflag RightToLeft IgnoreMarks;
        pos cursive @INIT <anchor NULL> <anchor 60 300>;
        pos cursive @MEDI <anchor 700 260> <anchor 40 220>;
        pos cursive @FINA <anchor 650 180> <anchor NULL>;
    } arabic_cursive;
    script latn;
    lookup latin_cursive {
        pos cursive u0061 <anchor 20 0> <anchor 380 120>;
        pos cursive u0062 <anchor 30 -60> <anchor 400 40>;
    } latin_cursive;
} curs;

feature mark {
    script arab;
    pos base [@LETTERS @INIT @MEDI @FINA u0627 lamalef] <anchor 200 600> mark @MARK;
} mark;

feature kern {
    script arab;
    pos u0627 u0628 -40;
    pos lamalef u0628 25;
} kern;
"""


def layouts(path):
    font = FT2Font(str(path))
    cases = []
    for size, dpi in SIZES:
        font.set_size(size, dpi)
        for text in STRINGS:
            items = [[int(item.glyph_index), round(item.x * 64), round(item.y * 64)]
                     for item in font._layout(text, NO_HINTING)]
            font.set_text(text, 0.0, flags=NO_HINTING)
            width, height = font.get_width_height()
            cases.append({"size": size, "dpi": dpi, "text": text, "glyphs": items,
                          "extents": [int(width), int(height), int(font.get_descent())]})
    return cases


def main():
    fallback = HERE / "arabic-fallback.ttf"
    build(fallback, LETTERS + MARKS + presentation_forms() + LIGATURES + [ord("a"), ord("b")],
          [], None)
    cursive = HERE / "arabic-cursive.ttf"
    shaped = []
    for name in ("beh", "teh", "meem", "hah"):
        shaped += [name + ".init", name + ".medi", name + ".fina"]
    build(cursive, LETTERS + MARKS + [ord("a"), ord("b")], shaped + ["lamalef"],
          CURSIVE_FEATURES)
    data = {"fallback": layouts(fallback), "cursive": layouts(cursive)}
    (HERE / "arabic.json").write_text(json.dumps(data, ensure_ascii=True, separators=(",", ":")) + "\n")
    print("wrote", fallback, cursive, HERE / "arabic.json")


if __name__ == "__main__":
    main()
