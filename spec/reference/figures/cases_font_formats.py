"""Text in OpenType fonts with CFF and CFF2 outlines and in a variable
TrueType font, at matplotlib's default ``text.hinting`` (FreeType's CFF
hinter for CFF fonts, its autohinter for the TrueType font, which has no
bytecode), under "force_autohint", and unhinted. Variable fonts are drawn
at their default instance.

The fonts are fixtures of ``crates/pltrs-font/tests/fixtures``, generated
with fontTools and added with ``fontManager.addfont``: "Plt Synthetic CFF"
(name-keyed CFF with subroutines, hints, flex and ``seac``), "Plt Var CFF"
(CFF2 with blends and hint masks) and "Plt Var" (``gvar``, ``HVAR``,
``MVAR``, ``avar``).
"""

from pathlib import Path

from matplotlib import font_manager

from spec_support import case

FIXTURES = Path(__file__).resolve().parents[3] / "crates/pltrs-font/tests/fixtures"
# Each family with the characters its font maps.
FONTS = [("synthetic-cff.otf", "Plt Synthetic CFF", "AOBFH CÁ´"),
         ("synthetic-var.otf", "Plt Var CFF", "AOH Á´"),
         ("synthetic-var.ttf", "Plt Var", "AOH Á´")]
SIZES = [6, 8, 10, 12, 15, 20, 28]


def _fonts(fig):
    """Each family at each size, a column per family."""
    for file, _, _ in FONTS:
        font_manager.fontManager.addfont(str(FIXTURES / file))
    for i, (_, family, chars) in enumerate(FONTS):
        for j, size in enumerate(SIZES):
            fig.text(0.02 + 0.33 * i, 0.95 - 0.13 * j, f"{chars}{chars}", family=family,
                     fontsize=size, va="top")


@case("font_formats/default_hinting", figsize=(6, 3.5), rc={"text.hinting": "default"})
def _(fig):
    _fonts(fig)


@case("font_formats/autohint", figsize=(6, 3.5), rc={"text.hinting": "force_autohint"})
def _(fig):
    _fonts(fig)


@case("font_formats/unhinted", figsize=(6, 3.5), dpi=150)
def _(fig):
    _fonts(fig)
