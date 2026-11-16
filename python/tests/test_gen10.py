"""
What the fifth round on the generated families and TAIL4's table showed
pltrs doing otherwise than matplotlib 3.11, each with a small class or
assignment of the kind libraries write: a colormap of the program's in a
print cut to its tight box once a colour bar ticks it, spine and arrow
classes giving their transform, the box a program gives a 3-D axis
(qutip's ``_get_coord_info``), the colour `EventCollection` sets, the
boxes of tick labels a program assigns on an axis, a spine class' own
``_adjust_location`` and what a print cut to its tight box measures under
a layout engine. Every expected value was recorded from matplotlib.
"""

import io
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, colors, patches, spines, text


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_a_tight_print_maps_through_the_programs_colormap_once_a_colorbar_ticks_it():
    shapes = []

    class Cmap(colors.ListedColormap):
        def __call__(self, X, *args, **kwargs):
            shapes.append(np.shape(X))
            return super().__call__(X, *args, **kwargs)

    fig, ax = plt.subplots(figsize=(3, 2))
    image = ax.imshow(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)),
                      cmap=Cmap(["r", "g", "b"]))
    fig.colorbar(image)
    fig.canvas.draw()
    image.set_clim(0, 1)
    fig.canvas.draw()
    del shapes[:]
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert shapes == [(154, 185), (3, 1), (154, 185), (3, 1)]


class _TransformedSpine(spines.Spine):
    """A spine class giving its transform (as its base does)."""

    def get_transform(self):
        return super().get_transform()


class _TransformedArrow(patches.FancyArrowPatch):
    """An arrow class giving its transform (as its base does)."""

    def get_transform(self):
        return super().get_transform()


def test_spine_and_arrow_classes_giving_their_transform_draw_as_their_base():
    def drawn(spine_class, arrow_class):
        fig, ax = plt.subplots(figsize=(3, 2))
        ax.spines["left"] = spine_class.linear_spine(ax, "left")
        ax.add_patch(arrow_class((0.1, 0.1), (0.8, 0.7), arrowstyle="->", mutation_scale=20))
        fig = pickle.loads(pickle.dumps(fig))
        fig.canvas.draw()
        return np.asarray(fig.canvas.buffer_rgba())

    assert np.array_equal(drawn(_TransformedSpine, _TransformedArrow),
                          drawn(spines.Spine, patches.FancyArrowPatch))


def _histogram(layout=None):
    """A 3-D axes whose x axis is drawn on a box shrunk by a quarter of
    its offsets (qutip's ``_remove_margins``), counting the calls of the
    axis' box methods."""
    fig = plt.figure(figsize=(4, 3), layout=layout)
    ax = fig.add_subplot(projection="3d")
    ax.bar3d([0, 1, 2], [0, 1, 2], [0, 0, 0], 0.5, 0.5, [1, 2, 3])
    counts = {"box": 0, "centres": 0}
    axis = ax.xaxis
    own_box, own_centres = axis._get_coord_info, axis._calc_centers_deltas

    def centres(maxs, mins):
        counts["centres"] += 1
        return own_centres(maxs, mins)

    def box():
        counts["box"] += 1
        mins, maxs, bounds_proj, highs = own_box()
        _, deltas = axis._calc_centers_deltas(maxs, mins)
        return mins + deltas / 4, maxs - deltas / 4, bounds_proj, highs

    axis._calc_centers_deltas = centres
    axis._get_coord_info = box
    return fig, ax, counts


@pytest.mark.parametrize("layout, step, expected", [
    (None, "draw", (4, 6)),
    (None, "measure", (2, 4)),
    (None, "tight print", (10, 16)),
    (None, "print", (4, 6)),
    ("constrained", "draw", (8, 14)),
    ("constrained", "tight print", (14, 24)),
    ("constrained", "print", (12, 20)),
    ("tight", "tight print", (14, 24)),
])
def test_a_box_a_program_gives_a_3d_axis_is_asked_as_matplotlib_asks_it(layout, step, expected):
    fig, ax, counts = _histogram(layout)
    if step == "draw":
        fig.canvas.draw()
    elif step == "measure":
        fig.get_tightbbox(fig.canvas.get_renderer())
    else:
        fig.savefig(io.BytesIO(), format="png",
                    bbox_inches="tight" if step == "tight print" else None)
    assert (counts["box"], counts["centres"]) == expected


