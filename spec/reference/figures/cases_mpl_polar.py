"""matplotlib's own image tests from ``tests/test_polar.py``,
``tests/test_contour.py`` and ``tests/test_colorbar.py``: every
``@image_comparison`` test and the ``fig_test`` side of every
``@check_figures_equal`` test.

Each case keeps the test's style. The test suite's default (also used by
``check_figures_equal``) is ``classic`` plus ``_classic_test_patch``: such
figures are 8x6 in and saved at classic's 100 dpi. Tests asking for
``mpl20``/``default`` run in the reference configuration. A ``dpi`` the test
saves at becomes the case's dpi, ``remove_text=True`` becomes
``remove_ticks_and_titles`` at the end of the build, and the tests' own
rcParams changes become the case's ``rc``. Parametrized tests contribute one
case per parameter set, and tests drawing several figures one case per
figure (``_1``, ``_2``, ...). Where a test goes through pyplot's current
axes or image, the case names the axes or mappable explicitly.

Deviations from the tests:

* ``np.random`` draws (seeded or not) come from ``spec_support.rng``, seeded
  with the test's seed.
* ``test_polar_units_1`` and ``test_polar_units_2`` plot
  ``matplotlib.testing.jpl_units`` quantities, registering its converters
  only while the case is built (``spec_support.units.jpl_registered``), as
  the test fixture restores the registry after each test. Their ``fig_ref``
  sides are ported too (``_ref``).
* ``test_contour_set_paths`` takes its replacement paths from a contour set
  drawn on a scratch figure, and ``test_colorbar_positioning`` contours the
  hatched set's data again where the test passes the set itself to
  ``contour``.
* ``test_colorbar_extension_shape`` and ``test_colorbar_extension_length``
  are covered by ``colorbar/extension_shape_rect`` and
  ``colorbar/extension_length_*``; the fourth figure of
  ``test_colorbar_positioning`` (axes lists, which disable ``use_gridspec``)
  is ported once.
"""

import datetime

import matplotlib
import matplotlib.style
import numpy as np
from matplotlib import cm
from matplotlib import colors as mcolors
from matplotlib import patches as mpatches
from matplotlib import ticker
from matplotlib.colorbar import Colorbar
from matplotlib.contour import ContourSet
from matplotlib.figure import Figure
from matplotlib.testing.decorators import remove_ticks_and_titles

from spec_support import case, rng
from spec_support.units import jpl_registered

GROUP = "mpl_polar"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}

#: What the tests written before ``pcolormesh.snap`` became the default keep.
NO_SNAP = {"pcolormesh.snap": False}


def classic(name, rc=None, **options):
    """A case in the test suite's default style (8x6 in at 100 dpi)."""
    options.setdefault("figsize", (8, 6))
    return case(f"{GROUP}/{name}", rc={**CLASSIC, **(rc or {})}, **options)


def mpl20(name, **options):
    """A case in the ``mpl20``/``default`` style (the reference configuration)."""
    return case(f"{GROUP}/{name}", **options)


def without_text(build):
    """``image_comparison(remove_text=True)``: no titles, no tick labels."""
    def wrapped(fig):
        build(fig)
        remove_ticks_and_titles(fig)
    return wrapped


# --- test_polar.py --------------------------------------------------------


def _spiral(start=0.0, stop=3.0):
    """``r = np.arange(start, stop, 0.01)`` and ``theta = 2πr``."""
    r = np.arange(start, stop, 0.01)
    return 2 * np.pi * r, r


def _spiral_axes(fig, start=0.0, stop=3.0):
    """Polar axes at ``(0.1, 0.1, 0.8, 0.8)`` with the spiral plotted."""
    ax = fig.add_axes((0.1, 0.1, 0.8, 0.8), polar=True)
    ax.plot(*_spiral(start, stop))
    return ax


@mpl20("polar_annotations")
def _(fig):
    r = np.arange(0.0, 1.0, 0.001)
    theta = 2.0 * 2.0 * np.pi * r
    ax = fig.add_subplot(polar=True)
    ax.plot(theta, r, color="#ee8d18", lw=3)
    ax.plot((0, 0), (0, 1), color="#0000ff", lw=1)
    ind = 800
    thisr, thistheta = r[ind], theta[ind]
    ax.plot([thistheta], [thisr], "o")
    ax.annotate("a polar annotation", xy=(thistheta, thisr), xytext=(0.05, 0.05),
                textcoords="figure fraction", arrowprops=dict(facecolor="black", shrink=0.05),
                horizontalalignment="left", verticalalignment="baseline")
    ax.tick_params(axis="x", tick1On=True, tick2On=True, direction="out")


