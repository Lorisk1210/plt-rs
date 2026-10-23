"""matplotlib's user guide pages on axes and artists
(``galleries/users_explain/axes`` and ``galleries/users_explain/artists``):
arranging axes, autoscaling, axis scales, ticks and units, colorbar
placement, the constrained and tight layout guides, the legend guide,
``subplot_mosaic``, color cycles, ``imshow`` extents, path effects, paths
and transforms. Each case ports one figure of a page, ``<page>_<n>`` where
``n`` counts the page's figures (``color_cycle`` draws only one).

The pages run as written, with these substitutions: ``np.random`` data
comes from ``spec_support.rng`` seeded with the page's seed (19680801,
also for the pages that do not seed), a fresh generator per figure, so a
figure reusing an earlier figure's data draws it again. rcParams a page
sets for all its figures become the cases' ``rc`` (``savefig.facecolor``
and ``figure.figsize`` of the layout guides, the ``concise`` date
converter from the fourth figure of ``axes_units`` on, the property cycle
of ``color_cycle``); figures made through ``pyplot`` (``plt.subplot``,
``plt.subplot2grid``, ``plt.text``) use the equivalent ``Figure`` methods.

Covered elsewhere: ``axes_ticks_4`` is ``gallery_ticks/tick_formatters``.
Left out: figures identical to an earlier one of the same page
(``mosaic_4``, ``colorbar_placement_13``), ``axes_units_12`` (its labels
print the reprs of locator and formatter objects, memory addresses
included), the ``plot_children`` figures of the constrained layout guide
(they draw through the private ``matplotlib._layoutgrid``), the cells
that only print, ``legend_guide_12`` (a handler class of its own drawing
legend handles as ellipses, which plt-rs has no counterpart for) and
``arranging_axes_4`` (its left column's tick at x = 0.5 falls on the
pixel boundary 137.5 px, which kiwisolver's round-off in the constrained
layout puts 3e-14 px below and plt-rs's exact solution right on, so the
tick snaps to different pixels) and ``arranging_axes_14`` (kiwisolver
solves its 144 nested axes with a round-off that changes from run to run,
moving about a hundred pixels: matplotlib does not reproduce its own
image).
"""

import matplotlib as mpl
import matplotlib.colors as mcolors
import matplotlib.dates as mdates
import matplotlib.gridspec as gridspec
import matplotlib.lines as mlines
import matplotlib.patches as mpatches
import matplotlib.patheffects as path_effects
import matplotlib.ticker as ticker
import matplotlib.transforms as transforms
import numpy as np
from cycler import cycler
from matplotlib.legend_handler import HandlerLine2D, HandlerTuple
from matplotlib.path import Path
from mpl_toolkits.axes_grid1 import Grid, make_axes_locatable

from spec_support import case, rng

G = "gallery_guides_axes"
SEED = 19680801


def gallery(name, **options):
    """Registers a case of this group."""
    return case(f"{G}/{name}", **options)


# --- arranging_axes ------------------------------------------------------------


def _annotate_axes(ax, text, fontsize=18):
    """``arranging_axes``' label in the middle of an axes."""
    ax.text(0.5, 0.5, text, transform=ax.transAxes,
            ha="center", va="center", fontsize=fontsize, color="darkgrey")


def _annotated_mosaic(fig, layout, **kwargs):
    """A constrained mosaic with every axes named by its key, under the
    ``plt.subplot_mosaic()`` title."""
    fig.set_layout_engine("constrained")
    for k, ax in fig.subplot_mosaic(layout, **kwargs).items():
        _annotate_axes(ax, f"axd[{k!r}]", fontsize=14)
    fig.suptitle("plt.subplot_mosaic()")


@gallery("arranging_axes_1", figsize=(4, 3))
def _(fig):
    w, h = 4, 3
    margin = 0.5
    fig.set_facecolor("lightblue")
    fig.add_axes((margin / w, margin / h, (w - 2 * margin) / w, (h - 2 * margin) / h))


@gallery("arranging_axes_2", figsize=(5.5, 3.5))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for row in range(2):
        for col in range(2):
            axs[row, col].annotate(f"axs[{row}, {col}]", (0.5, 0.5),
                                   transform=axs[row, col].transAxes,
                                   ha="center", va="center", fontsize=18,
                                   color="darkgrey")
    fig.suptitle("plt.subplots()")


@gallery("arranging_axes_3", figsize=(5.5, 3.5))
def _(fig):
    _annotated_mosaic(fig, [["upper left", "upper right"], ["lower left", "lower right"]])


@gallery("arranging_axes_5", figsize=(5.5, 3.5))
def _(fig):
    fig.set_layout_engine("compressed")
    fig.set_facecolor("lightblue")
    for ax in fig.subplots(2, 2).flat:
        ax.set_aspect(1)
    fig.suptitle("Fixed aspect Axes: compressed")


@gallery("arranging_axes_6", figsize=(5.5, 3.5))
def _(fig):
    _annotated_mosaic(fig, [["upper left", "right"], ["lower left", "right"]])


@gallery("arranging_axes_7", figsize=(5.5, 3.5))
def _(fig):
    gs_kw = dict(width_ratios=[1.4, 1], height_ratios=[1, 2])
    _annotated_mosaic(fig, [["upper left", "right"], ["lower left", "right"]], gridspec_kw=gs_kw)


@gallery("arranging_axes_8")
def _(fig):
    fig.set_layout_engine("constrained")
    subfigs = fig.subfigures(1, 2, wspace=0.07, width_ratios=[1.5, 1.])
    subfigs[0].subplots(2, 2)
    subfigs[0].set_facecolor("lightblue")
    subfigs[0].suptitle("subfigs[0]\nLeft side")
    subfigs[0].supxlabel("xlabel for subfigs[0]")
    subfigs[1].subplots(3, 1)
    subfigs[1].suptitle("subfigs[1]")
    subfigs[1].supylabel("ylabel for subfigs[1]")


@gallery("arranging_axes_9")
def _(fig):
    inner = [["innerA"], ["innerB"]]
    outer = [["upper left", inner], ["lower left", "lower right"]]
    fig.set_layout_engine("constrained")
    for k, ax in fig.subplot_mosaic(outer).items():
        _annotate_axes(ax, f"axd[{k!r}]")


@gallery("arranging_axes_10", figsize=(5.5, 3.5))
def _(fig):
    fig.set_layout_engine("constrained")
    spec = fig.add_gridspec(ncols=2, nrows=2)
    for name, cell in [("ax0", spec[0, 0]), ("ax1", spec[0, 1]),
                       ("ax2", spec[1, 0]), ("ax3", spec[1, 1])]:
        _annotate_axes(fig.add_subplot(cell), name)
    fig.suptitle("Manually added subplots using add_gridspec")


@gallery("arranging_axes_11", figsize=(5.5, 3.5))
def _(fig):
    fig.set_layout_engine("constrained")
    spec = fig.add_gridspec(2, 2)
    for name, cell in [("ax0", spec[0, :]), ("ax10", spec[1, 0]), ("ax11", spec[1, 1])]:
        _annotate_axes(fig.add_subplot(cell), name)
    fig.suptitle("Manually added subplots, spanning a column")


@gallery("arranging_axes_12")
def _(fig):
    fig.set_facecolor("lightblue")
    gs = fig.add_gridspec(nrows=3, ncols=3, left=0.05, right=0.75, hspace=0.1, wspace=0.05)
    for name, cell in [("ax0", gs[:-1, :]), ("ax1", gs[-1, :-1]), ("ax2", gs[-1, -1])]:
        _annotate_axes(fig.add_subplot(cell), name)
    fig.suptitle("Manual gridspec with right=0.75")


@gallery("arranging_axes_13")
def _(fig):
    fig.set_layout_engine("constrained")
    gs0 = fig.add_gridspec(1, 2)
    gs00 = gs0[0].subgridspec(2, 2)
    gs01 = gs0[1].subgridspec(3, 1)
    for a in range(2):
        for b in range(2):
            ax = fig.add_subplot(gs00[a, b])
            _annotate_axes(ax, f"axLeft[{a}, {b}]", fontsize=10)
            if a == 1 and b == 1:
                ax.set_xlabel("xlabel")
    for a in range(3):
        ax = fig.add_subplot(gs01[a])
        _annotate_axes(ax, f"axRight[{a}, {b}]")
        if a == 2:
            ax.set_ylabel("ylabel")
    fig.suptitle("nested gridspecs")


# --- autoscale -----------------------------------------------------------------

_SINC_X = np.linspace(-6, 6, 201)
_SINC_Y = np.sinc(_SINC_X)


def _sinc_field():
    """``autoscale``'s image: a sinc bump centred at (1, 1)."""
    xx, yy = np.meshgrid(_SINC_X, _SINC_X)
    return np.sinc(np.sqrt((xx - 1)**2 + (yy - 1)**2))


