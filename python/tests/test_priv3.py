"""
State programs give legends, patches, child Axes, spines and offset texts
that pltrs must draw as matplotlib does, not only read back: a legend
anchored after the fact (the figure's own box, which a tight save does
not move), a patch class whose path is its own (``get_path``,
``get_patch_transform``, a Wedge's ``_recompute_path``), a legend box
repacked the way legendkit does it, an assigned legend frame, an Axes'
assigned ``child_axes``, a spine given another side's ``spine_type`` and
an offset text's vertical alignment. Every expected value was recorded
from matplotlib 3.11.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.patches as mpatches
import pltrs.pyplot as plt
import pltrs.transforms as mtransforms
from pltrs.offsetbox import HPacker, VPacker
from pltrs.path import Path


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def rounded(bbox, digits=1):
    """A box's bounds, rounded."""
    return np.round(bbox.bounds, digits).tolist()


def extent_while_saved(fig, artist, **kwargs):
    """The artist's extent in the draw a save makes."""
    seen = []
    cid = fig.canvas.mpl_connect(
        "draw_event", lambda event: seen.append(rounded(artist.get_window_extent())))
    fig.savefig(io.BytesIO(), format="png", **kwargs)
    fig.canvas.mpl_disconnect(cid)
    return seen[-1]


def anchored_figure(how):
    """A figure legend anchored at (0.9, 0.9), with a text far from it so
    that a tight box differs from the figure's."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    line, = ax.plot([0, 1], label="a")
    if how == "after":
        legend = fig.legend([line], ["a"], loc="center")
        legend.set_bbox_to_anchor((0.9, 0.9))
    else:
        legend = fig.legend([line], ["a"], loc="center", bbox_to_anchor=(0.9, 0.9))
    fig.text(0.02, 0.02, "x", fontsize=8)
    return fig, legend


@pytest.mark.parametrize("how, tight", [
    ("after", [165.5, 128.4, 29.0, 13.2]),
    ("call", [166.5, 131.7, 29.0, 13.2]),
])
def test_a_figure_legend_anchored_after_the_fact_keeps_the_figures_box(how, tight):
    # seaborn's corner pair grid, saved tight: set_bbox_to_anchor anchors
    # to the figure's own box, the constructor's anchor to transSubfigure.
    fig, legend = anchored_figure(how)
    assert extent_while_saved(fig, legend) == [165.5, 128.4, 29.0, 13.2]
    assert extent_while_saved(fig, legend, bbox_inches="tight") == tight
    fig.canvas.draw()
    assert rounded(legend.get_window_extent()) == [165.5, 128.4, 29.0, 13.2]


def test_a_legend_anchored_after_the_fact_follows_size_and_dpi():
    fig, legend = anchored_figure("after")
    fig.set_size_inches(5, 4)
    fig.canvas.draw()
    assert rounded(legend.get_bbox_to_anchor()) == [225.0, 180.0, 0.0, 0.0]
    assert rounded(legend.get_window_extent()) == [210.5, 173.4, 29.0, 13.2]
    fig.set_dpi(80)
    fig.canvas.draw()
    assert rounded(legend.get_bbox_to_anchor()) == [360.0, 288.0, 0.0, 0.0]


def test_subfigure_and_axes_legends_anchored_after_the_fact():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    ax.plot([0, 1], label="a")
    sub = fig.subfigures(1, 2)[1]
    legend = sub.legend(ax.get_lines(), ["a"], loc="center")
    legend.set_bbox_to_anchor((0.5, 0.9))
    fig.canvas.draw()
    assert rounded(legend.get_bbox_to_anchor()) == [150.0, 135.0, 0.0, 0.0]
    axes_legend = ax.legend(loc="center")
    axes_legend.set_bbox_to_anchor((1.1, 0.5))
    fig.canvas.draw()
    assert rounded(axes_legend.get_window_extent()) == [181.0, 67.6, 29.0, 13.2]


TRIANGLE = Path([(0, 0), (1, 0), (0.5, 1), (0, 0)], closed=True)


class ArrowWedge(mpatches.Wedge):
    """dna_features_viewer's wedge: its path made by its own hook."""

    def _recompute_path(self):
        (x, y), r = self.center, self.r
        self._path = Path([(x, y), (x + r, y), (x, y + r), (x, y)], closed=True)


