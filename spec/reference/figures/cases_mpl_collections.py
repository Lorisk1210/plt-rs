"""matplotlib's own image tests from ``tests/test_collections.py`` and
``tests/test_patches.py``: every ``@image_comparison`` test and the
``fig_test`` side of every ``@check_figures_equal`` test.

Each case keeps the test's style. The test suite's default (also used by
``check_figures_equal``) is ``classic`` plus ``_classic_test_patch``: such
figures are 8x6 in and saved at classic's 100 dpi, or at classic's 80 dpi
figure resolution for the tests that set ``savefig.dpi`` to it. Tests asking
for ``mpl20``/``default`` run in the reference configuration.
``remove_text=True`` becomes ``remove_ticks_and_titles`` at the end of the
build, and the tests' own rcParams changes become the case's ``rc``.
Parametrized tests contribute one case per parameter set, and tests drawing
several figures one case per figure (``_1``, ``_2``, ...).
``test_units_rectangle`` registers the converters of
``matplotlib.testing.jpl_units`` only while the case is built
(``spec_support.units.jpl_registered``), as the test fixture restores the
registry after each test.
"""

import itertools
from datetime import datetime

import matplotlib
import matplotlib.style
import numpy as np
from matplotlib import collections as mcollections
from matplotlib import patches as mpatches
from matplotlib import path as mpath
from matplotlib import transforms as mtransforms
from matplotlib.testing.decorators import remove_ticks_and_titles
from mpl_toolkits.mplot3d import Axes3D

from spec_support import case
from spec_support.units import jpl_registered

GROUP = "mpl_collections"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


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


# --- test_collections.py: EventCollection --------------------------------

POSITIONS = np.array([0., 1., 2., 3., 5., 8., 13., 21.])
EXTRA_POSITIONS = np.array([34., 55., 89.])


def _event_collection(fig):
    """``generate_EventCollection_plot``: eight red events on y = 1."""
    coll = mcollections.EventCollection(
        POSITIONS, orientation="horizontal", lineoffset=1, linelength=.5,
        linewidth=2, color=[1, 0, 0, 1], linestyle="solid", antialiased=True)
    ax = fig.subplots()
    ax.add_collection(coll)
    ax.set_title("EventCollection: default")
    ax.set_xlim(-1, 22)
    ax.set_ylim(0, 2)
    return ax, coll


@mpl20("EventCollection__get_props")
def _(fig):
    _event_collection(fig)


@mpl20("EventCollection__set_positions")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.set_positions(np.hstack([POSITIONS, EXTRA_POSITIONS]))
    ax.set_title("EventCollection: set_positions")
    ax.set_xlim(-1, 90)


@mpl20("EventCollection__add_positions")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.switch_orientation()
    coll.add_positions(EXTRA_POSITIONS[0])
    coll.switch_orientation()
    ax.set_title("EventCollection: add_positions")
    ax.set_xlim(-1, 35)


@mpl20("EventCollection__append_positions")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.append_positions(EXTRA_POSITIONS[2])
    ax.set_title("EventCollection: append_positions")
    ax.set_xlim(-1, 90)


@mpl20("EventCollection__extend_positions")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.extend_positions(EXTRA_POSITIONS[1:])
    ax.set_title("EventCollection: extend_positions")
    ax.set_xlim(-1, 90)


@mpl20("EventCollection__switch_orientation")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.switch_orientation()
    ax.set_title("EventCollection: switch_orientation")
    ax.set_ylim(-1, 22)
    ax.set_xlim(0, 2)


@mpl20("EventCollection__switch_orientation_2x")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.switch_orientation()
    coll.switch_orientation()
    ax.set_title("EventCollection: switch_orientation 2x")


@mpl20("EventCollection__set_orientation")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.set_orientation("vertical")
    ax.set_title("EventCollection: set_orientation")
    ax.set_ylim(-1, 22)
    ax.set_xlim(0, 2)


@mpl20("EventCollection__set_linelength")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.set_linelength(15)
    ax.set_title("EventCollection: set_linelength")
    ax.set_ylim(-20, 20)


@mpl20("EventCollection__set_lineoffset")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.set_lineoffset(-5.)
    ax.set_title("EventCollection: set_lineoffset")
    ax.set_ylim(-6, -4)


def _register_set_prop(number, prop, value):
    """One figure of ``test__EventCollection__set_prop``."""
    def build(fig):
        ax, coll = _event_collection(fig)
        coll.set(**{prop: value})
        ax.set_title(f"EventCollection: set_{prop}")
    mpl20(f"EventCollection__set_prop_{number}")(build)


