"""
OpenType fonts with CFF and CFF2 outlines and variable TrueType fonts
through ``FT2Font``, in drawn text at each hinting mode and embedded as
Type 42 fonts in PDF and PostScript; AFM files of the system's font
directories in the AFM font list; and ``findfont``'s memo forgetting a
miss when ``addfont`` adds the family. The fonts are fixtures of
``pltrs-font``'s tests. The expected values were recorded from matplotlib
3.11.2 running the same scenarios.
"""

import io
import json
import os
import re
import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager
from pltrs.ft2font import FT2Font, LoadFlags

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "crates/pltrs-font/tests/fixtures"
CATALOGUE = "fontlist-v2.pltrs"


def scenario_load_glyph(file, flags):
    """Glyphs 2 to 5 of *file* at 12 points and 100 dpi, loaded with the
    ``LoadFlags`` named *flags*: hinted advance and bounding box."""
    font = FT2Font(str(FIXTURES / file))
    font.set_size(12, 100)
    glyphs = []
    for glyph in range(2, 6):
        loaded = font.load_glyph(glyph, getattr(LoadFlags, flags))
        glyphs.append([loaded.horiAdvance, list(loaded.bbox)])
    return glyphs


# The CFF hinter for DEFAULT and NO_AUTOHINT, the autohinter for
# FORCE_AUTOHINT.
LOAD_GLYPH = {
    "synthetic-cff.otf": {
        "DEFAULT": [[640, [21, 0, 619, 747]], [768, [32, -203, 715, 757]],
                    [704, [106, 0, 213, 405]], [640, [-481, 106, 693, 341]]],
        "NO_AUTOHINT": [[640, [21, 0, 619, 747]], [768, [32, -203, 715, 757]],
                        [704, [106, 0, 213, 405]], [640, [-481, 106, 693, 341]]],
        "FORCE_AUTOHINT": [[640, [21, 0, 619, 747]], [768, [64, -192, 736, 768]],
                           [704, [128, 0, 246, 405]], [640, [-470, 128, 704, 384]]],
    },
    "synthetic-var.otf": {
        "DEFAULT": [[640, [21, 0, 619, 746]], [640, [-257, 0, 768, 1130]],
                    [640, [64, 0, 576, 747]], [128, [0, 800, 128, 939]]],
        "NO_AUTOHINT": [[640, [21, 0, 619, 746]], [640, [-257, 0, 768, 1130]],
                        [640, [64, 0, 576, 747]], [128, [0, 800, 128, 939]]],
        "FORCE_AUTOHINT": [[640, [21, 0, 619, 736]], [640, [-256, 0, 768, 1142]],
                           [640, [64, 0, 576, 768]], [128, [0, 832, 128, 960]]],
    },
}


def scenario_set_text_autohinted():
    """``set_text`` of a CFF font with ``FORCE_AUTOHINT``: the glyphs'
    positions and the bitmap's size."""
    font = FT2Font(str(FIXTURES / "synthetic-cff.otf"))
    font.set_size(12, 100)
    positions = font.set_text("AOB", 0.0, flags=LoadFlags.FORCE_AUTOHINT)
    return [list(map(float, xy)) for xy in positions], list(font.get_width_height())


SET_TEXT_AUTOHINTED = ([[0.0, 0.0], [640.0, 0.0], [1408.0, 0.0]], [2112, 960])


def scenario_drawn_ink():
    """Text drawn in each fixture at each ``text.hinting``: the sum of its
    ink and the number of pixels it touches."""
    ink = {}
    for file in ["synthetic-cff.otf", "synthetic-var.otf", "synthetic-var.ttf"]:
        for hinting in ["none", "default", "native", "auto"]:
            with pltrs.rc_context({"text.hinting": hinting}):
                fig = plt.figure(figsize=(2, 0.5), dpi=100)
                prop = font_manager.FontProperties(fname=FIXTURES / file, size=14)
                fig.text(0.05, 0.3, "AOH", fontproperties=prop)
                fig.canvas.draw()
                pixels = 255 - np.asarray(fig.canvas.buffer_rgba())[..., 0].astype(int)
                ink[f"{file}:{hinting}"] = [int(pixels.sum()), int((pixels > 0).sum())]
                plt.close(fig)
    return ink