class Shifted(mpatches.Rectangle):
    """A rectangle placed by its own patch transform."""

    def get_patch_transform(self):
        return super().get_patch_transform() + mtransforms.Affine2D().translate(0.3, 0.1)


def painted_box(fig, colour):
    """The rows and columns of the drawing's pixels of *colour*: first
    and last of each."""
    pixels = np.asarray(fig.canvas.buffer_rgba())[..., :3]
    found = np.argwhere((pixels == colour).all(axis=-1))
    return [*found.min(axis=0).tolist(), *found.max(axis=0).tolist()]


#: Where each case's program path is painted, in matplotlib.
PAINTED = {
    "Rectangle.get_patch_transform": [51, 45, 73, 66],
    "Rectangle": [37, 29, 72, 65],
    "Ellipse": [40, 53, 49, 68],
    "Circle": [29, 52, 49, 72],
    "Polygon": [14, 14, 88, 88],
    "RegularPolygon": [29, 52, 49, 72],
    "Wedge": [14, 14, 88, 88],
    "Arc": [35, 51, 50, 73],
    "Annulus": [14, 14, 88, 88],
    "FancyBboxPatch": [14, 14, 88, 88],
    "PathPatch": [14, 14, 88, 88],
    "Wedge._recompute_path": [37, 36, 64, 63],
}


def own_path(cls):
    """A subclass of *cls* whose path is a triangle."""
    return type("Own" + cls.__name__, (cls,), {"get_path": lambda self: TRIANGLE})


@pytest.mark.parametrize("make, limits, extent, hits", [
    (lambda: Shifted((0.1, 0.1), 0.3, 0.3),
     [0.4, 0.2, 0.3, 0.3], [43.5, 26.4, 23.2, 23.1], [False, False, False]),
    (lambda: own_path(mpatches.Rectangle)((0.2, 0.2), 0.5, 0.5),
     [0.2, 0.2, 0.5, 0.5], [28.0, 26.4, 38.8, 38.5], [True, False, False]),
    (lambda: own_path(mpatches.Ellipse)((0.5, 0.5), 0.5, 0.3),
     [0.5, 0.5, 0.25, 0.15], [51.2, 49.5, 19.4, 11.5], [False, True, False]),
    (lambda: own_path(mpatches.Circle)((0.5, 0.5), 0.3),
     [0.5, 0.5, 0.3, 0.3], [51.2, 49.5, 23.2, 23.1], [False, True, False]),
    (lambda: own_path(mpatches.Polygon)([(0.1, 0.1), (0.9, 0.1), (0.9, 0.9)]),
     [0.0, 0.0, 1.0, 1.0], [12.5, 11.0, 77.5, 77.0], [True, True, True]),
    (lambda: own_path(mpatches.RegularPolygon)((0.5, 0.5), 6, radius=0.3),
     [0.5, 0.5, 0.3, 0.3], [51.2, 49.5, 23.2, 23.1], [False, True, False]),
    (lambda: own_path(mpatches.Wedge)((0.5, 0.5), 0.3, 10, 200),
     [0.0, 0.0, 1.0, 1.0], [12.5, 11.0, 77.5, 77.0], [True, True, True]),
    (lambda: own_path(mpatches.Arc)((0.5, 0.5), 0.6, 0.4, theta1=10, theta2=200, lw=3),
     [0.5, 0.5, 0.3, 0.2], [51.2, 49.5, 23.2, 15.4], [False, True, False]),
    (lambda: own_path(mpatches.Annulus)((0.5, 0.5), 0.4, 0.1),
     [0.0, 0.0, 1.0, 1.0], [12.5, 11.0, 77.5, 77.0], [True, True, True]),
    (lambda: own_path(mpatches.FancyBboxPatch)((0.2, 0.2), 0.5, 0.5),
     [0.0, 0.0, 1.0, 1.0], [12.5, 11.0, 77.5, 77.0], [True, True, True]),
    (lambda: own_path(mpatches.PathPatch)(Path([(0, 0), (1, 1)])),
     [0.0, 0.0, 1.0, 1.0], [12.5, 11.0, 77.5, 77.0], [True, True, True]),
    (lambda: ArrowWedge((0.3, 0.3), 0.4, 10, 200),
     [0.3, 0.3, 0.4, 0.4], [35.8, 34.1, 31.0, 30.8], [False, False, False]),
], ids=["Rectangle.get_patch_transform", "Rectangle", "Ellipse", "Circle", "Polygon",
        "RegularPolygon", "Wedge", "Arc", "Annulus", "FancyBboxPatch", "PathPatch",
        "Wedge._recompute_path"])
