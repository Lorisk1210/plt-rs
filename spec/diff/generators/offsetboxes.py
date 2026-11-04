"""Offset boxes as matplotlib's own algorithms use them, and what Axes clip
their artists to.

* ``program_boxes``: boxes of the program's (an ``OffsetBox`` subclass
  with matplotlib's ``_get_bbox_and_child_offsets``, subclasses of the
  package's boxes overriding ``get_bbox``, ``draw`` or
  ``_get_bbox_and_child_offsets``) holding the package's boxes, anchored,
  in annotation boxes and on their own, measured once drawn;
* ``changed_boxes``: the boxes of added boxes changed once drawn (packer
  settings, visibility, texts, children lists, padding, frames, zoom),
  measured after the next draw;
* ``patch_clipping``: the clip path and box of each kind of artist, the
  axis and its gridlines in Axes whose patch is a wedge, a circle, a
  rectangle of the program's or the unit square, and what clearing them
  leaves.
"""

from __future__ import annotations

from typing import Iterator

from . import common

#: The classes of the program's the box programs define.
CLASSES = [
    "from matplotlib.offsetbox import (AnchoredOffsetbox, AnnotationBbox, DrawingArea, HPacker, "
    "OffsetBox, OffsetImage, PaddedBox, TextArea, VPacker)",
    "from matplotlib.transforms import Bbox",
    """class Column(OffsetBox):
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
        return Bbox.from_bounds(0, 0, max(bb.width for bb in bboxes), y - sep), offsets""",
    """class BigArea(DrawingArea):
    def get_bbox(self, renderer):
        bb = super().get_bbox(renderer)
        return Bbox.from_bounds(bb.x0, bb.y0 - bb.height, bb.width, 2 * bb.height)""",
    """class Counting(TextArea):
    draws = 0
    def draw(self, renderer):
        type(self).draws += 1
        super().draw(renderer)""",
    """class Row(VPacker):
    def _get_bbox_and_child_offsets(self, renderer):
        bboxes = [c.get_bbox(renderer) for c in self.get_visible_children()]
        x, offsets = 0, []
        for bb in bboxes:
            offsets.append((x - bb.x0, 0))
            x += bb.width + 5
        y0, y1 = min(bb.y0 for bb in bboxes), max(bb.y1 for bb in bboxes)
        return Bbox.from_bounds(0, y0, x - 5, y1 - y0), offsets""",
]


def _program_boxes() -> Iterator[str]:
    yield "fig, ax = plt.subplots(figsize=(4, 3), dpi=100)"
    yield "area = DrawingArea(30, 15)"
    yield "area.add_artist(patches.Circle((7, 7), 7, fc='tab:blue'))"
    yield "text = TextArea('hello')"
    yield "packer = HPacker(children=[TextArea('a'), TextArea('bc')], sep=4)"
    yield "padded = PaddedBox(TextArea('pad'), pad=3, draw_frame=True)"
    yield "image = OffsetImage(z, zoom=2, cmap='gray')"
    yield "column = Column([text, area, packer, padded, image])"
    yield "anchored = AnchoredOffsetbox(loc='upper right', child=column)"
    yield "ax.add_artist(anchored)"
    yield "big = BigArea(20, 10)"
    yield "big.add_artist(patches.Rectangle((0, 0), 20, 10, fc='tab:red'))"
    yield "counted = Counting('counted')"
    yield "ax.add_artist(AnchoredOffsetbox(loc='upper left', child=VPacker(children=[big, counted]), frameon=False))"
    yield "row = Row(children=[TextArea('left'), TextArea('right')])"
    yield "ax.add_artist(AnnotationBbox(row, (0.5, 0.3)))"
    yield "alone = Column([TextArea('alone'), Column([TextArea('nested'), DrawingArea(10, 5)])])"
    yield "alone.set_offset((30, 30))"
    yield "ax.add_artist(alone)"
    yield "fig.canvas.draw()"
    yield "renderer = fig.canvas.get_renderer()"
    for box in ["text", "area", "packer", "padded", "image", "column", "big", "counted", "row", "alone",
                "alone.get_children()[1]"]:
        yield f"{box}.get_bbox(renderer).bounds"
        yield f"{box}.get_window_extent(renderer).bounds"
    yield "text.get_offset(), area.get_offset(), image.get_offset()"
    yield "packer.get_offset(packer.get_bbox(renderer), renderer)"
    yield "[type(c).__name__ for c in packer.get_visible_children()]"
    yield "anchored.get_window_extent(renderer).bounds"
    yield "Counting.draws"