DRAWN_INK = {
    "synthetic-cff.otf:none": [49234, 248], "synthetic-cff.otf:default": [49272, 247],
    "synthetic-cff.otf:native": [49272, 247], "synthetic-cff.otf:auto": [49421, 245],
    "synthetic-var.otf:none": [66027, 347], "synthetic-var.otf:default": [67093, 355],
    "synthetic-var.otf:native": [67093, 355], "synthetic-var.otf:auto": [66027, 348],
    "synthetic-var.ttf:none": [31131, 229], "synthetic-var.ttf:default": [30873, 216],
    "synthetic-var.ttf:native": [31122, 225], "synthetic-var.ttf:auto": [30873, 216],
}


def scenario_type42():
    """Text in each fixture saved as PostScript and PDF with Type 42 fonts:
    the PostScript font's glyph names, bounding box, full name and sfnt
    version, the version of the PDF's font program, and whether it is a
    ``CIDFontType2``."""
    facts = {}
    for file in ["synthetic-cff.otf", "synthetic-var.otf", "synthetic-var.ttf"]:
        prop = font_manager.FontProperties(fname=FIXTURES / file, size=12)
        with pltrs.rc_context({"pdf.fonttype": 42, "ps.fonttype": 42, "pdf.compression": 0}):
            fig = plt.figure(figsize=(2, 1))
            fig.text(0.1, 0.5, "AOH Á", fontproperties=prop)
            ps, pdf = io.BytesIO(), io.BytesIO()
            fig.savefig(ps, format="ps")
            fig.savefig(pdf, format="pdf")
            plt.close(fig)
        ps = ps.getvalue().decode("latin-1")
        start = ps.index("/FontType 42")
        font = ps[start:ps.index("definefont", start)]
        program = pdf.getvalue()
        stream = program.index(b"stream", program.index(b"/Length1")) + 7
        facts[file] = {
            "names": re.findall(r"^/(\S+) \d+ def$", font[font.index("/CharStrings"):], re.M),
            "bbox": re.search(r"/FontBBox \[([^\]]*)\]", font).group(1),
            "full": re.search(r"/FullName \(([^)]*)\)", font).group(1),
            "ps_version": font[font.index("/sfnts[<") + 8:][:8],
            "pdf_version": program[stream:stream + 4].hex(),
            "cid_type2": b"/CIDFontType2" in program,
        }
    return facts


# The CFF font's subset keeps the glyphs its `seac` draws (A and acute);
# the TrueType font's box is fontTools', its composite as stored.
TYPE42 = {
    "synthetic-cff.otf": {
        "names": [".notdef", "space", "A", "O", "acute", "Aacute", "hinted"],
        "bbox": "-450 -190 1040 1090", "full": "None",
        "ps_version": "4f54544f", "pdf_version": "4f54544f",
        "cid_type2": True,
    },
    "synthetic-var.otf": {
        "names": [".notdef", "space", "A", "O", "H", "Aacute"],
        "bbox": "-240 -10 720 1050", "full": "None",
        "ps_version": "4f54544f", "pdf_version": "4f54544f",
        "cid_type2": True,
    },
    "synthetic-var.ttf": {
        "names": [".notdef", "space", "A", "O", "H", "acute", "Aacute"],
        "bbox": "0 -10 580 880", "full": "None",
        "ps_version": "00010000", "pdf_version": "00010000",
        "cid_type2": True,
    },
}


def scenario_addfont_after_miss():
    """``findfont`` of a family no font has, then of the same family once
    ``addfont`` has added it, and the width of text in the family each
    time (in a new figure, which matplotlib lays out anew)."""
    prop = font_manager.FontProperties(family="Plt Var CFF")
    found, widths = [], []
    for add in [False, True]:
        if add:
            font_manager.fontManager.addfont(FIXTURES / "synthetic-var.otf")
        found.append(os.path.basename(font_manager.findfont(prop)))
        fig = plt.figure(figsize=(2, 0.5), dpi=100)
        text = fig.text(0.05, 0.3, "AOH", fontproperties=prop, size=14)
        fig.canvas.draw()
        widths.append(round(text.get_window_extent().width, 3))
    return found, widths


ADDFONT_AFTER_MISS = (["DejaVuSans.ttf", "synthetic-var.otf"], [41.656, 36.0])

