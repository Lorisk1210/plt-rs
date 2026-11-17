# AuxTransformBox holding a patch and a text in data units, rotated and scaled; extents follow the axes' limits.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.offsetbox import AuxTransformBox, AnchoredOffsetbox
from matplotlib.patches import Ellipse, Rectangle
from matplotlib.text import Text
from matplotlib.transforms import Affine2D

fig, ax = plt.subplots(figsize=(5, 4))
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
box = AuxTransformBox(ax.transData)
box.add_artist(Ellipse((0, 0), 2, 1, fc="C0", alpha=0.6))
box.add_artist(Rectangle((-1, -0.25), 2, 0.5, fc="none", ec="k"))
box.add_artist(Text(0, 0, "scale", ha="center", va="center"))
anch = AnchoredOffsetbox(loc="upper left", child=box, frameon=True, pad=0.3)
ax.add_artist(anch)
fig.canvas.draw()
r = fig.canvas.get_renderer()
print([round(v, 2) for v in box.get_bbox(r).bounds], [round(v, 2) for v in anch.get_window_extent(r).bounds])
ax.set_xlim(0, 5)
fig.canvas.draw()
print([round(v, 2) for v in box.get_bbox(r).bounds], [round(v, 2) for v in anch.get_window_extent(r).bounds])
box.aux_transform = Affine2D().rotate_deg(30) + ax.transData
fig.canvas.draw()
print([round(v, 2) for v in box.get_bbox(r).bounds])
box2 = AuxTransformBox(Affine2D().scale(20))
box2.add_artist(Rectangle((0, 0), 2, 1, fc="C1"))
anch2 = AnchoredOffsetbox(loc="lower right", child=box2, frameon=False)
ax.add_artist(anch2)
fig.canvas.draw()
print([round(v, 2) for v in box2.get_bbox(r).bounds], [round(v, 2) for v in anch2.get_window_extent(r).bounds])
print(len(box.get_children()), [type(c).__name__ for c in box.get_children()])
fig.savefig("auxbox.png")
