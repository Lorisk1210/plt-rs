# RectangleSelector on a scatter selects points (simulated drag); selected points coloured and shown in a second panel.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.widgets import RectangleSelector
from matplotlib.backend_bases import MouseEvent

rng = np.random.default_rng(101)
pts = rng.random((200, 2))
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4.5))
sc = a.scatter(pts[:, 0], pts[:, 1], s=10, c="0.6")
chosen = {}
def onselect(e1, e2):
    x0, x1 = sorted((e1.xdata, e2.xdata)); y0, y1 = sorted((e1.ydata, e2.ydata))
    inside = (pts[:, 0] >= x0) & (pts[:, 0] <= x1) & (pts[:, 1] >= y0) & (pts[:, 1] <= y1)
    sc.set_facecolors(np.where(inside[:, None], [[1, 0, 0, 1]], [[0.6, 0.6, 0.6, 1]]))
    b.cla()
    b.scatter(pts[inside, 0], pts[inside, 1], c="red", s=20)
    chosen["n"] = int(inside.sum())
sel = RectangleSelector(a, onselect, useblit=False, interactive=False)
fig.canvas.draw()
(px0, py0), (px1, py1) = a.transData.transform([(0.2, 0.2), (0.6, 0.7)])
MouseEvent("button_press_event", fig.canvas, px0, py0, 1)._process()
MouseEvent("motion_notify_event", fig.canvas, px1, py1, 1)._process()
MouseEvent("button_release_event", fig.canvas, px1, py1, 1)._process()
print(chosen, [round(v, 3) for v in b.get_xlim()])
fig.savefig("rect_select.png", dpi=50)
