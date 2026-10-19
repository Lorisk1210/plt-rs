"""matplotlib's own image tests from its smaller test modules
(``test_spines.py``, ``test_scale.py``, ``test_marker.py``,
``test_artist.py``, ``test_subplots.py`` and ``test_transforms.py``).

Each case is named after its test. ``@check_figures_equal`` tests
contribute their ``fig_test`` figure. Styles follow ``image_comparison``:
``_classic_test`` is the classic sheet with matplotlib's test patch, and
``remove_text=True`` blanks titles and tick labels after the build
(``remove_ticks_and_titles``). Random samples come from ``spec_support.rng``.
"""

import matplotlib.style
import numpy as np
from matplotlib import collections as mcollections
from matplotlib import markers
from matplotlib import patches as mpatches
from matplotlib import path as mpath
from matplotlib.ticker import NullFormatter

from spec_support import case, rng

CLASSIC_TEST = {**matplotlib.style.library["classic"],
                **matplotlib.style.library["_classic_test_patch"]}


def _remove_text(fig):
    """``remove_ticks_and_titles``: no titles, no tick labels."""
    fig.suptitle("")
    for ax in fig.axes:
        ax.set_title("")
        for axis in (ax.xaxis, ax.yaxis):
            axis.set_major_formatter(NullFormatter())
            axis.set_minor_formatter(NullFormatter())


# --- test_spines.py ------------------------------------------------------


@case("mpl_small/spines_axes_positions")
def _(fig):
    x = np.linspace(0, 2 * np.pi, 100)
    ax = fig.add_subplot(1, 1, 1)
    ax.set_title("centered spines")
    ax.plot(x, 2 * np.sin(x))
    ax.spines.right.set_position(("axes", 0.1))
    ax.yaxis.set_ticks_position("right")
    ax.spines.top.set_position(("axes", 0.25))
    ax.xaxis.set_ticks_position("top")
    ax.spines.left.set_color("none")
    ax.spines.bottom.set_color("none")


@case("mpl_small/spines_data_positions")
def _(fig):
    ax = fig.subplots()
    ax.spines.left.set_position(("data", -1.5))
    ax.spines.top.set_position(("data", 0.5))
    ax.spines.right.set_position(("data", -0.5))
    ax.spines.bottom.set_position("zero")
    ax.set_xlim([-2, 2])
    ax.set_ylim([-2, 2])
    ax.xaxis.set_ticks_position("both")
    ax.yaxis.set_ticks_position("both")


@case("mpl_small/spine_nonlinear_data_positions")
def _(fig):
    ax = fig.add_subplot()
    ax.set(xscale="log", xlim=(.1, 1))
    ax.spines.left.set_position(("data", 1))
    ax.spines.left.set_linewidth(2)
    ax.spines.right.set_position(("data", .1))
    ax.tick_params(axis="y", labelleft=False, direction="in")


@case("mpl_small/spines_capstyle", rc={**CLASSIC_TEST, "axes.linewidth": 20})
def _(fig):
    ax = fig.subplots()
    ax.set_xticks([])
    ax.set_yticks([])


@case("mpl_small/spines_black_axes", figsize=(4, 4),
      rc={**CLASSIC_TEST, "savefig.pad_inches": 0, "savefig.bbox": "tight"})
def _(fig):
    ax = fig.add_axes((0, 0, 1, 1))
    ax.set_xticklabels([])
    ax.set_yticklabels([])
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_facecolor((0, 0, 0))


# --- test_scale.py -------------------------------------------------------


@case("mpl_small/log_scales")
def _(fig):
    ax = fig.add_subplot(122, yscale="log", xscale="symlog")
    ax.axvline(24.1)
    ax.axhline(24.1)


@case("mpl_small/logit_scales", rc=CLASSIC_TEST)
def _(fig):
    ax = fig.subplots()
    x = np.array([0.001, 0.003, 0.01, 0.03, 0.1, 0.2, 0.3, 0.4, 0.5,
                  0.6, 0.7, 0.8, 0.9, 0.97, 0.99, 0.997, 0.999])
    ax.plot(x, 1.0 / x)
    ax.set_xscale("logit")
    ax.grid(True)
    _remove_text(fig)