for _number, (_prop, _value) in enumerate([("linestyle", "dashed"),
                                           ("linestyle", (0, (3.7, 1.6))),
                                           ("linewidth", 5)], start=1):
    _register_set_prop(_number, _prop, _value)


@mpl20("EventCollection__set_color")
def _(fig):
    ax, coll = _event_collection(fig)
    coll.set_color(np.array([0, 1, 1, 1]))
    ax.set_title("EventCollection: set_color")


# --- test_collections.py: other collections ------------------------------


@mpl20("collection_log_datalim")
def _(fig):
    x_vals = [4.38462e-6, 5.54929e-6, 7.02332e-6, 8.88889e-6, 1.12500e-5,
              1.42383e-5, 1.80203e-5, 2.28070e-5, 2.88651e-5, 3.65324e-5,
              4.62363e-5, 5.85178e-5, 7.40616e-5, 9.37342e-5, 1.18632e-4]
    y_vals = [0.0, 0.1, 0.182, 0.332, 0.604, 1.1, 2.0, 3.64, 6.64, 12.1, 22.0,
              39.6, 71.3]
    x, y = np.meshgrid(x_vals, y_vals)
    ax = fig.subplots()
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.scatter(x.flatten(), y.flatten())


@classic("EllipseCollection")
@without_text
def _(fig):
    ax = fig.subplots()
    x = np.arange(4)
    y = np.arange(3)
    X, Y = np.meshgrid(x, y)
    XY = np.vstack((X.ravel(), Y.ravel())).T
    ww = X / x[-1]
    hh = Y / y[-1]
    aa = np.ones_like(ww) * 20
    ec = mcollections.EllipseCollection(
        ww, hh, aa, units="x", offsets=XY, offset_transform=ax.transData,
        facecolors="none")
    ax.add_collection(ec)


@mpl20("polycollection_close", rc={"axes3d.automargin": True})
@without_text
def _(fig):
    verts_quad = [
        [[0., 0.], [0., 1.], [1., 1.], [1., 0.]],
        [[0., 1.], [2., 3.], [2., 2.], [1., 1.]],
        [[2., 2.], [2., 3.], [4., 1.], [3., 1.]],
        [[3., 0.], [3., 1.], [4., 1.], [4., 0.]]]
    ax = fig.add_axes(Axes3D(fig))
    colors = ["r", "g", "b", "y", "k"]
    zpos = list(range(5))
    poly = mcollections.PolyCollection(verts_quad * len(zpos), linewidth=0.25)
    poly.set_alpha(0.7)
    zs = []
    cs = []
    for z, c in zip(zpos, colors):
        zs.extend([z] * len(verts_quad))
        cs.extend([c] * len(verts_quad))
    poly.set_color(cs)
    ax.add_collection3d(poly, zs=zs, zdir="y")
    ax.set_xlim3d(0, 4)
    ax.set_zlim3d(0, 3)
    ax.set_ylim3d(0, 4)


@classic("scalarmap_change_cmap")
def _(fig):
    x, y, z = np.array(list(itertools.product(
        np.arange(0, 5, 1), np.arange(0, 5, 1), np.arange(0, 5, 1)))).T
    ax = fig.add_subplot(111, projection="3d")
    sc = ax.scatter(x, y, z, c=x + y, s=40, cmap="jet")
    fig.canvas.draw()
    sc.set_cmap("viridis")


@classic("regularpolycollection_rotate")
@without_text
def _(fig):
    xx, yy = np.mgrid[:10, :10]
    xy_points = np.transpose([xx.flatten(), yy.flatten()])
    rotations = np.linspace(0, 2 * np.pi, len(xy_points))
    ax = fig.subplots()
    for xy, alpha in zip(xy_points, rotations):
        col = mcollections.RegularPolyCollection(
            4, sizes=(100,), rotation=alpha,
            offsets=[xy], offset_transform=ax.transData)
        ax.add_collection(col)


class SquareCollection(mcollections.RegularPolyCollection):
    """``test_regularpolycollection_scale``'s squares, sized in data units."""

    def __init__(self, **kwargs):
        super().__init__(4, rotation=np.pi / 4., **kwargs)

    def get_transform(self):
        """Return transform scaling circle areas to data space."""
        ax = self.axes
        pts2pixels = 72.0 / ax.get_figure(root=True).dpi
        scale_x = pts2pixels * ax.bbox.width / ax.viewLim.width
        scale_y = pts2pixels * ax.bbox.height / ax.viewLim.height
        return mtransforms.Affine2D().scale(scale_x, scale_y)


