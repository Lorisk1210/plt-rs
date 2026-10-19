"""matplotlib's own image tests from ``tests/test_legend.py``,
``tests/test_offsetbox.py``, ``tests/test_dates.py``, ``tests/test_table.py``,
``tests/test_colors.py``, ``tests/test_triangulation.py``,
``tests/test_agg.py``, ``tests/test_agg_filter.py``, ``tests/test_skew.py``
and ``tests/test_units.py``: every ``@image_comparison`` test and the
``fig_test`` side of every ``@check_figures_equal`` test.

Each case is named after its test without the ``test_`` prefix; a test
drawing several figures becomes one case per figure (``_1``, ``_2``). Tests
without a style of their own draw under ``classic`` plus
``_classic_test_patch`` (matplotlib's test fixture applies it to
``check_figures_equal`` tests too) on classic's 8x6 in canvas saved at
100 dpi; ``mpl20`` and ``default`` need no rc. ``savefig_kwarg`` options
become the case's dpi, ``remove_text=True`` becomes
``remove_ticks_and_titles`` at the end of the build, and the tests' own
rcParams changes become the case's ``rc``. pyplot calls act on the case's
figure.

Deviations from the tests:

* ``test_light_source_topo_surface`` shades a synthetic elevation model (a
  faulted ridge given by a formula) instead of the
  ``jacksboro_fault_dem.npz`` sample data.
* ``test_diff_cell_table`` runs under matplotlib's ``text_placeholders``
  fixture, which draws text as rectangles; the case draws the real text.
* ``test_RRuleLocator`` and ``test_DateFormatter`` register
  ``matplotlib.testing.jpl_units``, which plays no part in plotting their
  ``datetime`` data; the cases leave it out.
* ``test_table_unit`` builds its table from objects of a class with a
  mocked unit converter; the case contributes the ``fig_ref`` side (the
  same table of ``"Hello"`` strings) as ``table_unit_ref``.

Not ported: ``test_set_line_coll_dash_image`` (a ``skewx`` projection made of
custom tick, axis and spine classes), ``test_numpy_facade`` (a custom unit
converter turning quantities into axis units and labels) and
``test_jpl_bar_units``/``test_jpl_barh_units`` (``jpl_units``).
"""

import datetime
import itertools

import matplotlib as mpl
import matplotlib.style
import numpy as np
from matplotlib import colorbar as mcolorbar
from matplotlib import colors as mcolors
from matplotlib import dates as mdates
from matplotlib import lines as mlines
from matplotlib import patches as mpatches
from matplotlib import transforms as mtransforms
from matplotlib import tri as mtri
from matplotlib.legend_handler import HandlerTuple
from matplotlib.offsetbox import (
    AnchoredOffsetbox, AnchoredText, DrawingArea, PaddedBox, TextArea)
from matplotlib.testing.decorators import remove_ticks_and_titles
from matplotlib.transforms import Bbox

from spec_support import case

GROUP = "mpl_misc"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def image_test(name, *, classic=False, figsize=None, dpi=100, remove_text=False,
               rc=None):
    """Register ``build`` as ``mpl_misc/<name>`` under the conditions of
    matplotlib's test decorators (see the module docstring)."""
    params = {**(CLASSIC if classic else {}), **(rc or {})}
    default_size = (8, 6) if classic else (6.4, 4.8)

    def register(build):
        def run(fig):
            build(fig)
            if remove_text:
                remove_ticks_and_titles(fig)

        case(f"{GROUP}/{name}", figsize=figsize or default_size, dpi=dpi,
             rc=params)(run)
        return build

    return register


def classic(name, **options):
    """A test in the test suite's default style."""
    return image_test(name, classic=True, **options)


# --- test_legend.py ------------------------------------------------------


@image_test("legend_auto1", remove_text=True)
def _(fig):
    ax = fig.subplots()
    x = np.arange(100)
    ax.plot(x, 50 - x, "o", label="y=1")
    ax.plot(x, x - 50, "o", label="y=-1")
    ax.legend(loc="best")


@image_test("legend_auto2", remove_text=True)
def _(fig):
    ax = fig.subplots()
    x = np.arange(100)
    b1 = ax.bar(x, x, align="edge", color="m")
    b2 = ax.bar(x, x[::-1], align="edge", color="g")
    ax.legend([b1[0], b2[0]], ["up", "down"], loc="best")


