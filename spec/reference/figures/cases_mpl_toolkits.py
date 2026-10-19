"""matplotlib's own image tests of ``mpl_toolkits``: every
``@image_comparison`` test of ``axes_grid1/tests/test_axes_grid1.py`` and of
``axisartist/tests/`` (``test_axislines.py``, ``test_axis_artist.py``,
``test_grid_helper_curvelinear.py`` and ``test_floating_axes.py``), and the
``fig_test`` side of the ``@check_figures_equal`` test.

Each case keeps the test's style. The test suite's default (also used by
``check_figures_equal``) is ``classic`` plus ``_classic_test_patch``: such
figures are 8x6 in and saved at classic's 100 dpi. Tests asking for
``mpl20``/``default`` run in the reference configuration. ``bbox_inches``
passed to ``savefig`` becomes ``savefig.bbox``, ``remove_text=True`` becomes
``remove_ticks_and_titles`` at the end of the build, and the tests' own
rcParams changes become the case's ``rc``. Tests drawing several figures
contribute one case per figure (``_1``, ``_2``). Where a test goes through
pyplot's current figure or axes, the case names them explicitly.

Deviations from the tests:

* ``np.random`` draws come from ``spec_support.rng``, seeded with the test's
  seed.
* The sample data ``axes_grid/bivariate_normal.npy`` is computed from the
  formula it was made with (:func:`bivariate_normal`).
* ``test_custom_transform`` and ``test_polar_box`` define a transform class
  and a ``FuncFormatter`` inline; the cases define them at module level.
* ``plt.figure(layout="tight")`` becomes ``fig.set_layout_engine("tight")``.
* ``plt.xticks(visible=False)`` hides the tick labels the current (inset)
  axes has drawn so far; the cases hide all of them with ``tick_params``,
  which looks the same.

Not ported: ``test_labelbase`` and ``test_ticklabels`` of
``test_axis_artist.py``, which place axisartist's internal text artists
through their private attributes (``_ref_angle``, ``_offset_radius``,
``_locs_angles_labels``); plt-rs keeps those parts inside its axis artists.
"""

from itertools import product

import matplotlib.style
import numpy as np
from matplotlib.patches import Circle, Ellipse
from matplotlib.path import Path
from matplotlib.projections import PolarAxes
from matplotlib.testing.decorators import remove_ticks_and_titles
from matplotlib.ticker import FuncFormatter
from matplotlib.transforms import Affine2D, Bbox, IdentityTransform, Transform, TransformedBbox
from mpl_toolkits.axes_grid1 import ImageGrid, host_subplot
from mpl_toolkits.axes_grid1.anchored_artists import (AnchoredAuxTransformBox,
                                                      AnchoredDirectionArrows,
                                                      AnchoredDrawingArea, AnchoredSizeBar)
from mpl_toolkits.axes_grid1.axes_rgb import RGBAxes
from mpl_toolkits.axes_grid1.inset_locator import (BboxConnectorPatch, inset_axes, mark_inset,
                                                   zoomed_inset_axes)
from mpl_toolkits.axes_grid1.parasite_axes import host_axes_class_factory
from mpl_toolkits.axisartist import (AxisArtistHelperRectlinear, SubplotHost, angle_helper,
                                     floating_axes)
from mpl_toolkits.axisartist import grid_helper_curvelinear
from mpl_toolkits.axisartist.axis_artist import AxisArtist, Ticks
from mpl_toolkits.axisartist.axislines import Axes as AxislinesAxes
from mpl_toolkits.axisartist.axislines import AxesZero, Subplot, SubplotZero
from mpl_toolkits.axisartist.grid_finder import FixedLocator

from spec_support import case, rng

GROUP = "mpl_toolkits"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}

#: ``savefig(bbox_inches="tight")``.
TIGHT = {"savefig.bbox": "tight"}

#: What the axisartist tests set "until the test image is regenerated".
TICKS_IN = {"xtick.direction": "in", "ytick.direction": "in"}


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


