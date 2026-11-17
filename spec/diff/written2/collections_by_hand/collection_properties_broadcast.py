# Per-element properties on a collection: lists shorter than the paths cycle, alpha arrays, hatch and linestyle lists, zorder.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.collections import PatchCollection
from matplotlib.patches import Circle

fig, ax = plt.subplots(figsize=(5, 2))
circles = [Circle((k, 0), 0.4) for k in range(6)]
pc = PatchCollection(circles, facecolors=["r", "g"], edgecolors=["k", "b", "y"], linewidths=[1, 2, 3, 4],
                     linestyles=["-", "--"], alpha=None)
ax.add_collection(pc)
ax.set_xlim(-1, 6)
ax.set_ylim(-1, 1)
fig.canvas.draw()
print(len(pc.get_facecolor()), len(pc.get_edgecolor()), list(pc.get_linewidth()), len(pc.get_linestyle()))
try:
    pc.set_alpha([0.1, 0.3, 0.5, 0.7, 0.9, 1.0])
    fig.canvas.draw()
    print(np.round(pc.get_facecolor()[:, 3], 2).tolist(), np.round(pc.get_edgecolor()[:, 3], 2).tolist()[:3])
except ValueError as e:
    print("ValueError six", str(e)[:60])
pc.set_facecolor(["r", "g", "b", "c", "m", "y"])
pc.set_edgecolor(["k"] * 6)
pc.set_alpha([0.1, 0.3, 0.5, 0.7, 0.9, 1.0])
fig.canvas.draw()
print(np.round(pc.get_facecolor()[:, 3], 2).tolist())
try:
    pc.set_alpha([0.5, 0.5])
    fig.canvas.draw()
except ValueError as e:
    print("ValueError", str(e)[:70])
pc.set_facecolor("none")
pc.set_hatch("//")
fig.canvas.draw()
print(pc.get_facecolor().tolist(), pc.get_hatch(), np.round(pc.get_alpha(), 2).tolist())
pc.set_linewidth(0)
pc.set_edgecolor("face")
print(pc.get_edgecolor().tolist(), pc._edge_default)
pc.set_capstyle("round")
pc.set_joinstyle("bevel")
print(pc.get_capstyle(), pc.get_joinstyle(), pc.get_zorder())
pc.set_zorder(0.5)
pc.set_urls(["u0", None, "u2"])
print(pc.get_zorder(), pc.get_urls())
pc.set_pickradius(7)
print(pc.get_pickradius(), pc.get_offset_transform() is ax.transData)
fig.savefig("broadcast.png")
