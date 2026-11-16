# from_levels_and_colors for a risk map with explicit categories and an extended top colour.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import from_levels_and_colors

levels = [0, 10, 20, 50, 100]
cmap, norm = from_levels_and_colors(levels, ["#2ca25f", "#fec44f", "#fe9929", "#d95f0e", "#993404"], extend="max")
y, x = np.mgrid[0:1:40j, 0:1:40j]
val = 150 * x * y
fig, ax = plt.subplots(figsize=(5, 4))
pc = ax.pcolormesh(val, cmap=cmap, norm=norm)
fig.colorbar(pc, ax=ax, extend="max")
print(cmap.N, norm.boundaries, norm.extend)
fig.savefig("risk_levels.png")
