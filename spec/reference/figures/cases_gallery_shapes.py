"""matplotlib's gallery sections "Shapes and collections"
(``galleries/examples/shapes_and_collections``), "Style sheets"
(``style_sheets``) and "axes_grid1" (``axes_grid1``), one case per figure
(``<example>_<n>`` for examples drawing several).

The examples run as written, with these substitutions: ``np.random`` data
comes from ``spec_support.rng`` seeded with the example's seed (a fresh
generator per figure; beta draws are ratios of its gamma draws), and the
sample data ``axes_grid/bivariate_normal.npy`` is computed from the formula
it was made with (:func:`_bivariate_normal`). Figures the examples make
through pyplot are the case's figure; ``plt.colorbar(im)`` is
``fig.colorbar(im)``, which steals room from the image's axes the same way.

Style sheets the examples activate with ``plt.style.use`` (or with
``plt.style.context`` around the whole figure) become the case's ``rc``,
active for the figure's whole lifetime as in the cases of
``cases_styles.py``. ``petroff10`` creates its figure outside the sheets
and fills each subfigure inside one, so it keeps its ``style.context``
blocks. ``style_sheets_reference`` draws one case per sheet, each pinned
to unhinted DejaVu Sans like ``cases_styles.py`` (``default`` is the
reference configuration itself).

Covered elsewhere: ``collections`` is ``patches/gallery_collections``.

Not ported: the second and third figures of
``make_room_for_ylabel_using_axesgrid``. Each axes there makes room for
the decorations of axes drawn after it, and measuring an axes puts it back
at its original position once it has been drawn, so the positions
matplotlib reports after drawing are not where it drew those axes (pltrs
reports where it drew them; ``toolkits/auto_adjustable_area_shared``
covers the layout without the side effect).
"""

import inspect

import matplotlib as mpl
import matplotlib.style
import numpy as np
from matplotlib import font_manager as fm
from matplotlib import lines as mlines
from matplotlib import patches as mpatches
from matplotlib import path as mpath
from matplotlib import transforms as mtransforms
from matplotlib.collections import EllipseCollection, LineCollection, PatchCollection
from matplotlib.markers import MarkerStyle
from matplotlib.offsetbox import AnchoredText
from matplotlib.patheffects import withStroke
from mpl_toolkits.axes_grid1 import (AxesGrid, Divider, ImageGrid, Size, SubplotDivider,
                                     host_subplot, inset_locator, make_axes_locatable)
from mpl_toolkits.axes_grid1.anchored_artists import (AnchoredDirectionArrows,
                                                      AnchoredDrawingArea, AnchoredSizeBar)
from mpl_toolkits.axes_grid1.axes_divider import (HBoxDivider, VBoxDivider,
                                                  make_axes_area_auto_adjustable)
from mpl_toolkits.axes_grid1.axes_rgb import RGBAxes, make_rgb_axes
from mpl_toolkits.axes_grid1.inset_locator import inset_axes, mark_inset, zoomed_inset_axes
from mpl_toolkits.axes_grid1.parasite_axes import HostAxes

from spec_support import case, rng

G = "gallery_shapes"

#: The examples' ``np.random.seed(19680801)``.
SEED = 19680801

Path = mpath.Path


def gallery(name, **options):
    """Registers a case of this group."""
    return case(f"{G}/{name}", **options)


def _beta(g, a, b, n):
    """``np.random.beta(a, b, size=n)`` for integer ``a`` and ``b``: the
    ratio ``x / (x + y)`` of gamma draws of shapes ``a`` and ``b``."""
    x = g.gamma(n, a)
    y = g.gamma(n, b)
    return x / (x + y)


def _bivariate_normal():
    """``cbook.get_sample_data("axes_grid/bivariate_normal.npy")``: ten times
    the difference of two bivariate normal densities on a 15x15 grid."""
    x = np.arange(-3.0, 4.001, 0.5)
    y = np.arange(-4.0, 3.001, 0.5)
    xx, yy = np.meshgrid(x, y)

    def density(sigmax, sigmay, mux, muy):
        return (np.exp(-((xx - mux) ** 2 / sigmax ** 2 + (yy - muy) ** 2 / sigmay ** 2) / 2)
                / (2 * np.pi * sigmax * sigmay))

    return (density(1.0, 1.0, 0.0, 0.0) - density(1.5, 0.5, 1.0, 1.0)) * 10


#: The extent the examples show the sample data in.
EXTENT = (-3, 4, -4, 3)


# --- shapes_and_collections: arrows and artists ----------------------------

_X_TAIL, _Y_TAIL = 0.1, 0.5
_X_HEAD, _Y_HEAD = 0.9, 0.8
_DX, _DY = _X_HEAD - _X_TAIL, _Y_HEAD - _Y_TAIL


@gallery("arrow_guide_1")
def _(fig):
    axs = fig.subplots(nrows=2)
    for ax in axs:
        ax.add_patch(mpatches.FancyArrowPatch((_X_TAIL, _Y_TAIL), (_X_HEAD, _Y_HEAD),
                                              mutation_scale=100))
    axs[1].set(xlim=(0, 2), ylim=(0, 2))


@gallery("arrow_guide_2")
def _(fig):
    axs = fig.subplots(nrows=2)
    for ax in axs:
        ax.add_patch(mpatches.FancyArrowPatch((_X_TAIL, _Y_TAIL), (_X_HEAD, _Y_HEAD),
                                              mutation_scale=100, transform=ax.transAxes))
    axs[1].set(xlim=(0, 2), ylim=(0, 2))


@gallery("arrow_guide_3")
def _(fig):
    axs = fig.subplots(nrows=2)
    for ax in axs:
        ax.add_patch(mpatches.Arrow(_X_TAIL, _Y_TAIL, _DX, _DY))
        ax.add_patch(mpatches.FancyArrow(_X_TAIL, _Y_TAIL - .4, _DX, _DY, width=.1,
                                         length_includes_head=True, color="C1"))
        ax.arrow(_X_TAIL + 1, _Y_TAIL - .4, _DX, _DY, width=.1, length_includes_head=True,
                 color="C2")
    axs[1].set(xlim=(0, 2), ylim=(0, 2))


@gallery("artist_reference", figsize=(6, 6))
def _(fig):
    codes, verts = zip(*[
        (Path.MOVETO, [0.018, -0.11]),
        (Path.CURVE4, [-0.031, -0.051]),
        (Path.CURVE4, [-0.115, 0.073]),
        (Path.CURVE4, [-0.03, 0.073]),
        (Path.LINETO, [-0.011, 0.039]),
        (Path.CURVE4, [0.043, 0.121]),
        (Path.CURVE4, [0.075, -0.005]),
        (Path.CURVE4, [0.035, -0.027]),
        (Path.CLOSEPOLY, [0.018, -0.11])])
    artists = [
        mpatches.Circle((0, 0), 0.1, ec="none"),
        mpatches.Rectangle((-0.025, -0.05), 0.05, 0.1, ec="none"),
        mpatches.Wedge((0, 0), 0.1, 30, 270, ec="none"),
        mpatches.RegularPolygon((0, 0), 5, radius=0.1),
        mpatches.Ellipse((0, 0), 0.2, 0.1),
        mpatches.Arrow(-0.05, -0.05, 0.1, 0.1, width=0.1),
        mpatches.PathPatch(mpath.Path(verts, codes), ec="none"),
        mpatches.FancyBboxPatch((-0.025, -0.05), 0.05, 0.1, ec="none",
                                boxstyle=mpatches.BoxStyle("Round", pad=0.02)),
        mlines.Line2D([-0.06, 0.0, 0.1], [0.05, -0.05, 0.05], lw=5),
    ]
    fig.set_layout_engine("constrained")
    axs = fig.subplots(3, 3)
    for i, (ax, artist) in enumerate(zip(axs.flat, artists)):
        artist.set(color=mpl.colormaps["hsv"](i / len(artists)))
        ax.add_artist(artist)
        ax.set(title=type(artist).__name__, aspect=1, xlim=(-.2, .2), ylim=(-.2, .2))
        ax.set_axis_off()