@classic("regularpolycollection_scale")
@without_text
def _(fig):
    ax = fig.subplots()
    squares = SquareCollection(
        sizes=[np.pi / 2], offsets=[(0, 0)], offset_transform=ax.transData)
    ax.add_collection(squares)
    ax.axis([-1, 1, -1, 1])


@classic("size_in_xy")
@without_text
def _(fig):
    ax = fig.subplots()
    e = mcollections.EllipseCollection(
        (10, 10), 10, 0, units="xy",
        offsets=[(10, 10), (15, 15)], offset_transform=ax.transData)
    ax.add_collection(e)
    ax.set_xlim(0, 30)
    ax.set_ylim(0, 30)


@mpl20("cap_and_joinstyle_image")
def _(fig):
    ax = fig.subplots()
    ax.set_xlim([-0.5, 1.5])
    ax.set_ylim([-0.5, 2.5])
    x = np.array([0.0, 1.0, 0.5])
    ys = np.array([[0.0], [0.5], [1.0]]) + np.array([[0.0, 0.0, 1.0]])
    segs = np.zeros((3, 3, 2))
    segs[:, :, 0] = x
    segs[:, :, 1] = ys
    line_segments = mcollections.LineCollection(segs, linewidth=[10, 15, 20])
    line_segments.set_capstyle("round")
    line_segments.set_joinstyle("miter")
    ax.add_collection(line_segments)
    ax.set_title("Line collection with customized caps and joinstyle")


@mpl20("scatter_post_alpha")
@without_text
def _(fig):
    sc = fig.subplots().scatter(range(5), range(5), c=range(5))
    sc.set_alpha(.1)


def _register_fill_between_set_data(suffix, kwargs):
    """``test_fill_between_poly_collection_set_data``: new data after the fill."""
    def build(fig):
        t = np.linspace(0, 16)
        f1 = np.sin(t)
        coll = fig.subplots().fill_between(t, -1, 1.2, **kwargs)
        coll.set_data(t, f1, f1 + 0.2)
    classic(f"fill_between_poly_collection_set_data{suffix}")(build)


_register_fill_between_set_data("", {})
_register_fill_between_set_data("_step_pre", {"step": "pre"})


@mpl20("check_masked_offsets")
@without_text
def _(fig):
    unmasked_x = [datetime(2022, 12, 15, 4, 49, second) for second in range(52, 57)]
    masked_y = np.ma.array([1, 2, 3, 4, 5], mask=[0, 1, 1, 0, 0])
    fig.subplots().scatter(unmasked_x, masked_y)


@classic("masked_set_offsets")
def _(fig):
    x = np.ma.array([1, 2, 3, 4, 5], mask=[0, 0, 1, 1, 0])
    y = np.arange(1, 6)
    ax = fig.add_subplot()
    scat = ax.scatter(x, y)
    scat.set_offsets(np.ma.column_stack([x, y]))
    ax.set_xticks([])
    ax.set_yticks([])


def _register_striped_lines(suffix, gapcolor):
    """``test_striped_lines``: dashed vertical lines with gaps in colour."""
    def build(fig):
        ax = fig.add_subplot(111)
        ax.set_xlim(0, 6)
        ax.set_ylim(0, 1)
        ax.vlines(range(1, 6), 0, 1, linewidth=20, linestyle=[":", "-", "--"],
                  gapcolor=gapcolor, alpha=0.5)
    classic(f"striped_lines_{suffix}")(build)


_register_striped_lines("orange", "orange")
_register_striped_lines("r_k", ["r", "k"])


@classic("hatch_linewidth")
def _(fig):
    polygons = [
        [(0.1, 0.1), (0.1, 0.4), (0.4, 0.4), (0.4, 0.1)],
        [(0.6, 0.6), (0.6, 0.9), (0.9, 0.9), (0.9, 0.6)],
    ]
    with matplotlib.rc_context({"hatch.linewidth": 2.0}):
        test = mcollections.PolyCollection(polygons, hatch="x")
    fig.add_subplot().add_collection(test)


# --- test_patches.py: rectangles, paths and wedges -----------------------


@classic("rotate_rect_draw")
def _(fig):
    ax = fig.add_subplot()
    rect = mpatches.Rectangle((0, 0), 1, 1)
    ax.add_patch(rect)
    rect.set_angle(30)


