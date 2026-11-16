# SpanSelector selecting an x range: simulate a drag with press/motion/release events.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.widgets import SpanSelector
from matplotlib.backend_bases import MouseEvent

x = np.linspace(0, 10, 200)
fig, (ax, zoom) = plt.subplots(2, 1, figsize=(6, 5))
ax.plot(x, np.sin(x))
zline, = zoom.plot(x, np.sin(x))
chosen = []


def onselect(xmin, xmax):
    chosen.append((round(xmin, 2), round(xmax, 2)))
    zoom.set_xlim(xmin, xmax)


span = SpanSelector(ax, onselect, "horizontal", useblit=False, props=dict(alpha=0.3, facecolor="tab:blue"), interactive=True)
fig.canvas.draw()
(x0, y0), (x1, _) = ax.transData.transform([(2, 0), (5, 0)])
for name, px in [("button_press_event", x0), ("motion_notify_event", (x0 + x1) / 2), ("motion_notify_event", x1), ("button_release_event", x1)]:
    MouseEvent(name, fig.canvas, px, y0, button=1)._process()
print(chosen, [round(v, 2) for v in span.extents])
fig.savefig("span.png")