@gallery("ellipse_arrow")
def _(fig):
    ax = fig.subplots(subplot_kw={"aspect": "equal"})
    ellipse = mpatches.Ellipse(xy=(2, 4), width=30, height=20, angle=35, facecolor="none",
                               edgecolor="b")
    ax.add_patch(ellipse)
    vertices = ellipse.get_co_vertices()
    t = mtransforms.Affine2D().rotate_deg(ellipse.angle)
    ax.plot(vertices[0][0], vertices[0][1], color="b", marker=MarkerStyle(">", "full", t),
            markersize=10)


@gallery("ellipse_demo_1")
def _(fig):
    g = rng(SEED)
    ells = []
    for _ in range(250):
        xy = g.uniform(2) * 10
        width, height = g.uniform(2)
        ells.append(mpatches.Ellipse(xy=xy, width=width, height=height,
                                     angle=g.next_f64() * 360))
    ax = fig.subplots()
    ax.set(xlim=(0, 10), ylim=(0, 10), aspect="equal")
    for e in ells:
        ax.add_artist(e)
        e.set_clip_box(ax.bbox)
        e.set_alpha(g.next_f64())
        e.set_facecolor(g.uniform(3))


@gallery("ellipse_demo_2")
def _(fig):
    ax = fig.subplots()
    ax.set(xlim=(-2.2, 2.2), ylim=(-2.2, 2.2), aspect="equal")
    for angle in np.arange(0, 180, 45):
        ax.add_artist(mpatches.Ellipse((0, 0), 4, 2, angle=angle, alpha=0.1))


# --- shapes_and_collections: fancy boxes ------------------------------------

#: ``inspect.signature`` of each ``BoxStyle`` class, one parameter per line.
_BOXSTYLE_SIGNATURES = {
    name: str(inspect.signature(cls))[1:-1].replace(", ", "\n")
    for name, cls in mpatches.BoxStyle.get_styles().items()
}


@gallery("fancybox_demo_1", figsize=(6, 6))
def _(fig):
    axs = fig.add_gridspec(6, 2, wspace=.5).subplots()
    for ax in axs.flat:
        ax.set_axis_off()
    for ax in axs[0, :]:
        ax.text(.2, .5, "boxstyle", transform=ax.transAxes, size="large", color="tab:blue",
                horizontalalignment="right", verticalalignment="center")
        ax.text(.4, .5, "default parameters", transform=ax.transAxes,
                horizontalalignment="left", verticalalignment="center")
    for ax, (stylename, signature) in zip(axs[1:, :].T.flat, _BOXSTYLE_SIGNATURES.items()):
        ax.text(.2, .5, stylename, bbox=dict(boxstyle=stylename, fc="w", ec="k"),
                transform=ax.transAxes, size="large", color="tab:blue",
                horizontalalignment="right", verticalalignment="center")
        ax.text(.4, .5, signature, transform=ax.transAxes,
                horizontalalignment="left", verticalalignment="center")


def _fancy_patch_around(ax, bb, **kwargs):
    """``add_fancy_patch_around``: a translucent pink box around ``bb``."""
    kwargs = {"facecolor": (1, 0.8, 1, 0.5), "edgecolor": (1, 0.5, 1, 0.5), **kwargs}
    fancy = mpatches.FancyBboxPatch(bb.p0, bb.width, bb.height, **kwargs)
    ax.add_patch(fancy)
    return fancy


def _bare_box(ax, bb):
    """The original ``bb``, drawn as a black square box with no padding."""
    _fancy_patch_around(ax, bb, boxstyle="square,pad=0", edgecolor="black", facecolor="none",
                        zorder=10)


@gallery("fancybox_demo_2", figsize=(8, 8))
def _(fig):
    axs = fig.subplots(2, 2)
    bb = mtransforms.Bbox([[0.3, 0.4], [0.7, 0.6]])
    ax = axs[0, 0]
    _fancy_patch_around(ax, bb, boxstyle="round,pad=0.1")
    ax.set(xlim=(0, 1), ylim=(0, 1), aspect=1, title='boxstyle="round,pad=0.1"')
    ax = axs[0, 1]
    fancy = _fancy_patch_around(ax, bb, boxstyle="round,pad=0.1")
    fancy.set_boxstyle("round,pad=0.1,rounding_size=0.2")
    ax.set(xlim=(0, 1), ylim=(0, 1), aspect=1,
           title='boxstyle="round,pad=0.1,rounding_size=0.2"')
    ax = axs[1, 0]
    _fancy_patch_around(ax, bb, boxstyle="round,pad=0.1", mutation_scale=2)
    ax.set(xlim=(0, 1), ylim=(0, 1), aspect=1,
           title='boxstyle="round,pad=0.1"\n mutation_scale=2')
    ax = axs[1, 1]
    _fancy_patch_around(ax, bb, boxstyle="round,pad=0.1", mutation_aspect=0.5)
    ax.set(xlim=(0, 1), ylim=(0, 1), title='boxstyle="round,pad=0.1"\nmutation_aspect=0.5')
    for ax in axs.flat:
        for patch in ax.patches:
            patch.axes.plot(*patch.get_path().vertices.T, ".", c=patch.get_edgecolor())
        _bare_box(ax, bb)
    fig.tight_layout()


@gallery("fancybox_demo_3", figsize=(6.5, 5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    bb = mtransforms.Bbox([[-0.5, -0.5], [0.5, 0.5]])
    for ax in (ax1, ax2):
        _bare_box(ax, bb)
        ax.set(xlim=(-1.5, 1.5), ylim=(-1.5, 1.5), aspect=2)
    _fancy_patch_around(ax1, bb, boxstyle="round,pad=0.5")
    ax1.set_title("aspect=2\nmutation_aspect=1")
    _fancy_patch_around(ax2, bb, boxstyle="round,pad=0.5", mutation_aspect=0.5)
    ax2.set_title("aspect=2\nmutation_aspect=0.5")


# --- shapes_and_collections: hatches -----------------------------------------


@gallery("hatchcolor_demo_1")
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.add_patch(mpatches.Rectangle((0.1, 0.5), 0.8, 0.3, hatch=".", hatchcolor="red",
                                     edgecolor="black", lw=2))
    ax1.add_patch(mpatches.Rectangle((0.1, 0.1), 0.8, 0.3, hatch="x", edgecolor="orange",
                                     lw=2))
    x = np.arange(1, 5)
    y = np.arange(1, 5)
    ax2.bar(x, y, facecolor="none", edgecolor="red", hatch="//", hatchcolor="blue")
    ax2.set_xlim(0, 5)
    ax2.set_ylim(0, 5)


@gallery("hatchcolor_demo_2")
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 1, 10)
    y = np.linspace(0, 1, 9)
    X, Y = np.meshgrid(x, y)
    X[1::2, :] += (x[1] - x[0]) / 2
    colors = [mpl.colormaps["rainbow"](val) for val in x]
    ax.scatter(X.ravel(), Y.ravel(), s=1700, facecolor="none", edgecolor="gray", linewidth=2,
               marker="h", hatch="xxx", hatchcolor=colors)
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)


@gallery("hatch_demo")
def _(fig):
    x = np.arange(1, 5)
    y1 = np.arange(1, 5)
    y2 = np.ones(y1.shape) * 4
    axs = fig.subplot_mosaic([["bar1", "patches"], ["bar2", "patches"]])
    axs["bar1"].bar(x, y1, edgecolor="black", hatch="/")
    axs["bar1"].bar(x, y2, bottom=y1, edgecolor="black", hatch="//")
    axs["bar2"].bar(x, y1, edgecolor="black", hatch=["--", "+", "x", "\\"])
    axs["bar2"].bar(x, y2, bottom=y1, edgecolor="black", hatch=["*", "o", "O", "."])
    x = np.arange(0, 40, 0.2)
    axs["patches"].fill_between(x, np.sin(x) * 4 + 30, y2=0, hatch="///", zorder=2, fc="c")
    axs["patches"].add_patch(mpatches.Ellipse((4, 50), 10, 10, fill=True, hatch="*",
                                              facecolor="y"))
    axs["patches"].add_patch(mpatches.Polygon([(10, 20), (30, 50), (50, 10)], hatch=r"\/...",
                                              facecolor="g"))
    axs["patches"].set_xlim(0, 40)
    axs["patches"].set_ylim(10, 60)
    axs["patches"].set_aspect(1)


