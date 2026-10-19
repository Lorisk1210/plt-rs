"""matplotlib's own image tests from ``tests/test_axes.py`` (up to line 2069):
every ``@image_comparison`` test and the ``fig_test`` side of every
``@check_figures_equal`` test (plus the ``fig_ref`` side where it draws
something the test side does not).

Each case keeps the test's style. The test suite's default (also used by
``check_figures_equal``) is ``classic`` plus ``_classic_test_patch``: such
figures are 8x6 in and saved at classic's 100 dpi. Tests asking for
``mpl20``/``default`` run in the reference configuration. ``remove_text=True``
becomes ``remove_ticks_and_titles`` at the end of the build, and the tests'
own rcParams changes become the case's ``rc``.

Data is the tests' own (formulas, ranges) except that ``np.random`` draws
come from ``spec_support.rng`` and wall-clock datetimes become fixed ones.
Tests built on ``matplotlib.testing.jpl_units`` (``formatter_ticker`` beyond
its first figure, ``fill_units``, ``axvspan_epoch``, ``axhspan_epoch``) are
not ported; figures a test draws twice (once more through ``data=``) are
ported once.
"""

import datetime
from itertools import product

import matplotlib
import matplotlib.style
import numpy as np
from matplotlib import collections as mcollections
from matplotlib import colors as mcolors
from matplotlib import patches as mpatches
from matplotlib import ticker as mticker
from matplotlib import transforms as mtransforms
from matplotlib.gridspec import GridSpec
from matplotlib.testing.decorators import remove_ticks_and_titles

from spec_support import case, rng

GROUP = "mpl_axes_a"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}

#: The values of the ``markevery`` tests.
MARKEVERY_CASES = [None, 8, (30, 8), [16, 24, 30], [0, -1], slice(100, 200, 3),
                   0.1, 0.3, 1.5, (0.0, 0.1), (0.45, 0.1)]


def classic(name, rc=None, **options):
    """A case in the test suite's default style (8x6 in at 100 dpi)."""
    return case(f"{GROUP}/{name}", figsize=(8, 6), rc={**CLASSIC, **(rc or {})}, **options)


def mpl20(name, **options):
    """A case in the ``mpl20``/``default`` style (the reference configuration)."""
    return case(f"{GROUP}/{name}", **options)


def without_text(build):
    """``image_comparison(remove_text=True)``: no titles, no tick labels."""
    def wrapped(fig):
        build(fig)
        remove_ticks_and_titles(fig)
    return wrapped


# --- labels, autocorrelation, spy, matshow -------------------------------


def _labelled_scatter(fig):
    ax = fig.subplots()
    sc = ax.scatter([1, 2], [1, 2], c=[1, 2], label="scatter")
    ax.legend()
    return ax, sc


@classic("invisible_axes")
def _(fig):
    fig.subplots().set_visible(False)


@classic("label_loc_vertical")
def _(fig):
    ax, sc = _labelled_scatter(fig)
    ax.set_ylabel("Y Label", loc="top")
    ax.set_xlabel("X Label", loc="right")
    fig.colorbar(sc).set_label("Z Label", loc="top")


@classic("label_loc_horizontal")
def _(fig):
    ax, sc = _labelled_scatter(fig)
    ax.set_ylabel("Y Label", loc="bottom")
    ax.set_xlabel("X Label", loc="left")
    fig.colorbar(sc, orientation="horizontal").set_label("Z Label", loc="left")


@classic("label_loc_rc")
def _(fig):
    with matplotlib.rc_context({"xaxis.labellocation": "right",
                                "yaxis.labellocation": "top"}):
        ax, sc = _labelled_scatter(fig)
        ax.set_ylabel("Y Label")
        ax.set_xlabel("X Label")
        fig.colorbar(sc, orientation="horizontal").set_label("Z Label")


@classic("acorr")
def _(fig):
    x = rng(19680801).normal(512).cumsum()
    fig.subplots().acorr(x, maxlags=511)


