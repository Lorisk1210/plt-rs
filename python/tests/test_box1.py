"""
What USR2's programs of clip paths, offset boxes and annotation arrows
showed pltrs doing otherwise than matplotlib 3.11, reduced to matplotlib's
API:
- Agg's last clip mask, which only an outline that is the same path again
  reuses (a fancy box builds its own anew at every draw);
- an arrow clipping at the size of its last draw;
- the clip path a clipped ``DrawingArea`` gives its children;
- the offsets of a drawn legend's inner boxes;
- an ``AuxTransformBox`` whose aux transform is assigned after its children
  were added;
- an annotation arrow's ``patchA``, shrinks, registered styles and the
  properties ``arrowprops`` set on it.
Every expected value was recorded from matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.patheffects as pe
import pltrs.pyplot as plt
from pltrs.offsetbox import AnchoredOffsetbox, AuxTransformBox, DrawingArea, OffsetBox
from pltrs.patches import (Circle, ConnectionPatch, ConnectionStyle, Ellipse,
                           FancyArrowPatch, FancyBboxPatch, Rectangle)
from pltrs.path import Path
from pltrs.transforms import Affine2D


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def rgb(fig):
    """The figure drawn by Agg, as an RGB array."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba())[..., :3]


def two_images_clipped_to(make):
    """Whether the right one of two images, in two Axes, clipped to the
    hidden patch *make(fig)* in figure coordinates, is drawn."""
    fig, axs = plt.subplots(1, 2, figsize=(6, 3), dpi=40)
    window = make(fig)
    fig.add_artist(window).set_visible(False)
    for ax in axs:
        ax.imshow(np.ones((3, 3)), cmap="autumn").set_clip_path(window)
    return bool((rgb(fig)[60, 150] == (255, 0, 0)).all())


def test_a_mask_is_reused_only_for_the_same_path():
    # A fancy box's outline is new at each draw: its mask, cut to the left
    # Axes, is not reused for the right one. An ellipse's is the shared
    # unit circle: matplotlib reuses its mask and loses the right image
    # (its own artefact, which pltrs keeps).
    assert two_images_clipped_to(lambda fig: FancyBboxPatch(
        (0.25, 0.2), 0.5, 0.6, boxstyle="square,pad=0", transform=fig.transFigure))
    assert not two_images_clipped_to(lambda fig: Ellipse(
        (0.5, 0.5), 1.4, 1.6, transform=fig.transFigure))


def test_an_arrow_clips_at_the_size_of_its_last_draw():
    # matplotlib's `get_path` sizes an arrow's head and tail at its last
    # draw, 1 pixel per point before any: a hidden arrow clips that way.
    fig = plt.figure(figsize=(4, 2), dpi=40)
    arrow = FancyArrowPatch((0.05, 0.5), (0.95, 0.5), transform=fig.transFigure,
                            arrowstyle="simple,head_width=8,tail_width=4",
                            mutation_scale=10)
    fig.add_artist(arrow).set_visible(False)
    ax = fig.add_axes((0, 0, 1, 1))
    ax.imshow(np.ones((2, 2)), cmap="autumn", aspect="auto").set_clip_path(arrow)
    red = (rgb(fig) == (255, 0, 0)).all(axis=-1)
    assert [int(red.sum()), int(red.any(axis=1).sum())] == [5560, 64]


def area_with_children():
    """A clipped drawing area anchored in the centre of an Axes, with a
    circle, a circle clipped to a box of its own and a circle clipped to
    a path of its own."""
    from pltrs.transforms import Bbox
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    area = DrawingArea(40, 20, clip=True)
    plain, boxed, pathed = (Circle((20, 10), 30) for _ in range(3))
    boxed.set_clip_box(Bbox([[0, 0], [100, 100]]))
    pathed.set_clip_path(Path([(0, 0), (1, 0), (1, 1)]), Affine2D())
    for child in (plain, boxed, pathed):
        area.add_artist(child)
    anchored = AnchoredOffsetbox(loc="center", child=area)
    ax.add_artist(anchored)
    return fig, area, anchored, (plain, boxed, pathed)


def outline(clip):
    """The vertices of the clip path *clip* in display pixels."""
    return np.round(clip.get_fully_transformed_path().vertices, 1).tolist()


def test_a_clipped_area_gives_its_outline_to_unclipped_children():
    fig, area, anchored, (plain, boxed, pathed) = area_with_children()
    assert plain.get_clip_path() is None
    fig.canvas.draw()
    clip = plain.get_clip_path()
    assert type(clip).__name__ == "TransformedPath"
    assert outline(clip) == [[63.0, 67.3], [63.0, 81.2], [90.8, 81.2], [90.8, 67.3]]
    # Children clipped by a box or a path of their own keep theirs.
    assert boxed.get_clip_path() is None
    assert type(pathed.get_clip_path()).__name__ == "TransformedPath"
    assert len(pathed.get_clip_path().get_fully_transformed_path().vertices) == 3
    # The outline keeps the area's size at its first draw and follows it.
    area.width = 80
    anchored.loc = 2
    fig.canvas.draw()
    assert plain.get_clip_path() is clip
    assert outline(clip) == [[25.0, 111.9], [25.0, 125.7], [52.8, 125.7], [52.8, 111.9]]


