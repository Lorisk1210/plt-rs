# get_cursor_data and format_cursor_data for images, pcolormesh, scatter and lines at synthetic mouse positions; mouseover flags.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent

fig, axs = plt.subplots(1, 3, figsize=(8, 3), dpi=80)
im = axs[0].imshow(np.arange(12).reshape(3, 4) * 0.1234, extent=(0, 4, 0, 3), cmap="viridis")
qm = axs[1].pcolormesh(np.arange(5), np.arange(4), np.arange(12).reshape(3, 4) ** 2)
rgb = axs[2].imshow(np.dstack([np.eye(3), np.zeros((3, 3)), np.ones((3, 3)) * 0.5]))
fig.canvas.draw()
for art, ax, pts in [(im, axs[0], [(0.5, 0.5), (3.5, 2.5), (5, 5)]), (qm, axs[1], [(0.5, 0.5), (3.2, 2.7)]),
                     (rgb, axs[2], [(0, 0), (2, 2)])]:
    for xd, yd in pts:
        x, y = ax.transData.transform((xd, yd))
        ev = MouseEvent("motion_notify_event", fig.canvas, x, y)
        data = art.get_cursor_data(ev)
        shown = art.format_cursor_data(data) if data is not None else None
        print(type(art).__name__, (xd, yd), None if data is None else np.round(np.atleast_1d(data), 4).tolist(), shown)
print(im.mouseover, qm.mouseover, axs[0].lines == [])
ln, = axs[0].plot([0, 4], [0, 3])
print(ln.mouseover, ln.get_cursor_data(None))
ln.set_mouseover(True)
print(ln.mouseover, ln in axs[0]._mouseover_set)
print(im.format_cursor_data(1.23456789), im.format_cursor_data(np.ma.masked), im.format_cursor_data(None))
im.set_norm(matplotlib.colors.BoundaryNorm([0, 0.5, 1, 1.5], 256))
print(im.format_cursor_data(0.7))
print(axs[0].format_coord(1.5, 2.5), axs[0].format_xdata(1.23456), axs[0].format_ydata(2))
fig.savefig("cursor.png")