@classic("acorr_integers")
def _(fig):
    x = np.ceil((rng(19680801).uniform(51) * 10).cumsum()).astype(np.int64)
    fig.subplots().acorr(x, maxlags=50)


@classic("spy")
def _(fig):
    a = (rng(19680801).uniform(32 * 32) < 0.5).astype(float).reshape((32, 32))
    axs = fig.subplots(2)
    axs[0].spy(a)
    axs[1].spy(a, marker=".", origin="lower")


@mpl20("matshow", figsize=(8, 6), dpi=80)
def _(fig):
    fig.add_subplot().matshow(rng(19680801).uniform(32 * 32).reshape((32, 32)))


# --- tickers and twins ----------------------------------------------------


@mpl20("formatter_ticker_001", rc={"lines.markeredgewidth": 30})
def _(fig):
    fig.subplots().set_xlabel("x-label 001")


@mpl20("twin_axis_locators_formatters")
def _(fig):
    vals = np.linspace(0, 1, num=5, endpoint=True)
    locs = np.sin(np.pi * vals / 2.0)
    ax1 = fig.add_subplot(1, 1, 1)
    ax1.margins(0)
    ax1.plot([0.1, 100], [0, 1])
    ax1.yaxis.set_major_locator(mticker.FixedLocator(locs))
    ax1.yaxis.set_minor_locator(mticker.FixedLocator([0.1, 0.2, 0.3]))
    ax1.yaxis.set_major_formatter(mticker.FormatStrFormatter("%08.2lf"))
    ax1.yaxis.set_minor_formatter(mticker.FixedFormatter(["tricks", "mind", "jedi"]))
    ax1.xaxis.set_major_locator(mticker.LinearLocator())
    ax1.xaxis.set_minor_locator(mticker.FixedLocator([15, 35, 55, 75]))
    ax1.xaxis.set_major_formatter(mticker.FormatStrFormatter("%05.2lf"))
    ax1.xaxis.set_minor_formatter(mticker.FixedFormatter(["c", "3", "p", "o"]))
    ax1.twiny()
    ax1.twinx()


def _twin_logscale(fig, twin):
    """Log scale set on the parent after twinning, then before."""
    twin_of = lambda ax: getattr(ax, f"twin{twin}")()  # noqa: E731
    set_log = lambda ax: getattr(ax, f"set_{twin}scale")("log")  # noqa: E731
    x = np.arange(1, 100)
    ax = fig.add_subplot(2, 1, 1)
    ax_twin = twin_of(ax)
    set_log(ax)
    ax_twin.plot(x, x)
    ax = fig.add_subplot(2, 1, 2)
    set_log(ax)
    twin_of(ax).plot(x, x)
    remove_ticks_and_titles(fig)


@classic("twin_logscale_x")
def _(fig):
    _twin_logscale(fig, "x")


@classic("twin_logscale_y")
def _(fig):
    _twin_logscale(fig, "y")


@classic("twinx_axis_scales")
def _(fig):
    x = np.array([0, 0.5, 1])
    x2 = np.array([0, 1, 2])
    ax = fig.add_axes((0, 0, 1, 1), autoscalex_on=False, autoscaley_on=False)
    ax.plot(x, 0.5 * x, color="blue", lw=10)
    ax2 = ax.twinx()
    ax2.plot(x2, 2 * x2, "r--", lw=5)
    ax.margins(0, 0)
    ax2.margins(0, 0)


@classic("minorticks_on_rcParams_both")
def _(fig):
    with matplotlib.rc_context({"xtick.minor.visible": True, "ytick.minor.visible": True}):
        fig.subplots().plot([0, 1], [0, 1])


# --- autoscaling and sticky edges ------------------------------------------


@classic("autoscale_tiny_range")
@without_text
def _(fig):
    for i, ax in enumerate(fig.subplots(2, 2).flat):
        ax.plot([0, 1], [1, 1 + 10 ** (-11 - i)])


