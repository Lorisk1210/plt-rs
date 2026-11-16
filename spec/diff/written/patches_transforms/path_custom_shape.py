# Custom shape from Path codes (MOVETO, CURVE4, CLOSEPOLY) drawn with PathPatch, a heart-like curve.
import matplotlib.pyplot as plt
from matplotlib.path import Path
from matplotlib.patches import PathPatch

verts = [(0, -1), (-1.5, 0.3), (-0.8, 1.4), (0, 0.6), (0.8, 1.4), (1.5, 0.3), (0, -1), (0, -1)]
codes = [Path.MOVETO, Path.CURVE4, Path.CURVE4, Path.CURVE4, Path.CURVE4, Path.CURVE4, Path.CURVE4, Path.CLOSEPOLY]
path = Path(verts, codes)
fig, ax = plt.subplots(figsize=(4, 4))
ax.add_patch(PathPatch(path, fc="crimson", ec="darkred", lw=2))
xs, ys = zip(*verts)
ax.plot(xs, ys, "o--", color="gray", ms=4, lw=0.5)
ax.set_xlim(-2, 2)
ax.set_ylim(-1.5, 2)
ax.set_aspect("equal")
print(path.get_extents().bounds)
print(path.contains_point((0, 0.2)))
fig.savefig("heart.png")
