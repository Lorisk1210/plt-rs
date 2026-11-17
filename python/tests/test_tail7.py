"""
What task TAIL7 found pltrs doing otherwise than matplotlib 3.11 in fonts
and text paths (USR2's measure): what an `FT2Font` reads back after
loading glyphs, the font list of matplotlib's look, the font lookups of
the PDF and PostScript core fonts, where a missing glyph is warned about,
and the path of a mathtext marker. Each expected value is recorded from
matplotlib.
"""

import io
import logging
import os
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager
from pltrs.ft2font import FT2Font, LoadFlags
from pltrs.markers import MarkerStyle
from pltrs.textpath import TextPath, TextToPath

TTF = os.path.join(pltrs.get_data_path(), "fonts", "ttf")


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def sans(size=None):
    font = FT2Font(os.path.join(TTF, "DejaVuSans.ttf"))
    if size is not None:
        font.set_size(size, 72)
    return font


# -- FT2Font -----------------------------------------------------------------------

def test_the_path_after_set_text_is_the_last_glyph_laid_out():
    font = sans(12)
    font.set_text("AV", 0.0, flags=LoadFlags.NO_HINTING)
    verts, codes = font.get_path()
    assert verts.shape == (9, 2) and codes.shape == (9,)
    assert np.allclose(verts[:2], [[3.438, 0.0], [0.094, 8.75]], atol=1e-3)


def test_a_rotated_text_is_bounded_by_its_turned_outlines():
    font = sans(12)
    font.set_text("Tg.", 30.0)
    assert (font.get_width_height(), font.get_descent()) == ((998, 744), -91)


@pytest.mark.parametrize("flags, xys, width", [
    (LoadFlags.DEFAULT, [0, 1251, 2551], 3202),
    (LoadFlags.NO_HINTING, [0, 0, 0], 0),
    (LoadFlags.TARGET_LIGHT, [0, 0, 0], 0),
    (LoadFlags.NO_SCALE, [0, 1, 2], 3),
])
def test_a_text_set_before_any_size_advances_as_the_load_flags_say(flags, xys, width):
    font = sans()
    laid_out = font.set_text("Tg.", 0.0, flags=flags)
    assert laid_out[:, 0].tolist() == xys
    assert font.get_width_height()[0] == width


def test_a_missing_character_is_passed_over_and_a_line_ends_the_layout():
    font = sans(12)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        assert font.set_text("aあb", 0.0).tolist() == [[0.0, 0.0], [448.0, 0.0]]
        assert font.get_num_glyphs() == 2
        assert font.set_text("a\nbc", 0.0).tolist() == [[0.0, 0.0]]
        assert font.get_width_height() == (448, 448)
        assert font.set_text("a\r\nbc", 0.0).tolist() == [[0.0, 0.0]]


# -- the font list ------------------------------------------------------------------

_FONT_LIST = """
import collections, os, sys
import pltrs
from pltrs import font_manager
ttf = os.path.join(pltrs.get_data_path(), "fonts", "ttf")
bundled = [entry for entry in font_manager.fontManager.ttflist
           if os.path.dirname(entry.fname) == ttf]
names = collections.Counter(entry.name for entry in bundled)
print(len(bundled), names["Inter"], names["DejaVu Sans"])
"""


def test_the_font_list_of_matplotlibs_look_holds_matplotlibs_bundled_fonts_alone():
    # In a process of matplotlib's look (`PLTRS_LOOK`, read on import).
    import subprocess
    import sys
    env = dict(os.environ, PLTRS_LOOK="matplotlib")
    out = subprocess.run([sys.executable, "-c", _FONT_LIST], env=env, check=True,
                         capture_output=True, text=True).stdout
    assert out.split() == ["40", "0", "4"]


# -- the core fonts ------------------------------------------------------------------

@pytest.fixture
def core_fonts_only():
    """The TrueType Helvetica and Times out of the font list, as on a
    system without them."""
    manager = font_manager.fontManager
    hidden = [entry for entry in manager.ttflist
              if entry.name.split()[0] in ("Helvetica", "Times")]
    for entry in hidden:
        manager.ttflist.remove(entry)
    yield
    for entry in hidden:
        manager.ttflist.append(entry)


class _Logged(logging.Handler):
    def __init__(self):
        super().__init__()
        self.messages = []

    def emit(self, record):
        self.messages.append(record.getMessage()[:40])


@pytest.mark.parametrize("fmt, rc, expected", [
    ("pdf", {"pdf.use14corefonts": True}, 2),
    ("ps", {"ps.useafm": True}, 1),
])
def test_the_core_fonts_are_looked_up_among_the_afm_fonts_alone(core_fonts_only, fmt, rc, expected):
    logged = _Logged()
    font_manager._log.addHandler(logged)
    try:
        with plt.rc_context({**rc, "font.family": "sans-serif",
                             "font.sans-serif": ["Helvetica"]}):
            fig = plt.figure(figsize=(2, 1))
            fig.text(0.1, 0.5, "Hello")
            fig.savefig(io.BytesIO(), format=fmt)
    finally:
        font_manager._log.removeHandler(logged)
    assert logged.messages == ["findfont: Failed to find font weight nor"] * expected


# -- missing glyphs -------------------------------------------------------------------

def caught(work):
    with warnings.catch_warnings(record=True) as warned:
        warnings.simplefilter("always")
        work()
    return [str(w.message) for w in warned]


def test_a_newline_is_warned_about_where_matplotlib_measures_it():
    newline = "Glyph 10 (\n) missing from font(s) DejaVu Sans."
    assert caught(lambda: TextPath((0, 0), "two\nlines", size=10)) == []
    assert caught(lambda: TextToPath().get_text_path(
        font_manager.FontProperties(size=12), "two\nlines")) == []
    assert caught(lambda: TextToPath().get_text_width_height_descent(
        "two\nlines", font_manager.FontProperties(size=12), ismath=False)) == [newline]
    assert caught(lambda: sans(12).set_text("a\tb", 0.0)) == [
        "Glyph 9 (\t) missing from font(s) DejaVu Sans."]


# -- a mathtext marker ---------------------------------------------------------------

def test_a_mathtext_marker_holds_its_text_path():
    path = MarkerStyle(r"$\heartsuit$").get_path()
    assert type(path) is TextPath and len(path.vertices) == 55
