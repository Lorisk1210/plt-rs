"""matplotlib's own image tests of ``test_axes.py``, third part: from
``test_bxp_customwidths`` to ``test_eb_line_zorder`` (box plots, violin
plots, error bars, histograms, stems, markers, the Mollweide grid, event
plots and line z-orders).

Each case reproduces the figure a test draws, under the conditions of
matplotlib's test decorators:

* ``@image_comparison`` uses the test's ``style`` ("default"/"mpl20" need
  no rc) or, without one, "classic" plus "_classic_test_patch"; so does
  every ``@check_figures_equal`` test (matplotlib's ``mpl_test_settings``
  fixture). Classic figures are 8x6 in saved at 100 dpi.
* ``savefig_kwarg={'dpi': ...}`` becomes the case's dpi.
* ``remove_text=True`` runs ``remove_ticks_and_titles`` after the build.
* ``@check_figures_equal`` tests contribute their ``fig_test`` figure;
  ``violinplot_color_specification_ref`` adds the ``fig_ref`` side that
  recolours the parts after drawing them.
* A test drawing several baseline images becomes ``<name>_1``, ``_2``, ...

Deviations from the tests:

* ``np.random`` data comes from ``spec_support.rng`` seeded with the test's
  seed; lognormal samples are the exponentials of its normal samples.
* ``bootstrap=`` and ``boxplot.bootstrap`` are dropped: matplotlib
  resamples with numpy's global random state, which cannot be reproduced.
  Notches use the Gaussian-based interval instead (identical images for
  ``boxplot_sym2``, ``boxplot_with_CIarray`` and
  ``boxplot_autorange_whiskers``, which draw no bootstrapped notch).
* Figures a test draws again from labeled data (``data=``) into the same
  baseline image are not repeated.
* ``test_stem`` draws its legend text instead of the ``text_placeholders``
  rectangles.
* ``test_violinplot_pandas_series`` needs pandas; its ``fig_ref`` side
  (plain arrays of 7, 9 and 11 samples) is ported instead.
"""

import datetime

import matplotlib
import matplotlib.style
import numpy as np
from cycler import cycler
from matplotlib import cbook
from matplotlib.testing.decorators import remove_ticks_and_titles

from spec_support import case, rng

GROUP = "mpl_axes_c"

#: ``image_comparison``'s default style.
CLASSIC = {
    **matplotlib.style.library["classic"],
    **matplotlib.style.library["_classic_test_patch"],
}

#: ``_bxp_test_helper`` sets the box line width to ``lines.linewidth``.
BXP_RC = {"boxplot.boxprops.linewidth": 1.5}

BOXPLOT_X = np.hstack([-25, np.linspace(-7, 7, 140), 25])


def image_test(name, *, classic=False, dpi=100, remove_text=False, rc=None):
    """Register ``build`` as ``mpl_axes_c/<name>`` under the conditions of
    matplotlib's test decorators (see the module docstring)."""
    params = {**(CLASSIC if classic else {}), **(rc or {})}
    figsize = (8, 6) if classic else (6.4, 4.8)

    def register(build):
        def run(fig):
            build(fig)
            if remove_text:
                remove_ticks_and_titles(fig)

        case(f"{GROUP}/{name}", figsize=figsize, dpi=dpi, rc=params)(run)
        return build

    return register


def figures_equal(name, **options):
    """A ``@check_figures_equal`` test: classic style, 8x6 in at 100 dpi."""
    return image_test(name, classic=True, **options)


# --- box plots -------------------------------------------------------------


def _bxp(fig, **bxp_kwargs):
    """``_bxp_test_helper``: lognormal statistics on a log axis."""
    data = np.exp(rng(937).normal(37 * 4, 1.25, 1.0)).reshape(37, 4)
    ax = fig.subplots()
    ax.set_yscale("log")
    ax.bxp(cbook.boxplot_stats(data), **bxp_kwargs)