def test_a_hidden_3d_axis_is_drawn_not_measured_and_no_axis_asks_nothing():
    fig, ax, counts = _histogram()
    ax.xaxis.set_visible(False)
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert (counts["box"], counts["centres"]) == (8, 12)
    fig, ax, counts = _histogram()
    ax.set_axis_off()
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert counts == {"box": 0, "centres": 0}


def test_a_3d_axis_is_measured_on_the_box_a_program_gives_it_before_any_draw():
    fig, ax, _ = _histogram()
    fresh = fig.get_tightbbox(fig.canvas.get_renderer()).bounds
    assert fresh == pytest.approx((0.895, 0.1993380, 2.5129375, 2.4406620), abs=1e-6)
    fig.canvas.draw()
    assert fig.get_tightbbox(fig.canvas.get_renderer()).bounds == pytest.approx(fresh)


def test_an_event_collection_sets_its_colour_through_set_colors_alone():
    called = []

    class Events(collections.EventCollection):
        def set_color(self, c):
            called.append("set_color")
            return super().set_color(c)

        def set_colors(self, c):
            called.append("set_colors")
            return super().set_colors(c)

    events = Events([1, 2, 3], color="r")
    assert called == ["set_colors"]
    assert events.get_color().tolist() == [1.0, 0.0, 0.0, 1.0]


def test_tick_label_boxes_assigned_on_an_axis_measure_it():
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 10], [0, 1000])
    ax.set_xlabel("x label")
    ax.set_ylabel("y label")
    called = []
    own = ax.xaxis._get_ticklabel_bboxes

    def inner_labels(ticks, renderer):
        called.append(len(ticks))
        return own(ticks[1:-1], renderer)

    ax.xaxis._get_ticklabel_bboxes = inner_labels
    tight = ax.get_tightbbox(fig.canvas.get_renderer())
    assert tight.bounds == pytest.approx((-28.114366, -21.503255, 298.114366, 197.503255),
                                         abs=1e-5)
    fig.tight_layout()
    assert ax.get_position().bounds == pytest.approx((0.268715, 0.292516, 0.681285, 0.632484),
                                                     abs=1e-5)
    assert called


class _Spine(spines.Spine):
    """A spine counting the calls of its own ``_adjust_location``."""

    calls = 0

    def _adjust_location(self):
        type(self).calls += 1
        return super()._adjust_location()


@pytest.mark.parametrize("side, far_labels, layout, expected", [
    ("left", False, None, (2, 6)),
    ("left", True, None, (1, 3)),
    ("bottom", False, None, (2, 6)),
    ("top", True, None, (2, 6)),
    ("right", False, None, (1, 3)),
    ("left", False, "constrained", (6, 10)),
    ("bottom", False, "tight", (6, 10)),
])
def test_a_spine_class_adjusts_itself_where_matplotlibs_draws_and_measures_do(
        side, far_labels, layout, expected):
    fig, ax = plt.subplots(figsize=(3, 2), layout=layout)
    ax.plot([0, 1], [0, 1])
    ax.set_xlabel("x")
    ax.set_ylabel("y")
    ax.spines[side] = _Spine.linear_spine(ax, side)
    if far_labels:
        ax.yaxis.set_label_position("right")
        ax.xaxis.set_label_position("top")
    _Spine.calls = 0
    fig.canvas.draw()
    drawn = _Spine.calls
    _Spine.calls = 0
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert (drawn, _Spine.calls) == expected


@pytest.mark.parametrize("layout, expected", [
    ("constrained", 3), ("tight", 3), (None, 1)])
def test_a_tight_print_under_an_engine_measures_as_matplotlibs(layout, expected):
    measured = []

    class Text(text.Text):
        def get_window_extent(self, renderer=None, dpi=None):
            measured.append(1)
            return super().get_window_extent(renderer, dpi)

    fig, ax = plt.subplots(figsize=(3, 2), layout=layout)
    ax.add_artist(Text(0.5, 0.5, "t", transform=ax.transAxes, clip_on=False))
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert len(measured) == expected
