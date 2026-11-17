# Convert 2-D artists to 3-D with art3d (lines, patches, path patches, collections, text) and read back their 3-D state.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Circle, PathPatch, Rectangle
from matplotlib.text import TextPath
from mpl_toolkits.mplot3d import art3d

fig = plt.figure(figsize=(5, 5), dpi=70)
ax = fig.add_subplot(projection="3d")
ln, = ax.plot([0, 1], [0, 1])
art3d.line_2d_to_3d(ln, zs=0.5, zdir="y")
c = Circle((0.5, 0.5), 0.3, fc="C1", alpha=0.5)
ax.add_patch(c)
art3d.patch_2d_to_3d(c, z=0.2, zdir="z")
r = Rectangle((0, 0), 0.4, 0.4, fc="C2")
ax.add_patch(r)
art3d.patch_2d_to_3d(r, z=0.9, zdir="x")
tp = PathPatch(TextPath((0, 0), "3D", size=0.3), fc="k")
ax.add_patch(tp)
art3d.pathpatch_2d_to_3d(tp, z=0.1, zdir="y")
sc = ax.scatter([0.1, 0.5, 0.9], [0.1, 0.5, 0.9], zs=[0, 0.5, 1])
lc = ax.add_collection(art3d.LineCollection([[(0, 0), (1, 1)], [(0, 1), (1, 0)]]))
art3d.line_collection_2d_to_3d(lc, zs=0.3)
txt = ax.text(0.5, 0.5, 0.5, "label", zdir="x")
fig.canvas.draw()
print(type(ln).__name__, np.round(np.array(ln.get_data_3d()), 3).tolist())
print(type(c).__name__, np.round(c._segment3d[:2], 3).tolist())
print(type(r).__name__, np.round(r._segment3d[:2], 3).tolist())
print(type(tp).__name__, len(tp._segment3d))
print(type(sc).__name__, np.round(sc._offsets3d[2], 3).tolist() if hasattr(sc, "_offsets3d") else None)
print(type(lc).__name__, np.round(lc._segments3d[0], 3).tolist())
print(type(txt).__name__, txt.get_position_3d(), np.round(art3d.get_dir_vector("x"), 3).tolist())
print(np.round(art3d.juggle_axes([1], [2], [3], "x"), 3).tolist(), np.round(art3d.rotate_axes([1], [2], [3], "y"), 3).tolist())
fig.savefig("art3d.png")