@case("mpl_small/logscale_mask", rc=CLASSIC_TEST)
def _(fig):
    xs = np.linspace(0, 50, 1001)
    ax = fig.subplots()
    ax.plot(np.exp(-xs**2))
    ax.set(yscale="log", yticks=10.**np.arange(-300, 0, 24))
    _remove_text(fig)


@case("mpl_small/logscale_nonpos_values")
def _(fig):
    xs = rng(19680801).normal(1000)
    (ax1, ax2), (ax3, ax4) = fig.subplots(2, 2)
    ax1.hist(xs, range=(-5, 5), bins=10)
    ax1.set_yscale("log")
    ax2.hist(xs, range=(-5, 5), bins=10)
    ax2.set_yscale("log", nonpositive="mask")
    xdata = np.arange(0, 10, 0.01)
    ydata = np.exp(-xdata)
    edata = 0.2 * (10 - xdata) * np.cos(5 * xdata) * np.exp(-xdata)
    ax3.fill_between(xdata, ydata - edata, ydata + edata)
    ax3.set_yscale("log")
    x = np.logspace(-1, 1)
    ax4.errorbar(x, x ** 3, yerr=x ** 2)
    ax4.set_yscale("log")
    ax4.set_xscale("log")
    ax4.set_yticks([1e-2, 1, 1e+2])
    _remove_text(fig)


@case("mpl_small/function_scales")
def _(fig):
    ax = fig.subplots()
    x = np.arange(1, 1000)
    ax.plot(x, x)
    ax.set_xscale("function", functions=(lambda x: x ** (1 / 2), lambda x: x ** 2))
    ax.set_xlim(1, 1000)
    _remove_text(fig)


# --- test_marker.py ------------------------------------------------------

MARKER_SIZE = 20 ** 2


@case("mpl_small/poly_marker")
def _(fig):
    ax = fig.add_subplot()
    for (x, y), marker in [((0, 0), (4, 0, 45)), ((1, 1), (4, 0)), ((1, 1.5), (4, 0, 0)),
                           ((2, 2), (5, 0)), ((2, 2.5), (5, 0, 0)), ((3, 3), (6, 0)),
                           ((3, 3.5), (6, 0, 0)), ((4, 4), (6, 0, 30)), ((5, 5), (8, 0, 22.5))]:
        ax.scatter([x], [y], marker=marker, s=MARKER_SIZE)
    ax.set(xlim=(-0.5, 5.5), ylim=(-0.5, 5.5))


@case("mpl_small/asterisk_marker")
def _(fig):
    ax = fig.add_subplot()
    ax.scatter([0], [0], marker=(4, 2), s=MARKER_SIZE)
    ax.scatter([0.5], [0.5], marker=(4, 2, 0), s=MARKER_SIZE)
    ax.scatter([1], [1], marker=(4, 2, 45), s=MARKER_SIZE)
    ax.set(xlim=(-0.5, 1.5), ylim=(-0.5, 1.5))


@case("mpl_small/text_marker")
def _(fig):
    ax = fig.add_subplot()
    ax.plot(0, 0, marker=r"$\bullet$", markersize=100, markeredgewidth=0)


#: ``test_marker_clipping``: 41 markers in rows of 7, two rows apart.
CLIP_COLUMNS = 7
CLIP_ROWS = len(markers.MarkerStyle.markers) // CLIP_COLUMNS + 1


@case("mpl_small/marker_clipping", figsize=(2 * 50 * CLIP_COLUMNS / 100, 2 * 50 * CLIP_ROWS * 2 / 100))
def _(fig):
    ax = fig.add_axes((0, 0, 1, 1))
    for i, marker in enumerate(markers.MarkerStyle.markers):
        x = i % CLIP_COLUMNS
        y = i // CLIP_COLUMNS * 2
        ax.plot([x, x], [y, y + 1], c="k", linestyle="-", lw=3, marker=marker, markersize=50,
                markeredgewidth=10, fillstyle="full", markerfacecolor="white")
    ax.set(xlim=(-0.5, CLIP_COLUMNS), ylim=(-0.5, 2 * CLIP_ROWS))
    ax.axis("off")


# --- test_artist.py ------------------------------------------------------


