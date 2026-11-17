# PolyCollection built by hand with a ScalarMappable array, norm and cmap; update arrays, set_verts with codes, and read colours.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection

polys = [[(i, j), (i + 0.9, j), (i + 0.45, j + 0.9)] for i in range(5) for j in range(4)]
vals = np.array([(i * 3 + j) % 7 for i in range(5) for j in range(4)], float)
fig, ax = plt.subplots(figsize=(4, 3))
pc = PolyCollection(polys, array=vals, cmap="viridis", edgecolors="k", linewidths=0.5)
ax.add_collection(pc)
ax.autoscale_view()
fig.canvas.draw()
print(pc.get_array().tolist()[:6], pc.get_clim(), np.round(pc.get_facecolor()[:2], 3).tolist())
pc.set_array(vals * 2)
pc.set_clim(0, 6)
fig.canvas.draw()
print(np.round(pc.get_facecolor()[:2], 3).tolist(), pc.norm.vmax)
pc.set_verts([np.array(p) * 0.5 for p in polys[:4]])
fig.canvas.draw()
print(len(pc.get_paths()), len(pc.get_array()), np.round(pc.get_facecolor(), 3).shape)
pc.set_verts_and_codes([[(0, 0), (1, 0), (1, 1), (0, 0)]], [[1, 2, 2, 79]])
print(pc.get_paths()[0].codes.tolist())
pc.set_array(None)
pc.set_facecolor(["r", "g"])
fig.canvas.draw()
print(np.round(pc.get_facecolor(), 3).tolist(), pc.get_array())
pc2 = PolyCollection(polys[:3], closed=False, facecolors="none")
ax.add_collection(pc2)
print([len(p.vertices) for p in pc2.get_paths()], pc2.get_paths()[0].codes)
fig.savefig("polys.png")
