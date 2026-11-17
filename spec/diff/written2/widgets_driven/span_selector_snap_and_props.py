# SpanSelector horizontal and vertical with snap_values, minspan, onmove_callback and props edits, driven by drags.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.widgets import SpanSelector

fig, (ah, av) = plt.subplots(1, 2, figsize=(6, 3), dpi=100)
for a in (ah, av):
    a.set_xlim(0, 10)
    a.set_ylim(0, 10)
fig.canvas.draw()
log = []
h = SpanSelector(ah, lambda lo, hi: log.append(("h", round(lo, 2), round(hi, 2))), "horizontal",
                 snap_values=np.arange(0, 10.5, 0.5), interactive=True, minspan=0.4,
                 onmove_callback=lambda lo, hi: log.append(("move", round(lo, 2), round(hi, 2))),
                 props=dict(facecolor="C1", alpha=0.3))
v = SpanSelector(av, lambda lo, hi: log.append(("v", round(lo, 2), round(hi, 2))), "vertical", interactive=True)


def drag(ax, p0, p1):
    x0, y0 = ax.transData.transform(p0)
    x1, y1 = ax.transData.transform(p1)
    MouseEvent("button_press_event", fig.canvas, x0, y0, 1)._process()
    MouseEvent("motion_notify_event", fig.canvas, (x0 + x1) / 2, (y0 + y1) / 2, 1)._process()
    MouseEvent("motion_notify_event", fig.canvas, x1, y1, 1)._process()
    MouseEvent("button_release_event", fig.canvas, x1, y1, 1)._process()


drag(ah, (1.13, 5), (4.71, 5))
drag(av, (5, 2.2), (5, 7.9))
drag(ah, (6, 5), (6.2, 5))
print(log)
print([round(x, 2) for x in h.extents], [round(x, 2) for x in v.extents], h.direction, v.direction)
h.extents = (3, 8)
print([round(x, 2) for x in h.extents])
h.direction = "vertical"
print(h.direction, [round(x, 2) for x in h.extents])
print(type(h._selection_artist).__name__, h._selection_artist.get_facecolor())
fig.savefig("span.png")