def _changed_boxes() -> Iterator[str]:
    yield "fig, ax = plt.subplots(figsize=(4, 3), dpi=100)"
    yield "t1, t2 = TextArea('hello'), TextArea('x')"
    yield "vp = VPacker(children=[t1, t2], sep=3, pad=2)"
    yield "anchored = AnchoredOffsetbox(loc='upper right', child=vp)"
    yield "ax.add_artist(anchored)"
    yield "renderer = fig.canvas.get_renderer()"
    yield "fig.canvas.draw()"
    yield "anchored.get_window_extent(renderer).bounds"
    changes = ["t1.set_text('hello world, long')", "t2.set_visible(False)", "t2.set_visible(True)",
               "vp.sep = 20", "vp.pad = 10", "vp.align = 'right'", "vp.mode = 'equal'",
               "vp.width = 150", "t2.set_multilinebaseline(True)", "t2.set_text('a\\nb')"]
    for change in changes:
        yield change
        yield "anchored.get_window_extent(renderer).bounds"
        yield "fig.canvas.draw()"
        yield "anchored.get_window_extent(renderer).bounds, t2.get_window_extent(renderer).bounds"
    yield "third = TextArea('third')"
    yield "third.set_figure(fig)"
    yield "vp.get_children().append(third)"
    yield "fig.canvas.draw()"
    yield "anchored.get_window_extent(renderer).bounds, third.get_window_extent(renderer).bounds"
    yield "padded = PaddedBox(TextArea('p'), pad=3, draw_frame=False)"
    yield "padded.set_offset((10, 10))"
    yield "ax.add_artist(padded)"
    yield "fig.canvas.draw()"
    yield "padded.get_window_extent(renderer).bounds, padded.patch.get_visible()"
    yield "padded.pad = 8"
    yield "padded.patch.set_visible(True)"
    yield "fig.canvas.draw()"
    yield "padded.get_window_extent(renderer).bounds, padded.patch.get_window_extent(renderer).bounds"
    yield "replaced = TextArea('replaced text')"
    yield "replaced.set_figure(fig)"
    yield "padded._children[0] = replaced"
    yield "fig.canvas.draw()"
    yield "padded.get_window_extent(renderer).bounds"
    yield "image = OffsetImage(z, zoom=1, cmap='gray')"
    yield "area = DrawingArea(10, 10)"
    yield "ax.add_artist(AnnotationBbox(HPacker(children=[image, area]), (0.4, 0.6)))"
    yield "fig.canvas.draw()"
    yield "image.get_window_extent(renderer).bounds, area.get_window_extent(renderer).bounds"
    yield "image.set_zoom(3)"
    yield "area.width = 30"
    yield "fig.canvas.draw()"
    yield "image.get_window_extent(renderer).bounds, area.get_window_extent(renderer).bounds"


def _patch_clipping() -> Iterator[str]:
    yield "from matplotlib.axes import Axes"
    yield "from matplotlib.projections import register_projection"
    yield ("register_projection(type('round_diff', (Axes,), {'name': 'round_diff', "
           "'_gen_axes_patch': lambda self: patches.Circle((0.5, 0.5), 0.5)}))")
    yield ("register_projection(type('inset_diff', (Axes,), {'name': 'inset_diff', "
           "'_gen_axes_patch': lambda self: patches.Rectangle((0.1, 0.1), 0.8, 0.8)}))")
    yield """def clipping(artist, ax):
    path, box = artist.get_clip_path(), artist.get_clip_box()
    return (None if path is None else (type(path).__name__, getattr(path, '_patch', None) is ax.patch),
            None if box is None else (type(box).__name__, box.bounds))"""
    yield "fig = plt.figure(figsize=(6, 4), dpi=50)"
    for index, projection in enumerate(["'polar'", "'mollweide'", "'round_diff'", "'inset_diff'", "None"]):
        yield f"ax = fig.add_subplot(2, 3, {index + 1}, projection={projection})"
        yield "line, = ax.plot([0, 1], [0, 1])"
        yield ("made = [line, ax.scatter([0, 1], [0, 1]), ax.add_patch(patches.Rectangle((0, 0), 0.5, 0.5)), "
               "ax.text(0.1, 0.1, 't'), ax.annotate('a', (0.2, 0.2)), "
               "ax.add_collection(collections.LineCollection([[(0, 0), (1, 1)]])), "
               "ax.add_artist(lines.Line2D([0, 1], [1, 0])), ax.fill_between([0, 1], [0, 1])]")
        yield "fig.canvas.draw()"
        yield "[clipping(artist, ax) for artist in made]"
        yield "tick = ax.xaxis.get_major_ticks()[0]"
        yield "[clipping(part, ax) for part in (tick.gridline, tick.tick1line, tick.label1, ax.xaxis, ax.title)]"
        yield "type(ax.patch).__name__"
        yield "line.get_clip_path() is line.get_clip_path()"
        yield "line.set_clip_path(None)"
        yield "clipping(line, ax)"
        yield "line.set_clip_box(None)"
        yield "clipping(line, ax)"


AREAS = {
    "program_boxes": (CLASSES, _program_boxes),
    "changed_boxes": (CLASSES[:1], _changed_boxes),
    "patch_clipping": ([], _patch_clipping),
}


def programs() -> Iterator[tuple[str, list[str]]]:
    """One program per area."""
    for name, (prelude, statements) in AREAS.items():
        yield name, common.HEADER + common.DATA + prelude + list(statements())