def test_a_drawn_legend_places_its_inner_boxes():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    ax.plot([0, 1], label="one")
    ax.scatter([0.5], [0.5], label="two")
    legend = ax.legend(loc="center", title="T")
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    boxes = legend.findobj(lambda a: isinstance(a, OffsetBox))
    assert [(type(box).__name__,
             [round(float(v), 1) for v in box.get_window_extent(renderer).bounds])
            for box in boxes] == [
        ("TextArea", [100.5, 81.6, 4.0, 7.7]), ("DrawingArea", [86.3, 72.1, 13.9, 4.9]),
        ("TextArea", [105.7, 70.4, 12.0, 7.7]), ("HPacker", [86.3, 70.4, 31.4, 7.7]),
        ("DrawingArea", [86.3, 60.9, 13.9, 4.9]), ("TextArea", [105.7, 59.3, 13.0, 7.7]),
        ("HPacker", [86.3, 59.3, 32.4, 7.7]), ("VPacker", [86.3, 59.3, 32.4, 18.8]),
        ("HPacker", [86.3, 59.3, 32.4, 18.8]), ("VPacker", [83.5, 56.5, 38.0, 35.5])]


def test_an_aux_transform_assigned_later_leaves_the_children():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    box = AuxTransformBox(ax.transData)
    box.add_artist(Rectangle((0, 0), 2, 1))
    ax.add_artist(AnchoredOffsetbox(loc="upper left", child=box))
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    before = box.get_bbox(renderer).bounds
    box.aux_transform = Affine2D().rotate_deg(30) + ax.transData
    fig.canvas.draw()
    assert box.get_bbox(renderer).bounds == pytest.approx(before)
    assert [round(float(v), 2) for v in before] == [0.0, 0.0, 31.0, 11.55]


class Elbow(ConnectionStyle._Base):
    """A connection style of the program's: along x, then along y."""

    def connect(self, posA, posB):
        (xa, ya), (xb, yb) = posA, posB
        return Path([(xa, ya), (xa, yb), (xb, yb)])


def test_an_annotation_arrow_reads_back_as_made_and_placed():
    ConnectionStyle._style_list["box1-elbow"] = Elbow
    try:
        fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
        note = ax.annotate(
            "n", (0.8, 0.2), (0.2, 0.6), bbox=dict(boxstyle="round"),
            arrowprops=dict(arrowstyle="-|>", connectionstyle="box1-elbow", shrinkA=3,
                            path_effects=[pe.Normal()], picker=True))
        plain = ax.annotate("p", (0.8, 0.8), (0.2, 0.9), arrowprops=dict(arrowstyle="->"))
        empty = ax.annotate("", (0.5, 0.5), (0.1, 0.1), arrowprops=dict(arrowstyle="->"))
        arrow = note.arrow_patch
        assert arrow.patchA is None
        fig.canvas.draw()
        assert type(arrow.get_connectionstyle()).__name__ == "Elbow"
        assert arrow.patchA is note.get_bbox_patch()
        assert type(plain.arrow_patch.patchA).__name__ == "Rectangle"
        assert empty.arrow_patch.patchA is None
        assert [arrow.shrinkA, type(arrow.shrinkA).__name__, arrow.shrinkB,
                type(arrow.shrinkB).__name__] == [3, "int", 2, "int"]
        assert [type(e).__name__ for e in arrow.get_path_effects()] == ["Normal"]
        assert arrow.get_picker() is True
    finally:
        del ConnectionStyle._style_list["box1-elbow"]
    with pytest.raises(AttributeError, match="FancyArrowPatch.set.. got an unexpected "
                                             "keyword argument 'nonsense'"):
        ax.annotate("x", (0.5, 0.5), (0.1, 0.1), arrowprops=dict(arrowstyle="->", nonsense=1))


def test_a_connection_outside_the_view_measures_its_path():
    # Not drawn (its end fails the annotation clip test), but measured.
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    patch = ax.add_artist(ConnectionPatch((5000, 1), (8000, 3), "data"))
    fig.canvas.draw()
    bounds = patch.get_window_extent(fig.canvas.get_renderer()).bounds
    assert np.round(bounds, 1).tolist() == [775025.0, 132.0, 465000.0, 231.0]


def test_relim_sizes_an_arrow_at_its_last_draw():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    line, = ax.plot([0, 10000], [0, 10])
    arrow = ax.add_patch(FancyArrowPatch((2000, 1), (5000, 3)))
    fig.canvas.draw()
    line.set_xdata([0, 10])
    arrow.set_positions((2, 1), (5, 3))
    ax.relim()
    assert np.round(ax.dataLim.bounds, 3).tolist() == [-7.57, 0.0, 24.642, 10.0]