@image_test("bxp_customwidths", dpi=40, remove_text=True, rc=BXP_RC)
def _(fig):
    _bxp(fig, widths=[0.10, 0.25, 0.65, 0.85])


@image_test("bxp_custompositions", dpi=40, remove_text=True, rc=BXP_RC)
def _(fig):
    _bxp(fig, positions=[1, 5, 6, 7])


@image_test("bxp_custom_capwidths", dpi=40, rc=BXP_RC)
def _(fig):
    _bxp(fig, capwidths=[0.0, 0.1, 0.5, 1.0])


@image_test("bxp_custom_capwidth", dpi=40, rc=BXP_RC)
def _(fig):
    _bxp(fig, capwidths=0.6)


@image_test("boxplot")
def _(fig):
    ax = fig.subplots()
    ax.boxplot([BOXPLOT_X, BOXPLOT_X], notch=1)
    ax.set_ylim(-30, 30)


@figures_equal("boxplot_masked")
def _(fig):
    x = np.linspace(-1, 1, 200)
    fig.subplots().boxplot(x[x >= 0])


@image_test("boxplot_custom_capwidths", dpi=40)
def _(fig):
    fig.subplots().boxplot([BOXPLOT_X, BOXPLOT_X], notch=1, capwidths=[0.01, 0.2])


@image_test("boxplot_sym2", remove_text=True)
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.boxplot([BOXPLOT_X, BOXPLOT_X], sym="^")
    ax1.set_ylim(-30, 30)
    ax2.boxplot([BOXPLOT_X, BOXPLOT_X], sym="g")
    ax2.set_ylim(-30, 30)