def test_a_patch_class_with_a_path_of_its_own_is_drawn_measured_and_hit_by_it(
        request, make, limits, extent, hits):
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    patch = make()
    patch.set_color("C1")
    ax.add_patch(patch)
    assert np.round(ax.dataLim.bounds, 3).tolist() == limits
    fig.canvas.draw()
    assert rounded(patch.get_window_extent()) == extent
    points = ax.transData.transform([(0.2, 0.2), (0.6, 0.6), (0.85, 0.15)])
    assert [bool(patch.contains_point(p)) for p in points] == hits
    assert painted_box(fig, (255, 127, 14)) == PAINTED[request.node.callspec.id]


def test_a_wedge_remakes_its_own_path_after_a_setter():
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    wedge = ArrowWedge((0.3, 0.3), 0.4, 10, 200)
    ax.add_patch(wedge)
    wedge.set_radius(0.2)
    fig.canvas.draw()
    assert np.round(wedge.get_path().vertices, 3).tolist()[:3] == [[0.3, 0.3], [0.5, 0.3], [0.3, 0.5]]
    assert rounded(wedge.get_window_extent()) == [35.8, 34.1, 15.5, 15.4]


@pytest.mark.parametrize("title_loc, extent", [
    ("top", [60.4, 56.5, 33.0, 35.5]),
    ("bottom", [60.4, 56.5, 33.0, 35.5]),
    ("left", [51.2, 62.1, 51.3, 24.4]),
    ("right", [51.2, 62.1, 51.3, 24.4]),
])
def test_a_legend_box_repacked_as_legendkit_does_is_drawn(title_loc, extent):
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    ax.plot([0, 1], label="aa")
    ax.plot([1, 0], label="b")
    legend = ax.legend(title="Title", loc="center")
    packer = VPacker if title_loc in ("top", "bottom") else HPacker
    children = [legend._legend_title_box, legend._legend_handle_box]
    if title_loc in ("bottom", "right"):
        children = children[::-1]
    alignment = "left" if packer is VPacker else "center"
    legend._legend_box = packer(pad=legend.borderpad * legend._fontsize,
                                sep=legend.labelspacing * legend._fontsize,
                                align=alignment, children=children)
    legend._legend_box.set_offset(legend._findoffset)
    fig.canvas.draw()
    assert rounded(legend.get_window_extent()) == extent


def test_an_assigned_legend_frame_is_drawn_as_matplotlibs():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    ax.plot([0, 1], label="a")
    ax.plot([1, 0], label="b")
    legend = ax.legend()
    legend.legendPatch = mpatches.FancyBboxPatch((0, 0), 1, 1, facecolor="red")
    fig.canvas.draw()
    frame = legend.legendPatch
    assert rounded(frame.get_bbox(), 3) == [102.528, 62.068, 29.0, 24.364]
    assert rounded(frame.get_window_extent(), 3) == [100.444, 59.984, 33.167, 28.531]
    assert painted_box(fig, (255, 0, 0)) == [62, 101, 88, 132]