def _hatch_swatches(fig, hatches):
    """``hatch_style_reference``'s figures: a labelled square per hatch."""
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 5)
    for ax, h in zip(axs.flat, hatches):
        ax.add_patch(mpatches.Rectangle((0, 0), 2, 2, fill=False, hatch=h))
        ax.text(1, -0.5, f"' {h} '", size=15, ha="center")
        ax.axis("equal")
        ax.axis("off")


@gallery("hatch_style_reference_1", figsize=(6.4, 3.2))
def _(fig):
    _hatch_swatches(fig, ["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*"])


@gallery("hatch_style_reference_2", figsize=(6.4, 3.2))
def _(fig):
    _hatch_swatches(fig, ["//", r"\\", "||", "--", "++", "xx", "oo", "OO", "..", "**"])


@gallery("hatch_style_reference_3", figsize=(6.4, 3.2))
def _(fig):
    _hatch_swatches(fig, ["/o", r"\|", "|*", r"\-", "+o", "x*", "o-", "O|", "O.", "*-"])


# --- shapes_and_collections: collections -------------------------------------


@gallery("ellipse_collection")
def _(fig):
    x = np.arange(10)
    y = np.arange(15)
    X, Y = np.meshgrid(x, y)
    XY = np.column_stack((X.ravel(), Y.ravel()))
    ax = fig.subplots()
    ec = EllipseCollection(X / 10.0, Y / 15.0, X * 9, units="x", offsets=XY,
                           offset_transform=ax.transData)
    ec.set_array((X + Y).ravel())
    ax.add_collection(ec)
    ax.set_xlabel("X")
    ax.set_ylabel("y")
    cbar = fig.colorbar(ec)
    cbar.set_label("X+Y")


def _arcs(radii):
    """Half circles of the given radii, 36 points each."""
    theta = np.linspace(0, np.pi, 36)
    return [np.column_stack([r * np.cos(theta), r * np.sin(theta)]) for r in radii]


def _arc_axes(fig):
    """An axes showing ``[-6, 6] x [0, 6]`` at equal aspect."""
    ax = fig.subplots()
    ax.set_xlim(-6, 6)
    ax.set_ylim(0, 6)
    ax.set_aspect("equal")
    return ax


@gallery("line_collection_1", figsize=(6.4, 3.2))
def _(fig):
    colors = ["indigo", "blue", "green", "yellow", "orange", "red"]
    ax = _arc_axes(fig)
    ax.add_collection(LineCollection(_arcs(np.linspace(4, 5, num=len(colors))), colors=colors,
                                     linewidths=4))


@gallery("line_collection_2", figsize=(6.4, 3))
def _(fig):
    radii = np.linspace(4, 5.5, num=15)
    ax = _arc_axes(fig)
    line_collection = LineCollection(_arcs(radii), array=radii, cmap="rainbow")
    ax.add_collection(line_collection)
    fig.colorbar(line_collection, label="Radius")
    ax.set_title("Line Collection with mapped colors")


@gallery("patch_collection")
def _(fig):
    g = rng(SEED)
    ax = fig.subplots()
    n = 3
    x, y, radii = g.uniform(n), g.uniform(n), 0.1 * g.uniform(n)
    patches = [mpatches.Circle((x1, y1), r) for x1, y1, r in zip(x, y, radii)]
    x, y, radii = g.uniform(n), g.uniform(n), 0.1 * g.uniform(n)
    theta1, theta2 = 360.0 * g.uniform(n), 360.0 * g.uniform(n)
    patches += [mpatches.Wedge((x1, y1), r, t1, t2)
                for x1, y1, r, t1, t2 in zip(x, y, radii, theta1, theta2)]
    patches += [
        mpatches.Wedge((.3, .7), .1, 0, 360),
        mpatches.Wedge((.7, .8), .2, 0, 360, width=0.05),
        mpatches.Wedge((.8, .3), .2, 0, 45),
        mpatches.Wedge((.8, .3), .2, 45, 90, width=0.10),
    ]
    patches += [mpatches.Polygon(g.uniform(n * 2).reshape(n, 2), closed=True) for _ in range(n)]
    colors = 100 * g.uniform(len(patches))
    p = PatchCollection(patches, alpha=0.4)
    p.set_array(colors)
    ax.add_collection(p)
    fig.colorbar(p, ax=ax)


@gallery("scatter")
def _(fig):
    g = rng(SEED)
    n = 50
    x, y, colors = g.uniform(n), g.uniform(n), g.uniform(n)
    area = (30 * g.uniform(n)) ** 2
    fig.subplots().scatter(x, y, s=area, c=colors, alpha=0.5)


# --- shapes_and_collections: paths -------------------------------------------


@gallery("compound_path")
def _(fig):
    codes = [Path.MOVETO] + [Path.LINETO] * 3 + [Path.CLOSEPOLY]
    vertices = [(1, 1), (1, 2), (2, 2), (2, 1), (0, 0)]
    codes += [Path.MOVETO] + [Path.LINETO] * 2 + [Path.CLOSEPOLY]
    vertices += [(4, 4), (5, 5), (5, 4), (0, 0)]
    ax = fig.subplots()
    ax.add_patch(mpatches.PathPatch(Path(vertices, codes), facecolor="none", edgecolor="green"))
    ax.set_title("A compound path")
    ax.autoscale_view()


#: The dolphin outline, in SVG path syntax.
DOLPHIN = """
M -0.59739425,160.18173 C -0.62740401,160.18885 -0.57867129,160.11183
-0.57867129,160.11183 C -0.57867129,160.11183 -0.5438361,159.89315
-0.39514638,159.81496 C -0.24645668,159.73678 -0.18316813,159.71981
-0.18316813,159.71981 C -0.18316813,159.71981 -0.10322971,159.58124
-0.057804323,159.58725 C -0.029723983,159.58913 -0.061841603,159.60356
-0.071265813,159.62815 C -0.080250183,159.65325 -0.082918513,159.70554
-0.061841203,159.71248 C -0.040763903,159.7194 -0.0066711426,159.71091
0.077336307,159.73612 C 0.16879567,159.76377 0.28380306,159.86448
0.31516668,159.91533 C 0.3465303,159.96618 0.5011127,160.1771
0.5011127,160.1771 C 0.63668998,160.19238 0.67763022,160.31259
0.66556395,160.32668 C 0.65339985,160.34212 0.66350443,160.33642
0.64907098,160.33088 C 0.63463742,160.32533 0.61309688,160.297
0.5789627,160.29339 C 0.54348657,160.28968 0.52329693,160.27674
0.50728856,160.27737 C 0.49060916,160.27795 0.48965803,160.31565
0.46114204,160.33673 C 0.43329696,160.35786 0.4570711,160.39871
0.43309565,160.40685 C 0.4105108,160.41442 0.39416631,160.33027
0.3954995,160.2935 C 0.39683269,160.25672 0.43807996,160.21522
0.44567915,160.19734 C 0.45327833,160.17946 0.27946869,159.9424
-0.061852613,159.99845 C -0.083965233,160.0427 -0.26176109,160.06683
-0.26176109,160.06683 C -0.30127962,160.07028 -0.21167141,160.09731
-0.24649368,160.1011 C -0.32642366,160.11569 -0.34521187,160.06895
-0.40622293,160.0819 C -0.467234,160.09485 -0.56738444,160.17461
-0.59739425,160.18173
"""