def _sticky_shared(fig, mesh_on_leader):
    """A mesh in one of two x-shared axes, the leader drawn at the bottom
    or the top."""
    z = np.arange(15).reshape(3, 5)
    leader = fig.add_subplot(212 if mesh_on_leader else 211)
    follower = fig.add_subplot(211 if mesh_on_leader else 212, sharex=leader)
    (leader if mesh_on_leader else follower).pcolormesh(z)


@classic("sticky_shared_axes")
def _(fig):
    _sticky_shared(fig, mesh_on_leader=False)


@classic("sticky_shared_axes_ref")
def _(fig):
    _sticky_shared(fig, mesh_on_leader=True)


@mpl20("sticky_tolerance")
@without_text
def _(fig):
    axs = fig.subplots(2, 2).flat
    width = .1
    axs[0].bar(x=0, height=width, bottom=20000.6)
    axs[0].bar(x=1, height=width, bottom=20000.1)
    axs[1].bar(x=0, height=-width, bottom=20000.6)
    axs[1].bar(x=1, height=-width, bottom=20000.1)
    axs[2].barh(y=0, width=-width, left=-20000.6)
    axs[2].barh(y=1, width=-width, left=-20000.1)
    axs[3].barh(y=0, width=width, left=-20000.6)
    axs[3].barh(y=1, width=width, left=-20000.1)


@mpl20("sticky_tolerance_contourf")
@without_text
def _(fig):
    x = y = [14496.71, 14496.75]
    fig.subplots().contourf(x, y, [[0, 1], [2, 3]])


# --- annotations, arrows, error bars, single points ------------------------


@mpl20("basic_annotate")
@without_text
def _(fig):
    t = np.arange(0.0, 5.0, 0.01)
    s = np.cos(2.0 * np.pi * t)
    ax = fig.add_subplot(autoscale_on=False, xlim=(-1, 5), ylim=(-3, 5))
    ax.plot(t, s, lw=3, color="purple")
    ax.annotate("local max", xy=(3, 1), xycoords="data", xytext=(3, 3),
                textcoords="offset points")


@classic("arrow_simple")
@without_text
def _(fig):
    kwargs = product((True, False), ("full", "left", "right"), (True, False))
    for i, (ax, kwarg) in enumerate(zip(fig.subplots(3, 4).flat, kwargs)):
        length_includes_head, shape, head_starts_at_zero = kwarg
        ax.set_xlim(-2, 2)
        ax.set_ylim(-2, 2)
        theta = 2 * np.pi * i / 12
        ax.arrow(0, 0, np.sin(theta), np.cos(theta), width=theta / 100,
                 length_includes_head=length_includes_head, shape=shape,
                 head_starts_at_zero=head_starts_at_zero, head_width=theta / 10,
                 head_length=theta / 10)


@classic("errorbar_dashes")
def _(fig):
    x = [1, 2, 3, 4]
    y = np.sin(x)
    fig.gca().errorbar(x, y, xerr=np.abs(y), yerr=np.abs(y), dashes=[2, 2])


@mpl20("single_point", rc={"lines.marker": "o", "axes.grid": True})
def _(fig):
    ax1, ax2 = fig.subplots(2)
    ax1.plot([0], [0], "o")
    ax2.plot([1], [1], "o")


@classic("shaped_data")
def _(fig):
    axs = fig.subplots(2)
    axs[0].plot(np.arange(10).reshape((1, -1)))
    axs[1].plot(np.arange(0, 100, 10).reshape((-1, 1)))


@mpl20("aitoff_proj", figsize=(8, 4.2))
@without_text
def _(fig):
    x = np.linspace(-np.pi, np.pi, 20)
    y = np.linspace(-np.pi / 2, np.pi / 2, 20)
    X, Y = np.meshgrid(x, y)
    ax = fig.subplots(subplot_kw=dict(projection="aitoff"))
    ax.grid()
    ax.plot(X.flat, Y.flat, "o", markersize=4)


# --- hexbin -----------------------------------------------------------------


