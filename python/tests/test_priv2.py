"""
State pltrs (Rust) owns that programs assign as matplotlib's attributes:
the names libraries' tests wait for (a collection's per-item transforms
and colours, an x axis' tick position, an Axes' axis map, the switches
`tick_params` was given, a 3-D collection's offsets between draws, a
legend anchored to a box), the public attributes matplotlib's stubs type
as assignable (a figure's patch and subplot parameters, a colorbar's
fields and extensions, a contour set's levels and labels, a quiver's
span, an Axes' spines and child Axes, a spine's axis and type), and the
one means `_values.assignable` behind them. Every expected value was
recorded from matplotlib 3.11.
"""

import pickle

import numpy as np
import pytest

import pltrs
import pltrs.collections as mcollections
import pltrs.colorbar as mcolorbar
import pltrs.gridspec as mgridspec
import pltrs.patches as mpatches
import pltrs.pyplot as plt
import pltrs.transforms as mtransforms
from pltrs.path import Path

Z = np.add.outer(np.linspace(-1, 1, 12), np.linspace(-1, 1, 12) ** 2)


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def drawn(fig):
    """The figure's pixels (RGB) after a draw."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba(), dtype=float)[..., :3].copy()


def test_a_collection_draws_the_item_transforms_assigned():
    # matplot2tikz's collection of transformed ellipses.
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    circles = mcollections.Collection(
        offsets=[(0.3, 0.3), (0.7, 0.7)], offset_transform=ax.transData)
    circles.set_transform(mtransforms.IdentityTransform())
    circles._paths = [Path.unit_circle()]
    ax.add_collection(circles, autolim=False)
    before = drawn(fig)
    circles._transforms = np.zeros((2, 3, 3))
    circles._transforms[..., 0, 0] = 10
    circles._transforms[..., 1, 1] = 5
    circles._transforms[..., 2, 2] = 1
    assert circles.get_transforms()[1].tolist() == [[10, 0, 0], [0, 5, 0], [0, 0, 1]]
    assert not np.array_equal(drawn(fig), before)


def test_a_scatter_reads_its_transforms_back_until_drawn():
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    dots = ax.scatter([0, 1], [0, 1], s=[20, 80])
    fig.canvas.draw()
    sized = dots.get_transforms()
    assigned = np.tile(np.eye(3), (2, 1, 1)) * 10
    dots._transforms = assigned
    np.testing.assert_array_equal(dots.get_transforms(), assigned)
    fig.canvas.draw()
    np.testing.assert_allclose(dots._transforms, sized)
    assert np.round(dots._transforms[1, 0, 0], 3) == 6.211


def test_assigned_colours_read_back_as_given_until_mapped_again():
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    dots = ax.scatter([0, 1], [0, 1], s=200)
    before = drawn(fig)
    dots._facecolors = np.array([[1, 0, 0, 1], [0, 1, 0, 1]])
    assert dots.get_facecolor().dtype.kind == "i"
    assert dots._facecolors.tolist() == [[1, 0, 0, 1], [0, 1, 0, 1]]
    assert not np.array_equal(drawn(fig), before)
    dots._edgecolors = np.array([[0, 0, 1, 1]])
    assert dots.get_edgecolor().tolist() == [[0, 0, 1, 1]]
    mapped = ax.scatter([0, 1], [1, 0], c=[1, 2], s=200)
    fig.canvas.draw()
    mapped._facecolors = np.array([[1, 0, 0, 1]])
    fig.canvas.draw()
    assert np.round(mapped._facecolors, 3).tolist() == [
        [0.267, 0.005, 0.329, 1.0], [0.993, 0.906, 0.144, 1.0]]


def test_the_x_axis_offset_text_follows_its_tick_position():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=60)
    ax.plot([1e6, 3e6], [0, 1])
    ax.xaxis.tick_top()
    assert ax.xaxis._tick_position == "top"
    ax.xaxis._tick_position = "bottom"
    fig.canvas.draw()
    assert ax.xaxis._tick_position == "bottom"
    assert ax.xaxis.offsetText.get_position()[1] == pytest.approx(10.7)


def test_the_axis_map_names_the_axes_of_every_projection():
    fig = plt.figure()
    flat = fig.add_subplot(1, 3, 1)
    deep = fig.add_subplot(1, 3, 2, projection="3d")
    polar = fig.add_subplot(1, 3, 3, projection="polar")
    assert {k: type(v).__name__ for k, v in flat._axis_map.items()} == {
        "x": "XAxis", "y": "YAxis"}
    assert {k: type(v).__name__ for k, v in deep._axis_map.items()} == {
        "x": "XAxis", "y": "YAxis", "z": "ZAxis"}
    assert {k: type(v).__name__ for k, v in polar._axis_map.items()} == {
        "x": "ThetaAxis", "y": "RadialAxis"}


def test_tick_switches_read_back_as_given():
    fig, ax = plt.subplots()
    ax.plot([1, 2])
    ax.tick_params(axis="x", which="both", bottom="off", top="on")
    ax.tick_params(axis="y", labelleft=0, right=1)
    tick = ax.xaxis.majorTicks[0]
    assert [tick.tick1line.get_visible(), tick.tick2line.get_visible(),
            tick.gridline.get_visible()] == ["off", "on", False]
    assert [ax.yaxis.majorTicks[0].label1.get_visible(),
            ax.yaxis.majorTicks[0].tick2line.get_visible()] == [0, 1]
    assert ax.xaxis._major_tick_kw == {
        "gridOn": False, "tick1On": "off", "tick2On": "on", "label1On": True,
        "label2On": False}
    fig.canvas.draw()
    assert ax.xaxis.get_major_ticks()[1].tick1line.get_visible() == "off"


def test_measuring_leaves_a_3d_collection_as_its_last_draw_left_it():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    dots = ax.scatter([0, 1, 2], [3, 4, 5], [6, 7, 8], c=["r", "g", "b"])
    ax.get_tightbbox(fig.canvas.get_renderer())
    assert dots.get_offsets().tolist() == [[0, 3], [1, 4], [2, 5]]
    assert dots.get_facecolor().tolist() == [[1, 0, 0, 1], [0, 0.5, 0, 1]]
    fig.canvas.draw()
    dots.set_offsets(np.zeros((2, 2)))
    ax.xaxis.get_ticklocs()
    assert dots.get_offsets().shape == (2, 2)


def test_a_legend_anchors_to_a_box_given():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([0, 1], label="a")
    legend = ax.legend(bbox_to_anchor=mtransforms.Bbox.from_bounds(10, 10, 50, 30),
                       bbox_transform=mtransforms.IdentityTransform(), loc="upper left")
    fig.canvas.draw()
    assert legend.get_bbox_to_anchor().bounds == (10, 10, 50, 30)
    assert np.round(legend.get_window_extent().bounds, 1).tolist() == [13.5, 23.3, 29.0, 13.2]


def test_a_figure_draws_the_patch_assigned():
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    patch = mpatches.Rectangle((0, 0), 1, 1, facecolor="yellow", transform=fig.transFigure)
    fig.patch = patch
    pixels = drawn(fig)
    assert fig.patch is patch
    assert pixels[0, 0].tolist() == [255, 255, 0]
    fig.set_facecolor("green")
    assert patch.get_facecolor() == (0.0, 0.5019607843137255, 0.0, 1.0)
    fig.patch = mpatches.Rectangle((0.1, 0.1), 0.5, 0.5, facecolor="red")
    pixels = drawn(fig)
    assert pixels[0, 0].tolist() == [255, 255, 255]
    pickle.loads(pickle.dumps(fig))


def test_assigned_subplot_parameters_place_the_subplots_added_after():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    params = mgridspec.SubplotParams(left=0.3)
    fig.subplotpars = params
    assert fig.subplotpars is params and params.left == 0.3
    fig.canvas.draw()
    assert np.round(ax.get_position().bounds, 3).tolist() == [0.125, 0.11, 0.775, 0.77]
    added = fig.add_subplot(2, 2, 4)
    assert np.round(added.get_position().bounds, 3).tolist() == [0.627, 0.11, 0.273, 0.35]
    fig.subplots_adjust(bottom=0.2)
    assert (params.left, params.bottom) == (0.3, 0.2)
    assert np.round(ax.get_position().bounds, 3).tolist() == [0.3, 0.2, 0.6, 0.68]
    sub = fig.subfigures(1, 2)[1]
    assert sub.subplotpars is params
    sub.subplotpars = own = mgridspec.SubplotParams(right=0.5)
    assert sub.subplotpars is own and sub.images == []


def test_colorbar_fields_are_kept_until_the_colorbar_updates():
    fig, ax = plt.subplots()
    image = ax.imshow(Z)
    cbar = fig.colorbar(image)
    plasma = plt.get_cmap("plasma")
    cbar.cmap, cbar.vmin = plasma, -10.0
    assert (cbar.cmap, cbar.vmin) == (plasma, -10.0)
    cbar.update_normal(image)
    assert cbar.cmap is image.cmap and cbar.vmin == image.norm.vmin
    cbar.orientation = "horizontal"
    assert cbar.orientation == "horizontal" and cbar.long_axis is cbar.ax.xaxis
    cbar.lines = kept = []
    cbar.add_lines([0.5], ["r"], [1], erase=False)
    assert cbar.lines is kept and len(kept) == 1
    cbar.outline = None
    assert cbar.outline is None


def test_an_assigned_extend_makes_room_for_the_extensions():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    cbar = fig.colorbar(ax.imshow(Z))
    fig.canvas.draw()
    cbar.extend = "both"
    fig.canvas.draw()
    assert np.round(cbar.ax.get_position().bounds, 3).tolist() == [0.784, 0.145, 0.038, 0.7]
    assert len(cbar._extend_patches) == 0
    cbar.extend = "neither"
    fig.canvas.draw()
    assert np.round(cbar.ax.get_position().bounds, 3).tolist() == [0.784, 0.11, 0.042, 0.77]
    cbar.extend = "sideways"
    assert cbar.extend == "sideways"


def test_contour_fields_read_back_as_assigned():
    fig, ax = plt.subplots()
    lines = ax.contour(Z, levels=[-0.5, 0, 0.5, 1])
    lines.clabel()
    for name, value in [("levels", np.array([0.0, 0.25])), ("cvalues", [5.0, 6.0]),
                        ("filled", True), ("extend", "both"), ("labelTexts", []),
                        ("labelCValues", []), ("labelLevelList", [0.5]), ("labelXYs", [])]:
        setattr(lines, name, value)
        assert getattr(lines, name) is value
    fig.canvas.draw()


def test_a_quiver_span_assigned_reads_back_until_the_next_draw():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    arrows = ax.quiver([0, 1], [0, 1], [1, 1], [1, 2])
    arrows.span = 10.0
    assert arrows.span == 10.0
    fig.canvas.draw()
    assert arrows.span == 1.0
    arrows.span = 3.0
    assert arrows.span == 3.0
    fig.canvas.draw()
    assert arrows.span == 1.0


def test_axes_spines_child_axes_and_spine_fields_are_assignable():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    ax.plot([0, 1])
    spines = ax.spines
    ax.spines = spines
    assert ax.spines is spines
    ax.child_axes = kept = []
    inset = ax.inset_axes([0.5, 0.5, 0.3, 0.3])
    assert kept == [inset] and ax.child_axes is kept
    left = ax.spines["left"]
    left.axis = None
    assert left.axis is None
    fig.canvas.draw()
    assert np.round(left.get_window_extent().bounds, 2).tolist() == [18.75, 16.5, 0.0, 115.5]
    left.spine_type = "right"
    assert left.spine_type == "right"
    outline = mcolorbar._ColorbarSpine(ax)
    assert (outline.spine_type, outline.get_zorder(), outline.axes) == ("colorbar", 2.5, ax)


def test_assignable_is_the_one_means_of_assigned_attributes():
    from pltrs import _values

    class Held:
        def __init__(self):
            self.set_to = None

        def get(self):
            return "held"

        def set(self, value):
            self.set_to = value

        through_setter = _values.assignable(get, set)

        @_values.assignable
        def kept(self):
            return "held"

    held = Held()
    assert held.kept == "held"
    held.kept = "assigned"
    assert held.kept == "assigned" and _values.take(held, "kept") == {"kept": "assigned"}
    held.through_setter = 3
    assert held.set_to == 3 and held.through_setter == "held"


def test_fields_the_wider_census_found():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    wedge = ax.add_patch(mpatches.Wedge((0.5, 0.5), 0.3, 10, 200))
    arrow_path = Path([(0, 0), (1, 0), (0, 1)])
    wedge._path = arrow_path
    assert wedge.get_path() is arrow_path
    patch = mpatches.Rectangle((0, 0), 1, 1, transform=fig.transFigure)
    assert patch._transformSet is True
    patch._transformSet = False
    ax.add_patch(patch)
    assert patch.get_data_transform() == ax.transData
    image = ax.imshow(Z)
    assert image._imcache is None
    dots = ax.scatter([0, 1], [0, 1])
    dots._sizes = np.array([10.0, 40.0])
    assert dots.get_sizes().tolist() == [10.0, 40.0]


def test_values_matplotlib_keeps_as_its_code_made_them():
    fig, ax = plt.subplots()
    assert ax.vlines([0], 0, 1).get_label() == "_child0"
    ax.xaxis._tick_position = "default"
    assert ax.xaxis._tick_position == "default"
    fig.canvas.draw()
    assert ax.xaxis.offsetText.get_position()[1] > 400
    fig3 = plt.figure()
    ax3 = fig3.add_subplot(projection="3d")
    text = ax3.text(0, 0, 0, "x", zdir="x")
    fig3.canvas.draw()
    assert (text._rotation, ax3._frameon) == (0.0, False)
    table = ax.table([["a", "b"]], colLabels=["c", "d"], cellLoc="left", colLoc="center")
    assert (table[0, 0]._loc, table[1, 0]._loc) == ("center", "left")
    cbar = fig.colorbar(ax.imshow(Z))
    cbar._set_view((-2, 2))
    assert cbar._get_view() == (-2, 2)
