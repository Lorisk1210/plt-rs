# Pan and zoom an axes by hand through start_pan/drag_pan/end_pan and _set_view_from_bbox, as toolbars do.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 4), dpi=100)
ax.plot(np.linspace(0, 10, 50), np.linspace(0, 10, 50) ** 0.5)
ax.set_xlim(0, 10)
ax.set_ylim(0, 4)
fig.canvas.draw()


def lims():
    return [round(v, 3) for v in (*ax.get_xlim(), *ax.get_ylim())]


x0, y0 = ax.transData.transform((5, 2))
ax.start_pan(x0, y0, 1)
ax.drag_pan(1, None, x0 + 40, y0 - 20)
ax.end_pan()
print("pan", lims())
ax.start_pan(x0, y0, 3)
ax.drag_pan(3, None, x0 + 30, y0 + 30)
ax.end_pan()
print("zoom-drag", lims())
ax.start_pan(x0, y0, 1)
ax.drag_pan(1, "x", x0 + 50, y0 + 50)
ax.end_pan()
print("pan x only", lims())
bx0, by0 = ax.transData.transform((2, 1))
bx1, by1 = ax.transData.transform((6, 3))
ax._set_view_from_bbox((bx0, by0, bx1, by1), direction="in")
print("zoom in", lims())
ax._set_view_from_bbox((bx0, by0, bx1, by1), direction="out")
print("zoom out", lims())
ax.set_xscale("log")
ax.set_xlim(0.1, 100)
fig.canvas.draw()
x0, y0 = ax.transData.transform((1, 2))
ax.start_pan(x0, y0, 1)
ax.drag_pan(1, None, x0 + 50, y0)
ax.end_pan()
print("log pan", lims())
print(ax.can_pan(), ax.can_zoom(), ax.get_navigate(), ax.get_navigate_mode())
fig.savefig("panzoom.png")
