# Points inside a hand-drawn polygon (Path.contains_points), coloured and counted, with the polygon outlined.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.path import Path
from matplotlib.patches import PathPatch

rng = np.random.default_rng(104)
pts = rng.random((300, 2))
poly = Path([(0.2, 0.2), (0.7, 0.15), (0.85, 0.6), (0.5, 0.9), (0.15, 0.6), (0.2, 0.2)], closed=True)
inside = poly.contains_points(pts)
fig, ax = plt.subplots(figsize=(5, 5))
ax.scatter(pts[~inside, 0], pts[~inside, 1], s=8, c="0.7")
ax.scatter(pts[inside, 0], pts[inside, 1], s=12, c="C3")
ax.add_patch(PathPatch(poly, fill=False, ec="k", lw=2, ls="--"))
ax.set_aspect("equal")
ax.set_title(f"{inside.sum()} selected")
print(int(inside.sum()), poly.contains_point((0.5, 0.5)), [round(v, 3) for v in poly.get_extents().bounds])
fig.savefig("lasso.png", dpi=50)