def _dolphin_path():
    """The example's parse of :data:`DOLPHIN`, shifted down by 160."""
    vertices = []
    codes = []
    parts = DOLPHIN.split()
    i = 0
    code_map = {"M": Path.MOVETO, "C": Path.CURVE4, "L": Path.LINETO}
    while i < len(parts):
        path_code = code_map[parts[i]]
        npoints = Path.NUM_VERTICES_FOR_CODE[path_code]
        codes.extend([path_code] * npoints)
        vertices.extend([[*map(float, y.split(","))] for y in parts[i + 1:][:npoints]])
        i += npoints + 1
    vertices = np.array(vertices)
    vertices[:, 1] -= 160
    return vertices, codes


@gallery("dolphin", figsize=(6, 6))
def _(fig):
    g = rng(SEED)
    r = g.uniform(50)
    t = g.uniform(50) * np.pi * 2.0
    x = r * np.cos(t)
    y = r * np.sin(t)
    ax = fig.subplots()
    circle = mpatches.Circle((0, 0), 1, facecolor="none", edgecolor=(0, 0.8, 0.8), linewidth=3,
                             alpha=0.5)
    ax.add_patch(circle)
    im = ax.imshow(g.uniform(100 * 100).reshape(100, 100), origin="lower", cmap="winter",
                   interpolation="spline36", extent=(-1, 1, -1, 1))
    im.set_clip_path(circle)
    ax.plot(x, y, "o", color=(0.9, 0.9, 1.0), alpha=0.8)
    vertices, codes = _dolphin_path()
    ax.add_patch(mpatches.PathPatch(Path(vertices, codes), facecolor=(0.6, 0.6, 0.6),
                                    edgecolor=(0.0, 0.0, 0.0)))
    vertices = mtransforms.Affine2D().rotate_deg(60).transform(vertices)
    ax.add_patch(mpatches.PathPatch(Path(vertices, codes), facecolor=(0.5, 0.5, 0.5),
                                    edgecolor=(0.0, 0.0, 0.0)))


@gallery("donut")
def _(fig):
    def wise(v):
        return {+1: "CCW", -1: "CW"}[v]

    def make_circle(r):
        t = np.arange(0, np.pi * 2.0, 0.01)
        return np.column_stack((r * np.cos(t), r * np.sin(t)))

    ax = fig.subplots()
    inside_vertices = make_circle(0.5)
    outside_vertices = make_circle(1.0)
    codes = np.full(len(inside_vertices), Path.LINETO)
    codes[0] = Path.MOVETO
    for i, (inside, outside) in enumerate(((1, 1), (1, -1), (-1, 1), (-1, -1))):
        vertices = np.concatenate((outside_vertices[::outside], inside_vertices[::inside]))
        vertices += (i * 2.5, 0)
        path = Path(vertices, np.concatenate((codes, codes)))
        ax.add_patch(mpatches.PathPatch(path, facecolor="#885500", edgecolor="black"))
        ax.annotate(f"Outside {wise(outside)},\nInside {wise(inside)}", (i * 2.5, -1.5),
                    va="top", ha="center")
    ax.set(xlim=(-2, 10), ylim=(-3, 2), aspect=1, title="Mmm, donuts!")


@gallery("path_patch")
def _(fig):
    ax = fig.subplots()
    path_data = [
        (Path.MOVETO, (1.58, -2.57)),
        (Path.CURVE4, (0.35, -1.1)),
        (Path.CURVE4, (-1.75, 2.0)),
        (Path.CURVE4, (0.375, 2.0)),
        (Path.LINETO, (0.85, 1.15)),
        (Path.CURVE4, (2.2, 3.2)),
        (Path.CURVE4, (3, 0.05)),
        (Path.CURVE4, (2.0, -0.5)),
        (Path.CLOSEPOLY, (1.58, -2.57)),
    ]
    codes, verts = zip(*path_data)
    path = Path(verts, codes)
    ax.add_patch(mpatches.PathPatch(path, facecolor="r", alpha=0.5))
    x, y = zip(*path.vertices)
    ax.plot(x, y, "go-")
    ax.grid()
    ax.axis("equal")


@gallery("quad_bezier")
def _(fig):
    ax = fig.subplots()
    pp1 = mpatches.PathPatch(
        Path([(0, 0), (1, 0), (1, 1), (0, 0)],
             [Path.MOVETO, Path.CURVE3, Path.CURVE3, Path.CLOSEPOLY]),
        fc="none", transform=ax.transData)
    ax.add_patch(pp1)
    ax.plot([0.75], [0.25], "ro")
    ax.set_title("The red point should be on the path")


# --- style_sheets --------------------------------------------------------------


def _sheet(name):
    """The rcParams of the bundled style sheet ``name``."""
    return dict(matplotlib.style.library[name])


def _cycle_length():
    """``len(plt.rcParams['axes.prop_cycle'])``."""
    return len(mpl.rcParams["axes.prop_cycle"])


def _shifted_sines(ax, fmt, length=6):
    """One sine per colour of the cycle, shifted evenly over ``length``."""
    x = np.linspace(0, length)
    for s in np.linspace(0, length, _cycle_length(), endpoint=False):
        ax.plot(x, np.sin(x + s), fmt)


def _image_and_patch(ax, g):
    """``image_and_patch_example``: a random image under a circle."""
    ax.imshow(g.uniform(20 * 20).reshape(20, 20), interpolation="none")
    ax.add_patch(mpatches.Circle((5, 5), radius=5, label="patch"))


def _beta_histograms(ax, g, bins, n=10000):
    """Four translucent beta-distribution histograms."""
    for a, b in ((10, 10), (4, 12), (50, 12), (6, 55)):
        ax.hist(_beta(g, a, b, n), histtype="stepfilled", bins=bins, alpha=0.8, density=True)


@gallery("bmh", rc=_sheet("bmh"))
def _(fig):
    ax = fig.subplots()
    _beta_histograms(ax, rng(SEED), bins=25)
    ax.set_title("'bmh' style sheet")


@gallery("dark_background", rc=_sheet("dark_background"))
def _(fig):
    ax = fig.subplots()
    _shifted_sines(ax, "o-")
    ax.set_xlabel("x-axis")
    ax.set_ylabel("y-axis")
    ax.set_title("'dark_background' style sheet")


def _noisy_lines(ax, g, slopes):
    """``sin(x) + slope * x`` plus normal noise, for each slope."""
    x = np.linspace(0, 10)
    for slope in slopes:
        ax.plot(x, np.sin(x) + slope * x + g.normal(50))


@gallery("fivethirtyeight", rc=_sheet("fivethirtyeight"))
def _(fig):
    ax = fig.subplots()
    _noisy_lines(ax, rng(SEED), (1, 0.5, 2, -0.5, -2, 0))
    ax.set_title("'fivethirtyeight' style sheet")


@gallery("ggplot", rc=_sheet("ggplot"))
def _(fig):
    g = rng(SEED)
    axs = fig.subplots(ncols=2, nrows=2)
    ax1, ax2, ax3, ax4 = axs.flat
    x, y = g.normal(2 * 200).reshape(2, 200)
    ax1.plot(x, y, "o")
    _shifted_sines(ax2, "-", length=2 * np.pi)
    ax2.margins(0)
    x = np.arange(5)
    y1, y2 = g.integers(2 * 5, 1, 25).reshape(2, 5)
    width = 0.25
    ax3.bar(x, y1, width)
    ax3.bar(x + width, y2, width, color=list(mpl.rcParams["axes.prop_cycle"])[2]["color"])
    ax3.set_xticks(x + width, labels=["a", "b", "c", "d", "e"])
    for color in mpl.rcParams["axes.prop_cycle"]:
        ax4.add_patch(mpatches.Circle(g.normal(2), radius=0.3, color=color["color"]))
    ax4.axis("equal")
    ax4.margins(0)


@gallery("grayscale", rc=_sheet("grayscale"))
def _(fig):
    ax1, ax2 = fig.subplots(ncols=2)
    fig.suptitle("'grayscale' style sheet")
    _shifted_sines(ax1, "o-")
    _image_and_patch(ax2, rng(SEED))


