# contains() on LineCollection, PathCollection and PolyCollection with pickradius; indices returned for points on edges and inside.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.collections import LineCollection, PolyCollection

fig, ax = plt.subplots(figsize=(4, 4), dpi=100)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
lc = LineCollection([[(1, 1), (9, 1)], [(1, 3), (9, 3)], [(5, 0), (5, 10)]], pickradius=4)
sc = ax.scatter([2, 4, 6], [6, 6, 6], s=100)
pc = PolyCollection([[(1, 7), (3, 7), (3, 9), (1, 9)], [(6, 7), (9, 7), (9, 9)]])
for c in (lc, pc):
    ax.add_collection(c)
fig.canvas.draw()


def at(x, y):
    px, py = ax.transData.transform((x, y))
    return MouseEvent("motion_notify_event", fig.canvas, px, py)


for pt in [(5, 1), (5, 3), (5.05, 2), (2, 6), (4.1, 6.1), (2, 8), (8, 7.5), (5, 5)]:
    ev = at(*pt)
    out = []
    for c in (lc, sc, pc):
        hit, info = c.contains(ev)
        out.append((hit, info.get("ind", np.array([])).tolist()))
    print(pt, out)
lc.set_pickradius(20)
print(lc.contains(at(5.4, 2))[1]["ind"].tolist())
sc.set_pickradius(0)
print(sc.contains(at(2.2, 6))[0])
