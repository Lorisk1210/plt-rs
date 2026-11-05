"""
What testing on macOS brought to light, checked on every system: text in
fonts with Apple Advanced Typography tables (the fonts of macOS) laid out
as matplotlib's HarfBuzz lays it out, `is_opentype_cff_font`'s
deprecation (macOS has OpenType CFF fonts to call it on), and the locale
formatting `ScalarFormatter` offers (matplotlib's suite calls it where a
German locale exists), and the Agg renderer's ``_renderer`` (the macOS
backend's suite reads it). Expected values come from matplotlib 3.11.
"""

import pathlib

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager, ticker
from pltrs.backends.backend_agg import RendererAgg
from pltrs.ft2font import FT2Font, LoadFlags

FIXTURES = pathlib.Path(__file__).resolve().parents[2] / "crates/pltrs-font/tests/fixtures"


@pytest.mark.parametrize("font, text, features, expected", [
    # morx: a ligature, a rearrangement, an insertion, a contextual form.
    ("morx", "fig rsk xy", None,
     [(29, 0, 0), (8, 743, 0), (1, 1115, 0), (16, 1307, 0), (15, 1820, 0),
      (10, 2315, 0), (33, 2722, 0), (1, 3535, 0), (32, 3727, 0), (19, 4523, 0)]),
    # The liga feature switches the morx chain's ligatures off.
    ("morx", "fi fl ffi", ["-liga"],
     [(7, 0, 0), (9, 354, 0), (1, 743, 0), (7, 935, 0), (11, 1289, 0),
      (1, 1714, 0), (7, 1906, 0), (7, 2260, 0), (9, 2614, 0)]),
    # Apple kern: a cross-stream state machine raising what follows.
    ("kern", "mn amn", None,
     [(12, 0, 92), (13, 442, 92), (1, 902, 92), (2, 1094, 92), (12, 1360, 184),
      (13, 1802, 184)]),
    ("kerx", "AVATAR To", None,
     [(20, 0, 0), (25, 530, 0), (20, 1202, 0), (24, 1786, 0), (20, 2440, 0),
      (23, 3024, 0), (1, 3661, 0), (24, 3853, 0), (14, 4469, 0)]),
    # trak: tracking once per grapheme, the mark stacked on its base.
    ("trak", "a\N{COMBINING ACUTE ACCENT}b", None, [(2, 0, 0), (28, 94, 432), (3, 293, 0)]),
])
def test_apple_advanced_typography_lays_out_as_harfbuzz(font, text, features, expected):
    ft = FT2Font(str(FIXTURES / f"aat-{font}.ttf"))
    ft.set_size(12, 72)
    items = ft._layout(text, LoadFlags.NO_HINTING, features=features)
    assert [(item.glyph_index, round(item.x * 64), round(item.y * 64))
            for item in items] == expected


def test_is_opentype_cff_font_is_deprecated():
    with pytest.warns(pltrs.MatplotlibDeprecationWarning,
                      match="is_opentype_cff_font function was deprecated in Matplotlib 3.11"):
        assert font_manager.is_opentype_cff_font(str(FIXTURES / "synthetic-cff.otf"))
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        assert not font_manager.is_opentype_cff_font(str(FIXTURES / "aat-morx.ttf"))


def test_scalar_formatter_formats_with_minus_and_locale():
    formatter = ticker.ScalarFormatter(useMathText=True, useLocale=False)
    assert formatter._format_maybe_minus_and_locale(
        "$\\mathdefault{%1.1f}$", -0.5) == "$\\mathdefault{\N{MINUS SIGN}0.5}$"
    formatter = ticker.ScalarFormatter(useMathText=False, useLocale=True)
    assert formatter._format_maybe_minus_and_locale("%1.2f", 1234.5) == "1234.50"
    formatter = ticker.ScalarFormatter(useMathText=True, useLocale=True)
    assert formatter._format_maybe_minus_and_locale(
        ",$\\mathdefault{,%1.1f},$", 0.5) == ",$\\mathdefault{,0.5},$"


def test_agg_renderers_expose_their_pixels():
    fig = plt.figure(figsize=(2, 1), dpi=100)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    assert type(renderer._renderer).__name__ == "RendererAgg"
    pixels = np.asarray(renderer._renderer)
    assert (pixels.shape, pixels.dtype) == ((100, 200, 4), np.uint8)
    assert (pixels == np.asarray(fig.canvas.buffer_rgba())).all()
    assert not memoryview(renderer._renderer).readonly
    undrawn = plt.figure(figsize=(1, 1), dpi=50)
    undrawn.draw_without_rendering()
    blank = np.asarray(undrawn.canvas.get_renderer()._renderer)
    assert blank.shape == (50, 50, 4)
    assert (blank == [255, 255, 255, 0]).all()
    assert (np.asarray(undrawn.canvas.buffer_rgba()) == blank).all()
    assert np.asarray(RendererAgg(10, 5, 72)._renderer).shape == (5, 10, 4)
    plt.close("all")
