r"""matplotlib's own non-image tests of texts and mathtext
(``matplotlib/tests/test_text.py`` and ``matplotlib/tests/test_mathtext.py``)
that the ``text_behaviour``, ``text_metrics``, ``text_shaping`` and
``mathtext`` areas leave out.

Area ``mpl_text``. Every op runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``).

Ops
---
``FontProperties.get_size_in_points``  {size, base, later} -> the size of
    ``FontProperties(size=size)`` made while ``font.size`` is ``base``,
    read once ``font.size`` is ``later``: a relative size is fixed when the
    properties are made (``test_text_size_binding``).
``Figure.stale``  {} -> the figure's staleness along ``test_text_stale``:
    after drawing ``plt.subplots(1, 2)``, after ``ax1.text``, after
    ``ax2.annotate`` and after drawing again.
``Text.get_math_fontfamily``  {font, math_fontfamily?} -> the math fonts
    of ``fig.text(0.1, 0.1, r"abc$abc\alpha$", font=font)`` under
    ``mathtext.fontset = "cm"``, with ``math_fontfamily`` given too when
    present: the font's description leaves the math fonts to
    ``mathtext.fontset`` (``test_default_math_fontfamily``) and an explicit
    ``math_fontfamily`` wins (``test_argument_order``, whose four keyword
    orders are one in pltrs, which sets the math fonts after the font).
``mathtext.math_to_image``  {s, color?} -> ``[depth, height, width]``: the
    depth ``math_to_image(s, buffer, color=color)`` returns and the size of
    the PNG it writes, a figure as large as the expression at 72 dpi saved
    at ``savefig.dpi`` (``test_math_to_image``).

The tests that only check that drawing works are cases of
``mpl_tests_text`` (``test_mathtext_cmr10_minus_sign``).

Covered elsewhere: ``test_fontinfo`` (the ``mpl_ft2font`` area reads
``get_sfnt_table``), ``test_mathtext_fallback_valid`` and
``test_mathtext_fallback_invalid`` (the ``rcsetup`` area validates
``mathtext.fallback``) and ``test_get_unicode_index_exception`` (a private
helper; unknown symbols are among the parse errors of the ``mathtext``
area). Left out, besides the tests of Python machinery the
``text_behaviour`` area lists: the usetex tests (no TeX) and
``test_metrics_cache`` and ``test_metrics_cache2`` (a Python cache).
"""

import io

import matplotlib as mpl
import matplotlib.pyplot as plt
from matplotlib import mathtext
from matplotlib.font_manager import FontProperties

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_text")
probe = prober(probes)


def _size_in_points(size, base, later):
    mpl.rcParams["font.size"] = base
    fp = FontProperties(size=size)
    mpl.rcParams["font.size"] = later
    return fp.get_size_in_points()


def test_text_size_binding():
    for size in ["large", "x-small", 14]:
        probe("FontProperties.get_size_in_points", _size_in_points, size=size, base=10,
              later=100)


def _stale():
    fig, (ax1, ax2) = plt.subplots(1, 2)
    fig.draw_without_rendering()
    out = [fig.stale]
    ax1.text(.5, .5, "aardvark")
    out.append(fig.stale)
    ax2.annotate("aardvark", xy=[.5, .5])
    out.append(fig.stale)
    fig.draw_without_rendering()
    return out + [fig.stale]


def test_text_stale():
    assert probe("Figure.stale", _stale) == [False, True, True, False]


def _math_fontfamily(font, math_fontfamily=None):
    fig = plt.figure()
    extra = {} if math_fontfamily is None else {"math_fontfamily": math_fontfamily}
    text = fig.text(0.1, 0.1, r"abc$abc\alpha$", font=font, **extra)
    return text.get_fontproperties().get_math_fontfamily()


def test_math_fontfamily():
    rc = {"mathtext.fontset": "cm"}
    for font in ["Arial", "DejaVu Serif:bold"]:
        probe("Text.get_math_fontfamily", _math_fontfamily, rc=rc, font=font)
        probe("Text.get_math_fontfamily", _math_fontfamily, rc=rc, font=font,
              math_fontfamily="dejavusans")


def _math_to_image(s, color=None):
    buffer = io.BytesIO()
    depth = mathtext.math_to_image(s, buffer, color=color)
    buffer.seek(0)
    return [depth, *plt.imread(buffer).shape[:2]]


def test_math_to_image():
    for s in ["$x^2$", r"$\frac{a}{b}\,y_i$"]:
        probe("mathtext.math_to_image", _math_to_image, s=s)
    probe("mathtext.math_to_image", _math_to_image, s="$x^2$", color="Maroon")