@case("mpl_small/clip_path_clipping", rc=CLASSIC_TEST)
def _(fig):
    exterior = mpath.Path.unit_rectangle().deepcopy()
    exterior.vertices *= 4
    exterior.vertices -= 2
    interior = mpath.Path.unit_circle().deepcopy()
    interior.vertices = interior.vertices[::-1]
    clip_path = mpath.Path.make_compound_path(exterior, interior)
    star = mpath.Path.unit_regular_star(6).deepcopy()
    star.vertices *= 2.6
    ax1, ax2 = fig.subplots(1, 2, sharex=True, sharey=True)
    col = mcollections.PathCollection([star], lw=5, edgecolor="blue", facecolor="red",
                                      alpha=0.7, hatch="*")
    col.set_clip_path(clip_path, ax1.transData)
    ax1.add_collection(col)
    patch = mpatches.PathPatch(star, lw=5, edgecolor="blue", facecolor="red", alpha=0.7,
                               hatch="*")
    patch.set_clip_path(clip_path, ax2.transData)
    ax2.add_patch(patch)
    ax1.set_xlim(-3, 3)
    ax1.set_ylim(-3, 3)
    _remove_text(fig)


@case("mpl_small/clipping_zoom")
def _(fig):
    ax = fig.add_axes((0, 0, 1, 1))
    line, = ax.plot([-3, 3], [-3, 3])
    square = mpath.Path([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]])
    line.set_clip_path(mpatches.PathPatch(square, transform=ax.transData))
    ax.set(xlim=(0.5, 0.75), ylim=(0.5, 0.75))


@case("mpl_small/hatching")
def _(fig):
    ax = fig.subplots(1, 1)
    ax.add_patch(mpatches.Rectangle((0, 0), 3, 4, hatch="/"))
    ax.add_collection(mcollections.RegularPolyCollection(
        4, sizes=[16000], offsets=[(1.5, 6.5)], offset_transform=ax.transData, hatch="/"))
    ax.add_patch(mpatches.Rectangle((4, 0), 3, 4, hatch="/", edgecolor="C1"))
    ax.add_collection(mcollections.RegularPolyCollection(
        4, sizes=[16000], offsets=[(5.5, 6.5)], offset_transform=ax.transData, hatch="/",
        edgecolor="C1"))
    ax.set_xlim(0, 7)
    ax.set_ylim(0, 9)
    _remove_text(fig)


@case("mpl_small/default_edges")
def _(fig):
    (ax1, ax2), (ax3, ax4) = fig.subplots(2, 2)
    ax1.plot(np.arange(10), np.arange(10), "x", np.arange(10) + 1, np.arange(10), "o")
    ax2.bar(np.arange(10), np.arange(10), align="edge")
    ax3.text(0, 0, "BOX", size=24, bbox=dict(boxstyle="sawtooth"))
    ax3.set_xlim(-1, 1)
    ax3.set_ylim(-1, 1)
    curve = mpath.Path([(0, 0), (1, 0), (1, 1), (0, 0)],
                       [mpath.Path.MOVETO, mpath.Path.CURVE3, mpath.Path.CURVE3,
                        mpath.Path.CLOSEPOLY])
    ax4.add_patch(mpatches.PathPatch(curve, fc="none", transform=ax4.transData))
    _remove_text(fig)


# --- test_subplots.py ----------------------------------------------------


@case("mpl_small/subplots_offset_text")
def _(fig):
    x = np.arange(0, 1e10, 1e9)
    y = np.arange(0, 100, 10) + 1e5
    axs = fig.subplots(2, 2, sharex="col", sharey="all")
    axs[0, 0].plot(x, x)
    axs[1, 0].plot(x, x)
    axs[0, 1].plot(y, x)
    axs[1, 1].plot(y, x)


@case("mpl_small/width_and_height_ratios")
def _(fig):
    fig.subplots(2, 3, height_ratios=[1, 2], width_ratios=[1, 3, 2])


@case("mpl_small/width_and_height_ratios_mosaic")
def _(fig):
    fig.subplot_mosaic([["A", "B", "B"], ["A", "C", "D"]], height_ratios=[1, 2],
                       width_ratios=[1, 3, 2])


# --- test_transforms.py --------------------------------------------------


@case("mpl_small/scale_swapping")
def _(fig):
    samples = rng(19680801).normal(10)
    x = np.linspace(-5, 5, 10)
    ax = fig.subplots()
    ax.hist(samples, log=True, density=True)
    ax.plot(x, np.exp(-(x**2) / 2) / np.sqrt(2 * np.pi))
    fig.canvas.draw()
    ax.set_yscale("linear")