def test_an_assigned_alignment_reads_back():
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="a")
    legend = ax.legend()
    legend._alignment = "right"
    assert legend._alignment == "right"
    assert legend.get_alignment() == "center"


def test_insets_left_out_of_assigned_child_axes_are_not_drawn():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    kept = ax.inset_axes([0.6, 0.1, 0.3, 0.3])
    dropped = ax.inset_axes([0.1, 0.6, 0.3, 0.3])
    kept.plot([1, 0])
    dropped.plot([0, 1])
    ax.child_axes = [kept]
    assert ax.child_axes == [kept]
    assert kept in ax.get_children() and dropped not in ax.get_children()
    fig.canvas.draw()
    # The one left out keeps the place it was last given.
    assert np.round(dropped.get_position().bounds, 4).tolist() == [0.2025, 0.572, 0.2325, 0.231]
    pixels = np.asarray(fig.canvas.buffer_rgba())[..., :3]
    x0, y0, x1, y1 = dropped.get_window_extent().extents
    rows = pixels.shape[0]
    assert (pixels[int(rows - y1 + 3):int(rows - y0 - 3), int(x0 + 3):int(x1 - 3)] == 255).all()


def test_a_spine_given_another_sides_type_is_measured_with_its_ticks():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    ax.yaxis.set_ticks_position("both")
    ax.spines["left"].spine_type = "right"
    fig.canvas.draw()
    assert rounded(ax.spines["left"].get_window_extent(), 2) == [18.75, 16.5, 2.43, 115.5]


def test_offset_texts_take_their_vertical_alignment_and_a_clear_resets_it():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    ax.plot([1e6, 3e6], [1e6, 3e6])
    ax.xaxis.offsetText.set_va("center")
    ax.yaxis.offsetText.set_va("top")
    fig.canvas.draw()
    assert rounded(ax.xaxis.offsetText.get_window_extent()) == [123.0, -0.9, 12.0, 6.9]
    assert rounded(ax.yaxis.offsetText.get_window_extent()) == [18.8, 127.1, 12.0, 6.9]
    ax.xaxis.tick_top()
    ax.yaxis.tick_right()
    ax.cla()
    ax.plot([1e6, 3e6], [1e6, 3e6])
    fig.canvas.draw()
    assert ax.xaxis.offsetText.get_va() == "top"
    assert ax.yaxis.offsetText.get_ha() == "left"


def test_a_cells_text_is_in_the_cells_figure_and_measured_where_drawn():
    # mne right-justifies a help table's first column by its cells' _loc.
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    table = ax.table(cellText=[["a", "bb"], ["ccc", "d"]], loc="center", cellLoc="left")
    for (row, col), cell in table.get_celld().items():
        if col == 0:
            cell._loc = "right"
    text = table[0, 0].get_text()
    assert text.get_figure(root=True) is fig
    fig.canvas.draw()
    assert rounded(text.get_window_extent()) == [24.6, 50.2, 4.0, 6.9]
    assert text.get_horizontalalignment() == "left"


def test_a_cells_text_measured_at_another_resolution_keeps_its_place():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    ax.plot([0, 1])
    table = ax.table(cellText=[["a"]], loc="bottom")
    fig.canvas.draw()
    text = table[0, 0].get_text()
    assert rounded(text.get_window_extent(dpi=100)) == [160.5, 5.4, 4.0, 13.9]


def test_an_arrow_class_with_its_own_get_path_draws_as_an_arrow():
    # matplotlib's arrow fills the path it makes in display coordinates,
    # not ``get_path``.
    def drawn(cls):
        fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
        ax.add_patch(cls((0.1, 0.1), (0.9, 0.8), arrowstyle="->", mutation_scale=20))
        fig.canvas.draw()
        return np.asarray(fig.canvas.buffer_rgba()).copy()

    own = own_path(mpatches.FancyArrowPatch)
    assert np.array_equal(drawn(own), drawn(mpatches.FancyArrowPatch))
