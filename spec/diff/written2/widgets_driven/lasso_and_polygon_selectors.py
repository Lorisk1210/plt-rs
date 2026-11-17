# LassoSelector and PolygonSelector driven by event sequences, then used to select scatter points with Path.contains_points.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.path import Path
from matplotlib.widgets import LassoSelector, PolygonSelector

fig, (a1, a2) = plt.subplots(1, 2, figsize=(6, 3), dpi=100)
grid = np.array([(x, y) for x in np.linspace(0.5, 9.5, 10) for y in np.linspace(0.5, 9.5, 10)])
for a in (a1, a2):
    a.scatter(grid[:, 0], grid[:, 1], s=5)
    a.set_xlim(0, 10)
    a.set_ylim(0, 10)
fig.canvas.draw()
result = {}
lasso = LassoSelector(a1, lambda verts: result.setdefault("lasso", verts))
poly = PolygonSelector(a2, lambda verts: result.setdefault("poly", list(verts)))


def ev(name, ax, xy, button=1):
    x, y = ax.transData.transform(xy)
    MouseEvent(name, fig.canvas, x, y, button)._process()


loop = [(2, 2), (8, 2), (8, 8), (5, 9), (2, 8)]
ev("button_press_event", a1, loop[0])
for p in loop[1:]:
    ev("motion_notify_event", a1, p)
ev("button_release_event", a1, loop[-1])
for p in [(1, 1), (6, 1), (6, 6), (1, 1)]:
    ev("motion_notify_event", a2, p)
    ev("button_press_event", a2, p)
    ev("button_release_event", a2, p)
lv = np.round(result["lasso"], 2)
print(len(lv), lv[0].tolist(), lv[-1].tolist())
print(np.round(result["poly"], 2).tolist(), [list(map(float, v)) for v in np.round(poly.verts, 2)])
inside_lasso = Path(result["lasso"]).contains_points(grid)
inside_poly = Path(result["poly"]).contains_points(grid)
print(int(inside_lasso.sum()), int(inside_poly.sum()))
poly.verts = [(0, 0), (9, 0), (9, 9)]
print(np.round(poly.verts, 2).tolist())
poly.clear()
print(poly.verts)
fig.savefig("lasso_poly.png")
