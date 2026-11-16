# One shared colour bar for a 2x3 grid of images with a common colour scale.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import Normalize

rng = np.random.default_rng(0)
data = [rng.normal(k, 1, (12, 12)) for k in range(6)]
norm = Normalize(vmin=-2, vmax=8)
fig, axs = plt.subplots(2, 3, figsize=(8, 5))
for ax, d, k in zip(axs.flat, data, range(6)):
    im = ax.imshow(d, cmap="viridis", norm=norm)
    ax.set_title(f"day {k + 1}")
    ax.set_xticks([])
    ax.set_yticks([])
cbar = fig.colorbar(im, ax=axs.ravel().tolist(), shrink=0.9, label="anomaly")
print(cbar.norm.vmin, cbar.norm.vmax)
fig.savefig("shared.png")