@classic("hexbin_extent")
@without_text
def _(fig):
    x, y = (np.arange(2000) / 2000).reshape((2, 1000))
    fig.subplots().hexbin(x, y, extent=[.1, .3, .6, .7])


@classic("hexbin_empty")
@without_text
def _(fig):
    ax = fig.subplots()
    ax.hexbin([], [])
    ax.hexbin([], [], bins="log")
    ax.hexbin([], [], C=[], reduce_C_function=np.max)
    ax.hexbin([], [], bins=np.arange(10))


def _hexbin_sample():
    """The correlated normal sample of the hexbin tests."""
    r = rng(19680801)
    n = 100000
    x = r.normal(n)
    return x, 2.0 + 3.0 * x + 4.0 * r.normal(n)


@mpl20("hexbin_log")
def _(fig):
    x, y = _hexbin_sample()
    ax = fig.subplots()
    h = ax.hexbin(x, np.power(2, y * 0.5), yscale="log", bins="log", marginals=True,
                  reduce_C_function=np.sum)
    fig.colorbar(h)


@mpl20("hexbin_linear")
@without_text
def _(fig):
    x, y = _hexbin_sample()
    fig.subplots().hexbin(x, y, gridsize=(10, 5), marginals=True, reduce_C_function=np.sum)


def _hexbin_mincnt(fig, **kwargs):
    ax = fig.subplots()
    ax.hexbin([0, 0, 6, 0], [0, 0, 0, 6], extent=[-10., 10, -10., 10], gridsize=(7, 7),
              mincnt=1, **kwargs)
    ax.set_facecolor("green")


@classic("hexbin_mincnt_behavior_upon_C_parameter")
def _(fig):
    _hexbin_mincnt(fig, C=[1] * 4, reduce_C_function=lambda v: sum(v))


@classic("hexbin_mincnt_behavior_upon_C_parameter_ref")
def _(fig):
    _hexbin_mincnt(fig)


# --- limits, images, collections --------------------------------------------


