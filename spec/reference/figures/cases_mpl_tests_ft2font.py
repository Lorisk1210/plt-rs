"""matplotlib's own tests of ``matplotlib.ft2font`` (``tests/test_ft2font.py``)
that draw: ``test_ft2font_drawing`` renders a 12 px ``M`` of DejaVu Sans,
autohinted, into a 10 x 11 bitmap without anti-aliasing.

The case draws the same glyph as figure text on a canvas of that size at
72 dpi, under ``text.hinting = force_autohint`` (``FT2Font``'s default
``FORCE_AUTOHINT``) and ``text.antialiased = False``.
"""

from spec_support import case

GROUP = "mpl_tests_ft2font"


@case(f"{GROUP}/drawing", figsize=(10 / 72, 11 / 72), dpi=72,
      rc={"text.hinting": "force_autohint", "text.antialiased": False})
def _(fig):
    fig.text(0, 1 / 11, "M", fontsize=12)
