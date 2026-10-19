"""matplotlib's own image tests of ``test_lines.py`` and ``test_path.py``:
dashes, draw styles, marker fill styles and ``markevery``, gap colours,
line width scaling, polygon clipping, log transforms through zero,
NaN-isolated points and the xkcd sketch style. (``test_arrow_contains_point``
is left out: it hit-tests arrows before their first draw, when matplotlib
lays them out with ``_dpi_cor`` 1, and points exactly on their shafts flip.)

Each case is named after its test. ``@check_figures_equal`` tests
contribute their ``fig_test`` figure. Tests without a style of their own
are drawn as matplotlib's test fixture draws them: classic with the test
patch, on classic's 8 × 6 inch figure. ``remove_text=True`` blanks titles and tick labels after the
build. Random samples come from ``spec_support.rng``.
"""

import itertools

import matplotlib
import matplotlib.pyplot as plt
import matplotlib.style
import numpy as np
from cycler import cycler
from matplotlib import lines as mlines
from matplotlib import transforms as mtransforms
from matplotlib.patches import Polygon
from matplotlib.ticker import NullFormatter

from spec_support import case, rng

CLASSIC_TEST = {**matplotlib.style.library["classic"],
                **matplotlib.style.library["_classic_test_patch"]}
#: The canvas of the classic test fixture's figures.
FIXTURE = (8, 6)


def _xkcd_rc():
    """The parameters ``plt.xkcd()`` changes on top of the classic test style."""
    with matplotlib.rc_context(CLASSIC_TEST):
        before = dict(matplotlib.rcParams)
        with plt.xkcd():
            after = dict(matplotlib.rcParams)
    return {**CLASSIC_TEST, **{k: v for k, v in after.items() if before[k] != v}}


XKCD = _xkcd_rc()


def _remove_text(fig):
    """``remove_ticks_and_titles``: no titles, no tick labels."""
    fig.suptitle("")
    for ax in fig.axes:
        ax.set_title("")
        for axis in (ax.xaxis, ax.yaxis):
            axis.set_major_formatter(NullFormatter())
            axis.set_minor_formatter(NullFormatter())


# --- test_lines.py -------------------------------------------------------


@case("mpl_lines/line_dashes", figsize=FIXTURE, rc=CLASSIC_TEST)
def _(fig):
    ax = fig.subplots()
    ax.plot(range(10), linestyle=(0, (3, 3)), lw=5)
    _remove_text(fig)


@case("mpl_lines/drawstyle_variants", figsize=FIXTURE, rc=CLASSIC_TEST)
def _(fig):
    axs = fig.subplots(6)
    for ax, ds in zip(axs.flat, ["default", "steps-mid", "steps-pre", "steps-post", "steps",
                                 None]):
        ax.plot(range(2000), drawstyle=ds)
        ax.set(xlim=(0, 2), ylim=(0, 2))
    _remove_text(fig)


@case("mpl_lines/no_subslice_with_transform", figsize=FIXTURE, rc=CLASSIC_TEST)
def _(fig):
    ax = fig.add_subplot()
    x = np.arange(2000)
    ax.plot(x, x, transform=mtransforms.Affine2D().translate(2000.0, 0.0) + ax.transData)


@case("mpl_lines/line_collection_dashes")
def _(fig):
    ax = fig.subplots()
    ax.contour(rng(0).normal(600).reshape(20, 30), linestyles=[(0, (3, 3))])
    _remove_text(fig)


@case("mpl_lines/marker_fill_styles", figsize=FIXTURE, rc=CLASSIC_TEST)
def _(fig):
    colors = itertools.cycle([[0, 0, 1], "g", "#ff0000", "c", "m", "y", np.array([0, 0, 0])])
    y = np.array([1, 1])
    x = np.array([0, 9])
    ax = fig.subplots()
    for j, marker in enumerate("ov^<>8sp*hHDdPX"):
        for i, fs in enumerate(mlines.Line2D.fillStyles):
            color = next(colors)
            ax.plot(j * 10 + x, y + i + .5 * (j % 2), marker=marker, markersize=20,
                    markerfacecoloralt="lightgreen", fillstyle=fs, label=fs, linewidth=5,
                    color=color, markeredgecolor=color, markeredgewidth=2)
    ax.set_ylim(0, 7.5)
    ax.set_xlim(-5, 155)
    _remove_text(fig)


@case("mpl_lines/scaled_lines")
def _(fig):
    th = np.linspace(0, 32)
    ax = fig.subplots()
    cy = cycler(matplotlib.rcParams["axes.prop_cycle"])
    for j, (ls, sty) in enumerate(zip(["dashed", "dotted", "dashdot"], cy)):
        for lw in np.linspace(.5, 10, 10):
            ax.plot(th, j * np.ones(50) + .1 * lw, linestyle=ls, lw=lw, **sty)


@case("mpl_lines/step_markers", figsize=FIXTURE, rc=CLASSIC_TEST)
def _(fig):
    fig.subplots().step([0, 1], "-o")