def bivariate_normal():
    """``cbook.get_sample_data("axes_grid/bivariate_normal.npy")``: ten times
    the difference of two bivariate normal densities on a 15x15 grid."""
    x = np.arange(-3.0, 4.001, 0.5)
    y = np.arange(-4.0, 3.001, 0.5)
    xx, yy = np.meshgrid(x, y)

    def density(sigmax, sigmay, mux, muy):
        return (np.exp(-((xx - mux) ** 2 / sigmax ** 2 + (yy - muy) ** 2 / sigmay ** 2) / 2)
                / (2 * np.pi * sigmax * sigmay))

    return (density(1.0, 1.0, 0.0, 0.0) - density(1.5, 0.5, 1.0, 1.0)) * 10


def polar_degrees():
    """``Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform()``."""
    return Affine2D().scale(np.pi / 180., 1.) + PolarAxes.PolarTransform()


# --- axes_grid1/tests/test_axes_grid1.py -----------------------------------


@mpl20("twin_axes_empty_and_removed", figsize=(8, 6),
       rc={"font.size": 8, "xtick.labelsize": 8, "ytick.labelsize": 8})
def _(fig):
    generators = ["twinx", "twiny", "twin"]
    modifiers = ["", "host invisible", "twin removed", "twin invisible",
                 "twin removed\nhost invisible"]
    h = host_subplot(len(modifiers) + 1, len(generators), 2, figure=fig)
    h.text(0.5, 0.5, "host_subplot", horizontalalignment="center", verticalalignment="center")
    for i, (mod, gen) in enumerate(product(modifiers, generators), len(generators) + 1):
        h = host_subplot(len(modifiers) + 1, len(generators), i, figure=fig)
        t = getattr(h, gen)()
        if "twin invisible" in mod:
            t.axis[:].set_visible(False)
        if "twin removed" in mod:
            t.remove()
        if "host invisible" in mod:
            h.axis[:].set_visible(False)
        h.text(0.5, 0.5, gen + ("\n" + mod if mod else ""),
               horizontalalignment="center", verticalalignment="center")
    fig.subplots_adjust(wspace=0.5, hspace=1)


def _zoomed_demo(fig, make_inset):
    """``test_inset_locator`` and ``test_inset_axes``: the sample image, an
    inset showing a part of it, marked in the parent, and a size bar."""
    ax = fig.subplots()
    z2 = np.zeros((150, 150))
    z = bivariate_normal()
    ny, nx = z.shape
    z2[30:30 + ny, 30:30 + nx] = z
    extent = (-3, 4, -4, 3)
    ax.imshow(z2, extent=extent, interpolation="nearest", origin="lower")
    axins = make_inset(ax)
    axins.imshow(z2, extent=extent, interpolation="nearest", origin="lower")
    axins.yaxis.get_major_locator().set_params(nbins=7)
    axins.xaxis.get_major_locator().set_params(nbins=7)
    axins.set_xlim(-1.5, -0.9)
    axins.set_ylim(-2.5, -1.9)
    axins.tick_params(labelbottom=False, labelleft=False)
    mark_inset(ax, axins, loc1=2, loc2=4, fc="none", ec="0.5")
    ax.add_artist(AnchoredSizeBar(ax.transData, 0.5, "0.5", loc="lower center", pad=0.1,
                                  borderpad=0.5, sep=5, frameon=False))


@mpl20("inset_locator", figsize=(5, 4))
@without_text
def _(fig):
    _zoomed_demo(fig, lambda ax: zoomed_inset_axes(ax, zoom=6, loc="upper right"))


@mpl20("inset_axes", figsize=(5, 4))
@without_text
def _(fig):
    _zoomed_demo(fig, lambda ax: inset_axes(ax, width=1., height=1., bbox_to_anchor=(1, 1),
                                            bbox_transform=ax.transAxes))


