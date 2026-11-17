# AnchoredOffsetbox with bbox_to_anchor given as 2-tuple, 4-tuple, Bbox and in figure / data transforms; positions read back.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.offsetbox import AnchoredText
from matplotlib.transforms import Bbox

fig, ax = plt.subplots(figsize=(5, 4))
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
specs = [
    ("tuple2", dict(bbox_to_anchor=(1.0, 1.0), bbox_transform=ax.transAxes, loc="lower right")),
    ("tuple4", dict(bbox_to_anchor=(0.1, 0.1, 0.4, 0.3), bbox_transform=ax.transAxes, loc="upper left")),
    ("bbox", dict(bbox_to_anchor=Bbox.from_bounds(2, 6, 3, 2), bbox_transform=ax.transData, loc="center")),
    ("figure", dict(bbox_to_anchor=(0.5, 0.02), bbox_transform=fig.transFigure, loc="lower center")),
    ("default", dict(loc="upper right")),
]
boxes = {}
for name, kw in specs:
    at = AnchoredText(name, prop=dict(size=8), **kw)
    ax.add_artist(at)
    boxes[name] = at
fig.canvas.draw()
r = fig.canvas.get_renderer()
for name, at in boxes.items():
    print(name, [round(v, 1) for v in at.get_window_extent(r).bounds],
          [round(v, 1) for v in at.get_bbox_to_anchor().bounds])
ax.set_xlim(0, 20)
fig.canvas.draw()
print([round(v, 1) for v in boxes["bbox"].get_window_extent(r).bounds])
boxes["default"].set_bbox_to_anchor((0, 0, 1, 1), transform=fig.transFigure)
fig.canvas.draw()
print([round(v, 1) for v in boxes["default"].get_window_extent(r).bounds])
fig.savefig("bbox_to_anchor.png")