@image_test("legend_auto3")
def _(fig):
    ax = fig.subplots()
    x = [0.9, 0.1, 0.1, 0.9, 0.9, 0.5]
    y = [0.95, 0.95, 0.05, 0.05, 0.5, 0.5]
    ax.plot(x, y, "o-", label="line")
    ax.set_xlim(0.0, 1.0)
    ax.set_ylim(0.0, 1.0)
    ax.legend(loc="best")


@image_test("various_labels", remove_text=True)
def _(fig):
    ax = fig.add_subplot(121)
    ax.plot(np.arange(4), "o", label=1)
    ax.plot(np.linspace(4, 4.1), "o", label="Développés")
    ax.plot(np.arange(4, 1, -1), "o", label="__nolegend__")
    ax.legend(numpoints=1, loc="best")


@image_test("labels_first", remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.plot(np.arange(10), "-o", label=1)
    ax.plot(np.ones(10) * 5, ":x", label="x")
    ax.plot(np.arange(20, 10, -1), "d", label="diamond")
    ax.legend(loc="best", markerfirst=False)


@image_test("multiple_keys", remove_text=True)
def _(fig):
    ax = fig.subplots()
    p1, = ax.plot([1, 2, 3], "-o")
    p2, = ax.plot([2, 3, 4], "-x")
    p3, = ax.plot([3, 4, 5], "-d")
    ax.legend([(p1, p2), (p2, p1), p3], ["two keys", "pad=0", "one key"],
              numpoints=1,
              handler_map={(p1, p2): HandlerTuple(ndivide=None),
                           (p2, p1): HandlerTuple(ndivide=None, pad=0)})


@image_test("alpha_rgba", remove_text=True, rc={"legend.framealpha": None})
def _(fig):
    ax = fig.subplots()
    ax.plot(range(10), lw=5)
    leg = ax.legend(["Longlabel that will go away"], loc="center")
    leg.legendPatch.set_facecolor([1, 0, 0, 0.5])


@image_test("alpha_rcparam", remove_text=True, rc={"legend.framealpha": .75})
def _(fig):
    ax = fig.subplots()
    ax.plot(range(10), lw=5)
    leg = ax.legend(["Longlabel that will go away"], loc="center")
    leg.legendPatch.set_facecolor([1, 0, 0, 0.5])


@image_test("fancy", remove_text=True)
def _(fig):
    ax = fig.add_subplot(121)
    ax.plot([5] * 10, "o--", label="XX")
    ax.scatter(np.arange(10), np.arange(10, 0, -1), label="XX\nXX")
    ax.errorbar(np.arange(10), np.arange(10), xerr=0.5, yerr=0.5, label="XX")
    ax.legend(loc="center left", bbox_to_anchor=[1.0, 0.5],
              ncols=2, shadow=True, title="My legend", numpoints=1)


@image_test("framealpha", remove_text=True)
def _(fig):
    ax = fig.add_subplot()
    x = np.linspace(1, 100, 100)
    ax.plot(x, x, label="mylabel", lw=10)
    ax.legend(framealpha=0.5, loc="upper right")


def _scatter_legend(fig, label):
    """``test_rc``: a scatter with its legend outside on the right."""
    ax = fig.add_subplot(121)
    ax.scatter(np.arange(10), np.arange(10, 0, -1), label=label)
    ax.legend(loc="center left", bbox_to_anchor=[1.0, 0.5], title="My legend")


@image_test("rc_1", remove_text=True, rc={"legend.scatterpoints": 3})
def _(fig):
    _scatter_legend(fig, "three")


@image_test("rc_2", remove_text=True, rc={"legend.scatterpoints": 1})
def _(fig):
    _scatter_legend(fig, "one")


@image_test("legend_expand", remove_text=True)
def _(fig):
    legend_modes = [None, "expand"]
    axs = fig.subplots(len(legend_modes), 1)
    x = np.arange(100)
    for ax, mode in zip(axs, legend_modes):
        ax.plot(x, 50 - x, "o", label="y=1")
        l1 = ax.legend(loc="upper left", mode=mode)
        ax.add_artist(l1)
        ax.plot(x, x - 50, "o", label="y=-1")
        l2 = ax.legend(loc="right", mode=mode)
        ax.add_artist(l2)
        ax.legend(loc="lower left", mode=mode, ncols=2)


@image_test("hatching", remove_text=True)
def _(fig):
    ax = fig.subplots()
    patch = mpatches.Rectangle((0, 0), 0.3, 0.3, hatch="xx",
                               label="Patch\ndefault color\nfilled")
    ax.add_patch(patch)
    patch = mpatches.Rectangle((0.33, 0), 0.3, 0.3, hatch="||", edgecolor="C1",
                               label="Patch\nexplicit color\nfilled")
    ax.add_patch(patch)
    patch = mpatches.Rectangle((0, 0.4), 0.3, 0.3, hatch="xx", fill=False,
                               label="Patch\ndefault color\nunfilled")
    ax.add_patch(patch)
    patch = mpatches.Rectangle((0.33, 0.4), 0.3, 0.3, hatch="||", fill=False,
                               edgecolor="C1",
                               label="Patch\nexplicit color\nunfilled")
    ax.add_patch(patch)
    ax.fill_between([0, .15, .3], [.8, .8, .8], [.9, 1.0, .9],
                    hatch="+", label="Path\ndefault color")
    ax.fill_between([.33, .48, .63], [.8, .8, .8], [.9, 1.0, .9],
                    hatch="+", edgecolor="C2", label="Path\nexplicit color")
    ax.set_xlim(-0.01, 1.1)
    ax.set_ylim(-0.01, 1.1)
    ax.legend(handlelength=4, handleheight=4)


@classic("reverse_legend_display")
def _(fig):
    ax = fig.subplots()
    ax.plot([1], "ro", label="first")
    ax.plot([2], "bx", label="second")
    ax.legend(reverse=True)


@image_test("legend_stackplot")
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 10, 10)
    y1 = 1.0 * x
    y2 = 2.0 * x + 1
    y3 = 3.0 * x + 2
    ax.stackplot(x, y1, y2, y3, labels=["y1", "y2", "y3"])
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 70)
    ax.legend(loc="best")