@mpl20("polar_coord_annotations")
@without_text
def _(fig):
    el = mpatches.Ellipse((0, 0), 10, 20, facecolor="r", alpha=0.5)
    ax = fig.add_subplot(aspect="equal")
    ax.add_artist(el)
    el.set_clip_box(ax.bbox)
    ax.annotate("the top", xy=(np.pi / 2., 10.), xytext=(np.pi / 3, 20.), xycoords="polar",
                textcoords="polar", arrowprops=dict(facecolor="black", shrink=0.05),
                horizontalalignment="left", verticalalignment="baseline", clip_on=True)
    ax.set_xlim(-20, 20)
    ax.set_ylim(-20, 20)


@mpl20("polar_alignment")
def _(fig):
    angles = np.arange(0, 360, 90)
    grid_values = [0, 0.2, 0.4, 0.6, 0.8, 1]
    rect = [0.1, 0.1, 0.8, 0.8]
    horizontal = fig.add_axes(rect, polar=True, label="horizontal")
    horizontal.set_thetagrids(angles)
    vertical = fig.add_axes(rect, polar=True, label="vertical")
    vertical.patch.set_visible(False)
    for i in range(2):
        fig.axes[i].set_rgrids(grid_values, angle=angles[i], horizontalalignment="left",
                               verticalalignment="top")


@classic("polar_wrap")
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(np.deg2rad([179, -179]), [0.2, 0.1])
    ax.plot(np.deg2rad([2, -2]), [0.2, 0.1])


@classic("polar_units_1")
def _(fig):
    import matplotlib.testing.jpl_units as units

    with jpl_registered():
        xs = [x * units.deg for x in [30.0, 45.0, 60.0, 90.0]]
        ys = [1.0, 2.0, 3.0, 4.0]
        fig.add_subplot(projection="polar").plot(xs, ys)


@classic("polar_units_2")
def _(fig):
    import matplotlib.testing.jpl_units as units

    with jpl_registered():
        xs_deg = [x * units.deg for x in [30.0, 45.0, 60.0, 90.0]]
        ys_km = [y * units.km for y in [1.0, 2.0, 3.0, 4.0]]
        fig.add_subplot(projection="polar").plot(xs_deg, ys_km, thetaunits="rad",
                                                 runits="km")


@classic("polar_units_1_ref")
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(np.deg2rad([30.0, 45.0, 60.0, 90.0]), [1.0, 2.0, 3.0, 4.0])
    ax.set(xlabel="deg")


@classic("polar_units_2_ref")
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(np.deg2rad([30.0, 45.0, 60.0, 90.0]), [1.0, 2.0, 3.0, 4.0])
    ax.xaxis.set_major_formatter(ticker.FuncFormatter("{:.12}".format))
    ax.set(xlabel="rad", ylabel="km")


@mpl20("polar_rmin")
def _(fig):
    ax = _spiral_axes(fig)
    ax.set_rmax(2.0)
    ax.set_rmin(0.5)


@mpl20("polar_negative_rmin")
def _(fig):
    ax = _spiral_axes(fig, -3.0, 0.0)
    ax.set_rmax(0.0)
    ax.set_rmin(-3.0)


@mpl20("polar_rorigin")
def _(fig):
    ax = _spiral_axes(fig)
    ax.set_rmax(2.0)
    ax.set_rmin(0.5)
    ax.set_rorigin(0.0)


@mpl20("polar_invertedylim")
def _(fig):
    ax = fig.add_axes((0.1, 0.1, 0.8, 0.8), polar=True)
    ax.set_ylim(2, 0)


@mpl20("polar_invertedylim_rorigin")
def _(fig):
    ax = fig.add_axes((0.1, 0.1, 0.8, 0.8), polar=True)
    ax.yaxis.set_inverted(True)
    ax.plot([0, 0], [0, 2], c="none")
    ax.margins(0)
    ax.set_rorigin(3)


@mpl20("polar_theta_position")
def _(fig):
    ax = _spiral_axes(fig)
    ax.set_theta_zero_location("NW", 30)
    ax.set_theta_direction("clockwise")


@mpl20("polar_rlabel_position")
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.set_rlabel_position(315)
    ax.tick_params(rotation="auto")


@mpl20("polar_title_position")
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.set_title("foo")


