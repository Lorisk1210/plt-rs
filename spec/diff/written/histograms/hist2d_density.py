# 2-D histogram of correlated data with a colour bar and log colour scale.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm

rng = np.random.default_rng(14)
x = rng.normal(0, 1, 20000)
y = 0.5 * x + rng.normal(0, 0.7, x.size)
fig, ax = plt.subplots(figsize=(5.5, 4.5))
h, xe, ye, img = ax.hist2d(x, y, bins=60, cmap="magma", norm=LogNorm(), cmin=1)
fig.colorbar(img, ax=ax, label="count")
ax.set_xlabel("x")
ax.set_ylabel("y")
print(h.shape, int(np.nansum(h)))
fig.savefig("hist2d.png")