@image_test("not_covering_scatter")
def _(fig):
    ax = fig.add_subplot()
    colors = ["b", "g", "r"]
    for n in range(3):
        ax.scatter([n], [n], color=colors[n])
    ax.legend(["foo", "foo", "foo"], loc="best")
    ax.set_xlim(-0.5, 2.2)
    ax.set_ylim(-0.5, 2.2)


@image_test("not_covering_scatter_transform")
def _(fig):
    ax = fig.add_subplot()
    offset = mtransforms.Affine2D().translate(-20, 20)
    x = np.linspace(0, 30, 1000)
    ax.plot(x, x)
    ax.scatter([20], [10], transform=offset + ax.transData)
    ax.legend(["foo", "bar"], loc="best")


@image_test("shadow_argument_types", remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.plot([1, 2, 3], label="test")
    legs = (ax.legend(loc="upper left", shadow=True),
            ax.legend(loc="upper right", shadow=False),
            ax.legend(loc="center left", shadow={"color": "red", "alpha": 0.1}),
            ax.legend(loc="center right",
                      shadow={"color": (0.1, 0.2, 0.5), "oy": -5}),
            ax.legend(loc="lower left", shadow={"color": "tab:cyan", "ox": 10}))
    for leg in legs:
        ax.add_artist(leg)
    ax.legend(loc="lower right")


@classic("ncol_ncols")
def _(fig):
    fig.legend(["a", "b", "c", "d", "e", "f"], ncol=3)


# --- test_offsetbox.py ---------------------------------------------------


@classic("offsetbox_clipping", remove_text=True)
def _(fig):
    ax = fig.subplots()
    size = 100
    da = DrawingArea(size, size, clip=True)
    bg = mpatches.Rectangle((0, 0), size, size, facecolor="#CCCCCC",
                            edgecolor="None", linewidth=0)
    line = mlines.Line2D([-size * .5, size * 1.5], [size / 2, size / 2],
                         color="black", linewidth=10)
    anchored_box = AnchoredOffsetbox(
        loc="center", child=da, pad=0., frameon=False,
        bbox_to_anchor=(.5, .5), bbox_transform=ax.transAxes, borderpad=0.)
    da.add_artist(bg)
    da.add_artist(line)
    ax.add_artist(anchored_box)
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)


