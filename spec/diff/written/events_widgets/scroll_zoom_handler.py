# A scroll-to-zoom handler written by the user, exercised with synthetic scroll events.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent

fig, ax = plt.subplots(figsize=(5, 4))
ax.plot(np.linspace(0, 10, 100), np.cos(np.linspace(0, 10, 100)))


def zoom(event, base=1.5):
    if event.inaxes is None:
        return
    scale = 1 / base if event.button == "up" else base
    lo, hi = ax.get_xlim()
    ax.set_xlim(event.xdata - (event.xdata - lo) * scale, event.xdata + (hi - event.xdata) * scale)


fig.canvas.mpl_connect("scroll_event", zoom)
fig.canvas.draw()
px, py = ax.transData.transform((5, 0))
for step in ["up", "up", "down"]:
    MouseEvent("scroll_event", fig.canvas, px, py, button=step, step=1 if step == "up" else -1)._process()
print([round(v, 4) for v in ax.get_xlim()])
fig.savefig("scrolled.png")
