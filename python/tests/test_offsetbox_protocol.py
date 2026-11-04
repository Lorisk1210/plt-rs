"""
matplotlib's offset-box protocol on the package's boxes: ``get_bbox``,
``get_window_extent``, ``get_offset``, ``set_offset``, ``draw`` and
``get_visible_children`` of `TextArea`, `DrawingArea`, the packers,
`PaddedBox` and `OffsetImage`, so that boxes of the program's (an
`OffsetBox` subclass with matplotlib's algorithms written in Python, a
subclass of a package class overriding ``draw``, ``get_bbox`` or
``_get_bbox_and_child_offsets``) size, place and draw them; and changes
made to boxes once they joined a figure (packer settings, visibility,
texts, children lists, padding, frames), which the next draw shows.
Each ``scenario_*`` returns what its test compares; the expected values
were recorded from matplotlib 3.11.2 running the same scenarios.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs.offsetbox import (AnchoredOffsetbox, AnnotationBbox, DrawingArea,
                             HPacker, OffsetBox, OffsetImage, PaddedBox,
                             TextArea, VPacker)
from pltrs.patches import Circle, Rectangle
from pltrs.transforms import Bbox

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


class Column(OffsetBox):
    """Children stacked left-aligned, drawn by matplotlib's own
    `OffsetBox.draw` over their ``get_bbox``."""

    def __init__(self, children, sep=2):
        super().__init__()
        self._children = children
        self.sep = sep

    def _get_bbox_and_child_offsets(self, renderer):
        sep = self.sep * renderer.points_to_pixels(1.)
        bboxes = [c.get_bbox(renderer) for c in self.get_visible_children()]
        y, offsets = 0, []
        for bb in reversed(bboxes):
            offsets.append((-bb.x0, y - bb.y0))
            y += bb.height + sep
        offsets.reverse()
        width = max(bb.width for bb in bboxes)
        return Bbox.from_bounds(0, 0, width, y - sep), offsets


class BigArea(DrawingArea):
    """A drawing area claiming twice its height."""

    def get_bbox(self, renderer):
        bb = super().get_bbox(renderer)
        return Bbox.from_bounds(bb.x0, bb.y0 - bb.height, bb.width, 2 * bb.height)


class Counting(TextArea):
    """A text area counting its draws."""

    draws = 0

    def draw(self, renderer):
        type(self).draws += 1
        super().draw(renderer)


class Row(VPacker):
    """A `VPacker` laying its children out side by side, 5 pixels apart."""

    def _get_bbox_and_child_offsets(self, renderer):
        bboxes = [c.get_bbox(renderer) for c in self.get_visible_children()]
        x, offsets = 0, []
        for bb in bboxes:
            offsets.append((x - bb.x0, 0))
            x += bb.width + 5
        y0 = min(bb.y0 for bb in bboxes)
        y1 = max(bb.y1 for bb in bboxes)
        return Bbox.from_bounds(0, y0, x - 5, y1 - y0), offsets


class Framed(PaddedBox):
    """A padded box counting its draws."""

    draws = 0

    def draw(self, renderer):
        type(self).draws += 1
        super().draw(renderer)


def _rounded(values, digits=3):
    """*values* as a list of floats rounded to *digits*."""
    return [round(float(v), digits) for v in np.ravel(values)]


def _pixels(fig):
    """The figure drawn, as an integer RGB array."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba())[..., :3].astype(int)


def _count(img, rgb):
    """How many pixels of *img* are the colour *rgb*."""
    return int((np.abs(img - np.array(rgb)).sum(axis=-1) == 0).sum())


def _dark(img):
    """How many pixels of *img* are dark (text, edges)."""
    return int((img.sum(axis=-1) < 200).sum())


#: tab:blue and tab:red as 8-bit RGB.
BLUE, RED = (31, 119, 180), (214, 39, 40)