@image_test("anchoredtext_horizontal_alignment", remove_text=True)
def _(fig):
    ax = fig.subplots()
    for loc, ha in [("center left", "left"), ("center", "center"),
                    ("center right", "right")]:
        ax.add_artist(AnchoredText("test\ntest long text", loc=loc, pad=0.2,
                                   prop={"ha": ha}))


@classic("textarea_set_text")
def _(fig):
    ax = fig.add_subplot()
    text = AnchoredText("Bar", "upper left")
    ax.add_artist(text)
    text.txt.set_text("Foo")


@image_test("paddedbox", remove_text=True)
def _(fig):
    ax = fig.subplots()
    pb = PaddedBox(TextArea("foo"), pad=5, patch_attrs={"facecolor": "r"},
                   draw_frame=True)
    ax.add_artist(AnchoredOffsetbox("upper left", child=pb))
    pb = PaddedBox(TextArea("bar"), pad=10, patch_attrs={"facecolor": "b"})
    ax.add_artist(AnchoredOffsetbox("upper right", child=pb))
    pb = PaddedBox(TextArea("foobar"), pad=15, draw_frame=True)
    ax.add_artist(AnchoredOffsetbox("lower right", child=pb))


# --- test_dates.py -------------------------------------------------------


@image_test("date_axhspan")
def _(fig):
    t0 = datetime.datetime(2009, 1, 20)
    tf = datetime.datetime(2009, 1, 21)
    ax = fig.subplots()
    ax.axhspan(t0, tf, facecolor="blue", alpha=0.25)
    ax.set_ylim(t0 - datetime.timedelta(days=5), tf + datetime.timedelta(days=5))
    fig.subplots_adjust(left=0.25)


@image_test("date_axvspan")
def _(fig):
    t0 = datetime.datetime(2000, 1, 20)
    tf = datetime.datetime(2010, 1, 21)
    ax = fig.subplots()
    ax.axvspan(t0, tf, facecolor="blue", alpha=0.25)
    ax.set_xlim(t0 - datetime.timedelta(days=720),
                tf + datetime.timedelta(days=720))
    fig.autofmt_xdate()


def _date_axhline(fig):
    """``test_date_axhline``: a horizontal line at a date."""
    t0 = datetime.datetime(2009, 1, 20)
    tf = datetime.datetime(2009, 1, 31)
    ax = fig.subplots()
    ax.axhline(t0, color="blue", lw=3)
    ax.set_ylim(t0 - datetime.timedelta(days=5), tf + datetime.timedelta(days=5))
    fig.subplots_adjust(left=0.25)
    return ax


image_test("date_axhline")(_date_axhline)


@image_test("date_axvline")
def _(fig):
    t0 = datetime.datetime(2000, 1, 20)
    tf = datetime.datetime(2000, 1, 21)
    ax = fig.subplots()
    ax.axvline(t0, color="red", lw=3)
    ax.set_xlim(t0 - datetime.timedelta(days=5), tf + datetime.timedelta(days=5))
    fig.autofmt_xdate()


@image_test("RRuleLocator")
def _(fig):
    import dateutil.rrule

    t0 = datetime.datetime(1000, 1, 1)
    tf = datetime.datetime(6000, 1, 1)
    ax = fig.add_subplot()
    ax.set_autoscale_on(True)
    ax.plot([t0, tf], [0.0, 1.0], marker="o")
    rrule = mdates.rrulewrapper(dateutil.rrule.YEARLY, interval=500)
    locator = mdates.RRuleLocator(rrule)
    ax.xaxis.set_major_locator(locator)
    ax.xaxis.set_major_formatter(mdates.AutoDateFormatter(locator))
    ax.autoscale_view()
    fig.autofmt_xdate()


@image_test("DateFormatter")
def _(fig):
    t0 = datetime.datetime(2001, 1, 1, 0, 0, 0)
    tf = datetime.datetime(2001, 1, 1, 0, 0, 1)
    ax = fig.add_subplot()
    ax.set_autoscale_on(True)
    ax.plot([t0, tf], [0.0, 1.0], marker="o")
    ax.autoscale_view()
    fig.autofmt_xdate()


@image_test("date_inverted_limit")
def _(fig):
    _date_axhline(fig).invert_yaxis()


# --- test_table.py -------------------------------------------------------


