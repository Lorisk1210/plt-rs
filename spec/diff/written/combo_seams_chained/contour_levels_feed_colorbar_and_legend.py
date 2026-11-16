# Read a contour set's levels and colours to build a legend of proxy patches and to colour a matching histogram.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

y, x = np.mgrid[-2:2:100j, -2:2:100j]
z = np.exp(-(x ** 2 + y ** 2)) * 3
fig, (a, b) = plt.subplots(1, 2, figsize=(11, 4.5))
cf = a.contourf(x, y, z, levels=5, cmap="viridis")
levels = cf.levels
colors = cf.cmap(cf.norm((levels[:-1] + levels[1:]) / 2))
a.legend(handles=[Patch(color=c, label=f"{lo:.1f}–{hi:.1f}") for c, lo, hi in zip(colors, levels[:-1], levels[1:])], fontsize=7)
counts, _ = np.histogram(z, bins=levels)
b.bar(range(len(counts)), counts, color=colors)
b.set_xticks(range(len(counts)), [f"{lo:.1f}" for lo in levels[:-1]])
print(np.round(levels, 3).tolist(), counts.tolist(), np.round(colors[0], 4).tolist())
fig.savefig("levels.png", dpi=50)
