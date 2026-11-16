# Read annotation positions (xy and text position in data units) back to draw leader lines manually in another axes.
import numpy as np
import matplotlib.pyplot as plt

fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4))
for k in range(3):
    a.annotate(f"p{k}", xy=(k, k ** 2), xytext=(k + 0.5, k ** 2 + 2), arrowprops=dict(arrowstyle="->"))
a.set_xlim(-1, 4); a.set_ylim(-1, 8)
fig.canvas.draw()
for ann in a.texts:
    xy = ann.xy
    txy = a.transData.inverted().transform(ann.get_window_extent().get_points().mean(axis=0))
    b.plot([xy[0], txy[0]], [xy[1], txy[1]], "k-")
    b.plot(*xy, "ro")
b.set_xlim(-1, 4); b.set_ylim(-1, 8)
print([a_.xy for a_ in a.texts], [a_.get_position() for a_ in a.texts])
fig.savefig("leaders.png", dpi=50)