@mpl20("fill_facecolor", figsize=(5, 5))
@without_text
def _(fig):
    ax = fig.subplots(1, 5)
    for i in range(1, 4):
        ax[i].yaxis.set_visible(False)
    ax[4].yaxis.tick_right()
    bbox = Bbox.from_extents(0, 0.4, 1, 0.6)
    # The styles of the connector patch and of the marks, each axes.
    styles = [{"fc": "b"}, {"facecolor": "y"}, {"color": "g"}, {"fc": "g", "fill": False}]
    for i, style in enumerate(styles):
        p = BboxConnectorPatch(TransformedBbox(bbox, ax[i].transData),
                               TransformedBbox(bbox, ax[i + 1].transData),
                               loc1a=1, loc2a=2, loc1b=4, loc2b=3, ec="r", **style)
        p.set_clip_on(False)
        ax[i].add_patch(p)
        axins = zoomed_inset_axes(ax[i], 1, loc="upper right")
        axins.set_xlim(0, 0.2)
        axins.set_ylim(0, 0.2)
        axins.xaxis.set_ticks([])
        axins.yaxis.set_ticks([])
        mark_inset(ax[i], axins, loc1=2, loc2=4, ec="0.5", **style)


def _zooming(fig, limits, inset_limits):
    ax = fig.subplots()
    ax.plot([1, 2, 3], [1, 2, 3])
    ax.axis(limits)
    inset_ax = zoomed_inset_axes(ax, zoom=2.5, loc="lower right")
    inset_ax.axis(inset_limits)


@mpl20("zooming_with_inverted_axes_1")
def _(fig):
    _zooming(fig, [1, 3, 1, 3], [1.1, 1.4, 1.1, 1.4])


@mpl20("zooming_with_inverted_axes_2")
def _(fig):
    _zooming(fig, [3, 1, 3, 1], [1.4, 1.1, 1.4, 1.1])


@mpl20("anchored_direction_arrows")
def _(fig):
    ax = fig.subplots()
    ax.imshow(np.zeros((10, 10)), interpolation="nearest")
    ax.add_artist(AnchoredDirectionArrows(ax.transAxes, "X", "Y"))


@mpl20("anchored_direction_arrows_many_args")
def _(fig):
    ax = fig.subplots()
    ax.imshow(np.ones((10, 10)))
    ax.add_artist(AnchoredDirectionArrows(
        ax.transAxes, "A", "B", loc="upper right", color="red", aspect_ratio=-0.5, pad=0.6,
        borderpad=2, frameon=True, alpha=0.7, sep_x=-0.06, sep_y=-0.08, back_length=0.1,
        head_width=9, head_length=10, tail_width=5))


@mpl20("image_grid_each_left_label_mode_all", figsize=(3, 3), rc=TIGHT)
def _(fig):
    imdata = np.arange(100).reshape((10, 10))
    grid = ImageGrid(fig, (1, 1, 1), nrows_ncols=(3, 2), axes_pad=(0.5, 0.3), cbar_mode="each",
                     cbar_location="left", cbar_size="15%", label_mode="all")
    for ax, cax in zip(grid, grid.cbar_axes):
        cax.colorbar(ax.imshow(imdata, interpolation="none"))


@mpl20("image_grid_single_bottom", figsize=(2.5, 1.5), rc=TIGHT)
def _(fig):
    imdata = np.arange(100).reshape((10, 10))
    grid = ImageGrid(fig, (0, 0, 1, 1), nrows_ncols=(1, 3), axes_pad=(0.2, 0.15),
                     cbar_mode="single", cbar_pad=0.3, cbar_location="bottom", cbar_size="10%",
                     label_mode="1")
    for i in range(3):
        im = grid[i].imshow(imdata, interpolation="none")
    grid.cbar_axes[0].colorbar(im)


@mpl20("image_grid", figsize=(4, 4), rc=TIGHT)
@without_text
def _(fig):
    im = np.arange(100).reshape((10, 10))
    grid = ImageGrid(fig, 111, nrows_ncols=(2, 2), axes_pad=0.1)
    for i in range(4):
        grid[i].imshow(im, interpolation="nearest")


