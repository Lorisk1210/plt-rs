# Register a box style, an arrow style and a connection style of the program's own and use them by name in annotations.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import BoxStyle, ArrowStyle, ConnectionStyle
from matplotlib.path import Path


class Tag:
    def __init__(self, pad=0.3):
        self.pad = pad

    def __call__(self, x0, y0, width, height, mutation_size):
        pad = mutation_size * self.pad
        x0, y0, x1, y1 = x0 - pad, y0 - pad, x0 + width + pad, y0 + height + pad
        tip = x1 + (y1 - y0) / 2
        return Path([(x0, y0), (x1, y0), (tip, (y0 + y1) / 2), (x1, y1), (x0, y1), (x0, y0)], closed=True)


BoxStyle._style_list["tag"] = Tag


class Elbow(ConnectionStyle._Base):
    def connect(self, posA, posB):
        (xa, ya), (xb, yb) = posA, posB
        return Path([(xa, ya), (xa, yb), (xb, yb)], [Path.MOVETO, Path.LINETO, Path.LINETO])


ConnectionStyle._style_list["elbow"] = Elbow

fig, ax = plt.subplots(figsize=(5, 3))
ax.set_xlim(0, 10)
ax.set_ylim(0, 6)
t = ax.text(2, 4, "tagged", bbox=dict(boxstyle="tag,pad=0.4", fc="khaki"))
ann = ax.annotate("elbowed", xy=(8, 1), xytext=(2, 2), arrowprops=dict(arrowstyle="-|>", connectionstyle="elbow"))
fig.canvas.draw()
print(type(t.get_bbox_patch().get_boxstyle()).__name__, round(t.get_bbox_patch().get_boxstyle().pad, 3))
print(type(ann.arrow_patch.get_connectionstyle()).__name__)
print("tag" in BoxStyle.get_styles(), "elbow" in ConnectionStyle.get_styles())
print([round(v, 2) for v in t.get_bbox_patch().get_window_extent().bounds])
fig.savefig("styles.png")