def _sigmoids(ax, markers=False):
    """One sigmoid per colour of the cycle, optionally marked at 20 points
    in the line's colour (``colored_lines_example``)."""
    t = np.linspace(-10, 10, 100)
    nb_colors = _cycle_length()
    for t0, a in zip(np.linspace(-5, 5, nb_colors), np.linspace(1, 1.5, nb_colors)):
        y = a / (1 + np.exp(-(t - t0)))
        line, = ax.plot(t, y, "-")
        if markers:
            point_indices = np.linspace(0, len(t) - 1, 20, dtype=int)
            ax.plot(t[point_indices], y[point_indices], "o", color=line.get_color())
    ax.set_xlim(-10, 10)


@gallery("petroff10", figsize=(6.4, 9.6))
def _(fig):
    g = rng(SEED)
    fig.set_layout_engine("compressed")
    for style, sfig in zip(["petroff6", "petroff8", "petroff10"], fig.subfigures(nrows=3)):
        sfig.suptitle(f"'{style}' style sheet")
        with matplotlib.style.context(style):
            ax1, ax2 = sfig.subplots(ncols=2)
            _sigmoids(ax1, markers=True)
            _image_and_patch(ax2, g)


@gallery("plot_solarizedlight2", rc=_sheet("Solarize_Light2"))
def _(fig):
    ax = fig.subplots()
    _noisy_lines(ax, rng(SEED), range(1, 9))
    ax.set_title("8 Random Lines - Line")
    ax.set_xlabel("x label", fontsize=14)
    ax.set_ylabel("y label", fontsize=14)


def _style_reference(fig, style_label):
    """``plot_figure``: the six demonstration axes under the active sheet."""
    prng = rng(96917002)
    fig.set_layout_engine("constrained")
    axs = fig.subplots(ncols=6, nrows=1)
    background_color = mpl.colors.rgb_to_hsv(mpl.colors.to_rgb(mpl.rcParams["figure.facecolor"]))[2]
    if background_color < 0.5:
        title_color = [0.8, 0.8, 1]
    else:
        title_color = np.array([19, 6, 84]) / 256
    fig.suptitle(style_label, x=0.01, ha="left", color=title_color, fontsize=14,
                 fontfamily="DejaVu Sans", fontweight="normal")

    ax = axs[0]
    for mu, sigma, marker in [(-.5, 0.75, "o"), (0.75, 1., "s")]:
        x, y = prng.normal(2 * 100, mu, sigma).reshape(2, 100)
        ax.plot(x, y, ls="none", marker=marker)
    ax.set_xlabel("X-label")
    ax.set_title("Axes title")

    ax = axs[1]
    _image_and_patch(ax, prng)
    ax.set_xticks([])
    ax.set_yticks([])

    ax = axs[2]
    x = np.arange(5)
    ya, yb = prng.integers(2 * 5, 5, 25).reshape(2, 5)
    width = 0.25
    ax.bar(x, ya, width)
    ax.bar(x + width, yb, width, color="C2")
    ax.set_xticks(x + width, labels=["a", "b", "c", "d", "e"])

    _sigmoids(axs[3])

    ax = axs[4]
    _beta_histograms(ax, prng, bins=30)
    ax.annotate("Annotation", xy=(0.25, 4.25), xytext=(0.9, 0.9), textcoords=ax.transAxes,
                va="top", ha="right", bbox=dict(boxstyle="round", alpha=0.2),
                arrowprops=dict(arrowstyle="->",
                                connectionstyle="angle,angleA=-95,angleB=35,rad=10"))

    ax = axs[5]
    for sty_dict, j in zip(mpl.rcParams["axes.prop_cycle"](), range(15)):
        ax.add_patch(mpatches.Circle(prng.normal(2, 0, 3), radius=1.0, color=sty_dict["color"]))
    ax.grid(visible=True)
    ax.set_title("ax.grid(True)", family="monospace", fontsize="small")
    ax.set_xlim([-4, 8])
    ax.set_ylim([-5, 6])
    ax.set_aspect("equal", adjustable="box")

    axs[4].add_artist(mpatches.Rectangle((1 + 0.025, -2), 0.05, 16, clip_on=False, color="gray"))


#: Parameters pinned on top of each sheet so that goldens do not depend on
#: the host (``cases_styles.PINNED``).
PINNED = {"font.family": "DejaVu Sans", "text.hinting": "none"}

#: ``style_list``: ``default`` and ``classic`` first, then every other
#: public sheet in alphabetical order.
STYLE_LIST = ["default", "classic"] + sorted(
    style for style in matplotlib.style.available
    if style != "classic" and not style.startswith("_"))


def _register_style_reference(style_label):
    rc = {} if style_label == "default" else {**_sheet(style_label), **PINNED}
    gallery(f"style_sheets_reference_{style_label}", figsize=(14.8, 2.8), rc=rc)(
        lambda fig: _style_reference(fig, style_label))


for _style_label in STYLE_LIST:
    _register_style_reference(_style_label)


# --- axes_grid1: dividers -----------------------------------------------------


def _label_cell(ax, text):
    """``label_axes``: centred text and no ticks."""
    ax.text(.5, .5, text, transform=ax.transAxes, horizontalalignment="center",
            verticalalignment="center")
    ax.tick_params(bottom=False, labelbottom=False, left=False, labelleft=False)


def _four_located_axes(fig, horiz, vert):
    """``simple_axes_divider1``'s four axes in the cells of a divider."""
    rect = (0.1, 0.1, 0.8, 0.8)
    div = Divider(fig, rect, horiz, vert, aspect=False)
    for (nx, ny, nx1), text in [((0, 0, None), "nx=0, ny=0"), ((0, 2, None), "nx=0, ny=2"),
                                ((2, 2, None), "nx=2, ny=2"), ((2, 0, 4), "nx=2, nx1=4, ny=0")]:
        ax = fig.add_axes(rect, axes_locator=div.new_locator(nx=nx, ny=ny, nx1=nx1))
        _label_cell(ax, text)


@gallery("simple_axes_divider1_1", figsize=(6, 6))
def _(fig):
    fig.suptitle("Fixed axes sizes, fixed paddings")
    _four_located_axes(fig, [Size.Fixed(1.), Size.Fixed(.5), Size.Fixed(1.5), Size.Fixed(.5)],
                       [Size.Fixed(1.5), Size.Fixed(.5), Size.Fixed(1.)])


@gallery("simple_axes_divider1_2", figsize=(6, 6))
def _(fig):
    fig.suptitle("Scalable axes sizes, fixed paddings")
    _four_located_axes(fig, [Size.Scaled(1.5), Size.Fixed(.5), Size.Scaled(1.), Size.Scaled(.5)],
                       [Size.Scaled(1.), Size.Fixed(.5), Size.Scaled(1.5)])


@gallery("simple_axes_divider3", figsize=(5.5, 4))
def _(fig):
    rect = (0.1, 0.1, 0.8, 0.8)
    ax = [fig.add_axes(rect, label="%d" % i) for i in range(4)]
    horiz = [Size.AxesX(ax[0]), Size.Fixed(.5), Size.AxesX(ax[1])]
    vert = [Size.AxesY(ax[0]), Size.Fixed(.5), Size.AxesY(ax[2])]
    divider = Divider(fig, rect, horiz, vert, aspect=False)
    ax[0].set_axes_locator(divider.new_locator(nx=0, ny=0))
    ax[1].set_axes_locator(divider.new_locator(nx=2, ny=0))
    ax[2].set_axes_locator(divider.new_locator(nx=0, ny=2))
    ax[3].set_axes_locator(divider.new_locator(nx=2, ny=2))
    ax[0].set_xlim(0, 2)
    ax[1].set_xlim(0, 1)
    ax[0].set_ylim(0, 1)
    ax[2].set_ylim(0, 2)
    divider.set_aspect(1.)
    for ax1 in ax:
        ax1.tick_params(labelbottom=False, labelleft=False)


def _fixed_size_axes(fig, h, v):
    """``demo_fixed_size_axes``: a line plot in the middle cell."""
    divider = Divider(fig, (0, 0, 1, 1), h, v, aspect=False)
    ax = fig.add_axes(divider.get_position(), axes_locator=divider.new_locator(nx=1, ny=1))
    ax.plot([1, 2, 3])