@mpl20("anchored_artists", figsize=(3, 3))
@without_text
def _(fig):
    ax = fig.subplots()
    ada = AnchoredDrawingArea(40, 20, 0, 0, loc="upper right", pad=0., frameon=False)
    ada.drawing_area.add_artist(Circle((10, 10), 10))
    ada.drawing_area.add_artist(Circle((30, 10), 5, fc="r"))
    ax.add_artist(ada)
    box = AnchoredAuxTransformBox(ax.transData, loc="upper left")
    box.drawing_area.add_artist(Ellipse((0, 0), width=0.1, height=0.4, angle=30, color="cyan"))
    ax.add_artist(box)
    ax.add_artist(AnchoredSizeBar(ax.transData, 0.2, r"0.2 units", loc="lower right", pad=0.3,
                                  borderpad=0.4, sep=4, fill_bar=True, frameon=False,
                                  label_top=True, prop={"size": 20}, size_vertical=0.05,
                                  color="green"))


@classic("mark_inset_unstales_viewlim")
def _(fig):
    inset, full = fig.subplots(1, 2)
    full.plot([0, 5], [0, 5])
    inset.set(xlim=(1, 2), ylim=(1, 2))
    mark_inset(full, inset, 1, 4)


@classic("rgb_axes")
@without_text
def _(fig):
    ax = RGBAxes(fig, (0.1, 0.1, 0.8, 0.8), pad=0.1)
    r = rng(19680801)
    red, green, blue = (r.uniform(25).reshape(5, 5) for _ in range(3))
    ax.imshow_rgb(red, green, blue, interpolation="none")


@mpl20("imagegrid_cbar_mode_edge", figsize=(18, 9))
@without_text
def _(fig):
    arr = np.arange(16).reshape((4, 4))
    positions = (241, 242, 243, 244, 245, 246, 247, 248)
    directions = ["row"] * 4 + ["column"] * 4
    cbar_locations = ["left", "right", "top", "bottom"] * 2
    for position, direction, location in zip(positions, directions, cbar_locations):
        grid = ImageGrid(fig, position, nrows_ncols=(2, 2), direction=direction,
                         cbar_location=location, cbar_size="20%", cbar_mode="edge")
        ax1, ax2, ax3, ax4 = grid
        ax1.imshow(arr, cmap="nipy_spectral")
        ax2.imshow(arr.T, cmap="hot")
        ax3.imshow(np.hypot(arr, arr.T), cmap="jet")
        ax4.imshow(np.arctan2(arr, arr.T), cmap="hsv")
        for ax in grid:
            ax.cax.cla()
            ax.cax.colorbar(ax.images[0])


@mpl20("anchored_locator_base_call", figsize=(3, 3))
def _(fig):
    fig1, _ = fig.subfigures(nrows=2, ncols=1)
    ax = fig1.subplots()
    ax.set(aspect=1, xlim=(-15, 15), ylim=(-20, 5))
    ax.set(xticks=[], yticks=[])
    axins = zoomed_inset_axes(ax, zoom=2, loc="upper left")
    axins.set(xticks=[], yticks=[])
    axins.imshow(bivariate_normal(), extent=(-3, 4, -4, 3), origin="lower")


# --- axisartist/tests/test_axislines.py ------------------------------------


@mpl20("SubplotZero")
def _(fig):
    ax = SubplotZero(fig, 1, 1, 1)
    fig.add_subplot(ax)
    ax.axis["xzero"].set_visible(True)
    ax.axis["xzero"].label.set_text("Axis Zero")
    for n in ["top", "right"]:
        ax.axis[n].set_visible(False)
    xx = np.arange(0, 2 * np.pi, 0.01)
    ax.plot(xx, np.sin(xx))
    ax.set_ylabel("Test")


@mpl20("Subplot")
def _(fig):
    ax = Subplot(fig, 1, 1, 1)
    fig.add_subplot(ax)
    xx = np.arange(0, 2 * np.pi, 0.01)
    ax.plot(xx, np.sin(xx))
    ax.set_ylabel("Test")
    ax.axis["left"].major_ticks.set_tick_out(False)
    ax.axis["right"].major_ticks.set_tick_out(False)
    ax.axis["bottom"].set_label("Tk0")


