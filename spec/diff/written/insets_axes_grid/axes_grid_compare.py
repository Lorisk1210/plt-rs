# AxesGrid / Grid of plots with label_mode "L" to compare four spectra with shared axes.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import Grid

fig = plt.figure(figsize=(7, 5))
grid = Grid(fig, 111, nrows_ncols=(2, 2), axes_pad=0.1, label_mode="L", share_all=True)
f = np.linspace(0, 50, 300)
for k, ax in enumerate(grid):
    ax.plot(f, np.exp(-((f - 10 * (k + 1)) / 3) ** 2), color=f"C{k}")
    ax.text(0.95, 0.85, f"peak {k + 1}", transform=ax.transAxes, ha="right")
grid[2].set_xlabel("frequency")
grid[0].set_ylabel("power")
fig.savefig("grid.png")
print(len(grid))