@gallery("demo_fixed_size_axes_1", figsize=(6, 6))
def _(fig):
    _fixed_size_axes(fig, [Size.Fixed(1.0), Size.Fixed(4.5)], [Size.Fixed(0.7), Size.Fixed(5.)])


@gallery("demo_fixed_size_axes_2", figsize=(6, 6))
def _(fig):
    _fixed_size_axes(fig, [Size.Fixed(1.0), Size.Scaled(1.), Size.Fixed(.2)],
                     [Size.Fixed(0.7), Size.Scaled(1.), Size.Fixed(.5)])


@gallery("demo_axes_divider", figsize=(6, 6))
def _(fig):
    z = _bivariate_normal()

    ax = fig.add_subplot(2, 2, 1)
    im = ax.imshow(z, extent=EXTENT)
    cb = fig.colorbar(im)
    cb.ax.yaxis.set_tick_params(labelright=False)

    divider = SubplotDivider(fig, 2, 2, 2, aspect=True)
    ax = fig.add_subplot(axes_locator=divider.new_locator(nx=0, ny=0))
    ax_cb = fig.add_subplot(axes_locator=divider.new_locator(nx=2, ny=0))
    divider.set_horizontal([Size.AxesX(ax), Size.Fixed(0.05), Size.Fixed(0.2)])
    divider.set_vertical([Size.AxesY(ax)])
    im = ax.imshow(z, extent=EXTENT)
    fig.colorbar(im, cax=ax_cb)
    ax_cb.yaxis.set_tick_params(labelright=False)

    ax = fig.add_subplot(2, 2, 3)
    ax_cb = make_axes_locatable(ax).append_axes("right", size="5%", pad=0.05)
    im = ax.imshow(z, extent=EXTENT)
    fig.colorbar(im, cax=ax_cb)
    ax_cb.yaxis.tick_right()
    ax_cb.yaxis.set_tick_params(labelright=False)

    ax = fig.add_subplot(2, 2, 4)
    ax2 = make_axes_locatable(ax).append_axes("right", size="100%", pad=0.05)
    ax.imshow(z, extent=EXTENT)
    ax2.imshow(z, extent=EXTENT)
    ax2.yaxis.set_tick_params(labelleft=False)


_ARR1 = np.arange(20).reshape((4, 5))
_ARR2 = np.arange(20).reshape((5, 4))


@gallery("demo_axes_hbox_divider_1")
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.imshow(_ARR1)
    ax2.imshow(_ARR2)
    divider = HBoxDivider(
        fig, 111,
        horizontal=[Size.AxesX(ax1), Size.Fixed(0.5), Size.AxesX(ax2)],
        vertical=[Size.AxesY(ax1), Size.Scaled(1), Size.AxesY(ax2)])
    ax1.set_axes_locator(divider.new_locator(0))
    ax2.set_axes_locator(divider.new_locator(2))


@gallery("demo_axes_hbox_divider_2")
def _(fig):
    ax1, ax2 = fig.subplots(2, 1)
    ax1.imshow(_ARR1)
    ax2.imshow(_ARR2)
    divider = VBoxDivider(
        fig, 111,
        horizontal=[Size.AxesX(ax1), Size.Scaled(1), Size.AxesX(ax2)],
        vertical=[Size.AxesY(ax1), Size.Fixed(0.5), Size.AxesY(ax2)])
    ax1.set_axes_locator(divider.new_locator(0))
    ax2.set_axes_locator(divider.new_locator(2))


@gallery("demo_colorbar_with_axes_divider")
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    fig.subplots_adjust(wspace=0.5)
    im1 = ax1.imshow([[1, 2], [3, 4]])
    cax1 = make_axes_locatable(ax1).append_axes("right", size="7%", pad="2%")
    fig.colorbar(im1, cax=cax1)
    im2 = ax2.imshow([[1, 2], [3, 4]])
    cax2 = make_axes_locatable(ax2).append_axes("top", size="7%", pad="2%")
    fig.colorbar(im2, cax=cax2, orientation="horizontal")
    cax2.xaxis.set_ticks_position("top")


@gallery("scatter_hist_locatable_axes", figsize=(5.5, 5.5))
def _(fig):
    g = rng(SEED)
    x = g.normal(1000)
    y = g.normal(1000)
    ax = fig.subplots()
    ax.scatter(x, y)
    ax.set_aspect(1.)
    divider = make_axes_locatable(ax)
    ax_histx = divider.append_axes("top", 1.2, pad=0.1, sharex=ax)
    ax_histy = divider.append_axes("right", 1.2, pad=0.1, sharey=ax)
    ax_histx.xaxis.set_tick_params(labelbottom=False)
    ax_histy.yaxis.set_tick_params(labelleft=False)
    binwidth = 0.25
    xymax = max(np.max(np.abs(x)), np.max(np.abs(y)))
    lim = (int(xymax / binwidth) + 1) * binwidth
    bins = np.arange(-lim, lim + binwidth, binwidth)
    ax_histx.hist(x, bins=bins)
    ax_histy.hist(y, bins=bins, orientation="horizontal")
    ax_histx.set_yticks([0, 50, 100])
    ax_histy.set_xticks([0, 50, 100])


@gallery("make_room_for_ylabel_using_axesgrid_1")
def _(fig):
    ax = fig.add_axes((0, 0, 1, 1))
    ax.set_yticks([0.5], labels=["very long label"])
    make_axes_area_auto_adjustable(ax)


# --- axes_grid1: image grids ---------------------------------------------------


@gallery("simple_axesgrid", figsize=(4, 4))
def _(fig):
    im1 = np.arange(100).reshape((10, 10))
    im2 = im1.T
    im3 = np.flipud(im1)
    im4 = np.fliplr(im2)
    grid = ImageGrid(fig, 111, nrows_ncols=(2, 2), axes_pad=0.1)
    for ax, im in zip(grid, [im1, im2, im3, im4]):
        ax.imshow(im)


@gallery("simple_axesgrid2", figsize=(5.5, 3.5))
def _(fig):
    grid = ImageGrid(fig, 111, nrows_ncols=(1, 3), axes_pad=0.1, label_mode="L")
    z = _bivariate_normal()
    vmin, vmax = z.min(), z.max()
    for ax, im in zip(grid, [z, z[:, :10], z[:, 10:]]):
        ax.imshow(im, origin="lower", vmin=vmin, vmax=vmax)


@gallery("demo_imagegrid_aspect")
def _(fig):
    grid1 = ImageGrid(fig, 121, (2, 2), axes_pad=0.1, aspect=True, share_all=True)
    for i in [0, 1]:
        grid1[i].set_aspect(2)
    grid2 = ImageGrid(fig, 122, (2, 2), axes_pad=0.1, aspect=True, share_all=True)
    for i in [1, 3]:
        grid2[i].set_aspect(2)


def _llc_ticks(grid):
    """Ticks at -2, 0 and 2 on the lower left axes (shared by all)."""
    grid.axes_llc.set(xticks=[-2, 0, 2], yticks=[-2, 0, 2])


@gallery("demo_axes_grid", figsize=(10.5, 2.5))
def _(fig):
    z = _bivariate_normal()

    grid = ImageGrid(fig, 141, nrows_ncols=(2, 2), axes_pad=0.05, label_mode="1")
    for ax in grid:
        ax.imshow(z, extent=EXTENT)
    _llc_ticks(grid)

    grid = ImageGrid(fig, 142, nrows_ncols=(2, 2), axes_pad=0.0, label_mode="L", share_all=True,
                     cbar_location="top", cbar_mode="single")
    for ax in grid:
        im = ax.imshow(z, extent=EXTENT)
    grid.cbar_axes[0].colorbar(im)
    for cax in grid.cbar_axes:
        cax.tick_params(labeltop=False)
    _llc_ticks(grid)

    grid = ImageGrid(fig, 143, nrows_ncols=(2, 2), axes_pad=0.1, label_mode="1", share_all=True,
                     cbar_location="top", cbar_mode="each", cbar_size="7%", cbar_pad="2%")
    for ax, cax in zip(grid, grid.cbar_axes):
        im = ax.imshow(z, extent=EXTENT)
        cax.colorbar(im)
        cax.tick_params(labeltop=False)
    _llc_ticks(grid)

    grid = ImageGrid(fig, 144, nrows_ncols=(2, 2), axes_pad=(0.45, 0.15), label_mode="1",
                     share_all=True, cbar_location="right", cbar_mode="each", cbar_size="7%",
                     cbar_pad="2%")
    limits = ((0, 1), (-2, 2), (-1.7, 1.4), (-1.5, 1))
    for ax, cax, vlim in zip(grid, grid.cbar_axes, limits):
        im = ax.imshow(z, extent=EXTENT, vmin=vlim[0], vmax=vlim[1])
        cb = cax.colorbar(im)
        cb.set_ticks((vlim[0], vlim[1]))
    _llc_ticks(grid)