@mpl20("ParasiteAxesAuxTrans", rc=TICKS_IN)
@without_text
def _(fig):
    data = np.ones((6, 6))
    data[2, 2] = 2
    data[0, :] = 0
    data[-2, :] = 0
    data[:, 0] = 0
    data[:, -2] = 0
    xx, yy = np.meshgrid(np.arange(6), np.arange(6))
    for i, name in enumerate(["pcolor", "pcolormesh", "contourf"]):
        ax1 = SubplotHost(fig, 1, 3, i + 1)
        fig.add_subplot(ax1)
        ax2 = ax1.get_aux_axes(IdentityTransform(), viewlim_mode=None)
        if name.startswith("pcolor"):
            getattr(ax2, name)(xx, yy, data[:-1, :-1])
        else:
            getattr(ax2, name)(xx, yy, data)
        ax1.set_xlim(0, 5)
        ax1.set_ylim(0, 5)
    ax2.contour(xx, yy, data, colors="k")


def _axisline_style(fig, yzero_style, **xzero_options):
    """The ``test_axisline_style*`` tests: arrows on the zero axes only."""
    ax = fig.add_subplot(axes_class=AxesZero)
    ax.axis["xzero"].set_axisline_style("-|>", **xzero_options)
    ax.axis["xzero"].set_visible(True)
    ax.axis["yzero"].set_axisline_style(yzero_style)
    ax.axis["yzero"].set_visible(True)
    for direction in ("left", "right", "bottom", "top"):
        ax.axis[direction].set_visible(False)


@mpl20("axisline_style", figsize=(2, 2), rc=TICKS_IN)
@without_text
def _(fig):
    _axisline_style(fig, "->")


@mpl20("axisline_style_size_color", figsize=(2, 2), rc=TICKS_IN)
@without_text
def _(fig):
    _axisline_style(fig, "->, size=1.5", size=2.0, facecolor="r")


@mpl20("axisline_style_tight", figsize=(2, 2), rc=TICKS_IN)
@without_text
def _(fig):
    fig.set_layout_engine("tight")
    _axisline_style(fig, "->, size=8", size=5, facecolor="g")


@mpl20("subplotzero_ylabel")
def _(fig):
    ax = fig.add_subplot(111, axes_class=SubplotZero)
    ax.set(xlim=(-3, 7), ylim=(-3, 7), xlabel="x", ylabel="y")
    zero_axis = ax.axis["xzero", "yzero"]
    zero_axis.set_visible(True)
    ax.axis["left", "right", "bottom", "top"].set_visible(False)
    zero_axis.set_axisline_style("->")


# --- axisartist/tests/test_axis_artist.py ----------------------------------


@mpl20("axis_artist_ticks")
def _(fig):
    ax = fig.subplots()
    ax.xaxis.set_visible(False)
    ax.yaxis.set_visible(False)
    locs_angles = [((i / 10, 0.0), i * 30) for i in range(-1, 12)]
    ticks_in = Ticks(ticksize=10, axis=ax.xaxis)
    ticks_in.set_locs_angles(locs_angles)
    ax.add_artist(ticks_in)
    ticks_out = Ticks(ticksize=10, tick_direction="out", color="C3", axis=ax.xaxis)
    ticks_out.set_locs_angles(locs_angles)
    ax.add_artist(ticks_out)


@mpl20("axis_artist")
def _(fig):
    ax = fig.subplots()
    ax.xaxis.set_visible(False)
    ax.yaxis.set_visible(False)
    for loc in ("left", "right", "bottom"):
        helper = AxisArtistHelperRectlinear.Fixed(ax, loc=loc)
        axisline = AxisArtist(ax, helper, offset=None, axis_direction=loc)
        axisline.major_ticks.set_tick_direction(
            {"left": "in", "right": "out", "bottom": "inout"}[loc])
        ax.add_artist(axisline)
    axisline.set_label("TTT")
    axisline.label.set_pad(5)
    ax.set_ylabel("Test")


# --- axisartist/tests/test_grid_helper_curvelinear.py ----------------------