@gallery("autoscale_1")
def _(fig):
    ax = fig.subplots()
    ax.plot(_SINC_X, _SINC_Y)


@gallery("autoscale_2")
def _(fig):
    ax = fig.subplots()
    ax.plot(_SINC_X, _SINC_Y)
    ax.margins(0.2, 0.2)


@gallery("autoscale_3")
def _(fig):
    ax = fig.subplots()
    ax.plot(_SINC_X, _SINC_Y)
    ax.margins(y=-0.2)


@gallery("autoscale_4", figsize=(12, 8))
def _(fig):
    zz = _sinc_field()
    ax = fig.subplots(ncols=2)
    ax[0].imshow(zz)
    ax[0].set_title("default margins")
    ax[1].imshow(zz)
    ax[1].margins(0.2)
    ax[1].set_title("margins(0.2)")


@gallery("autoscale_5", figsize=(16, 10))
def _(fig):
    zz = _sinc_field()
    ax = fig.subplots(ncols=3)
    ax[0].imshow(zz)
    ax[0].margins(0.2)
    ax[0].set_title("default use_sticky_edges\nmargins(0.2)")
    ax[1].imshow(zz)
    ax[1].margins(0.2)
    ax[1].use_sticky_edges = False
    ax[1].set_title("use_sticky_edges=False\nmargins(0.2)")
    ax[2].imshow(zz)
    ax[2].margins(-0.2)
    ax[2].set_title("default use_sticky_edges\nmargins(-0.2)")


@gallery("autoscale_6", figsize=(12, 8))
def _(fig):
    ax = fig.subplots(ncols=2)
    ax[0].plot(_SINC_X, _SINC_Y)
    ax[0].set_title("Single curve")
    ax[1].plot(_SINC_X, _SINC_Y)
    ax[1].plot(_SINC_X * 2.0, _SINC_Y)
    ax[1].set_title("Two curves")


@gallery("autoscale_7", figsize=(12, 8))
def _(fig):
    ax = fig.subplots(ncols=2)
    ax[0].plot(_SINC_X, _SINC_Y)
    ax[0].set_xlim(left=-1, right=1)
    ax[0].plot(_SINC_X + np.pi * 0.5, _SINC_Y)
    ax[0].set_title("set_xlim(left=-1, right=1)\n")
    ax[1].plot(_SINC_X, _SINC_Y)
    ax[1].set_xlim(left=-1, right=1)
    ax[1].plot(_SINC_X + np.pi * 0.5, _SINC_Y)
    ax[1].autoscale()
    ax[1].set_title("set_xlim(left=-1, right=1)\nautoscale()")


@gallery("autoscale_8")
def _(fig):
    ax = fig.subplots()
    ax.plot(_SINC_X, _SINC_Y)
    ax.margins(0.2, 0.2)
    ax.autoscale(enable=None, axis="x", tight=True)


@gallery("autoscale_9")
def _(fig):
    ax = fig.subplots()
    ax.plot(_SINC_X, np.sin(_SINC_X))


@gallery("autoscale_10", figsize=(9, 4))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    fig.suptitle("Margins are visually constant, even with different data limits and axis scales")
    ax1.plot([0, 10], [0, 1])
    ax1.margins(0.2)
    x = np.linspace(1, 20)
    ax2.semilogy(x, np.exp(x))
    ax2.margins(0.2)


@gallery("autoscale_11", figsize=(10, 4))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.plot([0.3, 4.7], [0.3, 4.7])
    ax1.set_title("autolimit_mode='data' (default)")
    with mpl.rc_context({"axes.autolimit_mode": "round_numbers"}):
        ax2.plot([0.3, 4.7], [0.3, 4.7])
        ax2.set_title("autolimit_mode='round_numbers'")
        ax2.autoscale_view()


# --- axes_scales ---------------------------------------------------------------

_SCALE_MOSAIC = [["linear", "linear-log"], ["log-linear", "log-log"]]


def _raised_sine():
    """``axes_scales``' curve: ``2 sin x + 3`` over ``[0, 3π)``."""
    x = np.arange(0, 3 * np.pi, 0.1)
    return x, 2 * np.sin(x) + 3


@gallery("axes_scales_1")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplot_mosaic(_SCALE_MOSAIC)
    x, y = _raised_sine()
    for name, ax in axs.items():
        xscale, yscale = name.split("-") if "-" in name else (name, name)
        ax.plot(x, y)
        ax.set_xscale(xscale)
        ax.set_yscale(yscale)
        ax.set_xlabel(xscale)
        ax.set_ylabel(yscale)


@gallery("axes_scales_2")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplot_mosaic(_SCALE_MOSAIC)
    x, y = _raised_sine()
    for name, method, xlabel, ylabel in [("linear", "plot", "linear", "linear"),
                                         ("linear-log", "semilogy", "linear", "log"),
                                         ("log-linear", "semilogx", "log", "linear"),
                                         ("log-log", "loglog", "log", "log")]:
        ax = axs[name]
        getattr(ax, method)(x, y)
        ax.set_xlabel(xlabel)
        ax.set_ylabel(ylabel)
        ax.set_title(f"{method}(x, y)")


@gallery("axes_scales_3")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplot_mosaic([["asinh", "symlog"], ["log", "logit"]])
    x = np.arange(0, 1000)
    for name, ax in axs.items():
        if name in ["asinh", "symlog"]:
            yy = x - np.mean(x)
        elif name in ["logit"]:
            yy = (x - np.min(x))
            yy = yy / np.max(np.abs(yy))
        else:
            yy = x
        ax.plot(yy, yy)
        ax.set_yscale(name)
        ax.set_title(name)


@gallery("axes_scales_4", figsize=(6.4, 3))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplot_mosaic([["log", "symlog"]])
    x = np.arange(0, 1000)
    for name, ax in axs.items():
        if name in ["log"]:
            ax.plot(x, x)
            ax.set_yscale("log", base=2)
            ax.set_title("log base=2")
        else:
            ax.plot(x - np.mean(x), x - np.mean(x))
            ax.set_yscale("symlog", linthresh=100)
            ax.set_title("symlog linthresh=100")


def _mercator(a):
    a = np.deg2rad(a)
    return np.rad2deg(np.log(np.abs(np.tan(a) + 1.0 / np.cos(a))))


def _inverse_mercator(a):
    a = np.deg2rad(a)
    return np.rad2deg(np.arctan(np.sinh(a)))


@gallery("axes_scales_5")
def _(fig):
    t = np.arange(0, 170.0, 0.1)
    s = t / 2.
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.plot(t, s, "-", lw=2)
    ax.set_yscale("function", functions=(_mercator, _inverse_mercator))
    ax.set_title("function: Mercator")
    ax.grid(True)
    ax.set_xlim(0, 180)
    ax.yaxis.set_minor_formatter(ticker.NullFormatter())
    ax.yaxis.set_major_locator(ticker.FixedLocator(np.arange(0, 90, 10)))


@gallery("axes_scales_6", figsize=(3.2, 3))
def _(fig):
    x = np.arange(0, 1000)
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.semilogy(x, x)


# --- axes_ticks ----------------------------------------------------------------


@gallery("axes_ticks_1", figsize=(5.4, 5.4))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 1)
    x = np.arange(100)
    for nn, ax in enumerate(axs):
        ax.plot(x, x)
        if nn == 1:
            ax.set_title("Manual ticks")
            ax.set_yticks(np.arange(0, 100.1, 100 / 3))
            xticks = np.arange(0.50, 101, 20)
            xlabels = [f"\\${x:1.2f}" for x in xticks]
            ax.set_xticks(xticks, labels=xlabels)
        else:
            ax.set_title("Automatic ticks")


@gallery("axes_ticks_2", figsize=(5.4, 5.4))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 1)
    x = np.arange(100)
    for nn, ax in enumerate(axs):
        ax.plot(x, x)
        if nn == 1:
            ax.set_title("Manual ticks")
            ax.set_yticks(np.arange(0, 100.1, 100 / 3))
            ax.set_yticks(np.arange(0, 100.1, 100 / 30), minor=True)
        else:
            ax.set_title("Automatic ticks")


def _tick_strip(ax, title):
    """``axes_ticks``' ``setup``: a bare strip named by ``title``."""
    ax.yaxis.set_major_locator(ticker.NullLocator())
    ax.spines[["left", "right", "top"]].set_visible(False)
    ax.xaxis.set_ticks_position("bottom")
    ax.tick_params(which="major", width=1.00, length=5)
    ax.tick_params(which="minor", width=0.75, length=2.5)
    ax.set_xlim(0, 5)
    ax.set_ylim(0, 1)
    ax.text(0.0, 0.2, title, transform=ax.transAxes,
            fontsize=14, fontname="Monospace", color="tab:blue")


