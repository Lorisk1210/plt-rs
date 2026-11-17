# Compound paths: make_compound_path of shapes with holes, its contains_point under both winding rules via a patch, and to_polygons.
import numpy as np
from matplotlib.path import Path
from matplotlib.patches import PathPatch
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

outer = Path.unit_rectangle().transformed(matplotlib.transforms.Affine2D().scale(4))
hole = Path.circle((2, 2), 1)
star = Path.unit_regular_star(5, 0.4).transformed(matplotlib.transforms.Affine2D().scale(0.5).translate(1, 3))
compound = Path.make_compound_path(outer, hole, star)
print(len(compound.vertices), np.unique(compound.codes, return_counts=True)[1].tolist())
polys = compound.to_polygons()
print(len(polys), [len(q) for q in polys])
polys_open = compound.to_polygons(closed_only=False)
print([len(q) for q in polys_open])
for pt in [(0.5, 0.5), (2, 2), (1, 3), (3.5, 3.5), (5, 5)]:
    print(pt, compound.contains_point(pt), outer.contains_point(pt), hole.contains_point(pt))
pts = np.array([(0.5, 0.5), (2, 2), (1, 3), (2, 1.5)])
print(compound.contains_points(pts).tolist(), compound.contains_points(pts, radius=0.3).tolist())
fig, ax = plt.subplots(figsize=(3, 3))
patch = ax.add_patch(PathPatch(compound, fc="C0", ec="k"))
ax.set_xlim(-0.5, 4.5)
ax.set_ylim(-0.5, 4.5)
fig.canvas.draw()
print(patch.contains_point(ax.transData.transform((2, 2))), patch.contains_point(ax.transData.transform((0.5, 0.5))))
fig.savefig("compound.png")
ext = compound.get_extents()
print(ext.bounds, compound.intersects_bbox(matplotlib.transforms.Bbox([[3, 3], [6, 6]])))
print(Path.make_compound_path().vertices.shape)