class ShearTransform(Transform):
    """``test_custom_transform``'s ``MyTransform``: ``(x, y - x)``, drawing
    paths through ``resolution`` steps per segment."""
    input_dims = output_dims = 2

    def __init__(self, resolution):
        Transform.__init__(self)
        self._resolution = resolution

    def transform(self, ll):
        x, y = ll.T
        return np.column_stack([x, y - x])

    transform_non_affine = transform

    def transform_path(self, path):
        ipath = path.interpolated(self._resolution)
        return Path(self.transform(ipath.vertices), ipath.codes)

    transform_path_non_affine = transform_path

    def inverted(self):
        return ShearTransformInv(self._resolution)


class ShearTransformInv(Transform):
    """``MyTransformInv``: ``(x, y + x)``."""
    input_dims = output_dims = 2

    def __init__(self, resolution):
        Transform.__init__(self)
        self._resolution = resolution

    def transform(self, ll):
        x, y = ll.T
        return np.column_stack([x, y + x])

    def inverted(self):
        return ShearTransform(self._resolution)


@mpl20("custom_transform", rc={"xtick.direction": "in", "ytick.direction": "inout"})
def _(fig):
    host_class = host_axes_class_factory(AxislinesAxes)
    tr = ShearTransform(1)
    grid_helper = grid_helper_curvelinear.GridHelperCurveLinear(tr)
    ax1 = host_class(fig, 1, 1, 1, grid_helper=grid_helper)
    fig.add_subplot(ax1)
    ax2 = ax1.get_aux_axes(tr, viewlim_mode="equal")
    ax2.plot([3, 6], [5.0, 10.])
    ax1.set_aspect(1.)
    ax1.set_xlim(0, 10)
    ax1.set_ylim(0, 10)
    ax1.grid(True)


def _cycle_extremes():
    """The extreme finder of the polar tests: degrees cycle, radii >= 0."""
    return angle_helper.ExtremeFinderCycle(20, 20, lon_cycle=360, lat_cycle=None,
                                           lon_minmax=None, lat_minmax=(0, np.inf))


def _eight(x, pos):
    """``test_polar_box``'s radius formatter."""
    return "eight" if x == 8 else f"{int(x)}"


@mpl20("polar_box", figsize=(5, 5), rc={"xtick.direction": "inout", "ytick.direction": "out"})
def _(fig):
    tr = polar_degrees()
    grid_helper = grid_helper_curvelinear.GridHelperCurveLinear(
        tr, extreme_finder=_cycle_extremes(), grid_locator1=angle_helper.LocatorDMS(12),
        tick_formatter1=angle_helper.FormatterDMS(), tick_formatter2=FuncFormatter(_eight))
    ax1 = SubplotHost(fig, 1, 1, 1, grid_helper=grid_helper)
    ax1.axis["right"].major_ticklabels.set_visible(True)
    ax1.axis["top"].major_ticklabels.set_visible(True)
    ax1.axis["right"].get_helper().nth_coord_ticks = 0
    ax1.axis["bottom"].get_helper().nth_coord_ticks = 1
    fig.add_subplot(ax1)
    ax1.axis["lat"] = axis = grid_helper.new_floating_axis(0, 45, axes=ax1)
    axis.label.set_text("Test")
    axis.label.set_visible(True)
    axis.get_helper().set_extremes(2, 12)
    ax1.axis["lon"] = axis = grid_helper.new_floating_axis(1, 6, axes=ax1)
    axis.label.set_text("Test 2")
    axis.get_helper().set_extremes(-180, 90)
    ax2 = ax1.get_aux_axes(tr, viewlim_mode="equal")
    ax2.plot(np.linspace(0, 30, 50), np.linspace(10, 10, 50))
    ax1.set_aspect(1.)
    ax1.set_xlim(-5, 12)
    ax1.set_ylim(-5, 10)
    ax1.grid(True)