@mpl20("units_rectangle")
def _(fig):
    import matplotlib.testing.jpl_units as U

    with jpl_registered():
        p = mpatches.Rectangle((5 * U.km, 6 * U.km), 1 * U.km, 2 * U.km)
        ax = fig.subplots()
        ax.add_patch(p)
        ax.set_xlim([4 * U.km, 7 * U.km])
        ax.set_ylim([5 * U.km, 9 * U.km])


@classic("dash_offset_patch_draw")
def _(fig):
    ax = fig.add_subplot()
    loc, width, height = (0.1, 0.1), 0.8, 0.8
    ax.add_patch(mpatches.Rectangle(loc, width, height, linewidth=3, edgecolor="b",
                                    linestyle=(0, [6, 6])))
    ax.add_patch(mpatches.Rectangle(loc, width, height, linewidth=3, edgecolor="r",
                                    linestyle=(6, [6, 6])))


@mpl20("clip_to_bbox")
def _(fig):
    ax = fig.subplots()
    ax.set_xlim([-18, 20])
    ax.set_ylim([-150, 100])
    path = mpath.Path.unit_regular_star(8).deepcopy()
    path.vertices *= [10, 100]
    path.vertices -= [5, 25]
    path2 = mpath.Path.unit_circle().deepcopy()
    path2.vertices *= [10, 100]
    path2.vertices += [10, -25]
    combined = mpath.Path.make_compound_path(path, path2)
    ax.add_patch(mpatches.PathPatch(
        combined, alpha=0.5, facecolor="coral", edgecolor="none"))
    bbox = mtransforms.Bbox([[-12, -77.5], [50, -110]])
    ax.add_patch(mpatches.PathPatch(
        combined.clip_to_bbox(bbox), alpha=0.5, facecolor="green", lw=4,
        edgecolor="black"))


def _cut_stars():
    """A circle with a six-pointed star cut out, and the same moved by (1, 1)."""
    star = mpath.Path.unit_regular_star(6)
    circle = mpath.Path.unit_circle()
    verts = np.concatenate([circle.vertices, star.vertices[::-1]])
    codes = np.concatenate([circle.codes, star.codes])
    return mpath.Path(verts, codes), mpath.Path(verts + 1, codes)


def _cut_star_pair(fig, collection_style, patch_style):
    """A cut star as a ``PathCollection`` and another as a ``PathPatch``."""
    cut_star1, cut_star2 = _cut_stars()
    ax = fig.add_subplot()
    ax.add_collection(mcollections.PathCollection([cut_star2], linewidth=5,
                                                  **collection_style))
    ax.add_patch(mpatches.PathPatch(cut_star1, linewidth=5, **patch_style))
    ax.set_xlim(-1, 2)
    ax.set_ylim(-1, 2)


@classic("patch_alpha_coloring")
@without_text
def _(fig):
    colors = {"facecolor": (1, 0, 0, 0.5), "edgecolor": (0, 0, 1, 0.75)}
    _cut_star_pair(fig, {"linestyles": "dashdot", **colors},
                   {"linestyle": "dashdot", **colors})


@classic("patch_alpha_override")
@without_text
def _(fig):
    style = {"alpha": 0.25, "facecolor": (1, 0, 0, 0.5), "edgecolor": (0, 0, 1, 0.75)}
    _cut_star_pair(fig, {"linestyles": "dashdot", **style},
                   {"linestyle": "dashdot", **style})


@classic("patch_custom_linestyle")
@without_text
def _(fig):
    colors = {"facecolor": (1, 0, 0), "edgecolor": (0, 0, 1)}
    dashes = (0, (5, 7, 10, 7))
    _cut_star_pair(fig, {"linestyles": [dashes], **colors},
                   {"linestyle": dashes, **colors})


@classic("patch_linestyle_none")
def _(fig):
    circle = mpath.Path.unit_circle()
    ax = fig.add_subplot()
    for i, ls in enumerate(["none", "None", " ", ""]):
        path = mpath.Path(circle.vertices + i, circle.codes)
        ax.add_patch(mpatches.PathPatch(path, linewidth=3, linestyle=ls,
                                        facecolor=(1, 0, 0), edgecolor=(0, 0, 1)))
    ax.set_xlim([-1, i + 1])
    ax.set_ylim([-1, i + 1])