def scenario_program_container():
    """A box of the program's holding a text area, a drawing area, an
    `HPacker`, a framed `PaddedBox` and an `OffsetImage`, anchored, and
    one added on its own at an offset."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    area = DrawingArea(30, 15)
    area.add_artist(Circle((7, 7), 7, fc="tab:blue"))
    text = TextArea("hello")
    packer = HPacker(children=[TextArea("a"), TextArea("bc")], sep=4)
    padded = PaddedBox(TextArea("pad"), pad=3, draw_frame=True)
    image = OffsetImage(np.zeros((3, 4)), zoom=2, cmap="gray")
    column = Column([text, area, packer, padded, image])
    anchored = AnchoredOffsetbox(loc="upper right", child=column)
    ax.add_artist(anchored)
    alone = Column([TextArea("alone"), TextArea("two")])
    alone.set_offset((30, 30))
    ax.add_artist(alone)
    img = _pixels(fig)
    renderer = fig.canvas.get_renderer()
    boxes = [text, area, packer, padded, image, column, alone]
    return {
        "bboxes": [_rounded(b.get_bbox(renderer).bounds) for b in boxes],
        "extents": [_rounded(b.get_window_extent(renderer).bounds) for b in boxes],
        "offsets": [_rounded(text.get_offset()), _rounded(area.get_offset()),
                    _rounded(packer.get_offset(packer.get_bbox(renderer), renderer))],
        "children": [type(c).__name__ for c in packer.get_visible_children()],
        "anchored": _rounded(anchored.get_window_extent(renderer).bounds),
        "pixels": (_count(img, BLUE), _count(img, (0, 0, 0)), _dark(img)),
    }


def test_program_container():
    assert scenario_program_container() == {
        "bboxes": [[0.0, -3.337, 35.0, 14.337], [-0.0, -0.0, 41.667, 20.833],
                   [0.0, -3.337, 31.556, 14.337], [-4.167, -7.503, 35.333, 22.67],
                   [0.0, 0.0, 11.111, 8.333], [0.0, 0.0, 41.667, 91.621],
                   [0.0, 0.0, 40.0, 31.451]],
        "extents": [[305.833, 237.163, 35.0, 14.337], [305.833, 213.552, 41.667, 20.833],
                    [305.833, 196.438, 31.556, 14.337], [305.833, 170.99, 35.333, 22.67],
                    [305.833, 159.879, 11.111, 8.333], [305.833, 159.879, 41.667, 91.621],
                    [30.0, 30.0, 40.0, 31.451]],
        "offsets": [[305.833, 240.5], [305.833, 213.552], [305.833, 199.775]],
        "children": ["TextArea", "TextArea"],
        "anchored": [300.278, 154.324, 52.778, 102.732],
        "pixels": (260, 1736, 2310),
    }


def scenario_boxes_in_trees():
    """The protocol of the boxes inside a tree pltrs (Rust) lays out: an
    anchored `PaddedBox` of a `VPacker`, a plain box at an offset and the
    box of an `AnnotationBbox`, measured once drawn."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    t1 = TextArea("hello")
    area = DrawingArea(30, 15)
    area.add_artist(Circle((7, 7), 7, fc="tab:blue"))
    ta, tb = TextArea("a"), TextArea("bc\nde", multilinebaseline=True)
    hp = HPacker(children=[ta, tb], sep=4, align="center")
    image = OffsetImage(np.zeros((3, 4)), zoom=2, cmap="gray")
    vp = VPacker(children=[t1, area, hp, image], sep=3, pad=2)
    padded = PaddedBox(vp, pad=4, draw_frame=True)
    anchored = AnchoredOffsetbox(loc="upper right", child=padded)
    ax.add_artist(anchored)
    placed = PaddedBox(TextArea("placed"), pad=2)
    placed.set_offset((20, 30))
    ax.add_artist(placed)
    annotation = AnnotationBbox(VPacker(children=[TextArea("ann")]), (0.3, 0.3))
    ax.add_artist(annotation)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    out = {}
    for name, box in [("t1", t1), ("area", area), ("ta", ta), ("tb", tb),
                      ("hp", hp), ("image", image), ("vp", vp),
                      ("padded", padded), ("anchored", anchored),
                      ("placed", placed), ("ann", annotation.offsetbox)]:
        try:
            offset = box.get_offset()
        except TypeError:
            offset = box.get_offset(box.get_bbox(renderer), renderer)
        out[name] = (_rounded(box.get_bbox(renderer).bounds),
                     _rounded(box.get_window_extent(renderer).bounds),
                     _rounded(offset), len(box.get_visible_children()))
    out["text"] = _rounded(t1._text.get_window_extent(renderer).bounds)
    return out