def _inner_title(ax, title, loc):
    """``add_inner_title``: frameless text with a white halo in a corner."""
    prop = dict(path_effects=[withStroke(foreground="w", linewidth=3)],
                size=mpl.rcParams["legend.fontsize"])
    ax.add_artist(AnchoredText(title, loc=loc, prop=prop, pad=0., borderpad=0.5,
                               frameon=False))


@gallery("demo_axes_grid2", figsize=(6, 6))
def _(fig):
    z = _bivariate_normal()
    zs = [z[i::3, :] for i in range(3)]
    extent = EXTENT[0], EXTENT[1] / 3., EXTENT[2], EXTENT[3]

    grid = ImageGrid(fig, 211, nrows_ncols=(1, 3), axes_pad=0.05, label_mode="1",
                     share_all=True, cbar_location="top", cbar_mode="each", cbar_size="7%",
                     cbar_pad="1%")
    grid[0].set(xticks=[-2, 0], yticks=[-2, 0, 2])
    for i, (ax, zi) in enumerate(zip(grid, zs)):
        im = ax.imshow(zi, origin="lower", extent=extent)
        cb = ax.cax.colorbar(im)
        if i in [1, 2]:
            cb.set_ticks([-1, 0, 1])
    for ax, im_title in zip(grid, ["Image 1", "Image 2", "Image 3"]):
        _inner_title(ax, im_title, loc="lower left")

    grid2 = ImageGrid(fig, 212, nrows_ncols=(1, 3), axes_pad=0.05, label_mode="1",
                      share_all=True, cbar_location="right", cbar_mode="single", cbar_size="10%",
                      cbar_pad=0.05)
    grid2[0].set(xlabel="X", ylabel="Y", xticks=[-2, 0], yticks=[-2, 0, 2])
    clim = (np.min(zs), np.max(zs))
    for ax, zi in zip(grid2, zs):
        im = ax.imshow(zi, clim=clim, origin="lower", extent=extent)
    ax.cax.colorbar(im)
    for ax, im_title in zip(grid2, ["(a)", "(b)", "(c)"]):
        _inner_title(ax, im_title, loc="upper left")


