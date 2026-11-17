# Collections with offsets in data coordinates and paths in pixels/points/axes coordinates; how markers stay sized on zoom.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.collections import PathCollection, RegularPolyCollection, StarPolygonCollection, AsteriskPolygonCollection
from matplotlib.path import Path
from matplotlib.transforms import Affine2D, IdentityTransform

fig, ax = plt.subplots(figsize=(4, 4), dpi=72)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
offs = np.array([[2, 2], [5, 5], [8, 8]])
pathcoll = PathCollection([Path.unit_rectangle()], offsets=offs, offset_transform=ax.transData,
                          transform=Affine2D().scale(10))
ax.add_collection(pathcoll)
reg = RegularPolyCollection(6, sizes=[200], offsets=offs + [0, 1], offset_transform=ax.transData)
star = StarPolygonCollection(5, sizes=[150], offsets=offs + [1, 0], offset_transform=ax.transData)
ast = AsteriskPolygonCollection(4, sizes=[100], offsets=offs - [1, 0], offset_transform=ax.transData)
for c in (reg, star, ast):
    ax.add_collection(c)
r = fig.canvas.get_renderer()
for lim in (10, 5):
    ax.set_xlim(0, lim)
    ax.set_ylim(0, lim)
    fig.canvas.draw()
    for c in (pathcoll, reg, star, ast):
        print(lim, type(c).__name__, [round(v, 1) for v in c.get_window_extent(r).bounds])
print(reg.get_numsides(), reg.get_rotation(), star.get_numsides())
print(type(pathcoll.get_offset_transform()).__name__, type(pathcoll.get_transform()).__name__)
pathcoll.set_offset_transform(ax.transAxes)
pathcoll.set_offsets([[0.5, 0.5]])
fig.canvas.draw()
print([round(v, 1) for v in pathcoll.get_window_extent(r).bounds])
pathcoll.set_transform(IdentityTransform())
fig.canvas.draw()
print([round(v, 1) for v in pathcoll.get_window_extent(r).bounds])
fig.savefig("offsets.png")