@mpl20("nonfinite_limits")
def _(fig):
    x = np.arange(0., np.e, 0.01)
    with np.errstate(divide="ignore"):
        y = np.log(x)
    x[len(x) // 2] = np.nan
    fig.subplots().plot(x, y)


def _limits_empty_data(fig, plot_fun):
    x = np.arange("2010-01-01", "2011-01-01", dtype="datetime64[D]")
    ax = fig.subplots()
    getattr(ax, plot_fun)([], [])
    getattr(ax, plot_fun)(x, range(len(x)), color="C0")


@mpl20("limits_empty_data_scatter")
def _(fig):
    _limits_empty_data(fig, "scatter")


@mpl20("limits_empty_data_plot")
def _(fig):
    _limits_empty_data(fig, "plot")


@mpl20("limits_empty_data_fill_between")
def _(fig):
    _limits_empty_data(fig, "fill_between")


def _cone(n=100):
    """``sqrt(x² + y² - xy)`` on an n×n integer grid centred on zero."""
    x, y = np.indices((n, n))
    x -= n // 2
    y -= n // 2
    return np.sqrt(x**2 + y**2 - x * y)


@mpl20("imshow", rc={"image.interpolation": "nearest"})
@without_text
def _(fig):
    fig.subplots().imshow(_cone())


@mpl20("imshow_clip", rc={"image.interpolation": "nearest"})
def _(fig):
    r = _cone()
    ax = fig.subplots()
    c = ax.contour(r, [100 / 4])
    clip_path = mtransforms.TransformedPath(c.get_paths()[0], c.get_transform())
    ax.imshow(r, clip_path=clip_path)


@classic("polycollection_joinstyle")
@without_text
def _(fig):
    ax = fig.subplots()
    verts = np.array([[1, 1], [1, 2], [2, 2], [2, 1]])
    ax.add_collection(mcollections.PolyCollection([verts], linewidths=40))
    ax.set_xbound(0, 3)
    ax.set_ybound(0, 3)


# --- fill_between -------------------------------------------------------------


@classic("fill_between_interpolate")
@without_text
def _(fig):
    x = np.arange(0.0, 2, 0.02)
    y1 = np.sin(2 * np.pi * x)
    y2 = 1.2 * np.sin(4 * np.pi * x)
    ax1, ax2 = fig.subplots(2, 1, sharex=True)
    ax1.plot(x, y1, x, y2, color="black")
    ax1.fill_between(x, y1, y2, where=y2 >= y1, facecolor="white", hatch="/",
                     interpolate=True)
    ax1.fill_between(x, y1, y2, where=y2 <= y1, facecolor="red", interpolate=True)
    y2 = np.ma.masked_greater(y2, 1.0)
    y2[0] = np.ma.masked
    ax2.plot(x, y1, x, y2, color="black")
    ax2.fill_between(x, y1, y2, where=y2 >= y1, facecolor="green", interpolate=True)
    ax2.fill_between(x, y1, y2, where=y2 <= y1, facecolor="red", interpolate=True)


@mpl20("fill_between_interpolate_decreasing", figsize=(9, 9))
@without_text
def _(fig):
    p = np.array([724.3, 700, 655])
    t = np.array([9.4, 7, 2.2])
    prof = np.array([7.9, 6.6, 3.8])
    ax = fig.subplots()
    ax.plot(t, p, "tab:red")
    ax.plot(prof, p, "k")
    ax.fill_betweenx(p, t, prof, where=prof < t, facecolor="blue", interpolate=True,
                     alpha=0.4)
    ax.fill_betweenx(p, t, prof, where=prof > t, facecolor="red", interpolate=True,
                     alpha=0.4)
    ax.set_xlim(0, 30)
    ax.set_ylim(800, 600)


@classic("fill_between_interpolate_nan")
@without_text
def _(fig):
    x = np.arange(10)
    y1 = np.asarray([8, 18, np.nan, 18, 8, 18, 24, 18, 8, 18])
    y2 = np.asarray([18, 11, 8, 11, 18, 26, 32, 30, np.nan, np.nan])
    ax = fig.subplots()
    ax.plot(x, y1, c="k")
    ax.plot(x, y2, c="b")
    ax.fill_between(x, y1, y2, where=y2 >= y1, facecolor="green", interpolate=True,
                    alpha=0.5)
    ax.fill_between(x, y1, y2, where=y1 >= y2, facecolor="red", interpolate=True,
                    alpha=0.5)


# --- symlog -------------------------------------------------------------------


@mpl20("symlog")
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1, 2, 4, 6, 9, 12, 24], [1000000, 500000, 100000, 100, 5, 0, 0, 0])
    ax.set_yscale("symlog")
    ax.set_xscale("linear")
    ax.set_ylim(-1, 10000000)


@classic("symlog2")
@without_text
def _(fig):
    x = np.arange(-50, 50, 0.001)
    axs = fig.subplots(5, 1)
    for ax, linthresh in zip(axs, [20., 2., 1., 0.1, 0.01]):
        ax.plot(x, x)
        ax.set_xscale("symlog", linthresh=linthresh)
        ax.grid(True)
    axs[-1].set_ylim(-0.1, 0.1)


# --- pcolormesh and pcolor ----------------------------------------------------


def _quad_grid(n):
    """The curvilinear grid and normalised radius of the pcolormesh tests."""
    X, Y = np.meshgrid(np.linspace(-1.5, 1.5, n), np.linspace(-1.5, 1.5, n * 2))
    qx = np.cos(Y) - np.cos(X) + 1.1
    qz = np.sin(Y) + np.sin(X)
    z = np.hypot(X, Y) / 5
    return qx, qz, (z - z.min()) / np.ptp(z)