def _edge_colorbars(fig, position, cmaps, label, **options):
    """``demo_bottom_cbar`` / ``demo_right_cbar``: a 2x2 grid with one
    colorbar per column or row."""
    grid = AxesGrid(fig, position, nrows_ncols=(2, 2), axes_pad=0.10, share_all=True,
                    label_mode="1", cbar_mode="edge", **options)
    z = _bivariate_normal()
    for i in range(4):
        im = grid[i].imshow(z, extent=EXTENT, cmap=cmaps[i // 2])
        if i % 2:
            grid.cbar_axes[i // 2].colorbar(im)
    for cax in grid.cbar_axes:
        cax.axis[cax.orientation].set_label(label)
    grid.axes_llc.set_xticks([-2, 0, 2])
    grid.axes_llc.set_yticks([-2, 0, 2])


@gallery("demo_edge_colorbar")
def _(fig):
    _edge_colorbars(fig, 121, ["autumn", "summer"], "Bar", cbar_location="bottom", cbar_pad=0.25,
                    cbar_size="15%", direction="column")
    _edge_colorbars(fig, 122, ["spring", "winter"], "Foo", cbar_location="right", cbar_size="7%",
                    cbar_pad="2%")


def _rgb_channels():
    """``get_rgb``: three overlapping 13x13 crops of the clipped, normalised
    sample data."""
    z = _bivariate_normal()
    z[z < 0] = 0.
    z = z / z.max()
    return z[:13, :13], z[2:, 2:], z[:13, 2:]


@gallery("demo_axes_rgb_1")
def _(fig):
    ax = RGBAxes(fig, [0.1, 0.1, 0.8, 0.8], pad=0.0)
    ax.imshow_rgb(*_rgb_channels())


@gallery("demo_axes_rgb_2")
def _(fig):
    ax = fig.subplots()
    ax_r, ax_g, ax_b = make_rgb_axes(ax, pad=0.02)
    r, g, b = _rgb_channels()
    channels = []
    for i, channel in enumerate((r, g, b)):
        cube = np.zeros((*channel.shape, 3))
        cube[:, :, i] = channel
        channels.append(cube)
    ax.imshow(channels[0] + channels[1] + channels[2])
    for axc, cube in zip((ax_r, ax_g, ax_b), channels):
        axc.imshow(cube)
    for axc in fig.axes:
        axc.tick_params(direction="in", color="w")
        axc.spines[:].set_color("w")


# --- axes_grid1: insets -----------------------------------------------------------


@gallery("demo_colorbar_with_inset_locator", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    im1 = ax1.imshow([[1, 2], [2, 3]])
    axins1 = inset_locator.inset_axes(ax1, width="50%", height="5%", loc="upper right")
    axins1.xaxis.set_ticks_position("bottom")
    fig.colorbar(im1, cax=axins1, orientation="horizontal", ticks=[1, 2, 3])
    im = ax2.imshow([[1, 2], [2, 3]])
    axins = inset_locator.inset_axes(ax2, width="5%", height="50%", loc="lower left",
                                     bbox_to_anchor=(1.05, 0., 1, 1),
                                     bbox_transform=ax2.transAxes, borderpad=0)
    fig.colorbar(im, cax=axins, ticks=[1, 2, 3])


def _hide_tick_labels(*axes):
    """No tick labels on the left or at the bottom."""
    for axi in axes:
        axi.tick_params(labelleft=False, labelbottom=False)


@gallery("inset_locator_demo_1", figsize=(5.5, 2.8))
def _(fig):
    ax, ax2 = fig.subplots(1, 2)
    _hide_tick_labels(
        inset_axes(ax, width=1.3, height=0.9),
        inset_axes(ax, width="30%", height="40%", loc="lower left"),
        inset_axes(ax2, width="30%", height=1., loc="upper left"),
        inset_axes(ax2, width="20%", height="20%", loc="lower right", borderpad=1))


@gallery("inset_locator_demo_2", figsize=(5.5, 2.8))
def _(fig):
    ax = fig.add_subplot(121)
    inset_axes(ax, width="50%", height="75%", bbox_to_anchor=(.2, .4, .6, .5),
               bbox_transform=ax.transAxes, loc="lower left")
    ax.add_patch(mpatches.Rectangle((.2, .4), .6, .5, ls="--", ec="c", fc="none",
                                    transform=ax.transAxes))
    ax.set(xlim=(0, 10), ylim=(0, 10))
    ax2 = fig.add_subplot(222)
    axins2 = inset_axes(ax2, width="30%", height="50%")
    ax3 = fig.add_subplot(224)
    axins3 = inset_axes(ax3, width="100%", height="100%", bbox_to_anchor=(.7, .5, .3, .5),
                        bbox_transform=ax3.transAxes)
    ax2.add_patch(mpatches.Rectangle((0, 0), 1, 1, ls="--", lw=2, ec="c", fc="none"))
    ax3.add_patch(mpatches.Rectangle((.7, .5), .3, .5, ls="--", lw=2, ec="c", fc="none"))
    _hide_tick_labels(axins2, axins3, ax2, ax3)


@gallery("inset_locator_demo_3", figsize=(5.5, 2.8))
def _(fig):
    ax = fig.add_subplot(131)
    axins = inset_axes(ax, width="100%", height="100%", bbox_to_anchor=(1.05, .6, .5, .4),
                       bbox_transform=ax.transAxes, loc="upper left", borderpad=0)
    axins.tick_params(left=False, right=True, labelleft=False, labelright=True)
    inset_axes(ax, width=0.5, height=0.4, bbox_to_anchor=(0.33, 0.25),
               bbox_transform=ax.transAxes, loc="lower left", borderpad=0)
    ax2 = fig.add_subplot(133)
    ax2.set_xscale("log")
    ax2.set(xlim=(1e-6, 1e6), ylim=(-2, 6))
    inset_axes(ax2, width="100%", height="100%", bbox_to_anchor=(1e-2, 2, 1e3, 3),
               bbox_transform=ax2.transData, loc="upper left", borderpad=0)
    transform = mtransforms.blended_transform_factory(fig.transFigure, ax2.transAxes)
    inset_axes(ax2, width="16%", height="34%", bbox_to_anchor=(0, 0, 1, 1),
               bbox_transform=transform, loc="lower center", borderpad=0)


def _size_bar(ax, size, label):
    """A frameless scale bar ``size`` data units long under the axes' middle."""
    ax.add_artist(AnchoredSizeBar(ax.transData, size, label, loc="lower center", pad=0.1,
                                  borderpad=0.5, sep=5, frameon=False))


def _seven_bins(axins):
    """At most seven ticks per axis, without labels."""
    axins.yaxis.get_major_locator().set_params(nbins=7)
    axins.xaxis.get_major_locator().set_params(nbins=7)
    axins.tick_params(labelleft=False, labelbottom=False)


@gallery("inset_locator_demo2", figsize=(6, 3))
def _(fig):
    ax, ax2 = fig.subplots(ncols=2)
    ax.set_aspect(1)
    axins = zoomed_inset_axes(ax, zoom=0.5, loc="upper right")
    _seven_bins(axins)
    _size_bar(ax, 0.5, "0.5")
    _size_bar(axins, 0.5, "0.5")
    z = _bivariate_normal()
    z2 = np.zeros((150, 150))
    ny, nx = z.shape
    z2[30:30 + ny, 30:30 + nx] = z
    ax2.imshow(z2, extent=EXTENT, origin="lower")
    axins2 = zoomed_inset_axes(ax2, zoom=6, loc="upper right")
    axins2.imshow(z2, extent=EXTENT, origin="lower")
    axins2.set_xlim(-1.5, -0.9)
    axins2.set_ylim(-2.5, -1.9)
    _seven_bins(axins2)
    mark_inset(ax2, axins2, loc1=2, loc2=4, fc="none", ec="0.5")


# --- axes_grid1: anchored artists ----------------------------------------------------


@gallery("demo_anchored_direction_arrows")
def _(fig):
    ax = fig.subplots()
    ax.imshow(rng(SEED).uniform(10 * 10).reshape(10, 10))
    ax.add_artist(AnchoredDirectionArrows(ax.transAxes, "X", "Y"))
    ax.add_artist(AnchoredDirectionArrows(
        ax.transAxes, "111", r"11$\overline{2}$", loc="upper right",
        arrow_props={"ec": "w", "fc": "none", "alpha": 1, "lw": 2}))
    ax.add_artist(AnchoredDirectionArrows(
        ax.transAxes, "111", r"11$\overline{2}$", loc="upper right",
        arrow_props={"ec": "none", "fc": "k"}, text_props={"ec": "w", "fc": "k", "lw": 0.4}))
    ax.add_artist(AnchoredDirectionArrows(
        ax.transAxes, "30", "120", loc="center", color="w", angle=30,
        fontproperties=fm.FontProperties(family="serif")))
    ax.add_artist(AnchoredDirectionArrows(
        ax.transAxes, "A", "B", loc="lower center", length=-0.15, sep_x=0.03, sep_y=0.03,
        color="r"))
    ax.add_artist(AnchoredDirectionArrows(
        ax.transAxes, "A", " B", loc="lower left", aspect_ratio=-1, sep_x=0.01, sep_y=-0.02,
        color="orange"))
    ax.add_artist(AnchoredDirectionArrows(
        ax.transAxes, " A", "B", loc="lower right", length=-0.15, aspect_ratio=-1, sep_y=-0.1,
        sep_x=0.04, color="cyan"))


@gallery("simple_anchored_artists")
def _(fig):
    ax = fig.subplots()
    ax.set_aspect(1.)
    at = AnchoredText("Figure 1a", loc="upper left", prop=dict(size=8), frameon=True)
    at.patch.set_boxstyle("round,pad=0.,rounding_size=0.2")
    ax.add_artist(at)
    at2 = AnchoredText("Figure 1(b)", loc="lower left", prop=dict(size=8), frameon=True,
                       bbox_to_anchor=(0., 1.), bbox_transform=ax.transAxes)
    at2.patch.set_boxstyle("round,pad=0.,rounding_size=0.2")
    ax.add_artist(at2)
    ada = AnchoredDrawingArea(20, 20, 0, 0, loc="upper right", pad=0., frameon=False)
    ada.da.add_artist(mpatches.Circle((10, 10), 10))
    ax.add_artist(ada)
    _size_bar(ax, 0.1, r"1$^{\prime}$")


# --- axes_grid1: host and parasite axes ----------------------------------------------


@gallery("parasite_simple")
def _(fig):
    host = host_subplot(111, figure=fig)
    par = host.twinx()
    host.set_xlabel("Distance")
    host.set_ylabel("Density")
    par.set_ylabel("Temperature")
    p1, = host.plot([0, 1, 2], [0, 1, 2], label="Density")
    p2, = par.plot([0, 1, 2], [0, 3, 2], label="Temperature")
    host.legend(labelcolor="linecolor")
    host.yaxis.label.set_color(p1.get_color())
    par.yaxis.label.set_color(p2.get_color())


@gallery("parasite_simple2")
def _(fig):
    obs = [["01_S1", 3.88, 0.14, 1970, 63],
           ["01_S4", 5.6, 0.82, 1622, 150],
           ["02_S1", 2.4, 0.54, 1570, 40],
           ["03_S1", 4.1, 0.62, 2380, 170]]
    ax_kms = fig.add_subplot(axes_class=HostAxes, aspect=1)
    pm_to_kms = 1. / 206265. * 2300 * 3.085e18 / 3.15e7 / 1.e5
    ax_pm = ax_kms.twin(mtransforms.Affine2D().scale(pm_to_kms, 1.))
    for n, ds, dse, w, we in obs:
        time = ((2007 + (10. + 4 / 30.) / 12) - 1988.5)
        v = ds / time * pm_to_kms
        ve = dse / time * pm_to_kms
        ax_kms.errorbar([v], [w], xerr=[ve], yerr=[we], color="k")
    ax_kms.axis["bottom"].set_label("Linear velocity at 2.3 kpc [km/s]")
    ax_kms.axis["left"].set_label("FWHM [km/s]")
    ax_pm.axis["top"].set_label(r"Proper Motion [$''$/yr]")
    ax_pm.axis["top"].label.set_visible(True)
    ax_pm.axis["right"].major_ticklabels.set_visible(False)
    ax_kms.set_xlim(950, 3700)
    ax_kms.set_ylim(950, 3100)


@gallery("simple_axisline4")
def _(fig):
    ax = host_subplot(111, figure=fig)
    xx = np.arange(0, 2 * np.pi, 0.01)
    ax.plot(xx, np.sin(xx))
    ax2 = ax.twin()
    ax2.set_xticks([0., .5 * np.pi, np.pi, 1.5 * np.pi, 2 * np.pi],
                   labels=["$0$", r"$\frac{1}{2}\pi$", r"$\pi$", r"$\frac{3}{2}\pi$", r"$2\pi$"])
    ax2.axis["right"].major_ticklabels.set_visible(False)
    ax2.axis["top"].major_ticklabels.set_visible(True)