def test_boxes_in_trees():
    assert scenario_boxes_in_trees() == {
        "t1": ([0.0, -3.337, 35.0, 14.337], [297.5, 228.83, 35.0, 14.337],
               [297.5, 232.167], 1),
        "area": ([-0.0, -0.0, 41.667, 20.833], [297.5, 203.83, 41.667, 20.833],
                 [297.5, 203.83], 1),
        "ta": ([0.0, -3.337, 9.0, 14.337], [297.5, 175.378, 9.0, 14.337],
               [297.5, 178.715], 1),
        "tb": ([0.0, -11.617, 18.0, 34.234], [312.056, 165.429, 18.0, 34.234],
               [312.056, 177.046], 1),
        "hp": ([0.0, 0.0, 32.556, 34.234], [297.5, 165.429, 32.556, 34.234],
               [297.5, 165.429], 2),
        "image": ([0.0, 0.0, 11.111, 8.333], [297.5, 152.929, 11.111, 8.333],
                  [297.5, 152.929], 1),
        "vp": ([-2.778, -82.015, 47.222, 95.793], [294.722, 150.151, 47.222, 95.793],
               [297.5, 232.167], 4),
        "padded": ([-8.333, -87.571, 58.333, 106.904],
                   [289.167, 144.596, 58.333, 106.904], [297.5, 232.167], 1),
        "anchored": ([-13.889, -93.126, 69.444, 118.015],
                     [283.611, 139.04, 69.444, 118.015], [297.5, 232.167], 0),
        "placed": ([-2.778, -6.114, 53.556, 19.892], [17.222, 23.886, 53.556, 19.892],
                   [20.0, 30.0], 1),
        "ann": ([0.0, -3.337, 27.0, 14.337], [129.5, 95.132, 27.0, 14.337],
                [129.5, 98.468], 1),
        "text": [297.5, 228.83, 35.0, 14.337],
    }