@image_test("boxplot_sym", dpi=40, remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.boxplot([BOXPLOT_X, BOXPLOT_X], sym="gs")
    ax.set_ylim(-30, 30)


def _autorange_whiskers(fig, autorange):
    x = np.hstack([0, np.ones(140), 2])
    ax = fig.subplots()
    ax.boxplot([x, x], notch=1, autorange=autorange)
    ax.set_ylim(-5, 5)


@image_test("boxplot_autorange_whiskers_1")
def _(fig):
    _autorange_whiskers(fig, False)


@image_test("boxplot_autorange_whiskers_2")
def _(fig):
    _autorange_whiskers(fig, True)


BOXPLOT_RC_AXES = [
    {
        "boxplot.notch": True,
        "boxplot.whiskers": [5, 95],
        "boxplot.flierprops.color": "b",
        "boxplot.flierprops.marker": "o",
        "boxplot.flierprops.markerfacecolor": "g",
        "boxplot.flierprops.markeredgecolor": "b",
        "boxplot.flierprops.markersize": 5,
        "boxplot.flierprops.linestyle": "--",
        "boxplot.flierprops.linewidth": 2.0,
        "boxplot.boxprops.color": "r",
        "boxplot.boxprops.linewidth": 2.0,
        "boxplot.boxprops.linestyle": "--",
        "boxplot.capprops.color": "c",
        "boxplot.capprops.linewidth": 2.0,
        "boxplot.capprops.linestyle": "--",
        "boxplot.medianprops.color": "k",
        "boxplot.medianprops.linewidth": 2.0,
        "boxplot.medianprops.linestyle": "--",
    },
    {
        "boxplot.whiskers": [0, 100],
        "boxplot.patchartist": True,
    },
    {
        "boxplot.whiskers": 2.0,
        "boxplot.showcaps": False,
        "boxplot.showbox": False,
        "boxplot.showfliers": False,
        "boxplot.showmeans": True,
        "boxplot.meanline": True,
        "boxplot.meanprops.color": "c",
        "boxplot.meanprops.linewidth": 2.0,
        "boxplot.meanprops.linestyle": "--",
        "boxplot.whiskerprops.color": "r",
        "boxplot.whiskerprops.linewidth": 2.0,
        "boxplot.whiskerprops.linestyle": "-.",
    },
]


@image_test("boxplot_rc_parameters", remove_text=True)
def _(fig):
    for ax, rc_axis in zip(fig.subplots(3), BOXPLOT_RC_AXES):
        with matplotlib.rc_context(rc_axis):
            ax.boxplot([BOXPLOT_X, BOXPLOT_X])


@image_test("boxplot_with_CIarray", dpi=40, remove_text=True)
def _(fig):
    ax = fig.subplots()
    cis = np.array([[-1.5, 3.0], [-1.0, 3.5]])
    ax.boxplot([BOXPLOT_X, BOXPLOT_X], usermedians=[None, 1.0], conf_intervals=cis, notch=1)
    ax.set_ylim(-30, 30)


@image_test("boxplot_no_weird_whisker", dpi=40, remove_text=True)
def _(fig):
    x = np.array([3, 9000, 150, 88, 350, 200000, 1400, 960], dtype=np.float64)
    ax = fig.add_subplot()
    ax.boxplot(x)
    ax.set_yscale("log")
    ax.yaxis.grid(False, which="minor")
    ax.xaxis.grid(False)


@image_test("boxplot_mod_artist_after_plotting", dpi=40, remove_text=True)
def _(fig):
    bp = fig.subplots().boxplot([0.15, 0.11, 0.06, 0.06, 0.12, 0.56, -0.56], sym="o")
    for artists in bp.values():
        for artist in artists:
            artist.set_color("green")


# --- violin plots ----------------------------------------------------------

NO_PARTS = {"showmeans": False, "showextrema": False, "showmedians": False}
QUANTILES = [[0.1, 0.9], [0.2, 0.8], [0.3, 0.7], [0.4, 0.6]]
ALL_PARTS = {"showmeans": True, "showextrema": True, "showmedians": True, "quantiles": QUANTILES}

#: name -> (seed, violinplot keywords) of the four-violin tests.
VIOLINS = {
    "vert_violinplot_baseline": (414213562, NO_PARTS),
    "vert_violinplot_showmeans": (732050807, {**NO_PARTS, "showmeans": True}),
    "vert_violinplot_showextrema": (236067977, {**NO_PARTS, "showextrema": True}),
    "vert_violinplot_showmedians": (645751311, {**NO_PARTS, "showmedians": True}),
    "vert_violinplot_showall": (316624790, ALL_PARTS),
    "vert_violinplot_custompoints_10": (605551275, {**NO_PARTS, "points": 10}),
    "vert_violinplot_custompoints_200": (123105625, {**NO_PARTS, "points": 200}),
    "horiz_violinplot_baseline": (358898943, NO_PARTS),
    "horiz_violinplot_showmedians": (795831523, {**NO_PARTS, "showmedians": True}),
    "horiz_violinplot_showmeans": (385164807, {**NO_PARTS, "showmeans": True}),
    "horiz_violinplot_showextrema": (567764362, {**NO_PARTS, "showextrema": True}),
    "horiz_violinplot_showall": (82762530, ALL_PARTS),
    "horiz_violinplot_custompoints_10": (403124237, {**NO_PARTS, "points": 10}),
    "horiz_violinplot_custompoints_200": (557438524, {**NO_PARTS, "points": 200}),
}


def _violins(seed, kwargs, orientation):
    def build(fig):
        r = rng(seed)
        data = [r.normal(100) for _ in range(4)]
        fig.add_subplot().violinplot(data, positions=range(4), orientation=orientation,
                                     **kwargs)

    return build


for _name, (_seed, _kwargs) in VIOLINS.items():
    _orientation = "horizontal" if _name.startswith("horiz") else "vertical"
    image_test(_name)(_violins(_seed, _kwargs, _orientation))


@image_test("violinplot_sides", remove_text=True)
def _(fig):
    ax = fig.add_subplot()
    data = [rng(19680801).normal(100)]
    for orientation, positions in (("horizontal", [0, -0.5, 0.5]), ("vertical", [4, 3.5, 4.5])):
        for pos, side in zip(positions, ["both", "low", "high"]):
            ax.violinplot(data, positions=[pos], orientation=orientation, showmeans=False,
                          showextrema=True, showmedians=True, side=side)


#: ``test_violinplot_color_specification``: face and line colours of each axes.
VIOLIN_COLORS = [{"facecolor": ("r", 0.5), "linecolor": ("r", 0.2)}, {"facecolor": "r"},
                 {"linecolor": "r"}]
VIOLIN_ALL_LINES = {"showmeans": True, "showextrema": True, "showmedians": True}


def _sorted_violin_data():
    r = rng(19680801)
    return [sorted(r.normal(100, 0, std)) for std in range(1, 4)]


@figures_equal("violinplot_color_specification")
def _(fig):
    data = _sorted_violin_data()
    for ax, colors in zip(fig.subplots(1, 3), VIOLIN_COLORS):
        ax.violinplot(data, **colors, **VIOLIN_ALL_LINES)


@figures_equal("violinplot_color_specification_ref")
def _(fig):
    data = _sorted_violin_data()
    for ax, colors in zip(fig.subplots(1, 3), VIOLIN_COLORS):
        parts = ax.violinplot(data, **VIOLIN_ALL_LINES)
        if "facecolor" in colors:
            for body in parts["bodies"]:
                body.set_facecolor(colors["facecolor"])
                body.set_alpha(None)
        if "linecolor" in colors:
            for name in ("cbars", "cmins", "cmaxes", "cmeans", "cmedians"):
                parts[name].set_edgecolor(colors["linecolor"])


@figures_equal("violinplot_single_list_quantiles")
def _(fig):
    data = [rng(110433579).normal(100)]
    fig.subplots().violinplot(data, quantiles=[0.1, 0.3, 0.9])


@figures_equal("violinplot_pandas_series_ref")
def _(fig):
    r = rng(110433579)
    fig.subplots().violinplot([r.normal(7), r.normal(9), r.normal(11)])


# --- error bars ------------------------------------------------------------

ERRORBAR_X = np.arange(0.1, 4, 0.5, dtype=np.longdouble)


@image_test("errorbar_1")
def _(fig):
    ax = fig.gca()
    ax.errorbar(ERRORBAR_X, np.exp(-ERRORBAR_X), xerr=0.2, yerr=0.4)
    ax.set_title("Simplest errorbars, 0.2 in x, 0.4 in y")


@image_test("errorbar_2")
def _(fig):
    x = ERRORBAR_X
    y = np.exp(-x)
    yerr = 0.1 + 0.2 * np.sqrt(x)
    xerr = 0.1 + yerr

    axs = fig.subplots(nrows=2, ncols=2, sharex=True)
    ax = axs[0, 0]
    ax.errorbar(x, y, yerr=yerr, fmt="o")
    ax.set_title("Vert. symmetric")
    ax.locator_params(nbins=4)

    ax = axs[0, 1]
    ax.errorbar(x, y, xerr=xerr, fmt="o", alpha=0.4)
    ax.set_title("Hor. symmetric w/ alpha")

    ax = axs[1, 0]
    ax.errorbar(x, y, yerr=[yerr, 2 * yerr], xerr=[xerr, 2 * xerr], fmt="--o")
    ax.set_title("H, V asymmetric")

    ax = axs[1, 1]
    ax.set_yscale("log")
    ylower = np.maximum(1e-2, y - yerr)
    yerr_lower = y - ylower
    ax.errorbar(x, y, yerr=[yerr_lower, 2 * yerr], xerr=xerr, fmt="o", ecolor="g", capthick=2)
    ax.set_title("Mixed sym., log y")
    ax.set_ylim(1e-2, 1e1)

    fig.suptitle("Variable errorbars")


@image_test("mixed_errorbar_polar_caps", classic=True, remove_text=True)
def _(fig):
    ax = fig.add_subplot(111, projection="polar")
    ax.errorbar([1, 2, 3], [0.9] * 3, xerr=0.35, yerr=0.2, fmt="o")
    ax.errorbar([np.pi / 2 + 0.1, np.pi + 0.1], [1.8, 2.2], xerr=0.8 * np.pi, yerr=0.15, fmt="o")
    ax.errorbar([4 * np.pi / 3 + 0.1, 5 * np.pi / 3 + 0.1, 2 * np.pi - 0.1], [1.1] * 3,
                xerr=[[0.3, 0.3, 0.2], [0.2, 0.3, 0.3]],
                yerr=[[0.35, 0.5, 0.5], [0.5, 0.35, 0.5]], fmt="o")
    ax.errorbar([2.1], [3.1], xerr=10, yerr=0.2, fmt="o")


@figures_equal("errorbar_cycle_ecolor")
def _(fig):
    x = np.arange(0.1, 4, 0.5)
    ax = fig.subplots()
    for n in range(4):
        y = np.exp(-x + n)
        ax.errorbar(x, y, yerr=y * 0.25, linestyle="-", marker="o", ecolor="black")


@image_test("errorbar_limits")
def _(fig):
    x = np.arange(0.5, 5.5, 0.5)
    y = np.exp(-x)
    xerr = 0.1
    yerr = 0.2
    ls = "dotted"

    ax = fig.subplots()
    ax.errorbar(x, y, xerr=xerr, yerr=yerr, ls=ls, color="blue")

    uplims = np.zeros_like(x)
    uplims[[1, 5, 9]] = True
    ax.errorbar(x, y + 0.5, xerr=xerr, yerr=yerr, uplims=uplims, ls=ls, color="green")

    lolims = np.zeros_like(x)
    lolims[[2, 4, 8]] = True
    ax.errorbar(x, y + 1.0, xerr=xerr, yerr=yerr, lolims=lolims, ls=ls, color="red")

    ax.errorbar(x, y + 1.5, marker="o", ms=6, xerr=xerr, yerr=yerr, lolims=lolims,
                uplims=uplims, ls=ls, color="magenta")

    xerr = 0.2
    yerr = np.full_like(x, 0.2)
    yerr[[3, 6]] = 0.3
    xlolims = lolims
    xuplims = uplims
    lolims = np.zeros_like(x)
    uplims = np.zeros_like(x)
    lolims[[6]] = True
    uplims[[3]] = True
    ax.errorbar(x, y + 2.1, marker="o", ms=6, xerr=xerr, yerr=yerr, xlolims=xlolims,
                xuplims=xuplims, uplims=uplims, lolims=lolims, ls="none", mec="blue",
                capsize=0, color="cyan")
    ax.set_xlim(0, 5.5)
    ax.set_title("Errorbar upper and lower limits")


@figures_equal("errorbar_with_prop_cycle", rc={
    "axes.prop_cycle": cycler(ls=["--", ":", "-."], marker=["s", "s", "o"],
                              mfc=["k", "y", "c"], color=["b", "g", "r"]),
})
def _(fig):
    ax = fig.subplots()
    ax.errorbar(x=[2, 4, 10], y=[0, 1, 2], yerr=0.5)
    ax.errorbar(x=[2, 4, 10], y=[2, 3, 4], yerr=0.5, color="tab:green")
    ax.errorbar(x=[2, 4, 10], y=[4, 5, 6], yerr=0.5, fmt="tab:blue")
    ax.set_xlim(1, 11)


@figures_equal("errorbar_every")
def _(fig):
    x = np.linspace(0, 1, 15)
    y = x * (1 - x)
    yerr = y / 6
    ax = fig.subplots()
    for color, shift in zip("rgbk", [0, 0, 2, 7]):
        y += 0.02
        ax.errorbar(x, y, yerr, errorevery=(shift, 4), capsize=4, c=color)
    ax.errorbar(x, y + 0.1, yerr, markevery=(1, 4), capsize=4, fmt="o")
    ax.errorbar(x, y + 0.2, yerr, errorevery=slice(2, None, 3), markevery=slice(2, None, 3),
                capsize=4, c="C0", fmt="o")
    ax.errorbar(x, y + 0.2, yerr, errorevery=[False, True, False] * 5,
                markevery=[False, True, False] * 5, capsize=4, c="C1", fmt="o")


@figures_equal("errorbar_nan")
def _(fig):
    ys = np.array([1, 2, np.nan, np.nan, 3])
    es = np.array([4, 5, np.nan, np.nan, 6])
    fig.add_subplot().errorbar(range(5), ys, yerr=es)


@figures_equal("errorbar_masked_negative")
def _(fig):
    mask = np.array([False, False, True, True, False])
    ys = np.ma.array([1, 2, 2, 2, 3], mask=mask)
    es = np.ma.array([4, 5, -1, -10, 6], mask=mask)
    fig.add_subplot().errorbar(range(5), ys, yerr=es)


@image_test("eb_line_zorder_1")
def _(fig):
    ax = fig.gca()
    ax.plot(list(range(10)), lw=10, zorder=5)
    ax.axhline(1, color="red", lw=10, zorder=1)
    ax.axhline(5, color="green", lw=10, zorder=10)
    ax.axvline(7, color="m", lw=10, zorder=7)
    ax.axvline(2, color="k", lw=10, zorder=3)
    ax.set_title("axvline and axhline zorder test")


@image_test("eb_line_zorder_2")
def _(fig):
    ax = fig.gca()
    ax.errorbar(list(range(10)), np.zeros(10), yerr=list(range(10)), zorder=5, lw=5, color="r")
    for j in range(10):
        ax.axhline(j, lw=5, color="k", zorder=j)
        ax.axhline(-j, lw=5, color="k", zorder=j)
    ax.set_title("errorbar zorder test")


# --- histograms ------------------------------------------------------------

D_1_3 = np.linspace(1, 3, 20)
D_0_10 = np.linspace(0, 10, 50)


@image_test("hist_stacked_stepfilled")
def _(fig):
    fig.subplots().hist((D_1_3, D_0_10), histtype="stepfilled", stacked=True)


@image_test("hist_offset")
def _(fig):
    ax = fig.subplots()
    ax.hist(D_0_10, bottom=5)
    ax.hist(D_1_3, bottom=15)


@image_test("hist_step", classic=True, remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.hist(D_1_3, histtype="step")
    ax.set_ylim(0, 10)
    ax.set_xlim(-1, 5)


@image_test("hist_step_horiz")
def _(fig):
    fig.subplots().hist((D_0_10, D_1_3), histtype="step", orientation="horizontal")


@image_test("hist_stacked_weighted")
def _(fig):
    w1 = np.linspace(0.01, 3.5, 50)
    w2 = np.linspace(0.05, 2.0, 20)
    fig.subplots().hist((D_0_10, D_1_3), weights=(w1, w2), histtype="stepfilled", stacked=True)


@image_test("hist_stacked_stepfilled_alpha")
def _(fig):
    fig.subplots().hist((D_1_3, D_0_10), histtype="stepfilled", stacked=True, alpha=0.5)


@image_test("hist_stacked_step")
def _(fig):
    fig.subplots().hist((D_1_3, D_0_10), histtype="step", stacked=True)


@image_test("hist_stacked_density")
def _(fig):
    fig.subplots().hist((D_1_3, D_0_10), stacked=True, density=True)


@image_test("hist_step_bottom", classic=True, remove_text=True)
def _(fig):
    fig.subplots().hist(D_1_3, bottom=np.arange(10), histtype="stepfilled")


@image_test("hist_stacked_bar")
def _(fig):
    d = [[100, 100, 100, 100, 200, 320, 450, 80, 20, 600, 310, 800],
         [20, 23, 50, 11, 100, 420], [120, 120, 120, 140, 140, 150, 180],
         [60, 60, 60, 60, 300, 300, 5, 5, 5, 5, 10, 300],
         [555, 555, 555, 30, 30, 30, 30, 30, 100, 100, 100, 100, 30, 30],
         [30, 30, 30, 30, 400, 400, 400, 400, 400, 400, 400, 400]]
    colors = [(0.5759849696758961, 1.0, 0.0), (0.0, 1.0, 0.350624650815206),
              (0.0, 1.0, 0.6549834156005998), (0.0, 0.6569064625276622, 1.0),
              (0.28302699607823545, 0.0, 1.0), (0.6849123462299822, 0.0, 1.0)]
    labels = ["first", "second", "third", "fourth", "fifth"]
    ax = fig.subplots()
    ax.hist(d, bins=10, histtype="barstacked", align="mid", color=colors, label=labels)
    ax.legend(loc="upper right", bbox_to_anchor=(1.0, 1.0), ncols=1)


#: The keyword of each ``test_hist_vectorized_params`` parametrization.
VECTORIZED = {
    "facecolor": ["b", "g", "r"],
    "edgecolor": ["b", "g", "r"],
    "hatch": ["/", "\\", "."],
    "linestyle": ["-", "--", ":"],
    "linewidth": [1, 1.5, 2],
    "color": ["b", "g", "r"],
}


def _hist_vectorized(keyword, values):
    def build(fig):
        r = rng(19680801)
        xs = [r.normal(n) for n in (20, 50, 100)]
        for histtype, ax in zip(("stepfilled", "step"), fig.subplots(2)):
            ax.hist(xs, bins=10, histtype=histtype, **{keyword: values})

    return build


for _keyword, _values in VECTORIZED.items():
    figures_equal(f"hist_vectorized_params_{_keyword}")(_hist_vectorized(_keyword, _values))


# --- stems -----------------------------------------------------------------


@image_test("stem", remove_text=True)
def _(fig):
    x = np.linspace(0.1, 2 * np.pi, 100)
    ax = fig.subplots()
    ax.stem(x, np.cos(x), linefmt="C2-.", markerfmt="k+", basefmt="C1-.", label="stem")
    ax.legend()


@image_test("stem_orientation", remove_text=True)
def _(fig):
    x = np.linspace(0.1, 2 * np.pi, 50)
    fig.subplots().stem(x, np.cos(x), linefmt="C2-.", markerfmt="kx", basefmt="C1-.",
                        orientation="horizontal")


# --- markers and alpha -----------------------------------------------------


@image_test("transparent_markers", classic=True, remove_text=True)
def _(fig):
    fig.subplots().plot(rng(0).uniform(50), "D", mfc="none", markersize=100)


@image_test("rgba_markers", classic=True, remove_text=True)
def _(fig):
    axs = fig.subplots(ncols=2)
    rcolors = [(1, 0, 0, 1), (1, 0, 0, 0.5)]
    bcolors = [(0, 0, 1, 1), (0, 0, 1, 0.5)]
    for ax, alpha in zip(axs, [None, 0.2]):
        for j, rcolor in enumerate(rcolors):
            for k, bcolor in enumerate(bcolors):
                ax.plot(j + 1, k + 1, "o", mfc=bcolor, mec=rcolor, alpha=alpha, ms=100, mew=20)
                ax.plot(j + 1, k + 3, "x", mec=rcolor, alpha=alpha, ms=100, mew=20)
    for ax in axs:
        ax.axis([-1, 4, 0, 5])


@image_test("alpha", classic=True, remove_text=True)
def _(fig):
    data = rng(0).uniform(50)
    ax = fig.subplots()
    ax.plot(data, "-D", color=[1, 0, 0], mfc=[1, 0, 0, 0.5], markersize=20, lw=10)
    ax.plot(data + 2, "-D", color=[1, 0, 0, 0.5], mfc=[1, 0, 0, 0.5], markersize=20, lw=10,
            alpha=1)
    ax.plot(data + 4, "-D", color=[1, 0, 0], mfc=[1, 0, 0], markersize=20, lw=10, alpha=0.5)
    ax.plot(data + 6, "-D", color=[1, 0, 0, 0.5], mfc=[1, 0, 0, 0.5], markersize=20, lw=10)
    ax.plot(data + 8, "-D", color=[1, 0, 0, 0.5], mfc=[1, 0, 0], markersize=20, lw=10)


@image_test("marker_styles", classic=True, remove_text=True)
def _(fig):
    markers = sorted(matplotlib.markers.MarkerStyle.markers, key=lambda x: str(type(x)) + str(x))
    markers.remove("none")
    markers = ["none", *markers]
    ax = fig.subplots()
    for y, marker in enumerate(markers):
        ax.plot((y % 2) * 5 + np.arange(10) * 10, np.ones(10) * 10 * y, linestyle="",
                marker=marker, markersize=10 + y / 5, label=marker)


@image_test("markers_fillstyle_rcparams")
def _(fig):
    ax = fig.subplots()
    x = np.arange(7)
    for idx, (style, marker) in enumerate([("top", "s"), ("bottom", "o"), ("none", "^")]):
        matplotlib.rcParams["markers.fillstyle"] = style
        ax.plot(x + idx, marker=marker)


@image_test("vertex_markers", classic=True, remove_text=True)
def _(fig):
    data = list(range(10))
    square = [(-1, -1), (1, -1), (1, 1), (-1, 1)]
    ax = fig.subplots()
    ax.plot(data, linestyle="", marker=tuple(square), mfc="k")
    ax.plot(data[::-1], linestyle="", marker=square, mfc="b")
    ax.set_xlim(-1, 10)
    ax.set_ylim(-1, 10)


@image_test("mollweide_grid", classic=True, remove_text=True)
def _(fig):
    fig.add_subplot(projection="mollweide").grid()


# --- event plots -----------------------------------------------------------


def _event_data():
    """32 + 6 rows of 20 uniform events (``np.random.random`` with seed 0)."""
    r = rng(0)
    data1 = r.uniform(32 * 20).reshape(32, 20).tolist()
    data2 = r.uniform(6 * 20).reshape(6, 20).tolist()
    return data1, data2


@image_test("eventplot", classic=True, remove_text=True)
def _(fig):
    data1, data2 = _event_data()
    colors = [[0, 1, 0.7]] * len(data1) + [[1, 0, 0], [0, 1, 0], [0, 0, 1], [1, 0.75, 0],
                                           [1, 0, 1], [0, 1, 1]]
    lineoffsets = (12 + np.arange(0, len(data1)) * 0.33).tolist() + [-15, -3, 1, 1.5, 6, 10]
    linelengths = [0.33] * len(data1) + [5, 2, 1, 1, 3, 1.5]
    fig.add_subplot().eventplot(data1 + data2, colors=colors, lineoffsets=lineoffsets,
                                linelengths=linelengths)


@image_test("eventplot_defaults", classic=True, remove_text=True)
def _(fig):
    data1, data2 = _event_data()
    fig.add_subplot().eventplot(data1 + data2)


@image_test("eventplot_problem_kwargs", classic=True, remove_text=True)
def _(fig):
    r = rng(0)
    data = [r.uniform(20).tolist(), r.uniform(10).tolist()]
    fig.add_subplot().eventplot(data, colors=["r", "b"], color=["c", "m"], linewidths=[2, 1],
                                linewidth=[1, 2], linestyles=["solid", "dashed"],
                                linestyle=["dashdot", "dotted"])


@figures_equal("eventplot_units_list")
def _(fig):
    ts_1 = [datetime.datetime(2021, 1, day) for day in (1, 2, 3)]
    ts_2 = [datetime.datetime(2021, 1, day) for day in (15, 16)]
    fig.subplots().eventplot([ts_1, ts_2])