@image_test("zorder", remove_text=True)
def _(fig):
    data = [[66386, 174296],
            [58230, 381139]]
    col_labels = ("Freeze", "Wind")
    row_labels = ["%d year" % x for x in (100, 50)]
    cell_text = []
    yoff = np.zeros(len(col_labels))
    for row in reversed(data):
        yoff += row
        cell_text.append(["%1.1f" % (x / 1000.0) for x in yoff])
    ax = fig.add_subplot()
    t = np.linspace(0, 2 * np.pi, 100)
    ax.plot(t, np.cos(t), lw=4, zorder=2)
    for loc, zorder in [("center", -2), ("upper center", 4)]:
        ax.table(cellText=cell_text, rowLabels=row_labels,
                 colLabels=col_labels, loc=loc, zorder=zorder)
    ax.set_yticks([])


@image_test("label_colours")
def _(fig):
    dim = 3
    colours = mpl.colormaps["RdYlGn"](np.linspace(0, 1, dim))
    cell_text = [["1"] * dim] * dim
    headers = ["Header"] * dim
    tables = [{"rowColours": colours},
              {"rowColours": colours, "rowLabels": headers},
              {"colColours": colours},
              {"colColours": colours, "colLabels": headers}]
    for i, options in enumerate(tables):
        ax = fig.add_subplot(4, 1, i + 1)
        ax.axis("off")
        ax.table(cellText=cell_text, loc="best", **options)


@image_test("diff_cell_table", figsize=(4, 9))
def _(fig):
    cells = ("horizontal", "vertical", "open", "closed", "T", "R", "B", "L")
    cell_text = [["1"] * len(cells)] * 2
    col_widths = [0.1] * len(cells)
    fig.set_layout_engine("tight")
    axs = fig.subplots(nrows=len(cells))
    for ax, cell in zip(axs, cells):
        ax.table(colWidths=col_widths, cellText=cell_text, loc="center",
                 edges=cell)
        ax.axis("off")


@image_test("auto_column")
def _(fig):
    axs = fig.subplots(4, 1)
    # A list, a tuple, the columns one by one, and no column fitted.
    widths = [[-1, 0, 1], (-1, 0, 1), None, ()]
    for ax, columns in zip(axs, widths):
        ax.axis("off")
        tb = ax.table(
            cellText=[["Fit Text", 2],
                      ["very long long text, Longer text than default", 1]],
            rowLabels=["A", "B"], colLabels=["Col1", "Col2"], loc="center")
        tb.auto_set_font_size(False)
        tb.set_fontsize(12)
        if columns is None:
            for column in (-1, 0, 1):
                tb.auto_set_column_width(column)
        elif columns:
            tb.auto_set_column_width(columns)


@classic("table_bbox")
def _(fig):
    cell_text = [[f"{x}" for x in row] for row in [[2, 3], [4, 5]]]
    fig.subplots().table(cellText=cell_text, rowLabels=("Ada", "Bob"),
                         colLabels=("Foo", "Bar"), loc="center",
                         bbox=[0.1, 0.2, 0.8, 0.6])


@classic("table_bbox_ref")
def _(fig):
    cell_text = [[f"{x}" for x in row] for row in [[2, 3], [4, 5]]]
    fig.subplots().table(cellText=cell_text, rowLabels=("Ada", "Bob"),
                         colLabels=("Foo", "Bar"), loc="center",
                         bbox=Bbox.from_extents(0.1, 0.2, 0.9, 0.8))


@classic("table_unit_ref")
def _(fig):
    fig.subplots().table([["Hello", "Hello"], ["Hello", "Hello"]])


# --- test_colors.py ------------------------------------------------------


@image_test("cmap_and_norm_from_levels_and_colors")
def _(fig):
    data = np.linspace(-2, 4, 49).reshape(7, 7)
    levels = [-1, 2, 2.5, 3]
    colors = ["red", "green", "blue", "yellow", "black"]
    cmap, norm = mcolors.from_levels_and_colors(levels, colors, extend="both")
    ax = fig.add_subplot()
    m = ax.pcolormesh(data, cmap=cmap, norm=norm)
    fig.colorbar(m)
    ax.tick_params(labelleft=False, labelbottom=False)