@classic("pcolormesh", rc={"pcolormesh.snap": False})
@without_text
def _(fig):
    qx, qz, z = _quad_grid(12)
    zm = np.ma.masked_where(np.abs(qz) < 0.5 * np.max(qz), z)
    ax1, ax2, ax3 = fig.subplots(1, 3)
    ax1.pcolormesh(qx, qz, zm[:-1, :-1], lw=0.5, edgecolors="k")
    ax2.pcolormesh(qx, qz, zm[:-1, :-1], lw=2, edgecolors=["b", "w"])
    ax3.pcolormesh(qx, qz, zm, shading="gouraud")


@classic("pcolormesh_small")
def _(fig):
    qx, qz, z = _quad_grid(3)
    zm = np.ma.masked_where(np.abs(qz) < 0.5 * np.max(qz), z)
    zm2 = np.ma.masked_where(qz < -0.5 * np.max(qz), z)
    (ax1, ax2), (ax3, ax4) = fig.subplots(2, 2)
    ax1.pcolormesh(qx, qz, zm[:-1, :-1], lw=0.5, edgecolors="k")
    ax2.pcolormesh(qx, qz, zm[:-1, :-1], lw=2, edgecolors=["b", "w"])
    ax3.pcolormesh(qx, qz, zm, shading="gouraud")
    ax4.pcolormesh(qx, qz, zm2, shading="gouraud")
    for ax in fig.axes:
        ax.set_axis_off()


@classic("pcolormesh_alpha", rc={"pcolormesh.snap": False})
@without_text
def _(fig):
    n = 12
    X, Y = np.meshgrid(np.linspace(-1.5, 1.5, n), np.linspace(-1.5, 1.5, n * 2))
    qx = X
    qy = Y + np.sin(X)
    z = np.hypot(X, Y) / 5
    z = (z - z.min()) / np.ptp(z)
    vir = matplotlib.colormaps["viridis"].resampled(16)
    colors = vir(np.arange(16))
    colors[:, 3] = 0.5 + 0.5 * np.sin(np.arange(16))
    cmap = mcolors.ListedColormap(colors)
    (ax1, ax2), (ax3, ax4) = fig.subplots(2, 2)
    for ax in ax1, ax2, ax3, ax4:
        ax.add_patch(mpatches.Rectangle((0, -1.5), 1.5, 3, facecolor=[.7, .1, .1, .5],
                                        zorder=0))
    ax1.pcolormesh(qx, qy, z[:-1, :-1], cmap=vir, alpha=0.4, shading="flat", zorder=1)
    ax2.pcolormesh(qx, qy, z, cmap=vir, alpha=0.4, shading="gouraud", zorder=1)
    ax3.pcolormesh(qx, qy, z[:-1, :-1], cmap=cmap, shading="flat", zorder=1)
    ax4.pcolormesh(qx, qy, z, cmap=cmap, shading="gouraud", zorder=1)


@classic("pcolormesh_rgba_3")
def _(fig):
    fig.subplots().pcolormesh(np.ones((5, 6, 3), dtype=float) / 2)


@classic("pcolormesh_rgba_4")
def _(fig):
    fig.subplots().pcolormesh(np.ones((5, 6, 4), dtype=float) / 2)


@classic("pcolormesh_nearest_noargs")
def _(fig):
    X, Y = np.meshgrid(np.arange(4), np.arange(7))
    fig.subplots().pcolormesh(X + Y, shading="nearest")


def _log_scale_mesh(fig, plot_fun):
    """A quadrilateral mesh starting at x = 0 on a log x axis."""
    x = np.linspace(0, 1, 11)
    if plot_fun == "pcolor":
        x[1] += 0.00001
    X, Y = np.meshgrid(x, np.linspace(1, 2, 5))
    C = X + Y
    ax = fig.subplots()
    if plot_fun == "pcolor":
        ax.pcolor(X, Y, C[:-1, :-1])
    else:
        ax.pcolormesh(X, Y, C)
    ax.set_xscale("log")


@classic("pcolormesh_log_scale")
def _(fig):
    _log_scale_mesh(fig, "pcolormesh")


@classic("pcolor_log_scale")
def _(fig):
    _log_scale_mesh(fig, "pcolor")


