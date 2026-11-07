"""Glyphs of OpenType fonts with CFF and CFF2 outlines and of a variable
TrueType font, as matplotlib's ``FT2Font`` loads them through FreeType at
each hinting mode: FreeType's CFF hinter (``LoadFlags.DEFAULT`` and
``NO_AUTOHINT`` on CFF fonts), its autohinter (``FORCE_AUTOHINT``, and
``DEFAULT`` on TrueType fonts without bytecode), and unhinted outlines.
Variable fonts are drawn at their default instance, as matplotlib draws
them.

Area ``font_formats``. A ``file`` is a fixture of
``crates/pltrs-font/tests/fixtures``, generated with fontTools by
``gen_synthetic_fonts.py`` and ``gen_variable_fonts.py``:
``synthetic-cff.otf`` (name-keyed CFF with subroutines, hints, flex and
``seac``), ``synthetic-var.otf`` (CFF2 with blends in its charstrings and
Private DICT, hint masks) and ``synthetic-var.ttf`` (``gvar``, ``HVAR``,
``MVAR``, ``avar``).

Ops
---
``ft2font.FT2Font.load_glyph``  {file, size, dpi, glyph, flags} -> ``{"advance",
    "linear", "path"}``: ``font = FT2Font(path)``, ``font.set_size(size,
    dpi)``, ``g = font.load_glyph(glyph, flags)`` with ``flags`` the
    ``LoadFlags`` named (``NO_HINTING``, ``DEFAULT``, ``NO_AUTOHINT`` or
    ``FORCE_AUTOHINT``); ``g.horiAdvance`` (null when unhinted),
    ``g.linearHoriAdvance`` and ``font.get_path()`` as ``[code, x * 64, y *
    64]`` (rounded).
``ft2font.FT2Font.metrics``  {file} -> ``[num_glyphs, units_per_EM,
    ascender, descender, height, family_name, style_name,
    postscript_name]``.
"""

from pathlib import Path

from matplotlib.ft2font import FT2Font, LoadFlags

from spec_support import ProbeSet

probes = ProbeSet("font_formats")

FIXTURES = Path(__file__).resolve().parents[3] / "crates/pltrs-font/tests/fixtures"
FILES = ["synthetic-cff.otf", "synthetic-var.otf", "synthetic-var.ttf"]
FLAGS = ["NO_HINTING", "DEFAULT", "NO_AUTOHINT", "FORCE_AUTOHINT"]
SIZES = [(6, 72), (9, 100), (12, 100), (17, 150), (31, 72)]


def _load_glyph(file, size, dpi, glyph, flags):
    font = FT2Font(str(FIXTURES / file))
    font.set_size(size, dpi)
    loaded = font.load_glyph(glyph, getattr(LoadFlags, flags))
    vertices, codes = font.get_path()
    path = [[int(code), round(x * 64), round(y * 64)]
            for (x, y), code in zip(vertices.tolist(), codes.tolist())]
    advance = None if flags == "NO_HINTING" else loaded.horiAdvance
    return {"advance": advance, "linear": loaded.linearHoriAdvance, "path": path}


def _metrics(file):
    font = FT2Font(str(FIXTURES / file))
    return [font.num_glyphs, font.units_per_EM, font.ascender, font.descender,
            font.height, font.family_name, font.style_name, font.postscript_name]


def test_glyphs():
    for file in FILES:
        glyphs = FT2Font(str(FIXTURES / file)).num_glyphs
        for flags in FLAGS:
            for size, dpi in SIZES:
                for glyph in range(glyphs):
                    probes.probe("ft2font.FT2Font.load_glyph", _load_glyph, file=file,
                                 size=size, dpi=dpi, glyph=glyph, flags=flags)


def test_metrics():
    for file in FILES:
        probes.probe("ft2font.FT2Font.metrics", _metrics, file=file)