@classic("boundarynorm_and_colorbarbase")
def _(fig):
    ax1 = fig.add_axes((0.05, 0.80, 0.9, 0.15))
    ax2 = fig.add_axes((0.05, 0.475, 0.9, 0.15))
    ax3 = fig.add_axes((0.05, 0.15, 0.9, 0.15))
    bounds = [-1, 2, 5, 7, 12, 15]
    cmap = mpl.colormaps["viridis"]
    norm = mcolors.BoundaryNorm(bounds, cmap.N)
    mcolorbar.ColorbarBase(ax1, cmap=cmap, norm=norm, extend="both",
                           orientation="horizontal", spacing="uniform")
    norm = mcolors.BoundaryNorm(bounds, cmap.N, extend="both")
    mcolorbar.ColorbarBase(ax2, cmap=cmap, norm=norm, orientation="horizontal")
    norm = mcolors.BoundaryNorm(bounds, cmap.N, extend="both")
    mcolorbar.ColorbarBase(ax3, cmap=cmap, norm=norm, extend="neither",
                           orientation="horizontal")


def _elevation():
    """A stand-in for the Jacksboro fault elevation model: a ridge with
    ripples, cut by a fault, on a 120 x 100 grid of 25 x 30 m cells."""
    y, x = np.mgrid[0:1:120j, 0:1:100j]
    ridge = 120 * np.exp(-(x - 0.6) ** 2 / 0.05) * (1 + 0.3 * np.sin(9 * y))
    ripples = 40 * np.sin(13 * x) * np.cos(11 * y)
    fault = 30 * np.tanh((y - 0.4 - 0.2 * x) / 0.02)
    return 350 + ridge + ripples + fault


@classic("light_source_topo_surface")
def _(fig):
    elev = _elevation()
    ls = mcolors.LightSource(315, 45)
    cmap = mpl.colormaps["gist_earth"]
    axs = fig.subplots(nrows=3, ncols=3)
    for row, mode in zip(axs, ["hsv", "overlay", "soft"]):
        for ax, ve in zip(row, [0.1, 1, 10]):
            rgb = ls.shade(elev, cmap, vert_exag=ve, dx=25.0, dy=30.0,
                           blend_mode=mode)
            ax.imshow(rgb)
            ax.set(xticks=[], yticks=[])


@classic("norm_update_figs")
def _(fig):
    ax = fig.add_subplot()
    z = np.arange(100).reshape((10, 10))
    norm = mcolors.Normalize(0, 1)
    ax.imshow(z, norm=norm)
    fig.canvas.draw()
    norm.vmin, norm.vmax = 10, 90


class CustomHalfNorm(mcolors.Norm):
    """``test_norm_abc``'s norm: halves the data, limits fixed at 0 and 1."""

    @property
    def vmin(self):
        return 0

    @property
    def vmax(self):
        return 1

    @property
    def clip(self):
        return False

    def __call__(self, value, clip=None):
        return value / 2

    def inverse(self, value):
        return 2 * value

    def autoscale(self, A):
        pass

    def autoscale_None(self, A):
        pass

    def scaled(self):
        return True

    @property
    def n_components(self):
        return 1


@classic("norm_abc", remove_text=True)
def _(fig):
    axes = fig.subplots(2, 2)
    r = np.linspace(-1, 3, 16 * 16).reshape((16, 16))
    colorizer = mpl.colorizer.Colorizer(cmap="viridis", norm=CustomHalfNorm())
    axes[0, 0].imshow(r, colorizer=colorizer)
    axes[0, 1].pcolor(r, colorizer=colorizer)
    axes[1, 0].contour(r, colorizer=colorizer)
    axes[1, 1].contourf(r, colorizer=colorizer)


# --- test_triangulation.py -----------------------------------------------


@image_test("tripcolor")
def _(fig):
    x = np.asarray([0, 0.5, 1, 0, 0.5, 1, 0, 0.5, 1, 0.75])
    y = np.asarray([0, 0, 0, 0.5, 0.5, 0.5, 1, 1, 1, 0.75])
    triangles = np.asarray([
        [0, 1, 3], [1, 4, 3],
        [1, 2, 4], [2, 5, 4],
        [3, 4, 6], [4, 7, 6],
        [4, 5, 9], [7, 4, 9], [8, 7, 9], [5, 8, 9]])
    triang = mtri.Triangulation(x, y, triangles)
    c_points = x + 0.5 * y
    xmid = x[triang.triangles].mean(axis=1)
    ymid = y[triang.triangles].mean(axis=1)
    c_faces = 0.5 * xmid + ymid
    ax = fig.add_subplot(121)
    ax.tripcolor(triang, c_points, edgecolors="k")
    ax.set_title("point colors")
    ax = fig.add_subplot(122)
    ax.tripcolor(triang, facecolors=c_faces, edgecolors="k")
    ax.set_title("facecolors")