def _datetime_mesh(fig, plot_fun):
    """1-D and 2-D datetime coordinates, with and without the extra row and
    column, and rotated tick labels."""
    fig.subplots_adjust(hspace=0.4, top=0.98, bottom=.15)
    base = datetime.datetime(2013, 1, 1)
    x = np.array([base + datetime.timedelta(days=d) for d in range(21)])
    y = np.arange(21)
    z1, z2 = np.meshgrid(np.arange(20), np.arange(20))
    z = z1 * z2
    getattr(fig.add_subplot(221), plot_fun)(x[:-1], y[:-1], z[:-1, :-1])
    getattr(fig.add_subplot(222), plot_fun)(x, y, z)
    x = np.repeat(x[np.newaxis], 21, axis=0)
    y = np.repeat(y[:, np.newaxis], 21, axis=1)
    getattr(fig.add_subplot(223), plot_fun)(x[:-1, :-1], y[:-1, :-1], z[:-1, :-1])
    getattr(fig.add_subplot(224), plot_fun)(x, y, z)
    for ax in fig.get_axes():
        for label in ax.get_xticklabels():
            label.set_ha("right")
            label.set_rotation(30)


@mpl20("pcolormesh_datetime_axis")
def _(fig):
    _datetime_mesh(fig, "pcolormesh")


@mpl20("pcolor_datetime_axis")
def _(fig):
    _datetime_mesh(fig, "pcolor")


@classic("pcolornearest")
def _(fig):
    z = rng(19680801).normal(18).reshape(2, 9)
    fig.subplots().pcolormesh(np.arange(0, 10), np.arange(0, 3), z, shading="flat")


@classic("pcolornearestunits")
def _(fig):
    base = datetime.datetime(2020, 1, 1)
    x = [base + datetime.timedelta(hours=h) for h in range(10)]
    z = rng(19680801).normal(18).reshape(2, 9)
    fig.subplots().pcolormesh(x, np.arange(0, 3), z, shading="flat")


def _pcolorauto(fig, snap):
    z = rng(19680801).normal(27).reshape(3, 9)
    fig.subplots().pcolormesh(np.arange(0, 10), np.arange(0, 4), z, snap=snap)


@classic("pcolorauto_snap_false")
def _(fig):
    _pcolorauto(fig, snap=False)


@classic("pcolorauto_snap_true")
def _(fig):
    _pcolorauto(fig, snap=True)


# --- canonical plot, arcs -------------------------------------------------------


@mpl20("canonical")
def _(fig):
    fig.subplots().plot([1, 2, 3])


@mpl20("arc_angles")
@without_text
def _(fig):
    w, h = 2, 1
    centre = (0.2, 0.5)
    scale = 2
    for i, ax in enumerate(fig.subplots(3, 3).flat):
        theta2 = i * 360 / 9
        theta1 = theta2 - 45
        ax.add_patch(mpatches.Ellipse(centre, w, h, alpha=0.3))
        ax.add_patch(mpatches.Arc(centre, w, h, theta1=theta1, theta2=theta2))
        ax.plot([scale * np.cos(np.deg2rad(theta1)) + centre[0], centre[0],
                 scale * np.cos(np.deg2rad(theta2)) + centre[0]],
                [scale * np.sin(np.deg2rad(theta1)) + centre[1], centre[1],
                 scale * np.sin(np.deg2rad(theta2)) + centre[1]])
        ax.set_xlim(-scale, scale)
        ax.set_ylim(-scale, scale)
        w *= 10
        h *= 10
        centre = (centre[0] * 10, centre[1] * 10)
        scale *= 10