@gallery("axes_ticks_3")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(8, 1)
    _tick_strip(axs[0], title="NullLocator()")
    axs[0].xaxis.set_major_locator(ticker.NullLocator())
    axs[0].xaxis.set_minor_locator(ticker.NullLocator())
    _tick_strip(axs[1], title="MultipleLocator(0.5)")
    axs[1].xaxis.set_major_locator(ticker.MultipleLocator(0.5))
    axs[1].xaxis.set_minor_locator(ticker.MultipleLocator(0.1))
    _tick_strip(axs[2], title="FixedLocator([0, 1, 5])")
    axs[2].xaxis.set_major_locator(ticker.FixedLocator([0, 1, 5]))
    axs[2].xaxis.set_minor_locator(ticker.FixedLocator(np.linspace(0.2, 0.8, 4)))
    _tick_strip(axs[3], title="LinearLocator(numticks=3)")
    axs[3].xaxis.set_major_locator(ticker.LinearLocator(3))
    axs[3].xaxis.set_minor_locator(ticker.LinearLocator(31))
    _tick_strip(axs[4], title="IndexLocator(base=0.5, offset=0.25)")
    axs[4].plot(range(0, 5), [0] * 5, color="white")
    axs[4].xaxis.set_major_locator(ticker.IndexLocator(base=0.5, offset=0.25))
    _tick_strip(axs[5], title="AutoLocator()")
    axs[5].xaxis.set_major_locator(ticker.AutoLocator())
    axs[5].xaxis.set_minor_locator(ticker.AutoMinorLocator())
    _tick_strip(axs[6], title="MaxNLocator(n=4)")
    axs[6].xaxis.set_major_locator(ticker.MaxNLocator(4))
    axs[6].xaxis.set_minor_locator(ticker.MaxNLocator(40))
    _tick_strip(axs[7], title="LogLocator(base=10, numticks=15)")
    axs[7].set_xlim(10**3, 10**10)
    axs[7].set_xscale("log")
    axs[7].xaxis.set_major_locator(ticker.LogLocator(base=10, numticks=15))


@gallery("axes_ticks_5", figsize=(6.4, 3.2))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2)
    for nn, ax in enumerate(axs):
        ax.plot(np.arange(100))
        if nn == 1:
            ax.grid("on")
            ax.tick_params(right=True, left=False, axis="y", color="r", length=16,
                           grid_color="none")
            ax.tick_params(axis="x", color="m", length=4, direction="in", width=4,
                           labelcolor="g", grid_color="b")


@gallery("axes_ticks_6")
def _(fig):
    g = rng(SEED)
    x = 1.1 * np.exp(-3 * g.uniform(500))
    y = g.uniform(500)
    ax = fig.subplots()
    ax.plot(x, y, "o")
    ax.grid()
    ticks = ax.set_xticks([0, 0.2, 0.4, 0.6, 0.8, 1, 1.2])
    ticks[5].label1.set_color("red")
    ticks[5].tick1line.set_markeredgecolor("red")
    ticks[5].gridline.set(visible=True, linestyle=":", linewidth=1.5, color="red")


# --- axes_units ----------------------------------------------------------------

_CONCISE = {"date.converter": "concise"}
_FRUIT = {"apple": 10, "orange": 15, "lemon": 5, "lime": 20}
_FLOAT_LABEL = {"rotation": 70, "color": "C1",
                "bbox": {"color": "white", "alpha": .7, "boxstyle": "round"}}


def _days_of_1980(ax):
    """Plots the day count against the days of 1980 up to June 24."""
    time = np.arange("1980-01-01", "1980-06-25", dtype="datetime64[D]")
    ax.plot(time, np.arange(len(time)))


def _fruit_bars(ax):
    ax.bar(list(_FRUIT.keys()), list(_FRUIT.values()))


@gallery("axes_units_1", figsize=(5.4, 2))
def _(fig):
    fig.set_layout_engine("constrained")
    _days_of_1980(fig.subplots())


@gallery("axes_units_2", figsize=(5.4, 2))
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    _days_of_1980(ax)
    ax.plot(0, 0, "d")
    ax.text(0, 0, " Float x=0", rotation=45)


@gallery("axes_units_3", figsize=(5.4, 2))
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    _days_of_1980(ax)
    ax.xaxis.set_major_locator(mdates.MonthLocator(bymonth=np.arange(1, 13, 2)))
    ax.xaxis.set_major_formatter(mdates.DateFormatter("%b"))
    ax.set_xlabel("1980")


@gallery("axes_units_4", figsize=(5.4, 2), rc=_CONCISE)
def _(fig):
    fig.set_layout_engine("constrained")
    _days_of_1980(fig.subplots())


@gallery("axes_units_5", figsize=(5.4, 3), rc=_CONCISE)
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 1)
    for ax in axs.flat:
        _days_of_1980(ax)
    axs[0].set_xlim(np.datetime64("1980-02-01"), np.datetime64("1980-04-01"))
    axs[1].set_xlim(3683, 3683 + 60)


@gallery("axes_units_6", figsize=(7, 3), rc=_CONCISE)
def _(fig):
    names = list(_FRUIT.keys())
    values = list(_FRUIT.values())
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 3, sharey=True)
    axs[0].bar(names, values)
    axs[1].scatter(names, values)
    axs[2].plot(names, values)
    fig.suptitle("Categorical Plotting")


@gallery("axes_units_7", figsize=(5, 3), rc=_CONCISE)
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    _fruit_bars(ax)
    ax.scatter(["lemon", "apple"], [7, 12])
    ax.plot(["pear", "orange", "apple", "lemon"], [13, 10, 7, 12], color="C1")


@gallery("axes_units_8", figsize=(5, 3), rc=_CONCISE)
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    _fruit_bars(ax)
    for x in [0, 2, 4, 2.5]:
        ax.plot(x, 2, "d", color="C1")
        ax.text(x, 3, f"Float x={x}", **_FLOAT_LABEL)


@gallery("axes_units_9", figsize=(5, 5), rc=_CONCISE)
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 1)
    ax = axs[0]
    _fruit_bars(ax)
    ax.set_xlim("orange", "lemon")
    ax.set_xlabel("limits set with categories")
    ax = axs[1]
    _fruit_bars(ax)
    ax.set_xlim(0.5, 2.5)
    ax.set_xlabel("limits set with floats")


@gallery("axes_units_10", figsize=(5.4, 2.5), rc=_CONCISE)
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    x = [str(xx) for xx in np.arange(100)]
    ax.plot(x, np.arange(100))
    ax.set_xlabel("x is list of strings")


@gallery("axes_units_11", figsize=(5.4, 2.5), rc=_CONCISE)
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    x = np.asarray([str(xx) for xx in np.arange(100)], dtype="float")
    ax.plot(x, np.arange(100))
    ax.set_xlabel("x is array of floats")


# --- colorbar_placement --------------------------------------------------------


def _cmap_columns(fig, g, shared):
    """``colorbar_placement``'s first figures: random meshes in a 2 × 2 grid,
    blue-red in the left column and viridis (and twice the values) in the
    right one, with a colorbar per axes or, ``shared``, per column."""
    axs = fig.subplots(2, 2)
    cmaps = ["RdBu_r", "viridis"]
    for col in range(2):
        for row in range(2):
            ax = axs[row, col]
            pcm = ax.pcolormesh(g.uniform(400).reshape(20, 20) * (col + 1), cmap=cmaps[col])
            if not shared:
                fig.colorbar(pcm, ax=ax)
        if shared:
            fig.colorbar(pcm, ax=axs[:, col], shrink=0.6)


def _summed_mesh(fig, layout):
    """A random mesh with a colorbar under the line of its column sums."""
    X = rng(SEED).normal(400).reshape(20, 20)
    fig.set_layout_engine(layout)
    axs = fig.subplots(2, 1, sharex=True)
    axs[0].plot(np.sum(X, axis=0))
    pcm = axs[1].pcolormesh(X)
    fig.colorbar(pcm, ax=axs[1], shrink=0.6)


def _padded_colorbars(fig, layout, title):
    """Three random meshes with colorbars at increasing pads."""
    g = rng(SEED)
    fig.set_layout_engine(layout)
    axs = fig.subplots(3, 1)
    for ax, pad in zip(axs, [0.025, 0.05, 0.1]):
        pcm = ax.pcolormesh(g.normal(400).reshape(20, 20), cmap="viridis")
        fig.colorbar(pcm, ax=ax, pad=pad, label=f"pad: {pad}")
    fig.suptitle(title)


def _inset_colorbar(fig, bounds, in_data=False):
    """A random mesh with a horizontal colorbar inset at ``bounds``, in axes
    or (``in_data``) data coordinates."""
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    pcm = ax.pcolormesh(rng(SEED).normal(400).reshape(20, 20), cmap="viridis")
    ax.set_ylim([-4, 20])
    cax = ax.inset_axes(bounds, transform=ax.transData if in_data else None)
    fig.colorbar(pcm, cax=cax, orientation="horizontal")