@classic("wedge_range")
@without_text
def _(fig):
    ax = fig.add_subplot()
    t1 = 2.313869244286224
    args = [[52.31386924, 232.31386924],
            [52.313869244286224, 232.31386924428622],
            [t1, t1 + 180.0],
            [0, 360],
            [90, 90 + 360],
            [-180, 180],
            [0, 380],
            [45, 46],
            [46, 45]]
    for i, (theta1, theta2) in enumerate(args):
        x = i % 3
        y = i // 3
        ax.add_artist(mpatches.Wedge((x * 3, y * 3), 1, theta1, theta2,
                                     facecolor="none", edgecolor="k", lw=3))
    ax.set_xlim(-2, 8)
    ax.set_ylim(-2, 9)


@mpl20("multi_color_hatch")
@without_text
def _(fig):
    ax = fig.subplots()
    rects = ax.bar(range(5), range(1, 6))
    for i, rect in enumerate(rects):
        rect.set_facecolor("none")
        rect.set_edgecolor(f"C{i}")
        rect.set_hatch("/")
    ax.autoscale_view()
    ax.autoscale(False)
    for i in range(5):
        with matplotlib.style.context({"hatch.color": f"C{i}"}):
            r = mpatches.Rectangle((i - .8 / 2, 5), .8, 1, hatch="//", fc="none")
        ax.add_patch(r)


# --- test_patches.py: connection patches and shadows ---------------------


@mpl20("connection_patch")
@without_text
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax2.add_artist(mpatches.ConnectionPatch(
        xyA=(0.1, 0.1), xyB=(0.9, 0.9), coordsA="data", coordsB="data",
        axesA=ax2, axesB=ax1, arrowstyle="->"))
    ax2.add_artist(mpatches.ConnectionPatch(
        xyA=(0.6, 1.0), xyB=(0.0, 0.2), coordsA="axes fraction",
        coordsB=ax2.get_yaxis_transform(), arrowstyle="-"))


@classic("connection_patch_fig", dpi=80)
def _(fig):
    ax1, _ = fig.subplots(1, 2)
    fig.add_artist(mpatches.ConnectionPatch(
        xyA=(.3, .2), coordsA="data", axesA=ax1,
        xyB=(-30, -20), coordsB="figure pixels",
        arrowstyle="->", shrinkB=5))


@classic("connection_patch_pixel_points", dpi=80)
def _(fig):
    ax1, _ = fig.subplots(1, 2)
    fig.add_artist(mpatches.ConnectionPatch(
        xyA=(.3, .2), coordsA="axes points", axesA=ax1,
        xyB=(-30, -20), coordsB="figure points",
        arrowstyle="->", shrinkB=5))


@classic("shadow", dpi=80)
def _(fig):
    ax = fig.subplots()
    rect = mpatches.Rectangle(xy=(.2, .3), width=.5, height=.5)
    ax.add_patch(rect)
    ax.add_patch(mpatches.Shadow(rect, ox=.1, oy=.2))


# --- test_patches.py: arcs and annuli ------------------------------------


@mpl20("large_arc")
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    for ax in [ax1, ax2]:
        ax.add_patch(mpatches.Arc((210, -2115), 4261, 4261, lw=2, color="k"))
        ax.set_axis_off()
        ax.set_aspect("equal")
    ax1.set_xlim(7, 8)
    ax1.set_ylim(5, 6)
    ax2.set_xlim(-25000, 18000)
    ax2.set_ylim(-20000, 6600)


@mpl20("rotated_arcs", figsize=(10, 10))
def _(fig):
    ax_arr = fig.subplots(2, 2, squeeze=False)
    scale = 10_000_000
    diag_centers = ((-1, -1), (-1, 1), (1, 1), (1, -1))
    on_axis_centers = ((0, 1), (1, 0), (0, -1), (-1, 0))
    skews = ((2, 2), (2, 1 / 10), (2, 1 / 100), (2, 1 / 1000))
    for ax, (sx, sy) in zip(ax_arr.ravel(), skews):
        k = 0
        for prescale, centers in zip((1 - .0001, (1 - .0001) / np.sqrt(2)),
                                     (on_axis_centers, diag_centers)):
            for j, (x_sign, y_sign) in enumerate(centers, start=k):
                ax.add_patch(mpatches.Arc(
                    (x_sign * scale * prescale, y_sign * scale * prescale),
                    scale * sx, scale * sy, lw=4, color=f"C{j}", zorder=1 + j,
                    angle=np.rad2deg(np.arctan2(y_sign, x_sign)) % 360,
                    label=f"big {j}", gid=f"big {j}"))
            k = j + 1
        ax.set_xlim(-scale / 4000, scale / 4000)
        ax.set_ylim(-scale / 4000, scale / 4000)
        ax.axhline(0, color="k")
        ax.axvline(0, color="k")
        ax.set_axis_off()
        ax.set_aspect("equal")


