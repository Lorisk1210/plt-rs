# Heat map of values spanning orders of magnitude: LogNorm plus annotations in scientific notation.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm

vals = 10.0 ** np.add.outer(np.arange(4), np.arange(5) * 0.5)
fig, ax = plt.subplots(figsize=(6, 4))
im = ax.imshow(vals, norm=LogNorm(), cmap="cividis")
for (i, j), v in np.ndenumerate(vals):
    ax.text(j, i, f"{v:.1e}", ha="center", va="center", fontsize=8, color="w" if v < 30 else "k")
fig.colorbar(im, ax=ax, label="value")
ax.set_xticks(range(5))
ax.set_yticks(range(4))
print(f"{vals.max():.3e}")
fig.savefig("log_heat.png")