def _random_image(fig, layout):
    """A 10 × 10 random image on its own axes, in ``layout``."""
    fig.set_layout_engine(layout)
    ax = fig.subplots()
    return ax, ax.imshow(rng(SEED).normal(100).reshape(10, 10), cmap="viridis")


@gallery("colorbar_placement_1")
def _(fig):
    _cmap_columns(fig, rng(SEED), shared=False)


@gallery("colorbar_placement_2")
def _(fig):
    _cmap_columns(fig, rng(SEED), shared=True)


@gallery("colorbar_placement_3", figsize=(4, 5))
def _(fig):
    _summed_mesh(fig, None)


@gallery("colorbar_placement_4", figsize=(4, 5))
def _(fig):
    _summed_mesh(fig, "constrained")


@gallery("colorbar_placement_5")
def _(fig):
    g = rng(SEED)
    fig.set_layout_engine("constrained")
    axs = fig.subplots(3, 3)
    for ax in axs.flat:
        pcm = ax.pcolormesh(g.uniform(400).reshape(20, 20))
    fig.colorbar(pcm, ax=axs[0, :2], shrink=0.6, location="bottom")
    fig.colorbar(pcm, ax=[axs[0, 2]], location="bottom")
    fig.colorbar(pcm, ax=axs[1:, :], location="right", shrink=0.6)
    fig.colorbar(pcm, ax=[axs[2, 1]], location="left")


@gallery("colorbar_placement_6", figsize=(5, 5))
def _(fig):
    _padded_colorbars(fig, "constrained", "layout='constrained'")


@gallery("colorbar_placement_7", figsize=(5, 5))
def _(fig):
    _padded_colorbars(fig, None, "No layout manager")


@gallery("colorbar_placement_8", figsize=(4, 4))
def _(fig):
    _inset_colorbar(fig, [0.3, 0.07, 0.4, 0.04])


@gallery("colorbar_placement_9", figsize=(4, 4))
def _(fig):
    _inset_colorbar(fig, [7.5, -1.7, 5, 1.2], in_data=True)


@gallery("colorbar_placement_10", figsize=(4, 4))
def _(fig):
    ax, pcm = _random_image(fig, "constrained")
    fig.colorbar(pcm, ax=ax)


@gallery("colorbar_placement_11", figsize=(4, 4))
def _(fig):
    ax, pcm = _random_image(fig, "compressed")
    ax.set_title("Colorbar with layout='compressed'", fontsize="medium")
    fig.colorbar(pcm, ax=ax)


@gallery("colorbar_placement_12", figsize=(4, 4))
def _(fig):
    ax, pcm = _random_image(fig, "constrained")
    cax = ax.inset_axes([1.04, 0.0, 0.05, 1.0])
    ax.set_title("Colorbar with inset_axes", fontsize="medium")
    fig.colorbar(pcm, cax=cax)


# --- constrainedlayout_guide ---------------------------------------------------

#: The rcParams the constrained and tight layout guides set first.
_LAYOUT_RC = {"savefig.facecolor": "0.8"}
_ARR = np.arange(100).reshape((10, 10))
_PC_KWARGS = {"rasterized": True, "cmap": "viridis",
              "norm": mcolors.Normalize(vmin=0., vmax=100.)}


def constrained(name, figsize=(4.5, 4.), rc=None):
    """Registers a figure of the constrained layout guide, on its
    ``figure.figsize`` of 4.5 × 4 in unless the figure sets its own."""
    return gallery(f"constrainedlayout_guide_{name}", figsize=figsize,
                   rc={**_LAYOUT_RC, **(rc or {})})


def _example_plot(ax, fontsize=12, hide_labels=False):
    """The layout guides' ``example_plot``: a line with three ticks per axis,
    labelled and titled at ``fontsize`` unless ``hide_labels``, which
    blanks the tick labels instead."""
    ax.plot([1, 2])
    ax.locator_params(nbins=3)
    if hide_labels:
        ax.set_xticklabels([])
        ax.set_yticklabels([])
    else:
        ax.set_xlabel("x-label", fontsize=fontsize)
        ax.set_ylabel("y-label", fontsize=fontsize)
        ax.set_title("Title", fontsize=fontsize)


def _example_grid(fig, nrows, ncols, layout, **example):
    """``example_plot`` on every axes of an ``nrows`` × ``ncols`` grid."""
    fig.set_layout_engine(layout)
    axs = fig.subplots(nrows, ncols, squeeze=False)
    for ax in axs.flat:
        _example_plot(ax, **example)
    return axs


def _meshes(fig, nrows, ncols):
    """A constrained grid of ``_ARR`` meshes; returns the axes and the last
    mesh."""
    fig.set_layout_engine("constrained")
    axs = fig.subplots(nrows, ncols)
    for ax in np.ravel(axs):
        im = ax.pcolormesh(_ARR, **_PC_KWARGS)
    return axs, im


def _spaced_grid(fig, nrows, ncols, hspace, wspace, gridspec_kw=None):
    """Label-less example plots in a constrained grid with 4 pt pads and the
    given relative spaces."""
    fig.set_layout_engine("constrained")
    for ax in fig.subplots(nrows, ncols, gridspec_kw=gridspec_kw).flat:
        _example_plot(ax, hide_labels=True)
    fig.get_layout_engine().set(w_pad=4 / 72, h_pad=4 / 72, hspace=hspace, wspace=wspace)


def _legend_pair(fig):
    """Two constrained axes, the right one with a labelled line; returns the
    right axes and its lines."""
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2)
    axs[0].plot(np.arange(10))
    return axs[1], axs[1].plot(np.arange(10), label="This is a plot")


def _colorbar_title(fig, layout, title, zoom=False):
    """A random 10 × 10 image with a colorbar, titled ``title``; ``zoom``
    limits it to rows 4 to 8."""
    fig.set_layout_engine(layout)
    ax = fig.subplots()
    pcm = ax.imshow(rng(SEED).normal(100).reshape(10, 10), cmap="viridis")
    if zoom:
        ax.set_ylim([4, 8])
    ax.set_title(title, fontsize="medium")
    fig.colorbar(pcm, ax=ax)


def _fixed_aspect_images(fig, layout):
    """Four shared ``_ARR`` images in a 2 × 2 grid laid out by ``layout``."""
    fig.set_layout_engine(layout)
    for ax in fig.subplots(2, 2, sharex=True, sharey=True).flat:
        ax.imshow(_ARR)
    fig.suptitle(f"fixed-aspect plots, layout='{layout}'")


@constrained("1")
def _(fig):
    _example_grid(fig, 1, 1, None, fontsize=24)


@constrained("2")
def _(fig):
    _example_grid(fig, 1, 1, "constrained", fontsize=24)


@constrained("3")
def _(fig):
    _example_grid(fig, 2, 2, None)


@constrained("4")
def _(fig):
    _example_grid(fig, 2, 2, "constrained")


@constrained("5", figsize=(4, 4))
def _(fig):
    axs, im = _meshes(fig, 1, 1)
    fig.colorbar(im, ax=axs, shrink=0.6)


@constrained("6", figsize=(4, 4))
def _(fig):
    axs, im = _meshes(fig, 2, 2)
    fig.colorbar(im, ax=axs, shrink=0.6)


@constrained("7", figsize=(4, 4))
def _(fig):
    axs, im = _meshes(fig, 3, 3)
    fig.colorbar(im, ax=axs[1:, 1], shrink=0.8)
    fig.colorbar(im, ax=axs[:, -1], shrink=0.6)


@constrained("8", figsize=(4, 4))
def _(fig):
    axs, im = _meshes(fig, 2, 2)
    fig.colorbar(im, ax=axs, shrink=0.6)
    fig.suptitle("Big Suptitle")


@constrained("9")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.plot(np.arange(10), label="This is a plot")
    ax.legend(loc="center left", bbox_to_anchor=(0.8, 0.5))


@constrained("10", figsize=(4, 2))
def _(fig):
    ax, _ = _legend_pair(fig)
    ax.legend(loc="center left", bbox_to_anchor=(0.8, 0.5))


@constrained("11", figsize=(4, 2))
def _(fig):
    ax, _ = _legend_pair(fig)
    leg = ax.legend(loc="center left", bbox_to_anchor=(0.8, 0.5))
    leg.set_in_layout(False)
    fig.canvas.draw()
    leg.set_in_layout(True)
    fig.set_layout_engine("none")


@constrained("12", figsize=(4, 2))
def _(fig):
    ax, lines = _legend_pair(fig)
    labels = [line.get_label() for line in lines]
    fig.legend(lines, labels, loc="center left", bbox_to_anchor=(0.8, 0.5),
               bbox_transform=ax.transAxes)