def _ring_triangulation(n_angles, n_radii, min_radius, first_angle):
    """Points on ``n_radii`` rings, every other ring turned by half a step,
    Delaunay-triangulated without the triangles inside ``min_radius``."""
    radii = np.linspace(min_radius, 0.95, n_radii)
    angles = np.linspace(first_angle, 2 * np.pi + first_angle, n_angles,
                         endpoint=False)
    angles = np.repeat(angles[..., np.newaxis], n_radii, axis=1)
    angles[:, 1::2] += np.pi / n_angles
    x = (radii * np.cos(angles)).flatten()
    y = (radii * np.sin(angles)).flatten()
    triang = mtri.Triangulation(x, y)
    triang.set_mask(np.hypot(x[triang.triangles].mean(axis=1),
                             y[triang.triangles].mean(axis=1)) < min_radius)
    return triang


@classic("tri_smooth_contouring", remove_text=True)
def _(fig):
    def z(x, y):
        r1 = np.hypot(0.5 - x, 0.5 - y)
        theta1 = np.arctan2(0.5 - x, 0.5 - y)
        r2 = np.hypot(-x - 0.2, -y - 0.2)
        theta2 = np.arctan2(-x - 0.2, -y - 0.2)
        z = -(2 * (np.exp((r1 / 10) ** 2) - 1) * 30. * np.cos(7. * theta1) +
              (np.exp((r2 / 10) ** 2) - 1) * 30. * np.cos(11. * theta2) +
              0.7 * (x ** 2 + y ** 2))
        return (np.max(z) - z) / (np.max(z) - np.min(z))

    n_angles = 20
    triang0 = _ring_triangulation(n_angles, 10, 0.15, n_angles)
    z0 = z(triang0.x, triang0.y)
    refiner = mtri.UniformTriRefiner(triang0)
    tri_refi, z_test_refi = refiner.refine_field(z0, subdiv=4)
    ax = fig.add_subplot()
    ax.triplot(triang0, lw=0.5, color="0.5")
    ax.tricontour(tri_refi, z_test_refi, levels=np.arange(0., 1., 0.025),
                  colors="black")


@classic("tri_smooth_gradient", remove_text=True)
def _(fig):
    triang = _ring_triangulation(30, 10, 0.2, 0)
    x, y = triang.x, triang.y
    z = np.cos(np.arctan2(y, x)) / (x ** 2 + y ** 2)
    v = (np.max(z) - z) / (np.max(z) - np.min(z))
    refiner = mtri.UniformTriRefiner(triang)
    tri_refi, z_test_refi = refiner.refine_field(v, subdiv=3)
    tci = mtri.CubicTriInterpolator(triang, -v)
    ex, ey = tci.gradient(triang.x, triang.y)
    e_norm = np.hypot(ex, ey)
    ax = fig.add_subplot()
    ax.set_aspect("equal")
    ax.triplot(triang, color="0.8")
    ax.tricontour(tri_refi, z_test_refi, levels=np.arange(0., 1., 0.01),
                  cmap=mpl.colormaps["hot"], linewidths=[2.0, 1.0, 1.0, 1.0])
    ax.quiver(triang.x, triang.y, ex / e_norm, ey / e_norm,
              units="xy", scale=10., zorder=3, color="blue",
              width=0.007, headwidth=3., headlength=4.)


@classic("triplot_with_ls")
def _(fig):
    fig.subplots().triplot([0, 2, 1], [0, 0, 1], [[0, 1, 2]], ls="--")


# --- test_agg.py and test_agg_filter.py ----------------------------------


def _smooth1d(x, window_len):
    """A Hanning-window moving average with reflected ends."""
    s = np.r_[2 * x[0] - x[window_len:1:-1], x, 2 * x[-1] - x[-1:-window_len:-1]]
    w = np.hanning(window_len)
    y = np.convolve(w / w.sum(), s, mode="same")
    return y[window_len - 1:-window_len + 1]


def _smooth2d(a, sigma=3):
    """``_smooth1d`` along both axes of ``a``."""
    window_len = max(int(sigma), 3) * 2 + 1
    a = np.apply_along_axis(_smooth1d, 0, a, window_len)
    return np.apply_along_axis(_smooth1d, 1, a, window_len)