@mpl20("polar_theta_limits", figsize=(8, 6))
def _(fig):
    theta, r = _spiral()
    theta_mins = np.arange(15.0, 361.0, 90.0)
    theta_maxs = np.arange(50.0, 361.0, 90.0)
    directions = ("out", "in", "inout")
    axs = fig.subplots(len(theta_mins), len(theta_maxs), subplot_kw={"polar": True})
    for i, start in enumerate(theta_mins):
        for j, end in enumerate(theta_maxs):
            ax = axs[i, j]
            ax.plot(theta, r)
            if start < end:
                ax.set_thetamin(start)
                ax.set_thetamax(end)
            else:
                ax.set_thetamin(end)
                ax.set_thetamax(start)
                ax.set_theta_direction("clockwise")
            ax.tick_params(tick1On=True, tick2On=True, direction=directions[i % len(directions)],
                           rotation="auto")
            ax.yaxis.set_tick_params(label2On=True, rotation="auto")
            ax.xaxis.get_major_locator().base.set_params(steps=[1, 2, 2.5, 5, 10])


@classic("polar_rlim")
def _(fig):
    ax = fig.subplots(subplot_kw={"polar": True})
    ax.set_rlim(top=10)
    ax.set_rlim(bottom=.5)


@classic("polar_rlim_bottom")
def _(fig):
    ax = fig.subplots(subplot_kw={"polar": True})
    ax.set_rlim(bottom=[.5, 10])


@classic("polar_interpolation_steps_constant_r")
def _(fig):
    fig.add_subplot(121, projection="polar").bar(
        [0], [1], 3 * np.pi, edgecolor="none", antialiased=False)
    fig.add_subplot(122, projection="polar").bar(
        [0], [1], -3 * np.pi, edgecolor="none", antialiased=False)


@classic("polar_interpolation_steps_variable_r")
def _(fig):
    line, = fig.add_subplot(projection="polar").plot([0, np.pi / 2], [1, 2])
    line.get_path()._interpolation_steps = 100


def _remove_all_but_lower_left(axs):
    for i in [0, 1, 3]:
        axs.flat[i].remove()


@classic("remove_shared_polar")
def _(fig):
    _remove_all_but_lower_left(fig.subplots(2, 2, sharey=True, subplot_kw={"projection": "polar"}))


@classic("remove_shared_polar_ref")
def _(fig):
    _remove_all_but_lower_left(fig.subplots(2, 2, sharex=True, subplot_kw={"projection": "polar"}))


@mpl20("polar_log")
def _(fig):
    ax = fig.add_subplot(polar=True)
    ax.set_rscale("log")
    ax.set_rlim(1, 1000)
    n = 100
    ax.plot(np.linspace(0, 2 * np.pi, n), np.logspace(0, 2, n))


@classic("polar_log_rorigin")
def _(fig):
    ax = fig.add_subplot(projection="polar", facecolor="red")
    ax.set_rscale("log")
    ax.set_rlim(1, 100)
    ax.set_rorigin(10**-3)
    ax.set_rticks(np.logspace(0, 2, 5))
    ax.tick_params(labelleft=False)


def _polar_errorbar(fig, order):
    theta = np.arange(0, 2 * np.pi, np.pi / 8)
    r = theta / np.pi / 2 + 0.5
    ax = fig.add_subplot(projection="polar")
    if order == "before":
        ax.set_theta_zero_location("N")
        ax.set_theta_direction(-1)
    ax.errorbar(theta, r, xerr=0.1, yerr=0.1, capsize=7, fmt="o", c="seagreen")
    if order == "after":
        ax.set_theta_zero_location("N")
        ax.set_theta_direction(-1)


for _order in ("before", "after"):
    mpl20(f"polar_errorbar_{_order}", figsize=(5, 5))(
        without_text(lambda fig, order=_order: _polar_errorbar(fig, order)))


# --- test_contour.py ------------------------------------------------------


@classic("contour_set_paths")
def _(fig):
    cs_test = fig.subplots().contour([[0, 1], [1, 2]])
    cs_ref = Figure().subplots().contour([[1, 0], [2, 1]])
    cs_test.set_paths(cs_ref.get_paths())


def _max_norm_grid():
    """``z = max(|x|, |y|)`` on the 10x10 integer grid."""
    x, y = np.meshgrid(np.arange(0, 10), np.arange(0, 10))
    return x, y, np.max(np.dstack([abs(x), abs(y)]), 2)