@constrained("13")
def _(fig):
    _spaced_grid(fig, 2, 2, hspace=0, wspace=0)


@constrained("14")
def _(fig):
    _spaced_grid(fig, 2, 2, hspace=0.2, wspace=0.2)


@constrained("15")
def _(fig):
    _spaced_grid(fig, 2, 3, hspace=0.2, wspace=0.2)


@constrained("16")
def _(fig):
    _spaced_grid(fig, 2, 2, hspace=0.0, wspace=0.0, gridspec_kw={"wspace": 0.3, "hspace": 0.2})


@constrained("17")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for pad, ax in zip([0, 0.05, 0.1, 0.2], axs.flat):
        pc = ax.pcolormesh(_ARR, **_PC_KWARGS)
        fig.colorbar(pc, ax=ax, shrink=0.6, pad=pad)
        ax.set_xticklabels([])
        ax.set_yticklabels([])
        ax.set_title(f"pad: {pad}")
    fig.get_layout_engine().set(w_pad=2 / 72, h_pad=2 / 72, hspace=0.2, wspace=0.2)


@constrained("18", figsize=(3, 3), rc={"figure.constrained_layout.use": True})
def _(fig):
    for ax in fig.subplots(2, 2).flat:
        _example_plot(ax)


@constrained("19")
def _(fig):
    fig.set_layout_engine("constrained")
    gs1 = gridspec.GridSpec(2, 1, figure=fig)
    _example_plot(fig.add_subplot(gs1[0]))
    _example_plot(fig.add_subplot(gs1[1]))


@constrained("20")
def _(fig):
    fig.set_layout_engine("constrained")
    gs0 = fig.add_gridspec(1, 2)
    gs1 = gs0[0].subgridspec(2, 1)
    _example_plot(fig.add_subplot(gs1[0]))
    _example_plot(fig.add_subplot(gs1[1]))
    gs2 = gs0[1].subgridspec(3, 1)
    for ss in gs2:
        ax = fig.add_subplot(ss)
        _example_plot(ax)
        ax.set_title("")
        ax.set_xlabel("")
    ax.set_xlabel("x-label", fontsize=12)


@constrained("21", figsize=(4, 6))
def _(fig):
    fig.set_layout_engine("constrained")
    gs0 = fig.add_gridspec(6, 2)
    _example_plot(fig.add_subplot(gs0[:3, 0]))
    _example_plot(fig.add_subplot(gs0[3:, 0]))
    for rows in [slice(0, 2), slice(2, 4), slice(4, None)]:
        _example_plot(fig.add_subplot(gs0[rows, 1]), hide_labels=True)
    fig.suptitle("Overlapping Gridspecs")


def _labelled_meshes(axs):
    """``_ARR`` meshes on ``axs``, each with labels and a title; returns the
    last mesh."""
    for ax in axs:
        pcm = ax.pcolormesh(_ARR, **_PC_KWARGS)
        ax.set_xlabel("x-label")
        ax.set_ylabel("y-label")
        ax.set_title("title")
    return pcm


@constrained("22")
def _(fig):
    fig.set_layout_engine("constrained")
    gs0 = fig.add_gridspec(1, 2, figure=fig, width_ratios=[1, 2])
    gs_left = gs0[0].subgridspec(2, 1)
    gs_right = gs0[1].subgridspec(2, 2)
    for gs in gs_left:
        _example_plot(fig.add_subplot(gs))
    axs = [fig.add_subplot(gs) for gs in gs_right]
    pcm = _labelled_meshes(axs)
    fig.suptitle("Nested plots using subgridspec")
    fig.colorbar(pcm, ax=axs)


@constrained("23")
def _(fig):
    fig.set_layout_engine("constrained")
    sfigs = fig.subfigures(1, 2, width_ratios=[1, 2])
    for ax in sfigs[0].subplots(2, 1).flat:
        _example_plot(ax)
    axs_right = sfigs[1].subplots(2, 2)
    pcm = _labelled_meshes(axs_right.flat)
    fig.colorbar(pcm, ax=axs_right)
    fig.suptitle("Nested plots using subfigures")


@constrained("24")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2)
    _example_plot(axs[0], fontsize=12)
    axs[1].set_position([0.2, 0.2, 0.4, 0.4])


@constrained("25", figsize=(5, 3))
def _(fig):
    _fixed_aspect_images(fig, "constrained")


@constrained("26", figsize=(5, 3))
def _(fig):
    _fixed_aspect_images(fig, "compressed")


@constrained("27", figsize=(3, 3))
def _(fig):
    _colorbar_title(fig, "constrained", "Colorbar with layout='constrained'")


@constrained("28", figsize=(3, 3))
def _(fig):
    _colorbar_title(fig, "compressed", "Colorbar with layout='compressed'")


@constrained("29", figsize=(3, 3))
def _(fig):
    _colorbar_title(fig, "compressed", "Layout='compressed' with zoom", zoom=True)


@constrained("30")
def _(fig):
    fig.set_layout_engine("constrained")
    for ax in [fig.add_subplot(2, 2, 1), fig.add_subplot(2, 2, 3), fig.add_subplot(2, 2, (2, 4))]:
        _example_plot(ax)
    fig.suptitle("Homogeneous nrows, ncols")


@constrained("31")
def _(fig):
    fig.set_layout_engine("constrained")
    for ax in [fig.add_subplot(2, 2, 1), fig.add_subplot(2, 2, 3), fig.add_subplot(1, 2, 2)]:
        _example_plot(ax)
    fig.suptitle("Mixed nrows, ncols")


def _subplot2grid(fig):
    """``plt.subplot2grid``'s four axes of the layout guides on a 3 × 3 grid,
    each an ``example_plot``."""
    gs = fig.add_gridspec(3, 3)
    for cell in [gs[0, 0], gs[0, 1:], gs[1:, :2], gs[1:, 2]]:
        _example_plot(fig.add_subplot(cell))


@constrained("32")
def _(fig):
    fig.set_layout_engine("constrained")
    _subplot2grid(fig)
    fig.suptitle("subplot2grid")


# --- legend_guide --------------------------------------------------------------


def _outside_legends(fig, label, locations):
    """A light grey constrained figure with a figure legend titled by its
    location at each of ``locations``."""
    fig.set_layout_engine("constrained")
    fig.set_facecolor("0.95")
    ax = fig.subplots()
    ax.plot([1, 2], [1, 2], label=label)
    for loc in locations:
        fig.legend(loc=loc, title=loc)


def _rising_falling(ax, labels, fmts=("", "")):
    """A rising and a falling line labelled ``labels``."""
    ax.plot([1, 2, 3], fmts[0], label=labels[0])
    ax.plot([3, 2, 1], fmts[1], label=labels[1])


@gallery("legend_guide_1")
def _(fig):
    ax = fig.subplots()
    red_patch = mpatches.Patch(color="red", label="The red data")
    ax.legend(handles=[red_patch])


@gallery("legend_guide_2")
def _(fig):
    ax = fig.subplots()
    blue_line = mlines.Line2D([], [], color="blue", marker="*", markersize=15, label="Blue stars")
    ax.legend(handles=[blue_line])


@gallery("legend_guide_3")
def _(fig):
    ax_dict = fig.subplot_mosaic([["top", "top"], ["bottom", "BLANK"]], empty_sentinel="BLANK")
    _rising_falling(ax_dict["top"], ["test1", "test2"])
    ax_dict["top"].legend(bbox_to_anchor=(0., 1.02, 1., .102), loc="lower left",
                          ncols=2, mode="expand", borderaxespad=0.)
    _rising_falling(ax_dict["bottom"], ["test1", "test2"])
    ax_dict["bottom"].legend(bbox_to_anchor=(1.05, 1), loc="upper left", borderaxespad=0.)


@gallery("legend_guide_4")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplot_mosaic([["left", "right"]])
    _rising_falling(axs["left"], ["test1", "test2"])
    _rising_falling(axs["right"], ["test3", "test4"], fmts=("C2", "C3"))
    fig.legend(loc="outside upper right")


@gallery("legend_guide_5", figsize=(6, 4))
def _(fig):
    _outside_legends(fig, "TEST", [f"outside {v} {h}" for v in ["upper", "lower"]
                                   for h in ["left", "center", "right"]])


@gallery("legend_guide_6", figsize=(6, 4))
def _(fig):
    _outside_legends(fig, "test", [f"outside {h} {v}" for v in ["upper", "center", "lower"]
                                   for h in ["left", "right"]])


@gallery("legend_guide_7")
def _(fig):
    ax = fig.subplots()
    line1, = ax.plot([1, 2, 3], label="Line 1", linestyle="--")
    line2, = ax.plot([3, 2, 1], label="Line 2", linewidth=4)
    first_legend = ax.legend(handles=[line1], loc="upper right")
    ax.add_artist(first_legend)
    ax.legend(handles=[line2], loc="lower right")


