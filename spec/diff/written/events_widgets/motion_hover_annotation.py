# Hover tooltip: motion_notify handler shows an annotation near the closest scatter point.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent

rng = np.random.default_rng(3)
pts = rng.uniform(0, 1, (20, 2))
names = [f"p{i}" for i in range(20)]
fig, ax = plt.subplots(figsize=(5, 4))
sc = ax.scatter(pts[:, 0], pts[:, 1])
tip = ax.annotate("", xy=(0, 0), xytext=(10, 10), textcoords="offset points", bbox=dict(fc="w"))
tip.set_visible(False)


def hover(event):
    if event.inaxes is not ax:
        return
    hit, info = sc.contains(event)
    tip.set_visible(hit)
    if hit:
        i = info["ind"][0]
        tip.xy = pts[i]
        tip.set_text(names[i])


fig.canvas.mpl_connect("motion_notify_event", hover)
fig.canvas.draw()
px, py = ax.transData.transform(pts[7])
MouseEvent("motion_notify_event", fig.canvas, px, py)._process()
print(tip.get_visible(), tip.get_text())
fig.savefig("hover.png")