def scenario_changed_after_joining():
    """An anchored `VPacker` changed once drawn: a text, a child's
    visibility, ``sep`` and ``pad`` (read back without a draw), a child
    appended to its children, ``align``, ``mode`` and
    ``multilinebaseline``; a plain `PaddedBox` given another ``pad``, its
    frame shown and its child replaced."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    t1, t2 = TextArea("hello"), TextArea("x")
    vp = VPacker(children=[t1, t2], sep=3, pad=2)
    anchored = AnchoredOffsetbox(loc="upper right", child=vp)
    ax.add_artist(anchored)
    renderer = fig.canvas.get_renderer()

    def extent():
        return _rounded(anchored.get_window_extent(renderer).bounds)

    fig.canvas.draw()
    out = {"before": extent()}
    t1.set_text("hello world, long")
    fig.canvas.draw()
    out["set_text"] = extent()
    t2.set_visible(False)
    fig.canvas.draw()
    out["hidden"] = extent()
    t2.set_visible(True)
    vp.sep = 20
    fig.canvas.draw()
    out["sep"] = extent()
    vp.pad = 10
    out["pad"] = extent()
    third = TextArea("third")
    third.set_figure(fig)  # As matplotlib's needs to be measured.
    vp.get_children().append(third)
    fig.canvas.draw()
    out["appended"] = extent()
    vp.align, vp.mode = "right", "equal"
    fig.canvas.draw()
    out["align"] = (extent(), _rounded(t2.get_window_extent(renderer).bounds))
    t2.set_multilinebaseline(True)
    t2.set_text("a\nb")
    fig.canvas.draw()
    out["multiline"] = extent()
    padded = PaddedBox(TextArea("p"), pad=3, draw_frame=False)
    padded.set_offset((10, 10))
    ax.add_artist(padded)
    fig.canvas.draw()
    out["padded"] = (_rounded(padded.get_window_extent(renderer).bounds),
                     padded.patch.get_visible())
    padded.pad = 8
    padded.patch.set_visible(True)
    img = _pixels(fig)
    out["framed"] = (_rounded(padded.get_window_extent(renderer).bounds),
                     _rounded(padded.patch.get_window_extent(renderer).bounds),
                     _dark(img))
    replaced = TextArea("replaced text")
    replaced.set_figure(fig)
    padded._children[0] = replaced
    fig.canvas.draw()
    out["replaced"] = _rounded(padded.get_window_extent(renderer).bounds)
    return out


def test_changed_after_joining():
    assert scenario_changed_after_joining() == {
        "before": [301.389, 207.549, 51.667, 49.507],
        "set_text": [219.389, 207.549, 133.667, 49.507],
        "hidden": [219.389, 226.052, 133.667, 31.003],
        "sep": [219.389, 183.938, 133.667, 73.118],
        "pad": [197.167, 161.716, 155.889, 95.34],
        "appended": [197.167, 119.601, 155.889, 137.454],
        "align": ([197.167, 91.824, 155.889, 165.232], [325.611, 181.16, 8.0, 14.337]),
        "multiline": [197.167, 33.474, 155.889, 223.582],
        "padded": ([5.833, 2.497, 17.333, 22.67], False),
        "framed": ([-1.111, -4.448, 31.222, 36.559], [-1.111, -4.448, 31.222, 36.559],
                   2543),
        "replaced": [-1.111, -4.448, 115.66, 36.559],
    }


def scenario_program_subclasses():
    """Subclasses of the package's boxes overriding ``get_bbox`` (a
    drawing area), ``draw`` (a text area, a padded box) and
    ``_get_bbox_and_child_offsets`` (a `VPacker`), in anchored and
    annotation boxes and on their own; nested boxes of the program's;
    a child appended and one hidden once drawn."""
    Counting.draws = Framed.draws = 0
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    big = BigArea(20, 10)
    big.add_artist(Rectangle((0, 0), 20, 10, fc="tab:blue"))
    after = TextArea("after")
    counted = Counting("counted")
    vp = VPacker(children=[big, after, counted], sep=2)
    ax.add_artist(AnchoredOffsetbox(loc="upper left", child=vp, frameon=False))
    left, right = TextArea("left"), DrawingArea(15, 15)
    right.add_artist(Circle((7.5, 7.5), 7.5, fc="tab:red"))
    row = Row(children=[left, right])
    ax.add_artist(AnnotationBbox(row, (0.6, 0.3), frameon=False))
    area = DrawingArea(20, 10)
    area.add_artist(Rectangle((0, 0), 20, 10, fc="tab:blue"))
    inner = Column([TextArea("in1"), area])
    outer = Column([TextArea("top"), inner])
    outer.set_offset((40, 40))
    ax.add_artist(outer)
    framed = Framed(TextArea("framed"), pad=3, draw_frame=True)
    framed.set_offset((200, 50))
    ax.add_artist(framed)
    img = _pixels(fig)
    renderer = fig.canvas.get_renderer()
    out = {
        "extents": [_rounded(b.get_window_extent(renderer).bounds)
                    for b in (big, after, counted, vp, left, right, row,
                              outer, inner, area, framed)],
        "pixels": (_count(img, BLUE), _count(img, RED), _dark(img)),
        "draws": (Counting.draws, Framed.draws),
    }
    added = TextArea("added")
    added.set_figure(fig)  # As matplotlib's needs to be measured.
    outer._children.append(added)
    img = _pixels(fig)
    out["added"] = (_rounded(added.get_window_extent(renderer).bounds),
                    _rounded(outer.get_window_extent(renderer).bounds), _dark(img))
    inner._children[0].set_visible(False)
    img = _pixels(fig)
    out["hidden"] = (_rounded(area.get_window_extent(renderer).bounds), _dark(img))
    out["draws"] += (Counting.draws, Framed.draws)
    return out


def test_program_subclasses():
    assert scenario_program_subclasses() == {
        "extents": [[62.5, 223.722, 27.778, 27.778], [62.5, 206.608, 33.75, 14.337],
                    [62.5, 189.493, 58.0, 14.337], [62.5, 189.493, 58.0, 62.007],
                    [211.708, 90.215, 22.75, 14.337], [239.458, 93.552, 20.833, 20.833],
                    [211.708, 90.215, 48.583, 24.17], [40.0, 40.0, 27.778, 48.118],
                    [40.0, 40.0, 27.778, 31.003], [40.0, 40.0, 27.778, 13.889],
                    [195.833, 42.497, 60.333, 22.67]],
        "pixels": (701, 305, 2104),
        "draws": (1, 1, 3, 3),
        "added": ([40.0, 40.0, 45.0, 14.337], [40.0, 40.0, 45.0, 65.232], 2174),
        "hidden": ([40.0, 57.114, 27.778, 13.889], 2155),
    }