@gallery("legend_guide_8")
def _(fig):
    ax = fig.subplots()
    line1, = ax.plot([3, 2, 1], marker="o", label="Line 1")
    ax.plot([1, 2, 3], marker="o", label="Line 2")
    ax.legend(handler_map={line1: HandlerLine2D(numpoints=4)}, handlelength=4)


@gallery("legend_guide_9")
def _(fig):
    z = rng(SEED).normal(10)
    ax = fig.subplots()
    red_dot, = ax.plot(z, "ro", markersize=15)
    white_cross, = ax.plot(z[:5], "w+", markeredgewidth=3, markersize=15)
    ax.legend([red_dot, (red_dot, white_cross)], ["Attr A", "Attr A+B"])


@gallery("legend_guide_10")
def _(fig):
    ax = fig.subplots()
    p1, = ax.plot([1, 2.5, 3], "r-d")
    p2, = ax.plot([3, 2, 1], "k-o")
    ax.legend([(p1, p2)], ["Two keys"], numpoints=1, handler_map={tuple: HandlerTuple(ndivide=None)})


class _AnyObject:
    """``legend_guide``'s handle that is no artist."""


class _AnyObjectHandler:
    """``legend_guide``'s handler of ``_AnyObject``: a red, hatched rectangle
    filling the handle box."""

    def legend_artist(self, legend, orig_handle, fontsize, handlebox):
        x0, y0 = handlebox.xdescent, handlebox.ydescent
        width, height = handlebox.width, handlebox.height
        patch = mpatches.Rectangle([x0, y0], width, height, facecolor="red",
                                   edgecolor="black", hatch="xx", lw=3,
                                   transform=handlebox.get_transform())
        handlebox.add_artist(patch)
        return patch


@gallery("legend_guide_11")
def _(fig):
    ax = fig.subplots()
    ax.legend([_AnyObject()], ["My first handler"], handler_map={_AnyObject: _AnyObjectHandler()})


# --- mosaic --------------------------------------------------------------------


def mosaic(name, **options):
    """Registers a figure of the ``subplot_mosaic`` guide."""
    return gallery(f"mosaic_{name}", **options)


def _identify_axes(ax_dict, fontsize=48):
    """``mosaic``'s ``identify_axes``: every axes' key, big, in its middle."""
    kw = dict(ha="center", va="center", fontsize=fontsize, color="darkgrey")
    for k, ax in ax_dict.items():
        ax.text(0.5, 0.5, k, transform=ax.transAxes, **kw)


def _identified(fig, layout, fontsize=48, **kwargs):
    """A constrained mosaic of ``layout`` with every axes identified."""
    fig.set_layout_engine("constrained")
    axd = fig.subplot_mosaic(layout, **kwargs)
    _identify_axes(axd, fontsize)
    return axd


def _plot_kinds(axes, hist_bins):
    """``mosaic``'s bar, line, histogram and image axes."""
    bar, plot, hist, image = axes
    bar.bar(["a", "b", "c"], [5, 7, 9])
    plot.plot([1, 2, 3])
    hist.hist(rng(SEED).normal(1_500), bins=hist_bins)
    image.imshow([[1, 2], [2, 1]])


def _zigzags(axd):
    """The same zigzag on every axes of the sharing figures."""
    for ax in axd.values():
        ax.plot([1, 2, 3, 4], [1, 4, 2, 3])


@mosaic("1")
def _(fig):
    fig.set_layout_engine("constrained")
    ax_array = fig.subplots(2, 2, squeeze=False)
    _plot_kinds(ax_array.flat, "auto")
    _identify_axes({(j, k): a for j, r in enumerate(ax_array) for k, a in enumerate(r)})


@mosaic("2")
def _(fig):
    fig.set_layout_engine("constrained")
    ax_dict = fig.subplot_mosaic([["bar", "plot"], ["hist", "image"]])
    _plot_kinds([ax_dict[k] for k in ["bar", "plot", "hist", "image"]], 10)
    _identify_axes(ax_dict)


@mosaic("3")
def _(fig):
    _identified(fig, """
    AB
    CD
    """)


@mosaic("5")
def _(fig):
    _identified(fig, """
    ABD
    CCD
    """)


@mosaic("6")
def _(fig):
    _identified(fig, """
    A.C
    BBB
    .D.
    """)


@mosaic("7")
def _(fig):
    _identified(fig, """
    aX
    Xb
    """, empty_sentinel="X")


@mosaic("8")
def _(fig):
    _identified(fig, """αб
       ℝ☢""")


@mosaic("9")
def _(fig):
    _identified(fig, """
    .a.
    bAc
    .d.
    """, height_ratios=[1, 3.5, 1], width_ratios=[1, 3.5, 1])


@mosaic("10")
def _(fig):
    layout = """AA
            BC"""
    for bottom, top, left, right in [(0.25, 0.95, 0.1, 0.5), (0.05, 0.75, 0.6, 0.95)]:
        axd = fig.subplot_mosaic(layout, gridspec_kw={
            "bottom": bottom, "top": top, "left": left, "right": right,
            "wspace": 0.5, "hspace": 0.5,
        })
        _identify_axes(axd)


@mosaic("11")
def _(fig):
    layout = """AA
            BC"""
    fig.set_layout_engine("constrained")
    for subfig in fig.subfigures(nrows=1, ncols=2):
        _identify_axes(subfig.subplot_mosaic(layout))


@mosaic("12")
def _(fig):
    _identified(fig, "AB", subplot_kw={"projection": "polar"})


@mosaic("13")
def _(fig):
    _identify_axes(fig.subplot_mosaic("AB;CD", per_subplot_kw={
        "A": {"projection": "polar"},
        ("C", "D"): {"xscale": "log"},
    }))


@mosaic("14")
def _(fig):
    _identify_axes(fig.subplot_mosaic("AB;CD", per_subplot_kw={
        "AD": {"projection": "polar"},
        "BC": {"facecolor": ".9"},
    }))


@mosaic("15")
def _(fig):
    _identified(fig, "AB;CD", subplot_kw={"facecolor": "xkcd:tangerine"}, per_subplot_kw={
        "B": {"facecolor": "xkcd:water blue"},
        "D": {"projection": "polar", "facecolor": "w"},
    })


@mosaic("16")
def _(fig):
    _identified(fig, [["main", "zoom"], ["main", "BLANK"]], empty_sentinel="BLANK",
                width_ratios=[2, 1])


@mosaic("17")
def _(fig):
    inner = [["inner A"], ["inner B"]]
    _identified(fig, [["main", inner], ["bottom", "bottom"]], fontsize=36, empty_sentinel=None)


@mosaic("18")
def _(fig):
    layout = np.zeros((4, 4), dtype=int)
    for j in range(4):
        layout[j, j] = j + 1
    _identified(fig, layout, empty_sentinel=0)


@mosaic("19")
def _(fig):
    fig.set_layout_engine("constrained")
    axd = fig.subplot_mosaic("AC;BC", sharex=True)
    for k in "ABC":
        axd[k].set_title(f"Axes {k}")
    _zigzags(axd)
    _identify_axes(axd)


@mosaic("20")
def _(fig):
    fig.set_layout_engine("constrained")
    axd = fig.subplot_mosaic(".A;BC", sharey=True)
    _zigzags(axd)
    _identify_axes(axd)


@mosaic("21")
def _(fig):
    fig.set_layout_engine("constrained")
    axd = fig.subplot_mosaic(".A;BC", sharey=True)
    _zigzags(axd)
    axd["A"].tick_params(labelleft=True)
    _identify_axes(axd)


# --- tight_layout_guide --------------------------------------------------------


def tight(name, **options):
    """Registers a figure of the tight layout guide."""
    return gallery(f"tight_layout_guide_{name}", rc=_LAYOUT_RC, **options)


def _tight_examples(fig, axes, **params):
    """``example_plot`` on ``axes``, then ``tight_layout(**params)``."""
    for ax in axes:
        _example_plot(ax)
    fig.tight_layout(**params)


def _gridspec_column(fig):
    """Two example plots on a 2 × 1 ``GridSpec``; returns the grid."""
    gs1 = gridspec.GridSpec(2, 1)
    _example_plot(fig.add_subplot(gs1[0]))
    _example_plot(fig.add_subplot(gs1[1]))
    return gs1


def _legend_line(fig, label, in_layout):
    """A labelled line with a legend leaning out of the axes, tightly laid
    out with or without the legend."""
    ax = fig.subplots()
    ax.plot(range(10), label=label)
    leg = ax.legend(bbox_to_anchor=(0.7, 0.5), loc="center left")
    leg.set_in_layout(in_layout)
    fig.tight_layout()