@classic("arc_ellipse")
@without_text
def _(fig):
    xcenter, ycenter = 0.38, 0.52
    width, height = 1e-1, 3e-1
    angle = -30
    theta = np.deg2rad(np.arange(360))
    rtheta = np.deg2rad(angle)
    rotation = np.array([[np.cos(rtheta), -np.sin(rtheta)],
                         [np.sin(rtheta), np.cos(rtheta)]])
    x, y = np.dot(rotation, [width / 2. * np.cos(theta), height / 2. * np.sin(theta)])
    x += xcenter
    y += ycenter
    # The test passes linewidth=1 to the first fill only: classic's
    # patch.linewidth is 1 too.
    for subplot, aspect, color in [(211, "auto", "yellow"), (212, "equal", "green")]:
        ax = fig.add_subplot(subplot, aspect=aspect)
        ax.fill(x, y, alpha=0.2, facecolor=color, edgecolor=color, linewidth=1, zorder=1)
        ax.add_patch(mpatches.Arc((xcenter, ycenter), width, height, angle=angle,
                                  linewidth=2, fill=False, zorder=2))


# --- markevery and marker edges ---------------------------------------------------


def _markevery_styles(fig, fmt_prefix):
    x = np.linspace(0, 10, 100)
    y = np.sin(x) * np.sqrt(x / 10 + 0.5)
    ax = fig.subplots()
    ax.plot(x, y, f"{fmt_prefix}o", label="default")
    ax.plot(x, y + 1, f"{fmt_prefix}d", markevery=None, label="mark all")
    ax.plot(x, y + 2, f"{fmt_prefix}s", markevery=10, label="mark every 10")
    ax.plot(x, y + 3, f"{fmt_prefix}+", markevery=(5, 20), label="mark every 20 starting at 5")
    ax.legend()


@mpl20("markevery")
@without_text
def _(fig):
    _markevery_styles(fig, "")


@mpl20("markevery_line")
@without_text
def _(fig):
    _markevery_styles(fig, "-")


def _markevery_grid(fig, x, y, polar=False, scale=None, limits=None):
    """One subplot per ``markevery`` value on a 4x3 grid."""
    gs = GridSpec(len(MARKEVERY_CASES) // 3 + 1, 3)
    for i, markevery in enumerate(MARKEVERY_CASES):
        ax = fig.add_subplot(gs[i // 3, i % 3], polar=polar)
        ax.set_title("markevery=%s" % str(markevery))
        if scale is not None:
            ax.set_xscale(scale)
            ax.set_yscale(scale)
        ax.plot(x, y, "o", ls="-", ms=4, markevery=markevery)
        if limits is not None:
            ax.set_xlim(limits[0])
            ax.set_ylim(limits[1])


def _markevery_sine():
    delta = 0.11
    x = np.linspace(0, 10 - 2 * delta, 200) + delta
    return x, np.sin(x) + 1.0 + delta


@classic("markevery_linear_scales")
@without_text
def _(fig):
    _markevery_grid(fig, *_markevery_sine())


@classic("markevery_linear_scales_zoomed")
@without_text
def _(fig):
    _markevery_grid(fig, *_markevery_sine(), limits=((6, 6.7), (1.1, 1.7)))


@classic("markevery_log_scales")
@without_text
def _(fig):
    _markevery_grid(fig, *_markevery_sine(), scale="log")


@mpl20("markevery_polar")
@without_text
def _(fig):
    r = np.linspace(0, 3.0, 200)
    _markevery_grid(fig, 2 * np.pi * r, r, polar=True)


@classic("markevery_linear_scales_nans")
@without_text
def _(fig):
    x, y = _markevery_sine()
    y[:10] = y[-20:] = y[50:70] = np.nan
    _markevery_grid(fig, x, y)


@classic("marker_edges")
@without_text
def _(fig):
    x = np.linspace(0, 1, 10)
    ax = fig.subplots()
    ax.plot(x, np.sin(x), "y.", ms=30.0, mew=0, mec="r")
    ax.plot(x + 0.1, np.sin(x), "y.", ms=30.0, mew=1, mec="r")
    ax.plot(x + 0.2, np.sin(x), "y.", ms=30.0, mew=2, mec="b")


@mpl20("bar_tick_label_single")
def _(fig):
    fig.gca().bar(0, 1, align="edge", tick_label="0")
