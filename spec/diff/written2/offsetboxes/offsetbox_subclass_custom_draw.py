# Subclass OffsetBox: a badge that draws a round box and a label, sized from the text; used in an AnchoredOffsetbox and alone.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.offsetbox import OffsetBox, TextArea, AnchoredOffsetbox
from matplotlib.patches import FancyBboxPatch
from matplotlib.transforms import Bbox


class Badge(OffsetBox):
    def __init__(self, label, color="C0", pad=4):
        super().__init__()
        self._text = TextArea(label, textprops=dict(color="w", weight="bold"))
        self._children = [self._text]
        self._color = color
        self._pad = pad
        self._patch = FancyBboxPatch((0, 0), 1, 1, boxstyle="round,pad=0", fc=color, ec="none")

    def get_bbox(self, renderer):
        tb = self._text.get_bbox(renderer)
        return Bbox.from_bounds(tb.x0 - self._pad, tb.y0 - self._pad, tb.width + 2 * self._pad, tb.height + 2 * self._pad)

    def get_offset(self, bbox, renderer):
        return super().get_offset(bbox, renderer)

    def draw(self, renderer):
        bbox = self.get_bbox(renderer)
        ox, oy = self.get_offset(bbox, renderer)
        self._patch.set_bounds(ox + bbox.x0, oy + bbox.y0, bbox.width, bbox.height)
        self._patch.set_figure(self.get_figure(root=False))
        self._patch.draw(renderer)
        self._text.set_offset((ox, oy))
        self._text.draw(renderer)
        self.stale = False


fig, ax = plt.subplots(figsize=(4, 3))
badges = [AnchoredOffsetbox(loc=loc, child=Badge(text, color), frameon=False)
          for loc, text, color in [("upper left", "NEW", "C3"), ("lower right", "beta 2", "C0"), ("center", "ok", "C2")]]
for b in badges:
    ax.add_artist(b)
fig.canvas.draw()
r = fig.canvas.get_renderer()
for b in badges:
    child = b.get_child()
    print(type(child).__name__, [round(v, 1) for v in child.get_bbox(r).bounds],
          [round(v, 1) for v in b.get_window_extent(r).bounds])
print([round(v, 1) for v in badges[0].get_child()._patch.get_window_extent(r).bounds])
fig.savefig("badges.png")
