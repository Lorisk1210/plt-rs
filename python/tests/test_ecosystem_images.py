"""
What plotnine's and astropy's image comparisons need of matplotlib that
its own suite does not reach (``cargo xtask ecosystem suite`` compares
their images with baselines made under matplotlib 3.11.2), each drawn as
matplotlib 3.11 draws it. The expected values were recorded from
matplotlib 3.11.2 running the same code.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs.axes import Axes
from pltrs.collections import LineCollection
from pltrs.offsetbox import AnchoredOffsetbox, AuxTransformBox, TextArea, VPacker
from pltrs.text import Text
from pltrs.transforms import Bbox, IdentityTransform

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def _rounded(values, digits=3):
    """*values* (nested sequences of numbers) rounded, as lists."""
    if np.iterable(values):
        return [_rounded(v, digits) for v in values]
    return round(float(values), digits)


def test_tick_labels_stay_where_they_are_moved_across_their_axis():
    # plotnine justifies tick labels itself: it moves each y tick label
    # to an x of its own (and each x tick label to a y).
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    fig.canvas.draw()
    for tick in ax.yaxis.get_major_ticks():
        tick.label1.set_x(-0.1)
    for tick in ax.xaxis.get_major_ticks():
        tick.label1.set_y(-0.1)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    ytick, xtick = ax.yaxis.get_major_ticks()[1], ax.xaxis.get_major_ticks()[1]
    assert _rounded(ytick.label1.get_position()) == [-0.1, 2.0]
    assert _rounded(ytick.label1.get_window_extent(renderer).bounds) == [
        0.68, 50.823, 6.0, 10.0]
    assert _rounded(xtick.label1.get_window_extent(renderer).bounds) == [
        77.64, -9.872, 6.0, 10.0]


def test_tick_lines_measure_as_lines():
    # plotnine sizes its axis margins by the extents of the tick lines.
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    ytick, xtick = ax.yaxis.get_major_ticks()[1], ax.xaxis.get_major_ticks()[1]
    assert _rounded(ytick.tick1line.get_window_extent(renderer).bounds) == [
        34.25, 55.274, 3.5, 3.5]
    assert _rounded(xtick.tick1line.get_window_extent(renderer).bounds) == [
        78.89, 22.01, 3.5, 3.5]


def test_anchored_boxes_of_the_program_place_their_child_themselves():
    # plotnine's legends: an anchored box with a `get_offset` of its own.
    class Flex(AnchoredOffsetbox):
        def __init__(self, **kwargs):
            super().__init__(loc="center", **kwargs)

        def get_offset(self, bbox, renderer):
            x0, y0 = self.get_bbox_to_anchor().p0
            return x0 - bbox.width - bbox.x0, y0 - bbox.y0

    fig = plt.figure(figsize=(4, 3), dpi=72)
    box = VPacker(children=[TextArea("title"), TextArea("3")], align="left",
                  pad=0, sep=5)
    anchored = Flex(child=box, pad=0, frameon=False, borderpad=0)
    fig.add_artist(anchored)
    anchored.set_bbox_to_anchor((0.9, 0.3), fig.transFigure)
    fig.canvas.draw()
    extent = anchored.get_window_extent(fig.canvas.get_renderer())
    assert _rounded(extent.bounds) == [239.2, 64.8, 20.0, 25.805]


def test_aux_boxes_measure_their_collections():
    fig = plt.figure(figsize=(4, 3), dpi=72)
    aux = AuxTransformBox(IdentityTransform())
    aux.add_artist(LineCollection([[(0, 10), (5, 10)], [(18, 10), (23, 10)]]))
    aux.add_artist(Text(27, 0, "3.0", va="center"))
    anchored = AnchoredOffsetbox("upper right", child=aux, pad=0, borderpad=0,
                                 bbox_to_anchor=(1, 1),
                                 bbox_transform=fig.transFigure)
    fig.add_artist(anchored)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    assert _rounded(aux.get_bbox(renderer).bounds) == [0.0, 0.0, 42.0, 15.0]
    assert _rounded(anchored.get_window_extent(renderer).bounds) == [
        246.0, 201.0, 42.0, 15.0]


def test_anchored_boxes_measure_the_boxes_of_the_program_inside():
    # plotnine's colorbar legends, measured before the figure is drawn.
    class Sized(AuxTransformBox):
        def get_bbox(self, renderer):
            return super().get_bbox(renderer)

    fig = plt.figure(figsize=(4, 3), dpi=72)
    aux = Sized(IdentityTransform())
    aux.add_artist(LineCollection([[(0, 0), (23, 0)], [(0, 40), (23, 40)]]))
    aux.add_artist(Text(27, 0, "3.0"))
    anchored = AnchoredOffsetbox(
        "upper right", child=VPacker(children=[TextArea("gear"), aux], pad=0, sep=2),
        pad=0, borderpad=0, bbox_to_anchor=(1, 1), bbox_transform=fig.transFigure)
    fig.add_artist(anchored)
    extent = anchored.get_window_extent(fig.canvas.get_renderer())
    assert _rounded(extent.bounds) == [246.0, 161.195, 42.0, 54.805]


def test_an_artist_drawn_again_in_its_draw_shows_every_drawing():
    # astropy's tick labels: one text, moved and drawn once per label.
    class Labels(Text):
        def draw(self, renderer):
            for text, y in [("60", 0.2), ("70", 0.5), ("80", 0.8)]:
                self.set_text(text)
                self.set_position((0.5, y))
                super().draw(renderer)

    fig = plt.figure(figsize=(2, 2), dpi=50)
    fig.add_artist(Labels(0, 0, "", transform=fig.transFigure, ha="center",
                          va="center"))
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())[..., 0]
    rows = np.nonzero((pixels < 128).any(axis=1))[0]
    assert rows.tolist() == [17, 18, 19, 20, 21, 47, 48, 49, 50, 51,
                             77, 78, 79, 80, 81]


def test_paths_changed_in_place_change_what_their_collection_draws():
    # astropy moves the lines of a contour set into another coordinate
    # system by setting their vertices.
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    ax.set_axis_off()
    ax.set_xlim(0, 20)
    ax.set_ylim(0, 20)
    y, x = np.mgrid[0:20, 0:20]
    contours = ax.contour(np.exp(-((x - 10) ** 2 + (y - 10) ** 2) / 20),
                          levels=[0.5], colors="k")
    paths = contours.get_paths()
    for path in paths:
        path.vertices = path.vertices * 0.5
    assert contours.get_paths() is paths
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())[..., 0]
    ys, xs = np.nonzero(pixels < 128)
    assert [xs.min(), xs.max(), ys.min(), ys.max()] == [24, 39, 62, 76]


class _OwnTickParams(Axes):
    """An Axes class taking ``tick_params`` for itself, as astropy's
    ``WCSAxes`` does."""

    def tick_params(self, axis="both", **kwargs):
        pass


def test_axes_classes_with_their_own_tick_params_keep_the_ticks_of_tick():
    fig = plt.figure()
    made = _OwnTickParams(fig, [0.1, 0.1, 0.8, 0.8])
    fig.add_axes(made)
    added = fig.add_axes([0.1, 0.1, 0.8, 0.8], axes_class=_OwnTickParams)
    for ax in (made, added):
        tick = ax.yaxis.get_major_ticks()[0]
        assert [tick.tick1line.get_visible(), tick.tick2line.get_visible(),
                tick.label1.get_visible(), tick.label2.get_visible()] == [
                    True, True, True, False]
        assert ax.xaxis.get_ticks_position() == "default"


def test_tight_layout_asks_axes_classes_measuring_themselves():
    class Measured(Axes):
        def draw(self, renderer):
            super().draw(renderer)

        def get_tightbbox(self, renderer=None, *args, **kwargs):
            bbox = self.get_window_extent(renderer)
            return Bbox.from_extents(bbox.x0 - 40, bbox.y0 - 30, bbox.x1, bbox.y1)

    fig = plt.figure(figsize=(4, 3), dpi=72)
    ax = fig.add_subplot(axes_class=Measured)
    fig.tight_layout()
    assert _rounded(ax.get_position().bounds, 4) == [0.1764, 0.1889, 0.7861, 0.7611]