#: ``test_markevery``'s data and every kind of ``markevery`` it tries.
MARKEVERY_X = np.linspace(0, 1, 14)
MARKEVERY_Y = rng(42).uniform(14)
MARKEVERY_CASES = [None, 4, (2, 5), [1, 5, 11], [0, -1], slice(5, 10, 2),
                   np.arange(14)[MARKEVERY_Y > 0.5], 0.3, (0.3, 0.4)]


@case("mpl_lines/markevery_figure", figsize=FIXTURE, rc=CLASSIC_TEST)
def _(fig):
    for markevery in MARKEVERY_CASES[:-2]:
        fig.add_artist(mlines.Line2D(MARKEVERY_X, MARKEVERY_Y, marker="o", markevery=markevery))


@case("mpl_lines/markevery_axes", figsize=FIXTURE, rc=CLASSIC_TEST)
def _(fig):
    for ax, markevery in zip(fig.subplots(3, 3).flat, MARKEVERY_CASES):
        ax.plot(MARKEVERY_X, MARKEVERY_Y, "-gD", markevery=markevery)


@case("mpl_lines/striped_line")
def _(fig):
    r = rng(19680801)
    ax = fig.subplots()
    ax.plot(r.uniform(12), color="orange", gapcolor="blue", linestyle="--", lw=5,
            label="blue in orange")
    ax.plot(r.uniform(12), color="red", gapcolor="black", linestyle=(0, (2, 5, 4, 2)), lw=5,
            label="black in red", alpha=0.5)
    ax.legend(handlelength=5)
    _remove_text(fig)


@case("mpl_lines/odd_dashes", figsize=FIXTURE, rc=CLASSIC_TEST)
def _(fig):
    fig.add_subplot().plot([1, 2], dashes=[1, 2, 3])


#: ``test_markevery_prop_cycle``'s ``markevery`` values.
CYCLED_MARKEVERY = [None, 8, (30, 8), [16, 24, 30], [0, -1], slice(100, 200, 3), 0.1, 0.3,
                    1.5, (0.0, 0.1), (0.45, 0.1)]


@case("mpl_lines/markevery_prop_cycle", figsize=FIXTURE,
      rc={**CLASSIC_TEST, "axes.prop_cycle": cycler(
          markevery=CYCLED_MARKEVERY,
          color=matplotlib.colormaps["jet"](np.linspace(0.2, 0.8, len(CYCLED_MARKEVERY))))})
def _(fig):
    x = np.linspace(-1, 1)
    ax = fig.add_subplot()
    for i in range(len(CYCLED_MARKEVERY)):
        ax.plot(5 * x**2 - i, "o-")


# --- test_path.py --------------------------------------------------------


@case("mpl_lines/path_clipping", figsize=(6.0, 6.2), rc=CLASSIC_TEST)
def _(fig):
    for i, xy in enumerate([
            [(200, 200), (200, 350), (400, 350), (400, 200)],
            [(200, 200), (200, 350), (400, 350), (400, 100)],
            [(200, 100), (200, 350), (400, 350), (400, 100)],
            [(200, 100), (200, 415), (400, 350), (400, 100)],
            [(200, 100), (200, 415), (400, 415), (400, 100)],
            [(200, 415), (400, 415), (400, 100), (200, 100)],
            [(400, 415), (400, 100), (200, 100), (200, 415)]]):
        ax = fig.add_subplot(4, 2, i + 1)
        ax.set_xlim(0, 640)
        ax.set_ylim(140, 400)
        ax.add_patch(Polygon(xy, facecolor="none", edgecolor="red", closed=True))
    _remove_text(fig)


@case("mpl_lines/semi_log_with_zero")
def _(fig):
    x = np.arange(-10, 10)
    y = (1.0 - 1.0 / (x**2 + 1))**20
    ax = fig.subplots()
    ax.semilogy(x, y, "-o", lw=15, markeredgecolor="k")
    ax.set_ylim(1e-7, 1)
    ax.grid(True)


@case("mpl_lines/xkcd", figsize=FIXTURE, rc=XKCD)
def _(fig):
    x = np.linspace(0, 2 * np.pi, 100)
    fig.subplots().plot(x, np.sin(x))
    _remove_text(fig)


@case("mpl_lines/xkcd_marker", figsize=FIXTURE, rc=XKCD)
def _(fig):
    x = np.linspace(0, 5, 8)
    ax = fig.subplots()
    ax.plot(x, x, "+", ms=10)
    ax.plot(x, 5 - x, "o", ms=10)
    ax.plot(x, 2.5 * np.ones(8), "^", ms=10)
    _remove_text(fig)


@case("mpl_lines/marker_paths", figsize=FIXTURE, rc=CLASSIC_TEST)
def _(fig):
    n = 7
    ax = fig.subplots()
    ax.errorbar(np.arange(n), np.ones(n) + 4, np.ones(n))
    ax.set_xlim(-1, n)
    ax.set_ylim(-1, 7)
    _remove_text(fig)


@case("mpl_lines/nan_path")
def _(fig):
    ax = fig.subplots()
    ax.plot([0, np.nan, 2, np.nan, 4, 5, 6], "-o")
    ax.plot([np.nan, 7, np.nan, 9, 10, np.nan, 12], "-o")
    _remove_text(fig)
