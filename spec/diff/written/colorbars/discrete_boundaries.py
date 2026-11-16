# Discrete colour bar for a land-use classification with BoundaryNorm and centred class labels.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import ListedColormap, BoundaryNorm

classes = ["water", "forest", "farm", "urban"]
cmap = ListedColormap(["#4575b4", "#1a9850", "#fee08b", "#d73027"])
norm = BoundaryNorm([-0.5, 0.5, 1.5, 2.5, 3.5], cmap.N)
rng = np.random.default_rng(8)
grid = rng.integers(0, 4, (15, 20))
fig, ax = plt.subplots(figsize=(6, 4))
im = ax.imshow(grid, cmap=cmap, norm=norm)
cb = fig.colorbar(im, ax=ax, ticks=range(4))
cb.ax.set_yticklabels(classes)
cb.ax.tick_params(length=0)
ax.set_title("Land use")
print(norm.boundaries, norm.Ncmap)
fig.savefig("landuse.png")
