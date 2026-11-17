# AxesImage.get_cursor_data at edges, with extent and origin flips, and format_cursor_data for norms and colorbar-less images.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib import colors

fig, axs = plt.subplots(1, 3, figsize=(7, 2.5), dpi=80)
data = np.arange(20, dtype=float).reshape(4, 5) / 3
specs = [dict(), dict(origin="lower", extent=(0, 10, 0, 4)), dict(extent=(10, 0, 4, 0), norm=colors.LogNorm(0.1, 10))]
ims = [ax.imshow(data, **kw) for ax, kw in zip(axs, specs)]
fig.canvas.draw()
for ax, im in zip(axs, ims):
    x0, x1 = ax.get_xlim()
    y0, y1 = ax.get_ylim()
    for fx, fy in [(0.01, 0.01), (0.5, 0.5), (0.99, 0.99), (1.2, 0.5)]:
        xd, yd = x0 + fx * (x1 - x0), y0 + fy * (y1 - y0)
        px, py = ax.transData.transform((xd, yd))
        ev = MouseEvent("motion_notify_event", fig.canvas, px, py)
        v = im.get_cursor_data(ev)
        print(type(im.norm).__name__, (fx, fy), None if v is None else round(float(v), 4), im.format_cursor_data(v) if v is not None else None)
print(ims[0].format_cursor_data(1e-9), ims[0].format_cursor_data(123456.0), ims[2].format_cursor_data(5.0))
ims[1].set_clim(0, 1)
x, y = axs[1].transData.transform((5, 2))
ev = MouseEvent("motion_notify_event", fig.canvas, x, y)
print(round(float(ims[1].get_cursor_data(ev)), 4), ims[1].format_cursor_data(ims[1].get_cursor_data(ev)))
ims[1].set_norm(colors.BoundaryNorm([0, 2, 4, 7], 256))
print(ims[1].format_cursor_data(3.0), ims[1].format_cursor_data(6.9))
ims[1].set_array(np.ma.masked_greater(data, 3))
print(ims[1].get_cursor_data(ev), axs[1].format_coord(5, 2))