def _drop_shadow(im, dpi):
    """``test_agg_filter``'s ``DropShadowFilter(4)``: the artist's alpha at
    30 %, blurred over 4 points, in black (its offsets are zero)."""
    sigma = 4
    pad = int(sigma * 3 / 72 * dpi)
    padded_src = np.pad(im, [(pad, pad), (pad, pad), (0, 0)], "constant")
    tgt_image = np.zeros_like(padded_src)
    tgt_image[:, :, 3] = _smooth2d(padded_src[:, :, 3] * 0.3, sigma / 72 * dpi)
    return tgt_image, -pad, -pad


@classic("agg_filter", remove_text=True)
def _(fig):
    ax = fig.subplots()
    line1, = ax.plot([0.1, 0.5, 0.9], [0.1, 0.9, 0.5], "bo-",
                     mec="b", mfc="w", lw=5, mew=3, ms=10, label="Line 1")
    line2, = ax.plot([0.1, 0.5, 0.9], [0.5, 0.2, 0.7], "ro-",
                     mec="r", mfc="w", lw=5, mew=3, ms=10, label="Line 1")
    for line in [line1, line2]:
        shadow, = ax.plot(line.get_xdata(), line.get_ydata())
        shadow.update_from(line)
        shadow.set_transform(mtransforms.offset_copy(
            line.get_transform(), fig, x=4.0, y=-6.0, units="points"))
        shadow.set_zorder(line.get_zorder() - 0.5)
        shadow.set_agg_filter(_drop_shadow)
        shadow.set_rasterized(True)
    ax.set_xlim(0., 1.)
    ax.set_ylim(0., 1.)
    ax.xaxis.set_visible(False)
    ax.yaxis.set_visible(False)


@image_test("agg_filter_alpha")
def _(fig):
    ax = fig.add_subplot()
    x, y = np.mgrid[0:7, 0:8]
    mesh = ax.pcolormesh(x ** 2 - y ** 2, cmap="Reds", zorder=5)

    def manual_alpha(im, dpi):
        im[:, :, 3] *= 0.6
        return im, 0, 0

    mesh.set_agg_filter(manual_alpha)
    mesh.set_rasterized(True)
    ax.plot([0, 4, 7], [1, 3, 8])


# --- test_skew.py --------------------------------------------------------


@classic("skew_rectangle", figsize=(8, 8), remove_text=True)
def _(fig):
    axes = fig.subplots(5, 5, sharex=True, sharey=True).flat
    rotations = list(itertools.product([-3, -1, 0, 1, 3], repeat=2))
    axes[0].set_xlim([-3, 3])
    axes[0].set_ylim([-3, 3])
    axes[0].set_aspect("equal", share=True)
    for ax, (xrots, yrots) in zip(axes, rotations):
        xdeg, ydeg = 45 * xrots, 45 * yrots
        t = mtransforms.Affine2D().skew_deg(xdeg, ydeg)
        ax.set_title(f"Skew of {xdeg} in X and {ydeg} in Y")
        ax.add_patch(mpatches.Rectangle([-1, -1], 2, 2,
                                        transform=t + ax.transData,
                                        alpha=0.5, facecolor="coral"))
    fig.subplots_adjust(wspace=0, left=0.01, right=0.99, bottom=0.01, top=0.99)


# --- test_units.py -------------------------------------------------------


class Quantity:
    """``test_units``' array facade with units (no converter registered)."""

    def __init__(self, data, units):
        self.magnitude = data
        self.units = units

    def __copy__(self):
        return Quantity(self.magnitude, self.units)

    def __getattr__(self, attr):
        return getattr(self.magnitude, attr)

    def __getitem__(self, item):
        if np.iterable(self.magnitude):
            return Quantity(self.magnitude[item], self.units)
        return Quantity(self.magnitude, self.units)

    def __array__(self):
        return np.asarray(self.magnitude)


@image_test("plot_masked_units", remove_text=True)
def _(fig):
    data = np.linspace(-5, 5)
    data_masked = np.ma.array(data, mask=(data > -2) & (data < 2))
    fig.subplots().plot(Quantity(data_masked, "meters"))


@classic("subclass")
def _(fig):
    class Subdate(datetime.datetime):
        pass

    fig.subplots().plot(Subdate(2000, 1, 1), 0, "o")
