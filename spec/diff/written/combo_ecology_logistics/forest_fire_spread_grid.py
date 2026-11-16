# Fire spread cellular automaton: snapshots as categorical images, burned-area curve, and a final-state bar of states.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import ListedColormap

rng = np.random.default_rng(214)
grid = (rng.random((40, 40)) < 0.6).astype(int)
grid[20, 20] = 2
cmap = ListedColormap(["#d2b48c", "#228b22", "#ff4500", "#333333"])
snaps, burned = [], []
for step in range(30):
    burning = grid == 2
    nb = np.zeros_like(burning)
    nb[1:] |= burning[:-1]; nb[:-1] |= burning[1:]; nb[:, 1:] |= burning[:, :-1]; nb[:, :-1] |= burning[:, 1:]
    grid = np.where(burning, 3, np.where((grid == 1) & nb, 2, grid))
    burned.append(int((grid == 3).sum()))
    if step % 10 == 0:
        snaps.append(grid.copy())
fig, axs = plt.subplots(1, 5, figsize=(16, 3.5))
for ax, s, k in zip(axs, snaps, (0, 10, 20)):
    ax.imshow(s, cmap=cmap, vmin=0, vmax=3); ax.set_title(f"step {k}"); ax.axis("off")
axs[3].plot(burned); axs[3].set_title("burned cells")
axs[4].bar(["empty", "tree", "fire", "ash"], np.bincount(grid.ravel(), minlength=4), color=cmap.colors)
fig.tight_layout()
print(burned[-1], np.bincount(grid.ravel(), minlength=4).tolist())
fig.savefig("fire.png", dpi=50)