# What the system AFM fonts of ``$XDG_DATA_HOME/fonts`` become: the
# entries of the AFM font list from that directory, and what findfont
# finds in their family.
SYSTEM_AFM = """
import json, os
from pltrs import font_manager as fm
fonts = os.path.join(os.environ["XDG_DATA_HOME"], "fonts")
mine = sorted([os.path.basename(e.fname), e.name, e.style, e.variant,
               str(e.weight), e.stretch]
              for e in fm.fontManager.afmlist if e.fname.startswith(fonts))
found = [os.path.basename(fm.findfont(
             fm.FontProperties(family="Smoke Serif", weight=w, style=s),
             fontext="afm"))
         for w, s in [("bold", "italic"), ("normal", "normal"),
                      ("light", "oblique")]]
print(json.dumps([mine, found]))
"""

SYSTEM_AFM_RESULT = [
    [["smokeserif-bi.afm", "Smoke Serif", "italic", "normal", "bold", "normal"],
     ["smokeserif-cl.afm", "Smoke Serif", "normal", "normal", "light", "condensed"],
     ["smokeserif.AFM", "Smoke Serif", "normal", "normal", "roman", "normal"]],
    ["smokeserif-bi.afm", "smokeserif.AFM", "smokeserif-bi.afm"],
]


def _smoke_serif(directory, name, header):
    """A copy of the bundled Times-Roman AFM as *name* in *directory*,
    with the header lines *header* replaced."""
    source = ROOT / "crates/pltrs-font/fonts/afm/ptmr8a.afm"
    lines = []
    for line in source.read_text(encoding="latin-1").splitlines():
        key = line.split(" ", 1)[0]
        lines.append(f"{key} {header[key]}" if key in header else line)
    directory.mkdir(parents=True, exist_ok=True)
    (directory / name).write_text("\n".join(lines) + "\n", encoding="latin-1")


@pytest.mark.parametrize("file", sorted(LOAD_GLYPH))
@pytest.mark.parametrize("flags", ["DEFAULT", "NO_AUTOHINT", "FORCE_AUTOHINT"])
def test_load_glyph(file, flags):
    assert scenario_load_glyph(file, flags) == LOAD_GLYPH[file][flags]


def test_set_text_autohinted():
    assert scenario_set_text_autohinted() == SET_TEXT_AUTOHINTED


def test_drawn_ink():
    assert scenario_drawn_ink() == DRAWN_INK


def test_type42():
    assert scenario_type42() == TYPE42


def test_addfont_after_miss():
    manager = font_manager.fontManager
    before = len(manager.ttflist)
    manager._findfont_cached.cache_clear()
    try:
        assert scenario_addfont_after_miss() == ADDFONT_AFTER_MISS
    finally:
        for entry in manager.ttflist[before:]:
            manager.ttflist.remove(entry)
        manager._findfont_cached.cache_clear()


def test_system_afm_fonts(tmp_path):
    """Read from the directories and then from the catalogue."""
    fonts = tmp_path / "data" / "fonts"
    family = {"FamilyName": "Smoke Serif"}
    _smoke_serif(fonts / "afm", "smokeserif-bi.afm", {
        **family, "FontName": "SmokeSerif-BoldItalic",
        "FullName": "Smoke Serif Bold Italic", "Weight": "Bold",
        "ItalicAngle": "-15.5"})
    _smoke_serif(fonts, "smokeserif.AFM", {
        **family, "FontName": "SmokeSerif-Regular", "FullName": "Smoke Serif"})
    _smoke_serif(fonts, "smokeserif-cl.afm", {
        **family, "FontName": "SmokeSerif-CondensedLight",
        "FullName": "Smoke Serif Condensed Light", "Weight": "Light"})
    env = {key: value for key, value in os.environ.items()
           if key not in ("PLTRSCONFIGDIR", "MPLCONFIGDIR")}
    env.update(PLTRSCONFIGDIR=str(tmp_path / "config"),
               XDG_DATA_HOME=str(tmp_path / "data"))
    for _ in range(2):
        result = subprocess.run([sys.executable, "-c", SYSTEM_AFM], env=env,
                                capture_output=True, text=True, check=False)
        assert result.returncode == 0, result.stderr
        assert json.loads(result.stdout.splitlines()[-1]) == SYSTEM_AFM_RESULT
        assert (tmp_path / "config" / CATALOGUE).is_file()
