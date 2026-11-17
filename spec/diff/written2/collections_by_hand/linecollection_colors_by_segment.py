# LineCollection colouring each segment of a curve by its slope, with linewidths varying, a BoundaryNorm and a colorbar.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection
from matplotlib.colors import BoundaryNorm, ListedColormap

x = np.linspace(0, 3 * np.pi, 200)
y = np.sin(x)
pts = np.c_[x, y].reshape(-1, 1, 2)
segs = np.concatenate([pts[:-1], pts[1:]], axis=1)
slope = np.diff(y) / np.diff(x)
fig, (a1, a2) = plt.subplots(2, 1, figsize=(5, 4), sharex=True)
lc = LineCollection(segs, cmap="coolwarm", norm=plt.Normalize(-1, 1), linewidths=1 + 2 * np.abs(slope))
lc.set_array(slope)
a1.add_collection(lc)
a1.autoscale_view()
fig.colorbar(lc, ax=a1)
cmap = ListedColormap(["r", "g", "b"])
norm = BoundaryNorm([-1, -0.5, 0.5, 1], cmap.N)
lc2 = LineCollection(segs, cmap=cmap, norm=norm)
lc2.set_array(y[:-1])
a2.add_collection(lc2)
a2.set_ylim(-1.1, 1.1)
fig.canvas.draw()
print(len(lc.get_segments()), np.round(lc.get_linewidths()[:3], 3).tolist(), np.round(lc.get_colors()[0], 3).tolist())
print(sorted({tuple(np.round(c, 2)) for c in lc2.get_colors()}))
print([round(v, 3) for v in a1.get_xlim()], [round(v, 3) for v in a1.get_ylim()])
lc.set_segments(segs[::2])
lc.set_linestyle("--")
fig.canvas.draw()
print(len(lc.get_segments()), lc.get_linestyle()[0][1] is not None, len(lc.get_array()))
fig.savefig("slope_colors.png")