def _imshow_colorbar(fig, axes_locatable):
    """An image of ``_ARR`` with a colorbar, tightly laid out; the colorbar
    axes is appended by ``make_axes_locatable`` when ``axes_locatable``."""
    ax = fig.add_subplot()
    im = ax.imshow(_ARR, interpolation="none")
    if axes_locatable:
        cax = make_axes_locatable(ax).append_axes("right", "5%", pad="3%")
        fig.colorbar(im, cax=cax)
    else:
        fig.colorbar(im)
    fig.tight_layout()


@tight("1")
def _(fig):
    _example_plot(fig.subplots(), fontsize=24)


@tight("2")
def _(fig):
    _example_plot(fig.subplots(), fontsize=24)
    fig.tight_layout()


@tight("3")
def _(fig):
    for ax in fig.subplots(nrows=2, ncols=2).flat:
        _example_plot(ax)


@tight("4")
def _(fig):
    _tight_examples(fig, fig.subplots(nrows=2, ncols=2).flat)


@tight("5")
def _(fig):
    _tight_examples(fig, fig.subplots(nrows=2, ncols=2).flat, pad=0.4, w_pad=0.5, h_pad=1.0)


@tight("6")
def _(fig):
    _tight_examples(fig, [fig.add_subplot(221), fig.add_subplot(223), fig.add_subplot(122)])


@tight("7")
def _(fig):
    _subplot2grid(fig)
    fig.tight_layout()


@tight("8", figsize=(5, 4))
def _(fig):
    ax = fig.add_subplot()
    ax.imshow(_ARR, interpolation="none")
    fig.tight_layout()


@tight("9")
def _(fig):
    _gridspec_column(fig).tight_layout(fig)


@tight("10")
def _(fig):
    _gridspec_column(fig).tight_layout(fig, rect=[0, 0, 0.5, 1.0])


@tight("11", figsize=(4, 3))
def _(fig):
    _legend_line(fig, "A simple plot", True)


@tight("12", figsize=(4, 3))
def _(fig):
    _legend_line(fig, "B simple plot", False)


@tight("13")
def _(fig):
    grid = Grid(fig, rect=111, nrows_ncols=(2, 2), axes_pad=0.25, label_mode="L")
    for ax in grid:
        _example_plot(ax)
    ax.title.set_visible(False)
    fig.tight_layout()


@tight("14", figsize=(4, 4))
def _(fig):
    _imshow_colorbar(fig, axes_locatable=False)


@tight("15", figsize=(4, 4))
def _(fig):
    _imshow_colorbar(fig, axes_locatable=True)


# --- color_cycle ---------------------------------------------------------------


@gallery("color_cycle", rc={
    "lines.linewidth": 4,
    "axes.prop_cycle": cycler(color=["r", "g", "b", "y"]) + cycler(linestyle=["-", "--", ":", "-."]),
})
def _(fig):
    x = np.linspace(0, 2 * np.pi, 50)
    offsets = np.linspace(0, 2 * np.pi, 4, endpoint=False)
    yy = np.transpose([np.sin(x + phi) for phi in offsets])
    custom_cycler = cycler(color=["c", "m", "y", "k"]) + cycler(lw=[1, 2, 3, 4])
    ax0, ax1 = fig.subplots(nrows=2)
    ax0.plot(yy)
    ax0.set_title("Set default color cycle to rgby")
    ax1.set_prop_cycle(custom_cycler)
    ax1.plot(yy)
    ax1.set_title("Set axes color cycle to cmyk")
    fig.subplots_adjust(hspace=0.3)


# --- imshow_extent -------------------------------------------------------------


def _index_to_coordinate(index, extent, origin):
    """The pixel center of an index."""
    left, right, bottom, top = extent
    hshift = 0.5 * np.sign(right - left)
    left, right = left + hshift, right - hshift
    vshift = 0.5 * np.sign(top - bottom)
    bottom, top = bottom + vshift, top - vshift
    if origin == "upper":
        bottom, top = top, bottom
    return {
        "[0, 0]": (left, bottom),
        "[M', 0]": (left, top),
        "[0, N']": (right, bottom),
        "[M', N']": (right, top),
    }[index]


def _lookup_extent(origin):
    """The extent for label positioning when none is given."""
    if origin == "lower":
        return (-0.5, 6.5, -0.5, 5.5)
    return (-0.5, 6.5, 5.5, -0.5)


def _index_label_pos(index, extent, origin, inverted_xindex):
    """The position and horizontal alignment of an index label."""
    if extent is None:
        extent = _lookup_extent(origin)
    left, right, bottom, top = extent
    x, y = _index_to_coordinate(index, extent, origin)
    is_x0 = index[-2:] == "0]"
    halign = "left" if is_x0 ^ inverted_xindex else "right"
    hshift = 0.5 * np.sign(left - right)
    x += hshift * (1 if is_x0 else -1)
    return x, y, halign


def _index_color(index, data, cmap):
    """The data color of an index."""
    val = {
        "[0, 0]": data[0, 0],
        "[0, N']": data[0, -1],
        "[M', 0]": data[-1, 0],
        "[M', N']": data[-1, -1],
    }[index]
    return cmap(val / data.max())


def _extent_none_text(ax):
    ax.text(3, 2.5, "equals\nextent=None", size="large", ha="center", va="center", color="w")


def _imshow_with_labels(ax, data, extent, origin, xlim, ylim):
    """``imshow()`` with extent and index labels."""
    im = ax.imshow(data, origin=origin, extent=extent)
    left, right, bottom, top = im.get_extent()
    if xlim is None or top > bottom:
        upper_string, lower_string = "top", "bottom"
    else:
        upper_string, lower_string = "bottom", "top"
    if ylim is None or left < right:
        port_string, starboard_string = "left", "right"
        inverted_xindex = False
    else:
        port_string, starboard_string = "right", "left"
        inverted_xindex = True
    bbox_kwargs = {"fc": "w", "alpha": .75, "boxstyle": "round4"}
    ann_kwargs = {"xycoords": "axes fraction", "textcoords": "offset points", "bbox": bbox_kwargs}
    ax.annotate(upper_string, xy=(.5, 1), xytext=(0, -1), ha="center", va="top", **ann_kwargs)
    ax.annotate(lower_string, xy=(.5, 0), xytext=(0, 1), ha="center", va="bottom", **ann_kwargs)
    ax.annotate(port_string, xy=(0, .5), xytext=(1, 0), ha="left", va="center", rotation=90,
                **ann_kwargs)
    ax.annotate(starboard_string, xy=(1, .5), xytext=(-1, 0), ha="right", va="center",
                rotation=-90, **ann_kwargs)
    ax.set_title(f"origin: {origin}")
    for index in ["[0, 0]", "[0, N']", "[M', 0]", "[M', N']"]:
        tx, ty, halign = _index_label_pos(index, extent, origin, inverted_xindex)
        facecolor = _index_color(index, data, im.get_cmap())
        ax.text(tx, ty, index, color="white", ha=halign, va="center",
                bbox={"boxstyle": "square", "facecolor": facecolor})
    if xlim:
        ax.set_xlim(*xlim)
    if ylim:
        ax.set_ylim(*ylim)


def _imshow_demo_grid(fig, extents, xlim=None, ylim=None):
    """``generate_imshow_demo_grid``: a row per extent with its values and
    the image drawn from the upper and from the lower origin."""
    N = len(extents)
    fig.set_layout_engine("tight")
    gs = gridspec.GridSpec(N, 5, figure=fig)
    columns = {"label": [fig.add_subplot(gs[j, 0]) for j in range(N)],
               "upper": [fig.add_subplot(gs[j, 1:3]) for j in range(N)],
               "lower": [fig.add_subplot(gs[j, 3:5]) for j in range(N)]}
    x, y = np.ogrid[0:6, 0:7]
    data = x + y
    for origin in ["upper", "lower"]:
        for ax, extent in zip(columns[origin], extents):
            _imshow_with_labels(ax, data, extent, origin, xlim, ylim)
    columns["label"][0].set_title("extent=")
    for ax, extent in zip(columns["label"], extents):
        if extent is None:
            text = "None"
        else:
            left, right, bottom, top = extent
            text = (f"left: {left:0.1f}\nright: {right:0.1f}\n"
                    f"bottom: {bottom:0.1f}\ntop: {top:0.1f}\n")
        ax.text(1., .5, text, transform=ax.transAxes, ha="right", va="center")
        ax.axis("off")
    return columns


_EXTENTS = [(-0.5, 6.5, -0.5, 5.5),
            (-0.5, 6.5, 5.5, -0.5),
            (6.5, -0.5, -0.5, 5.5),
            (6.5, -0.5, 5.5, -0.5)]


@gallery("imshow_extent_1", figsize=(6, 11.25 / 5))
def _(fig):
    _imshow_demo_grid(fig, extents=[None])


