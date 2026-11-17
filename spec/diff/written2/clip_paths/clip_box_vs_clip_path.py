# Combine set_clip_box with a TransformedBbox and set_clip_path on the same artists, including text and collections.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.transforms import Bbox, TransformedBbox
from matplotlib.patches import Ellipse

fig, ax = plt.subplots(figsize=(5, 4))
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
box = TransformedBbox(Bbox([[2, 2], [8, 8]]), ax.transData)
sc = ax.scatter(np.linspace(0, 10, 40), np.linspace(10, 0, 40) % 7, s=80, c=np.arange(40))
sc.set_clip_box(box)
txt = ax.text(1, 5, "clipped text " * 4, fontsize=14, clip_on=True)
txt.set_clip_box(box)
ell = Ellipse((5, 5), 8, 4, angle=30, transform=ax.transData, fc="none")
ax.add_patch(ell)
bars = ax.bar(np.arange(1, 10), np.linspace(1, 9, 9), alpha=0.4)
for b in bars:
    b.set_clip_path(ell)
fig.canvas.draw()
print([round(v, 2) for v in sc.get_clip_box().bounds], txt.get_clip_on())
print(type(bars[0].get_clip_path()).__name__, bars[0].get_clip_box() is ax.bbox)
gc_like = sc.get_clip_box()
print(gc_like.contains(*ax.transData.transform((5, 5))), gc_like.contains(*ax.transData.transform((1, 1))))
fig.savefig("clip_box.png")
ax.set_xlim(0, 20)
fig.canvas.draw()
print([round(v, 2) for v in sc.get_clip_box().bounds])
sc.set_clip_box(None)
print(sc.get_clip_box())
fig.savefig("clip_box2.png")
w = txt.get_window_extent()
print(round(w.width, 1) > 0, round(w.height, 1))