@mpl20("contour_manual_labels", figsize=(6, 2), dpi=200)
@without_text
def _(fig):
    ax = fig.add_subplot()
    cs = ax.contour(*_max_norm_grid())
    ax.clabel(cs, manual=np.array([(1.0, 3.0), (1.0, 4.4), (1.0, 6.0)]))
    ax.clabel(cs, manual=np.array([(2.0, 3.0), (2.0, 4.4), (2.0, 6.0)]), fontsize="small",
              colors=("r", "g"))


@mpl20("contour_label_with_disconnected_segments")
@without_text
def _(fig):
    x, y = np.mgrid[-1:1:21j, -1:1:21j]
    z = 1 / np.sqrt(0.01 + (x + 0.3) ** 2 + y ** 2)
    z += 1 / np.sqrt(0.01 + (x - 0.3) ** 2 + y ** 2)
    cs = fig.add_subplot().contour(x, y, z, levels=[7])
    cs.clabel(manual=[(0.2, 0.1)])


@classic("given_colors_levels_and_extends", rc=NO_SNAP)
@without_text
def _(fig):
    axs = fig.subplots(2, 4)
    data = np.arange(12).reshape(3, 4)
    colors = ["red", "yellow", "pink", "blue", "black"]
    levels = [2, 4, 8, 10]
    for i, ax in enumerate(axs.flat):
        filled = i % 2 == 0.
        extend = ["neither", "min", "max", "both"][i // 2]
        if filled:
            first_color = 1 if extend in ["max", "neither"] else None
            last_color = -1 if extend in ["min", "neither"] else None
            c = ax.contourf(data, colors=colors[first_color:last_color], levels=levels,
                            extend=extend)
        else:
            c = ax.contour(data, colors=colors[:-1], levels=levels, extend=extend)
        fig.colorbar(c, ax=ax)


@mpl20("hatch_colors")
@without_text
def _(fig):
    ax = fig.subplots()
    cf = ax.contourf([[0, 1], [1, 2]], hatches=["-", "/", "\\", "//"], cmap="gray")
    cf.set_edgecolors(["blue", "grey", "yellow", "red"])


@mpl20("log_locator_levels")
def _(fig):
    ax = fig.subplots()
    x = np.linspace(-3.0, 3.0, 100)
    y = np.linspace(-2.0, 2.0, 100)
    X, Y = np.meshgrid(x, y)
    data = np.exp(-X**2 - Y**2) + 50 * np.exp(-(X * 10)**2 - (Y * 10)**2)
    c = ax.contourf(data, locator=ticker.LogLocator())
    fig.colorbar(c, ax=ax)


@mpl20("contour_datetime_axis")
def _(fig):
    fig.subplots_adjust(hspace=0.4, top=0.98, bottom=.15)
    base = datetime.datetime(2013, 1, 1)
    x = np.array([base + datetime.timedelta(days=d) for d in range(20)])
    y = np.arange(20)
    z1, z2 = np.meshgrid(np.arange(20), np.arange(20))
    z = z1 * z2
    fig.add_subplot(221).contour(x, y, z)
    fig.add_subplot(222).contourf(x, y, z)
    x = np.repeat(x[np.newaxis], 20, axis=0)
    y = np.repeat(y[:, np.newaxis], 20, axis=1)
    fig.add_subplot(223).contour(x, y, z)
    fig.add_subplot(224).contourf(x, y, z)
    for ax in fig.get_axes():
        for label in ax.get_xticklabels():
            label.set_ha("right")
            label.set_rotation(30)


@mpl20("labels")
@without_text
def _(fig):
    X, Y = np.meshgrid(np.arange(-3.0, 3.0, 0.025), np.arange(-2.0, 2.0, 0.025))
    Z1 = np.exp(-(X**2 + Y**2) / 2) / (2 * np.pi)
    Z2 = (np.exp(-(((X - 1) / 1.5)**2 + ((Y - 1) / 0.5)**2) / 2)
          / (2 * np.pi * 0.5 * 1.5))
    ax = fig.subplots(1, 1)
    CS = ax.contour(X, Y, 10.0 * (Z2 - Z1))
    CS.clabel()
    for x, y in [(-2, .5), (0, -1.5), (2.8, 1)]:
        CS.add_label_near(x, y, inline=True, transform=None)
    for x, y in [(216, 177), (359, 290), (521, 406)]:
        CS.add_label_near(x, y, inline=True, transform=False)


def _corner_mask(fig, corner_mask):
    n = 60
    stream = rng(1)
    x, y = np.meshgrid(np.linspace(0, 2.0, n), np.linspace(0, 2.0, n))
    z = np.cos(7 * x) * np.sin(8 * y) + stream.uniform(n * n).reshape(n, n)
    mask = stream.uniform(n * n).reshape(n, n) >= 0.95
    fig.add_subplot().contourf(np.ma.array(z, mask=mask), corner_mask=corner_mask)


for _number, _corner in enumerate((False, True), start=1):
    classic(f"corner_mask_{_number}")(
        without_text(lambda fig, corner=_corner: _corner_mask(fig, corner)))


@mpl20("contourf_log_extension", figsize=(10, 5), rc=NO_SNAP)
@without_text
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    fig.subplots_adjust(left=0.05, right=0.95)
    data = np.power(10, np.linspace(-7.5, 9.5, 1200)).reshape(30, 40)
    levels = np.power(10., np.arange(-4., 7.))
    c1 = ax1.contourf(data, norm=mcolors.LogNorm(vmin=data.min(), vmax=data.max()),
                      locator=ticker.FixedLocator(10.**np.arange(-8, 12, 2)))
    c2 = ax2.contourf(data, levels=levels,
                      norm=mcolors.LogNorm(vmin=levels.min(), vmax=levels.max()),
                      extend="neither")
    c3 = ax3.contourf(data, levels=levels,
                      norm=mcolors.LogNorm(vmin=levels.min(), vmax=levels.max()),
                      extend="both")
    fig.colorbar(c1, ax=ax1)
    fig.colorbar(c2, ax=ax2)
    fig.colorbar(c3, ax=ax3)


@mpl20("contour_addlines", rc=NO_SNAP)
@without_text
def _(fig):
    ax = fig.subplots()
    X = rng(19680812).uniform(100).reshape(10, 10) * 10000
    pcm = ax.pcolormesh(X)
    cont = ax.contour(X + 1000)
    cb = fig.colorbar(pcm)
    cb.add_lines(cont)


@mpl20("contour_uneven", rc=NO_SNAP)
@without_text
def _(fig):
    z = np.arange(24).reshape(4, 6)
    axs = fig.subplots(1, 2)
    for ax, spacing in zip(axs, ("proportional", "uniform")):
        cs = ax.contourf(z, levels=[2, 4, 6, 10, 20])
        fig.colorbar(cs, ax=ax, spacing=spacing)


@classic("contour_closed_line_loop", figsize=(2, 2))
@without_text
def _(fig):
    z = [[0, 0, 0], [0, 2, 0], [0, 0, 0], [2, 1, 2]]
    ax = fig.subplots()
    ax.contour(z, [0.5], linewidths=[20], alpha=0.7)
    ax.set_xlim(-0.1, 2.1)
    ax.set_ylim(-0.1, 3.1)


@classic("contour_manual", figsize=(4, 4))
@without_text
def _(fig):
    ax = fig.subplots()
    cmap = "viridis"
    lines0 = [[[2, 0], [1, 2], [1, 3]]]
    lines1 = [[[3, 0], [3, 2]], [[3, 3], [3, 4]]]
    filled01 = [[[0, 0], [0, 4], [1, 3], [1, 2], [2, 0]]]
    filled12 = [[[2, 0], [3, 0], [3, 2], [1, 3], [1, 2]],
                [[1, 4], [3, 4], [3, 3]]]
    ContourSet(ax, [0, 1, 2], [filled01, filled12], filled=True, cmap=cmap)
    ContourSet(ax, [1, 2], [lines0, lines1], linewidths=3, colors=["r", "k"])
    segs = [[[4, 0], [7, 0], [7, 3], [4, 3], [4, 0],
             [5, 1], [5, 2], [6, 2], [6, 1], [5, 1]]]
    kinds = [[1, 2, 2, 2, 79, 1, 2, 2, 2, 79]]
    ContourSet(ax, [2, 3], [segs], [kinds], filled=True, cmap=cmap)
    ContourSet(ax, [2], [segs], [kinds], colors="k", linewidths=3)


@classic("contour_line_start_on_corner_edge", figsize=(6, 5))
@without_text
def _(fig):
    ax = fig.subplots()
    x, y = np.meshgrid([0, 1, 2, 3, 4], [0, 1, 2])
    z = 1.2 - (x - 2)**2 + (y - 1)**2
    mask = np.zeros_like(z, dtype=bool)
    mask[1, 1] = mask[1, 3] = True
    z = np.ma.array(z, mask=mask)
    filled = ax.contourf(x, y, z, corner_mask=True)
    cbar = fig.colorbar(filled)
    lines = ax.contour(x, y, z, corner_mask=True, colors="k")
    cbar.add_lines(lines)


@mpl20("contourf_rasterize", dpi=25)
def _(fig):
    ax = fig.subplots()
    circle = mpatches.Circle([0.5, 0.5], 0.5, transform=ax.transAxes)
    ax.contourf([[0, 1], [1, 0]], clip_path=circle, rasterized=True)


@classic("all_algorithms")
@without_text
def _(fig):
    algorithms = ["mpl2005", "mpl2014", "serial", "threaded"]
    x, y = np.meshgrid(np.linspace(0.0, 1.0, 10), np.linspace(0.0, 1.0, 6))
    z = np.sin(15 * x) * np.cos(10 * y) + rng(2981).normal(60, scale=0.5).reshape(6, 10)
    mask = np.zeros_like(z, dtype=bool)
    mask[3, 7] = True
    z = np.ma.array(z, mask=mask)
    for ax, algorithm in zip(fig.subplots(2, 2).ravel(), algorithms):
        ax.contourf(x, y, z, algorithm=algorithm)
        ax.contour(x, y, z, algorithm=algorithm, colors="k")
        ax.set_title(algorithm)


@classic("contour_aliases")
def _(fig):
    data = np.arange(100).reshape((10, 10)) ** 2
    fig.add_subplot().contour(data, linestyle=":")


# --- test_colorbar.py -----------------------------------------------------

#: The data and levels most colorbar tests contour.
RAMP = np.arange(1200).reshape(30, 40)
RAMP_LEVELS = [0, 200, 400, 600, 800, 1000, 1200]


def _positioning_horizontal(fig, use_gridspec):
    cs = fig.add_subplot().contourf(RAMP, levels=RAMP_LEVELS)
    fig.colorbar(cs, orientation="horizontal", use_gridspec=use_gridspec)


def _positioning_locations(fig, use_gridspec):
    for i, location in enumerate(["left", "right", "top", "bottom"]):
        cs = fig.add_subplot(2, 2, i + 1).contourf(RAMP, levels=RAMP_LEVELS)
        fig.colorbar(cs, location=location, use_gridspec=use_gridspec)


def _positioning_double(fig, use_gridspec):
    ax = fig.add_subplot()
    data_2nd = np.array([[2, 3, 2, 3], [1.5, 2, 2, 3], [2, 3, 3, 4]])
    data_2nd = np.repeat(np.repeat(data_2nd, 10, axis=1), 10, axis=0)
    color_mappable = ax.contourf(RAMP, levels=RAMP_LEVELS, extend="both")
    hatch_mappable = ax.contourf(data_2nd, levels=[1, 2, 3], colors="none",
                                 hatches=["/", "o", "+"], extend="max")
    ax.contour(data_2nd, levels=[1, 2, 3], colors="black")
    fig.colorbar(color_mappable, location="left", label="variable 1",
                 use_gridspec=use_gridspec)
    fig.colorbar(hatch_mappable, location="right", label="variable 2",
                 use_gridspec=use_gridspec)


def _positioning_sharing(fig):
    ax1 = fig.add_subplot(211, anchor="NE", aspect="equal")
    ax1.contourf(RAMP, levels=RAMP_LEVELS)
    ax2 = fig.add_subplot(223)
    ax2.contourf(RAMP, levels=RAMP_LEVELS)
    ax3 = fig.add_subplot(224)
    cs = ax3.contourf(RAMP, levels=RAMP_LEVELS)
    fig.colorbar(cs, ax=[ax2, ax3, ax1], location="right", pad=0.0, shrink=0.5, panchor=False)
    fig.colorbar(cs, ax=[ax2, ax3, ax1], location="left", shrink=0.5, panchor=False)
    fig.colorbar(cs, ax=[ax1], location="bottom", panchor=False, anchor=(0.8, 0.5), shrink=0.6)


for _suffix, _use_gridspec in (("_gridspec", True), ("", False)):
    for _number, _figure in enumerate(
            (_positioning_horizontal, _positioning_locations, _positioning_double), start=1):
        mpl20(f"colorbar_positioning{_suffix}_{_number}", dpi=40)(without_text(
            lambda fig, draw=_figure, use=_use_gridspec: draw(fig, use)))
mpl20("colorbar_positioning_4", dpi=40)(without_text(_positioning_sharing))


@classic("contour_colorbar", figsize=(4, 2))
@without_text
def _(fig):
    ax = fig.subplots()
    data = RAMP - 500
    levels = np.array(RAMP_LEVELS) - 500
    CS = ax.contour(data, levels=levels, extend="both")
    fig.colorbar(CS, orientation="horizontal", extend="both")
    fig.colorbar(CS, orientation="vertical")


@classic("gridspec_make_colorbar", dpi=40)
@without_text
def _(fig):
    for index, orientation in ((121, "vertical"), (122, "horizontal")):
        cs = fig.add_subplot(index).contourf(RAMP, levels=RAMP_LEVELS)
        fig.colorbar(cs, use_gridspec=True, orientation=orientation)
    fig.subplots_adjust(top=0.95, right=0.95, bottom=0.2, hspace=0.25)


@classic("colorbar_single_scatter", dpi=40)
@without_text
def _(fig):
    cmap = matplotlib.colormaps["jet"].resampled(16)
    cs = fig.add_subplot().scatter([0], [0], [50], c=[50], cmap=cmap)
    fig.colorbar(cs)


@classic("colorbar_closed_patch", rc=NO_SNAP)
@without_text
def _(fig):
    ax1 = fig.add_axes((0.05, 0.85, 0.9, 0.1))
    ax2 = fig.add_axes((0.1, 0.65, 0.75, 0.1))
    ax3 = fig.add_axes((0.05, 0.45, 0.9, 0.1))
    ax4 = fig.add_axes((0.05, 0.25, 0.9, 0.1))
    ax5 = fig.add_axes((0.05, 0.05, 0.9, 0.1))
    cmap = matplotlib.colormaps["RdBu"].resampled(5)
    im = ax1.pcolormesh(np.linspace(0, 10, 16).reshape((4, 4)), cmap=cmap)
    cbar_kw = dict(orientation="horizontal", values=np.linspace(0, 10, 5), ticks=[])
    with matplotlib.rc_context({"axes.linewidth": 16}):
        fig.colorbar(im, cax=ax2, extend="both", extendfrac=0.5, **cbar_kw)
        fig.colorbar(im, cax=ax3, extend="both", **cbar_kw)
        fig.colorbar(im, cax=ax4, extend="both", extendrect=True, **cbar_kw)
        fig.colorbar(im, cax=ax5, extend="neither", **cbar_kw)


@mpl20("keeping_xlabel")
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(np.arange(25).reshape((5, 5)))
    cbar = fig.colorbar(im)
    cbar.ax.set_xlabel("Visible Xlabel")
    cbar.set_label("YLabel")


@mpl20("colorbar_change_lim_scale")
@without_text
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots(1, 2)
    pc = ax[0].pcolormesh(np.arange(100).reshape(10, 10) + 1)
    cb = fig.colorbar(pc, ax=ax[0], extend="both")
    cb.ax.set_yscale("log")
    pc = ax[1].pcolormesh(np.arange(100).reshape(10, 10) + 1)
    cb = fig.colorbar(pc, ax=ax[1], extend="both")
    cb.ax.set_ylim(20, 90)


@classic("axes_handles_same_functions")
def _(fig):
    ax = fig.add_subplot()
    pc = ax.pcolormesh(np.ones(300).reshape(10, 30))
    cax = fig.add_axes((0.9, 0.1, 0.03, 0.8))
    cb = fig.colorbar(pc, cax=cax)
    cb.ax.set_yticks(np.arange(0, 20))
    cb.ax.set_yscale("log")
    cb.ax.set_position([0.92, 0.1, 0.02, 0.7])


@mpl20("twoslope_colorbar")
@without_text
def _(fig):
    ax = fig.subplots()
    norm = mcolors.TwoSlopeNorm(20, 5, 95)
    pc = ax.pcolormesh(np.arange(1, 11), np.arange(1, 11), np.arange(100).reshape(10, 10),
                       norm=norm, cmap="RdBu_r")
    fig.colorbar(pc)


@mpl20("proportional_colorbars")
@without_text
def _(fig):
    x = y = np.arange(-3.0, 3.01, 0.025)
    X, Y = np.meshgrid(x, y)
    Z = (np.exp(-X**2 - Y**2) - np.exp(-(X - 1)**2 - (Y - 1)**2)) * 2
    levels = [-1.25, -0.5, -0.125, 0.125, 0.5, 1.25]
    cmap = mcolors.ListedColormap(["0.3", "0.5", "white", "lightblue", "steelblue"],
                                  under="darkred", over="crimson")
    norm = mcolors.BoundaryNorm(levels, cmap.N)
    axs = fig.subplots(2, 2)
    for i, extend in enumerate(["neither", "both"]):
        for j, spacing in enumerate(["uniform", "proportional"]):
            CS3 = axs[i, j].contourf(X, Y, Z, levels, cmap=cmap, norm=norm, extend=extend)
            fig.colorbar(CS3, spacing=spacing, ax=axs[i, j])


@mpl20("colorbar_extend_drawedges", figsize=(10, 4), rc={"axes.linewidth": 2})
@without_text
def _(fig):
    params = [("both", 1), ("min", 0), ("max", 0), ("neither", -1)]
    subfigs = fig.subfigures(1, 2)
    for orientation, subfig in zip(["horizontal", "vertical"], subfigs):
        if orientation == "horizontal":
            axs = subfig.subplots(4, 1)
        else:
            axs = subfig.subplots(1, 4)
        fig.subplots_adjust(left=0.05, bottom=0.05, right=0.95, top=0.95)
        for ax, (extend, coloroffset) in zip(axs, params):
            cmap = matplotlib.colormaps["viridis"]
            bounds = np.arange(5)
            nb_colors = len(bounds) + coloroffset
            colors = cmap(np.linspace(100, 255, nb_colors).astype(int))
            cmap, norm = mcolors.from_levels_and_colors(bounds, colors, extend=extend)
            Colorbar(ax, cmap=cmap, norm=norm, orientation=orientation, drawedges=True)
            if orientation == "horizontal":
                ax.set_xlim(1.1, 2.9)
            else:
                ax.set_ylim(1.1, 2.9)


@mpl20("colorbar_contourf_extend_patches", figsize=(10, 4), rc={"axes.linewidth": 2})
@without_text
def _(fig):
    params = [
        ("both", 5, ["\\", "//"]),
        ("min", 7, ["+"]),
        ("max", 2, ["|", "-", "/", "\\", "//"]),
        ("neither", 10, ["//", "\\", "||"]),
    ]
    subfigs = fig.subfigures(1, 2)
    fig.subplots_adjust(left=0.05, bottom=0.05, right=0.95, top=0.95)
    x = np.linspace(-2, 3, 50)
    y = np.linspace(-2, 3, 30)
    z = np.cos(x[np.newaxis, :]) + np.sin(y[:, np.newaxis])
    cmap = matplotlib.colormaps["viridis"]
    for orientation, subfig in zip(["horizontal", "vertical"], subfigs):
        axs = subfig.subplots(2, 2).ravel()
        for ax, (extend, levels, hatches) in zip(axs, params):
            cs = ax.contourf(x, y, z, levels, hatches=hatches, cmap=cmap, extend=extend)
            subfig.colorbar(cs, ax=ax, orientation=orientation, fraction=0.4, extendfrac=0.2,
                            aspect=5)


@mpl20("nonorm", figsize=(6, 1))
def _(fig):
    data = [1, 2, 3, 4, 5]
    ax = fig.subplots()
    fig.subplots_adjust(bottom=0.5)
    norm = mcolors.NoNorm(vmin=min(data), vmax=max(data))
    cmap = matplotlib.colormaps["viridis"].resampled(len(data))
    mappable = cm.ScalarMappable(norm=norm, cmap=cmap)
    fig.colorbar(mappable, cax=ax, orientation="horizontal")


@mpl20("boundaries", figsize=(2, 2))
@without_text
def _(fig):
    ax = fig.subplots()
    pc = ax.pcolormesh(rng(19680808).normal(100).reshape(10, 10), cmap="RdBu_r")
    fig.colorbar(pc, ax=ax, boundaries=np.linspace(-3, 3, 7))


@classic("colorbar_extend_alpha", dpi=40)
@without_text
def _(fig):
    ax = fig.subplots()
    im = ax.imshow([[0, 1], [2, 3]], alpha=0.3, interpolation="none")
    fig.colorbar(im, extend="both", boundaries=[0.5, 1.5, 2.5])


@classic("passing_location")
def _(fig):
    ax = fig.add_subplot()
    im = ax.imshow([[0, 1], [2, 3]])
    fig.colorbar(im, cax=ax.inset_axes([0, 1.05, 1, 0.05]), location="top")