@mpl20("axis_direction", figsize=(5, 5))
def _(fig):
    grid_helper = grid_helper_curvelinear.GridHelperCurveLinear(
        polar_degrees(), extreme_finder=_cycle_extremes(),
        grid_locator1=angle_helper.LocatorDMS(12), tick_formatter1=angle_helper.FormatterDMS())
    ax1 = SubplotHost(fig, 1, 1, 1, grid_helper=grid_helper)
    for axis in ax1.axis.values():
        axis.set_visible(False)
    fig.add_subplot(ax1)
    for name, value, direction in [("lat1", 130, "left"), ("lat2", 50, "right")]:
        ax1.axis[name] = axis = grid_helper.new_floating_axis(
            0, value, axes=ax1, axis_direction=direction)
        axis.label.set_text("Test")
        axis.label.set_visible(True)
        axis.get_helper().set_extremes(0.001, 10)
    ax1.axis["lon"] = axis = grid_helper.new_floating_axis(1, 10, axes=ax1,
                                                           axis_direction="bottom")
    axis.label.set_text("Test 2")
    axis.get_helper().set_extremes(50, 130)
    axis.major_ticklabels.set_axis_direction("top")
    axis.label.set_axis_direction("top")
    grid_helper.grid_finder.grid_locator1.set_params(nbins=5)
    grid_helper.grid_finder.grid_locator2.set_params(nbins=5)
    ax1.set_aspect(1.)
    ax1.set_xlim(-8, 8)
    ax1.set_ylim(-4, 12)
    ax1.grid(True)


# --- axisartist/tests/test_floating_axes.py --------------------------------


def _scatter_path(ax1, tr):
    """The data of the floating axes tests, clipped to the axes patch."""
    ax2 = ax1.get_aux_axes(tr)
    xx, yy = [67, 90, 75, 30], [2, 5, 8, 4]
    ax2.scatter(xx, yy)
    line, = ax2.plot(xx, yy, "k-")
    line.set_clip_path(ax1.patch)


def _floating_helper(tr, extremes, degrees_step):
    return floating_axes.GridHelperCurveLinear(
        tr, extremes=extremes, grid_locator1=angle_helper.LocatorDMS(degrees_step),
        grid_locator2=FixedLocator([2, 4, 6, 8, 10]),
        tick_formatter1=angle_helper.FormatterDMS(), tick_formatter2=None)


@mpl20("curvelinear3", figsize=(5, 5))
def _(fig):
    tr = polar_degrees()
    grid_helper = _floating_helper(tr, (0, 360, 10, 3), 15)
    ax1 = fig.add_subplot(axes_class=floating_axes.FloatingAxes, grid_helper=grid_helper)
    r_scale = 10
    tr2 = Affine2D().scale(1, 1 / r_scale) + tr
    grid_helper2 = floating_axes.GridHelperCurveLinear(
        tr2, extremes=(0, 360, 10 * r_scale, 3 * r_scale),
        grid_locator2=FixedLocator([30, 60, 90]))
    ax1.axis["right"] = grid_helper2.new_fixed_axis("right", axes=ax1)
    ax1.axis["left"].label.set_text("Test 1")
    ax1.axis["right"].label.set_text("Test 2")
    ax1.axis["left", "right"].set_visible(False)
    axis = grid_helper.new_floating_axis(1, 7, axes=ax1, axis_direction="bottom")
    ax1.axis["z"] = axis
    axis.toggle(all=True, label=True)
    axis.label.set_text("z = ?")
    axis.label.set_visible(True)
    axis.line.set_color("0.5")
    _scatter_path(ax1, tr)


@mpl20("curvelinear4", figsize=(5, 5))
def _(fig):
    tr = polar_degrees()
    grid_helper = _floating_helper(tr, (120, 30, 10, 0), 5)
    ax1 = fig.add_subplot(axes_class=floating_axes.FloatingAxes, grid_helper=grid_helper)
    ax1.clear()
    ax1.axis["left"].label.set_text("Test 1")
    ax1.axis["right"].label.set_text("Test 2")
    ax1.axis["top"].set_visible(False)
    axis = grid_helper.new_floating_axis(1, 70, axes=ax1, axis_direction="bottom")
    ax1.axis["z"] = axis
    axis.toggle(all=True, label=True)
    axis.label.set_axis_direction("top")
    axis.label.set_text("z = ?")
    axis.label.set_visible(True)
    axis.line.set_color("0.5")
    _scatter_path(ax1, tr)
