# Poly3DCollection of a hand-built cube with per-face colours, shade=True and a LightSource; facecolors after drawing.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.colors import LightSource
from mpl_toolkits.mplot3d.art3d import Poly3DCollection

v = np.array([[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0], [0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]], float)
faces = [[0, 1, 2, 3], [4, 5, 6, 7], [0, 1, 5, 4], [2, 3, 7, 6], [1, 2, 6, 5], [0, 3, 7, 4]]
polys = [v[f] for f in faces]
fig = plt.figure(figsize=(8, 4), dpi=60)
results = []
for k, (shade, ls) in enumerate([(False, None), (True, None), (True, LightSource(azdeg=45, altdeg=10))]):
    ax = fig.add_subplot(1, 3, k + 1, projection="3d")
    coll = Poly3DCollection(polys, facecolors=["C0", "C1", "C2", "C3", "C4", "C5"], edgecolors="k",
                            shade=shade, lightsource=ls, alpha=0.9)
    ax.add_collection3d(coll)
    ax.set(xlim=(0, 1), ylim=(0, 1), zlim=(0, 1))
    results.append((ax, coll))
fig.canvas.draw()
for ax, coll in results:
    print(np.round(coll.get_facecolor()[:3], 3).tolist())
    print(round(coll.do_3d_projection(), 4), len(coll.get_paths()))
coll = results[0][1]
coll.set_verts([p * 0.5 for p in polys])
coll.set_facecolor("r")
fig.canvas.draw()
print(np.round(coll.get_facecolor()[0], 3).tolist(), [np.round(p, 3).tolist() for p in coll._faces[:1]] if hasattr(coll, "_faces") else None)
coll.set_zsort("min")
fig.canvas.draw()
print(coll._sort_zpos, coll.get_zorder())
fig.savefig("cubes.png")