@gallery("imshow_extent_2", figsize=(6, 4 * 11.25 / 5))
def _(fig):
    columns = _imshow_demo_grid(fig, _EXTENTS)
    _extent_none_text(columns["upper"][1])
    _extent_none_text(columns["lower"][0])


@gallery("imshow_extent_3", figsize=(6, 5 * 11.25 / 5))
def _(fig):
    _imshow_demo_grid(fig, extents=[None] + _EXTENTS, xlim=(-2, 8), ylim=(-1, 6))


# --- patheffects_guide ---------------------------------------------------------


@gallery("patheffects_guide_1", figsize=(5, 1.5))
def _(fig):
    text = fig.text(0.5, 0.5, "Hello path effects world!\nThis is the normal "
                              "path effect.\nPretty dull, huh?",
                    ha="center", va="center", size=20)
    text.set_path_effects([path_effects.Normal()])


@gallery("patheffects_guide_2")
def _(fig):
    ax = fig.add_subplot()
    ax.text(0.5, 0.5, "Hello path effects world!",
            path_effects=[path_effects.withSimplePatchShadow()])
    ax.plot([0, 3, 2, 5], linewidth=5, color="blue",
            path_effects=[path_effects.SimpleLineShadow(), path_effects.Normal()])


@gallery("patheffects_guide_3", figsize=(7, 1))
def _(fig):
    text = fig.text(0.5, 0.5, "This text stands out because of\n"
                              "its black border.", color="white",
                    ha="center", va="center", size=30)
    text.set_path_effects([path_effects.Stroke(linewidth=3, foreground="black"),
                           path_effects.Normal()])


@gallery("patheffects_guide_4", figsize=(8.5, 1))
def _(fig):
    t = fig.text(0.02, 0.5, "Hatch shadow", fontsize=75, weight=1000, va="center")
    t.set_path_effects([
        path_effects.PathPatchEffect(offset=(4, -4), hatch="xxxx", facecolor="gray"),
        path_effects.PathPatchEffect(edgecolor="white", linewidth=1.1, facecolor="black")])


# --- paths ---------------------------------------------------------------------


@gallery("paths_1")
def _(fig):
    verts = [(0., 0.), (0., 1.), (1., 1.), (1., 0.), (0., 0.)]
    codes = [Path.MOVETO, Path.LINETO, Path.LINETO, Path.LINETO, Path.CLOSEPOLY]
    ax = fig.subplots()
    ax.add_patch(mpatches.PathPatch(Path(verts, codes), facecolor="orange", lw=2))
    ax.set_xlim(-2, 2)
    ax.set_ylim(-2, 2)


@gallery("paths_2")
def _(fig):
    verts = [(0., 0.), (0.2, 1.), (1., 0.8), (0.8, 0.)]
    codes = [Path.MOVETO, Path.CURVE4, Path.CURVE4, Path.CURVE4]
    ax = fig.subplots()
    ax.add_patch(mpatches.PathPatch(Path(verts, codes), facecolor="none", lw=2))
    xs, ys = zip(*verts)
    ax.plot(xs, ys, "x--", lw=2, color="black", ms=10)
    ax.text(-0.05, -0.05, "P0")
    ax.text(0.15, 1.05, "P1")
    ax.text(1.05, 0.85, "P2")
    ax.text(0.85, -0.05, "P3")
    ax.set_xlim(-0.1, 1.1)
    ax.set_ylim(-0.1, 1.1)


@gallery("paths_3")
def _(fig):
    ax = fig.subplots()
    n, bins = np.histogram(rng(SEED).normal(1000), 100)
    left = np.array(bins[:-1])
    right = np.array(bins[1:])
    bottom = np.zeros(len(left))
    top = bottom + n
    nverts = len(left) * (1 + 3 + 1)
    verts = np.zeros((nverts, 2))
    codes = np.full(nverts, Path.LINETO, dtype=int)
    codes[0::5] = Path.MOVETO
    codes[4::5] = Path.CLOSEPOLY
    verts[0::5, 0] = left
    verts[0::5, 1] = bottom
    verts[1::5, 0] = left
    verts[1::5, 1] = top
    verts[2::5, 0] = right
    verts[2::5, 1] = top
    verts[3::5, 0] = right
    verts[3::5, 1] = bottom
    patch = mpatches.PathPatch(Path(verts, codes), facecolor="green", edgecolor="yellow", alpha=0.5)
    ax.add_patch(patch)
    ax.set_xlim(left[0], right[-1])
    ax.set_ylim(bottom.min(), top.max())


# --- transforms_tutorial -------------------------------------------------------


def _damped_sine(ax):
    """The tutorial's damped sine over ``[0, 10)`` in limits ``(0, 10)`` ×
    ``(-1, 1)``."""
    x = np.arange(0, 10, 0.005)
    ax.plot(x, np.exp(-x / 2.) * np.sin(2 * np.pi * x))
    ax.set_xlim(0, 10)
    ax.set_ylim(-1, 1)


def _green_cloud(ax, yscale):
    """1000 faint green dots in ``[0, 10) × [0, 10 yscale)``."""
    x, y = rng(SEED).uniform(2000, 0, 10).reshape(2, 1000)
    ax.plot(x, y * yscale, "go", alpha=0.2)


def _inch_circle(fig):
    """A cloud with a circle of 1 in radius 2.5 in right and 2 in up from the
    figure's corner."""
    ax = fig.subplots()
    _green_cloud(ax, 10.)
    ax.add_patch(mpatches.Circle((2.5, 2), 1.0, transform=fig.dpi_scale_trans,
                                 facecolor="blue", alpha=0.75))


@gallery("transforms_tutorial_1")
def _(fig):
    _damped_sine(fig.subplots())


@gallery("transforms_tutorial_2")
def _(fig):
    ax = fig.subplots()
    _damped_sine(ax)
    xdata, ydata = 5, 0
    xdisplay, ydisplay = ax.transData.transform((xdata, ydata))
    bbox = dict(boxstyle="round", fc="0.8")
    arrowprops = dict(arrowstyle="->", connectionstyle="angle,angleA=0,angleB=90,rad=10")
    offset = 72
    ax.annotate(f"data = ({xdata:.1f}, {ydata:.1f})", (xdata, ydata),
                xytext=(-2 * offset, offset), textcoords="offset points",
                bbox=bbox, arrowprops=arrowprops)
    ax.annotate(f"display = ({xdisplay:.1f}, {ydisplay:.1f})", (xdisplay, ydisplay),
                xytext=(0.5 * offset, -offset), xycoords="figure pixels",
                textcoords="offset points", bbox=bbox, arrowprops=arrowprops)


@gallery("transforms_tutorial_3")
def _(fig):
    for i, label in enumerate(("A", "B", "C", "D")):
        ax = fig.add_subplot(2, 2, i + 1)
        ax.text(0.05, 0.95, label, transform=ax.transAxes, fontsize=16, fontweight="bold", va="top")


@gallery("transforms_tutorial_4")
def _(fig):
    ax = fig.subplots()
    _green_cloud(ax, 1.)
    ax.add_patch(mpatches.Circle((0.5, 0.5), 0.25, transform=ax.transAxes,
                                 facecolor="blue", alpha=0.75))


@gallery("transforms_tutorial_5")
def _(fig):
    ax = fig.subplots()
    ax.hist(rng(SEED).normal(1000), 30)
    ax.set_title(r"$\sigma=1 \/ \dots \/ \sigma=2$", fontsize=16)
    trans = transforms.blended_transform_factory(ax.transData, ax.transAxes)
    ax.add_patch(mpatches.Rectangle((1, 0), width=1, height=1, transform=trans,
                                    color="yellow", alpha=0.5))


@gallery("transforms_tutorial_6", figsize=(5, 4))
def _(fig):
    _inch_circle(fig)


@gallery("transforms_tutorial_7", figsize=(7, 2))
def _(fig):
    _inch_circle(fig)


@gallery("transforms_tutorial_8")
def _(fig):
    ax = fig.subplots()
    xdata, ydata = (0.2, 0.7), (0.5, 0.5)
    ax.plot(xdata, ydata, "o")
    ax.set_xlim(0, 1)
    trans = (fig.dpi_scale_trans +
             transforms.ScaledTranslation(xdata[0], ydata[0], ax.transData))
    ax.add_patch(mpatches.Ellipse((0, 0), 150 / 72, 130 / 72, angle=40, fill=None, transform=trans))


@gallery("transforms_tutorial_9")
def _(fig):
    ax = fig.subplots()
    x = np.arange(0., 2., 0.01)
    y = np.sin(2 * np.pi * x)
    line, = ax.plot(x, y, lw=3, color="blue")
    offset = transforms.ScaledTranslation(2 / 72., -2 / 72., fig.dpi_scale_trans)
    ax.plot(x, y, lw=3, color="gray", transform=ax.transData + offset,
            zorder=0.5 * line.get_zorder())
    ax.set_title("creating a shadow effect with an offset transform")