def _annuli(fig, circular, elliptical):
    """A green circular annulus and a hatched magenta elliptical one."""
    ax = fig.subplots()
    cir = mpatches.Annulus(*circular, fc="g")
    ell = mpatches.Annulus(*elliptical, fc="m", ec="b", alpha=0.5, hatch="xxx")
    ax.add_patch(cir)
    ax.add_patch(ell)
    ax.set_aspect("equal")
    return cir, ell


@mpl20("annulus")
def _(fig):
    _annuli(fig, ((0.5, 0.5), 0.2, 0.05), ((0.5, 0.5), (0.5, 0.3), 0.1, 45))


@mpl20("annulus_setters")
def _(fig):
    cir, ell = _annuli(fig, ((0., 0.), 0.2, 0.01), ((0., 0.), (1, 2), 0.1, 0))
    cir.center = (0.5, 0.5)
    cir.radii = 0.2
    cir.width = 0.05
    ell.center = (0.5, 0.5)
    ell.radii = (0.5, 0.3)
    ell.width = 0.1
    ell.angle = 45


@mpl20("annulus_setters2")
def _(fig):
    cir, ell = _annuli(fig, ((0., 0.), 0.2, 0.01), ((0., 0.), (1, 2), 0.1, 0))
    cir.center = (0.5, 0.5)
    cir.set_semimajor(0.2)
    cir.set_semiminor(0.2)
    cir.width = 0.05
    ell.center = (0.5, 0.5)
    ell.set_semimajor(0.5)
    ell.set_semiminor(0.3)
    ell.width = 0.1
    ell.angle = 45


@mpl20("autoscale_arc", figsize=(4, 1))
@without_text
def _(fig):
    axs = fig.subplots(1, 3)
    arc_lists = (
        [mpatches.Arc((0, 0), 1, 1, theta1=0, theta2=90)],
        [mpatches.Arc((0.5, 0.5), 1.5, 0.5, theta1=10, theta2=20)],
        [mpatches.Arc((0.5, 0.5), 1.5, 0.5, theta1=10, theta2=20),
         mpatches.Arc((0.5, 0.5), 2.5, 0.5, theta1=110, theta2=120),
         mpatches.Arc((0.5, 0.5), 3.5, 0.5, theta1=210, theta2=220),
         mpatches.Arc((0.5, 0.5), 4.5, 0.5, theta1=310, theta2=320)])
    for ax, arcs in zip(axs, arc_lists):
        for arc in arcs:
            ax.add_patch(arc)
        ax.autoscale()


@classic("arc_in_collection")
def _(fig):
    arc = mpatches.Arc([.5, .5], .5, 1, theta1=0, theta2=60, angle=20)
    fig.subplots().add_collection(mcollections.PatchCollection(
        patches=[arc], facecolors="none", edgecolors="k"))


@classic("modifying_arc")
def _(fig):
    arc = mpatches.Arc([.5, .5], 1.5, 1, theta1=0, theta2=60, angle=10)
    fig.subplots().add_patch(arc)
    arc.set_width(.5)
    arc.set_angle(20)


@classic("set_and_get_hatch_linewidth")
def _(fig):
    ax = fig.add_subplot()
    ax.add_patch(mpatches.Rectangle((0, 0), 1, 1, hatch="x"))
    ax.patches[0].set_hatch_linewidth(2.0)


@mpl20("patch_edgegapcolor_visual")
@without_text
def _(fig):
    ax = fig.subplots()
    ax.add_patch(mpatches.Rectangle((0.1, 0.1), 0.3, 0.3, fill=False,
                                    edgecolor="blue", edgegapcolor="orange",
                                    linestyle="--", linewidth=3))
    ax.add_patch(mpatches.Ellipse((0.7, 0.3), 0.3, 0.2, fill=False,
                                  edgecolor="red", edgegapcolor="yellow",
                                  linestyle=":", linewidth=3))
    ax.add_patch(mpatches.Polygon([[0.1, 0.6], [0.3, 0.9], [0.4, 0.6]], fill=False,
                                  edgecolor="green", edgegapcolor="purple",
                                  linestyle="-.", linewidth=3))
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.set_aspect("equal")
